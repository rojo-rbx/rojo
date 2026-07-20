use std::fs;

use insta::{assert_snapshot, assert_yaml_snapshot, with_settings};
use rbx_dom_weak::types::Ref;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use tempfile::tempdir;
use uuid::Uuid;

use crate::rojo_test::{
    internable::InternAndRedact,
    serve_util::{deserialize_msgpack, run_serve_test, serialize_to_xml_model, TestServeSession},
};

use librojo::{
    exec::{MAX_PENDING_JOBS, MAX_SOURCE_SIZE_BYTES},
    web_api::{
        ExecJobClaimResponse, ExecJobCompletionRequest, ExecJobResponse,
        ExecJobState as ApiExecJobState, ExecJobSubmissionRequest, ExecLog, ExecLogLevel,
        ExecValue, SerializeResponse, SocketPacketType,
    },
};

#[test]
fn exec_jobs_submit_claim_complete_and_release_slot() {
    run_serve_test("empty", |session, _redactions| {
        let submission_response = session.post_api_exec_job(&ExecJobSubmissionRequest {
            script_name: "first.lua".to_owned(),
            source: "return true".to_owned(),
        });
        assert_eq!(submission_response.status(), StatusCode::CREATED);
        let submission_body = submission_response.bytes().unwrap();
        let submission_value: serde_json::Value = deserialize_msgpack(&submission_body).unwrap();
        assert!(submission_value.get("source").is_none());
        let first: ExecJobResponse = deserialize_msgpack(&submission_body).unwrap();
        let second = submit_exec_job(&session, "second.lua", "return 'second'");

        assert_eq!(first.script_name, "first.lua");
        assert_eq!(first.state, ApiExecJobState::Pending);
        assert_eq!(second.state, ApiExecJobState::Pending);

        let status_response = session.get_api_exec_status(&first.job_id);
        assert_eq!(status_response.status(), StatusCode::OK);
        let status_body = status_response.bytes().unwrap();
        let status_value: serde_json::Value = deserialize_msgpack(&status_body).unwrap();
        assert_eq!(status_value["state"], "pending");
        assert!(status_value.get("source").is_none());

        let claim_response = session.get_api_exec_next();
        assert_eq!(claim_response.status(), StatusCode::OK);
        let claim: ExecJobClaimResponse = decode_response(claim_response);
        assert_eq!(claim.job_id, first.job_id);
        assert_eq!(claim.source, "return true");
        assert_eq!(claim.state, ApiExecJobState::Claimed);

        let claimed_status: ExecJobResponse =
            decode_response(session.get_api_exec_status(&first.job_id));
        assert_eq!(claimed_status.state, ApiExecJobState::Claimed);

        let blocked = session.get_api_exec_next();
        assert_eq!(blocked.status(), StatusCode::NO_CONTENT);
        assert!(blocked.bytes().unwrap().is_empty());

        let expected_logs = vec![ExecLog {
            level: ExecLogLevel::Print,
            message: "finished".to_owned(),
        }];
        let completion = ExecJobCompletionRequest::Success {
            result: Some(ExecValue::Boolean { value: true }),
            logs: expected_logs.clone(),
        };
        let completion_response = session.post_api_exec_complete(&first.job_id, &completion);
        assert_eq!(completion_response.status(), StatusCode::OK);
        let completed: ExecJobResponse = decode_response(completion_response);
        assert_eq!(completed.state, ApiExecJobState::Succeeded);
        assert_eq!(completed.result, Some(ExecValue::Boolean { value: true }));
        assert_eq!(completed.logs, Some(expected_logs));

        let succeeded_status: ExecJobResponse =
            decode_response(session.get_api_exec_status(&first.job_id));
        assert_eq!(succeeded_status.state, ApiExecJobState::Succeeded);

        let second_claim: ExecJobClaimResponse = decode_response(session.get_api_exec_next());
        assert_eq!(second_claim.job_id, second.job_id);
        assert_eq!(second_claim.source, "return 'second'");

        let second_completion = ExecJobCompletionRequest::Success {
            result: Some(ExecValue::Nil),
            logs: Vec::new(),
        };
        assert_eq!(
            session
                .post_api_exec_complete(&second.job_id, &second_completion)
                .status(),
            StatusCode::OK,
        );

        let empty = session.get_api_exec_next();
        assert_eq!(empty.status(), StatusCode::NO_CONTENT);
        assert!(empty.bytes().unwrap().is_empty());
    });
}

#[test]
fn exec_jobs_record_failure_and_timeout_outcomes() {
    run_serve_test("empty", |session, _redactions| {
        let compile_job = submit_exec_job(&session, "compile.lua", "not valid luau");
        assert_eq!(
            decode_response::<ExecJobClaimResponse>(session.get_api_exec_next()).job_id,
            compile_job.job_id,
        );
        let compile_failure = ExecJobCompletionRequest::CompileFailure {
            error: "expected identifier".to_owned(),
            traceback: None,
            logs: Vec::new(),
        };
        let compile_response =
            session.post_api_exec_complete(&compile_job.job_id, &compile_failure);
        assert_eq!(compile_response.status(), StatusCode::OK);
        let compile_status: ExecJobResponse = decode_response(compile_response);
        assert_eq!(compile_status.state, ApiExecJobState::Failed);
        assert_eq!(compile_status.error.as_deref(), Some("expected identifier"));
        assert_eq!(compile_status.traceback, None);

        assert_eq!(
            session
                .post_api_exec_complete(&compile_job.job_id, &compile_failure)
                .status(),
            StatusCode::CONFLICT,
        );

        let runtime_job = submit_exec_job(&session, "runtime.lua", "error('boom')");
        decode_response::<ExecJobClaimResponse>(session.get_api_exec_next());
        let expected_logs = vec![ExecLog {
            level: ExecLogLevel::Warn,
            message: "about to fail".to_owned(),
        }];
        let runtime_failure = ExecJobCompletionRequest::RuntimeFailure {
            error: "boom".to_owned(),
            traceback: Some("runtime.lua:1".to_owned()),
            logs: expected_logs.clone(),
        };
        assert_eq!(
            session
                .post_api_exec_complete(&runtime_job.job_id, &runtime_failure)
                .status(),
            StatusCode::OK,
        );
        let runtime_status: ExecJobResponse =
            decode_response(session.get_api_exec_status(&runtime_job.job_id));
        assert_eq!(runtime_status.state, ApiExecJobState::Failed);
        assert_eq!(runtime_status.error.as_deref(), Some("boom"));
        assert_eq!(runtime_status.traceback.as_deref(), Some("runtime.lua:1"));
        assert_eq!(runtime_status.logs, Some(expected_logs));

        let rejected_job = submit_exec_job(&session, "rejected.lua", "return nil");
        decode_response::<ExecJobClaimResponse>(session.get_api_exec_next());
        let rejected = ExecJobCompletionRequest::Rejected {
            error: "Rojo exec is only available in edit mode".to_owned(),
            traceback: None,
            logs: Vec::new(),
        };
        let rejected_status: ExecJobResponse =
            decode_response(session.post_api_exec_complete(&rejected_job.job_id, &rejected));
        assert_eq!(rejected_status.state, ApiExecJobState::Failed);

        let timeout_job = submit_exec_job(&session, "timeout.lua", "task.wait(60)");
        decode_response::<ExecJobClaimResponse>(session.get_api_exec_next());
        let timeout = ExecJobCompletionRequest::Timeout {
            error: "execution timed out".to_owned(),
            traceback: Some("timeout.lua:1".to_owned()),
            logs: Vec::new(),
        };
        let timeout_status: ExecJobResponse =
            decode_response(session.post_api_exec_complete(&timeout_job.job_id, &timeout));
        assert_eq!(timeout_status.state, ApiExecJobState::TimedOut);
        assert_eq!(timeout_status.error.as_deref(), Some("execution timed out"));

        let retained_timeout: ExecJobResponse =
            decode_response(session.get_api_exec_status(&timeout_job.job_id));
        assert_eq!(retained_timeout.state, ApiExecJobState::TimedOut);
    });
}

#[test]
fn exec_jobs_validate_requests_routes_and_limits() {
    run_serve_test("empty", |session, _redactions| {
        let pending = submit_exec_job(&session, "pending.lua", "return nil");
        let success = ExecJobCompletionRequest::Success {
            result: None,
            logs: Vec::new(),
        };
        assert_eq!(
            session
                .post_api_exec_complete(&pending.job_id, &success)
                .status(),
            StatusCode::CONFLICT,
        );

        assert_eq!(
            session.get_api_exec_status("not-a-uuid").status(),
            StatusCode::BAD_REQUEST,
        );
        let unknown_id = Uuid::new_v4().to_string();
        assert_eq!(
            session.get_api_exec_status(&unknown_id).status(),
            StatusCode::NOT_FOUND,
        );
        assert_eq!(
            session
                .post_api_exec_complete(&unknown_id, &success)
                .status(),
            StatusCode::NOT_FOUND,
        );

        decode_response::<ExecJobClaimResponse>(session.get_api_exec_next());
        let malformed_path = format!("/api/exec/jobs/{}/complete", pending.job_id);
        assert_eq!(
            session
                .api_request(reqwest::Method::POST, &malformed_path, Some(vec![0xc1]))
                .status(),
            StatusCode::BAD_REQUEST,
        );

        let oversized_completion = ExecJobCompletionRequest::RuntimeFailure {
            error: "x".repeat(300 * 1024),
            traceback: None,
            logs: Vec::new(),
        };
        assert_eq!(
            session
                .post_api_exec_complete(&pending.job_id, &oversized_completion)
                .status(),
            StatusCode::PAYLOAD_TOO_LARGE,
        );
        assert_eq!(
            session
                .post_api_exec_complete(&pending.job_id, &success)
                .status(),
            StatusCode::OK,
        );

        for (method, path) in [
            (reqwest::Method::GET, "/api/exec/jobs/"),
            (reqwest::Method::GET, "/api/exec/jobs/next/extra"),
            (
                reqwest::Method::POST,
                "/api/exec/jobs/not-a-uuid/complete/extra",
            ),
        ] {
            assert_eq!(
                session.api_request(method, path, None).status(),
                StatusCode::NOT_FOUND,
            );
        }
        assert_eq!(
            session
                .api_request(reqwest::Method::POST, "/api/exec/jobs/next", None)
                .status(),
            StatusCode::NOT_FOUND,
        );

        assert_eq!(
            session
                .post_api_exec_job(&ExecJobSubmissionRequest {
                    script_name: String::new(),
                    source: "return nil".to_owned(),
                })
                .status(),
            StatusCode::BAD_REQUEST,
        );
        assert_eq!(
            session
                .post_api_exec_job(&ExecJobSubmissionRequest {
                    script_name: "bad\nname.lua".to_owned(),
                    source: "return nil".to_owned(),
                })
                .status(),
            StatusCode::BAD_REQUEST,
        );
        assert_eq!(
            session
                .api_request(reqwest::Method::POST, "/api/exec/jobs", Some(vec![0xc1]))
                .status(),
            StatusCode::BAD_REQUEST,
        );
        assert_eq!(
            session
                .post_api_exec_job(&ExecJobSubmissionRequest {
                    script_name: "large.lua".to_owned(),
                    source: "x".repeat(MAX_SOURCE_SIZE_BYTES + 1),
                })
                .status(),
            StatusCode::PAYLOAD_TOO_LARGE,
        );

        for index in 0..MAX_PENDING_JOBS {
            assert_eq!(
                session
                    .post_api_exec_job(&ExecJobSubmissionRequest {
                        script_name: format!("queue-{index}.lua"),
                        source: "return nil".to_owned(),
                    })
                    .status(),
                StatusCode::CREATED,
            );
        }
        assert_eq!(
            session
                .post_api_exec_job(&ExecJobSubmissionRequest {
                    script_name: "queue-overflow.lua".to_owned(),
                    source: "return nil".to_owned(),
                })
                .status(),
            StatusCode::TOO_MANY_REQUESTS,
        );
    });
}

fn submit_exec_job(session: &TestServeSession, script_name: &str, source: &str) -> ExecJobResponse {
    let response = session.post_api_exec_job(&ExecJobSubmissionRequest {
        script_name: script_name.to_owned(),
        source: source.to_owned(),
    });
    assert_eq!(response.status(), StatusCode::CREATED);
    decode_response(response)
}

fn decode_response<T: DeserializeOwned>(response: reqwest::blocking::Response) -> T {
    let body = response.bytes().expect("Failed to read response body");
    deserialize_msgpack(&body).expect("Server returned malformed response")
}

#[test]
fn rejects_dns_rebinding_requests() {
    run_serve_test("empty", |session, _redactions| {
        let port = session.port();
        let local_host = format!("localhost:{port}");

        // A request carrying a local Host header is served normally.
        assert_eq!(
            session
                .api_rojo_response_with_headers(&[("host", &local_host)])
                .status(),
            reqwest::StatusCode::OK,
        );

        // A request whose Host is a foreign hostname, as a DNS-rebound page
        // would send, is rejected with a generic 404 that reveals nothing about
        // the server.
        assert_rejected(session.api_rojo_response_with_headers(&[("host", "evil.com")]));

        // Even with a local Host, a present-but-foreign Origin is rejected.
        let foreign_origin = format!("http://evil.com:{port}");
        assert_rejected(
            session.api_rojo_response_with_headers(&[
                ("host", &local_host),
                ("origin", &foreign_origin),
            ]),
        );
    });
}

/// Asserts that a Host/Origin rejection is a generic 404 whose body and
/// content-type do not identify the server as Rojo.
fn assert_rejected(response: reqwest::blocking::Response) {
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(
        !content_type.contains("msgpack"),
        "rejection should not use the msgpack API content-type, got {content_type:?}",
    );

    let body = response.text().expect("Failed to read response body");
    let body_lower = body.to_lowercase();
    assert!(
        !body_lower.contains("rojo") && !body_lower.contains("rebinding"),
        "rejection body should not identify the server, got {body:?}",
    );
}

#[test]
fn allows_api_open_from_loopback_peer() {
    run_serve_test("empty", |session, _redactions| {
        // The harness always connects over loopback, so the local-only gate on
        // /api/open must let the request through. A bogus instance id then fails
        // id parsing with 400, which confirms we got past the gate rather than
        // being rejected with 403.
        assert_eq!(
            session.api_open_status("not-a-real-ref"),
            reqwest::StatusCode::BAD_REQUEST,
        );
    });
}

#[test]
fn empty() {
    run_serve_test("empty", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("empty_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "empty_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn scripts() {
    run_serve_test("scripts", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("scripts_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        with_settings!({ sort_maps => true }, {
            assert_yaml_snapshot!(
                "scripts_all",
                read_response.intern_and_redact(&mut redactions, root_id)
            );
        });

        fs::write(session.path().join("src/foo.lua"), "Updated foo!").unwrap();

        let socket_packet = session
            .get_api_socket_packet(SocketPacketType::Messages, 0)
            .unwrap();
        assert_yaml_snapshot!(
            "scripts_subscribe",
            socket_packet.intern_and_redact(&mut redactions, ())
        );

        let read_response = session.get_api_read(root_id).unwrap();
        with_settings!({ sort_maps => true }, {
            assert_yaml_snapshot!(
                "scripts_all-2",
                read_response.intern_and_redact(&mut redactions, root_id)
            );
        });
    });
}

#[test]
fn add_folder() {
    run_serve_test("add_folder", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("add_folder_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "add_folder_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        fs::create_dir(session.path().join("src/my-new-folder")).unwrap();

        let socket_packet = session
            .get_api_socket_packet(SocketPacketType::Messages, 0)
            .unwrap();
        assert_yaml_snapshot!(
            "add_folder_subscribe",
            socket_packet.intern_and_redact(&mut redactions, ())
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "add_folder_all-2",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn remove_file() {
    run_serve_test("remove_file", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("remove_file_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "remove_file_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        fs::remove_file(session.path().join("src/hello.txt")).unwrap();

        let socket_packet = session
            .get_api_socket_packet(SocketPacketType::Messages, 0)
            .unwrap();
        assert_yaml_snapshot!(
            "remove_file_subscribe",
            socket_packet.intern_and_redact(&mut redactions, ())
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "remove_file_all-2",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn edit_init() {
    run_serve_test("edit_init", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("edit_init_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "edit_init_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        fs::write(session.path().join("src/init.lua"), b"-- Edited contents").unwrap();

        let socket_packet = session
            .get_api_socket_packet(SocketPacketType::Messages, 0)
            .unwrap();
        assert_yaml_snapshot!(
            "edit_init_subscribe",
            socket_packet.intern_and_redact(&mut redactions, ())
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "edit_init_all-2",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn move_folder_of_stuff() {
    run_serve_test("move_folder_of_stuff", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("move_folder_of_stuff_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "move_folder_of_stuff_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        // Create a directory full of stuff we can move in
        let src_dir = tempdir().unwrap();
        let stuff_path = src_dir.path().join("new-stuff");

        fs::create_dir(&stuff_path).unwrap();

        // Make a bunch of random files in our stuff folder
        for i in 0..10 {
            let file_name = stuff_path.join(format!("{}.txt", i));
            let file_contents = format!("File #{}", i);

            fs::write(file_name, file_contents).unwrap();
        }

        // We're hoping that this rename gets picked up as one event. This test
        // will fail otherwise.
        fs::rename(stuff_path, session.path().join("src/new-stuff")).unwrap();

        let socket_packet = session
            .get_api_socket_packet(SocketPacketType::Messages, 0)
            .unwrap();
        assert_yaml_snapshot!(
            "move_folder_of_stuff_subscribe",
            socket_packet.intern_and_redact(&mut redactions, ())
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "move_folder_of_stuff_all-2",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn empty_json_model() {
    run_serve_test("empty_json_model", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("empty_json_model_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "empty_json_model_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        fs::write(
            session.path().join("src/test.model.json"),
            r#"{"ClassName": "Model"}"#,
        )
        .unwrap();

        let socket_packet = session
            .get_api_socket_packet(SocketPacketType::Messages, 0)
            .unwrap();
        assert_yaml_snapshot!(
            "empty_json_model_subscribe",
            socket_packet.intern_and_redact(&mut redactions, ())
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "empty_json_model_all-2",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
#[ignore = "Rojo does not watch missing, optional files for changes."]
fn add_optional_folder() {
    run_serve_test("add_optional_folder", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("add_optional_folder", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "add_optional_folder_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        fs::create_dir(session.path().join("create-later")).unwrap();

        let socket_packet = session
            .get_api_socket_packet(SocketPacketType::Messages, 0)
            .unwrap();
        assert_yaml_snapshot!(
            "add_optional_folder_subscribe",
            socket_packet.intern_and_redact(&mut redactions, ())
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "add_optional_folder_all-2",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn sync_rule_alone() {
    run_serve_test("sync_rule_alone", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("sync_rule_alone_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "sync_rule_alone_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn sync_rule_complex() {
    run_serve_test("sync_rule_complex", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("sync_rule_complex_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "sync_rule_complex_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn sync_rule_no_extension() {
    run_serve_test("sync_rule_no_extension", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!(
            "sync_rule_no_extension_info",
            redactions.redacted_yaml(info)
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "sync_rule_no_extension_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn no_name_default_project() {
    run_serve_test("no_name_default_project", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!(
            "no_name_default_project_info",
            redactions.redacted_yaml(info)
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "no_name_default_project_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn no_name_project() {
    run_serve_test("no_name_project", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("no_name_project_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "no_name_project_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn no_name_top_level_project() {
    run_serve_test("no_name_top_level_project", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!(
            "no_name_top_level_project_info",
            redactions.redacted_yaml(info)
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "no_name_top_level_project_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        let project_path = session.path().join("default.project.json");
        let mut project_contents = fs::read_to_string(&project_path).unwrap();
        project_contents.push('\n');
        fs::write(&project_path, project_contents).unwrap();

        // The cursor shouldn't be changing so this snapshot is fine for testing
        // the response.
        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "no_name_top_level_project_all-2",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn sync_rule_no_name_project() {
    run_serve_test("sync_rule_no_name_project", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!(
            "sync_rule_no_name_project_info",
            redactions.redacted_yaml(info)
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "sync_rule_no_name_project_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn ref_properties() {
    run_serve_test("ref_properties", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("ref_properties_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "ref_properties_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        fs::write(
            session.path().join("ModelTarget.model.json"),
            r#"{
                "className": "Folder",
                "attributes": {
                    "Rojo_Id": "model target 2"
                },
                "children": [
                  {
                    "name": "ModelPointer",
                    "className": "Model",
                    "attributes": {
                      "Rojo_Target_PrimaryPart": "model target 2"
                    }
                  },
                  {
                    "name": "ProjectPointer",
                    "className": "Model",
                    "attributes": {
                      "Rojo_Target_PrimaryPart": "project target"
                    }
                  }
                ]
              }"#,
        )
        .unwrap();

        let socket_packet = session
            .get_api_socket_packet(SocketPacketType::Messages, 0)
            .unwrap();
        assert_yaml_snapshot!(
            "ref_properties_subscribe",
            socket_packet.intern_and_redact(&mut redactions, ())
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "ref_properties_all-2",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn ref_properties_remove() {
    run_serve_test("ref_properties_remove", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("ref_properties_remove_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "ref_properties_remove_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        fs::remove_file(session.path().join("src/target.model.json")).unwrap();

        let socket_packet = session
            .get_api_socket_packet(SocketPacketType::Messages, 0)
            .unwrap();
        assert_yaml_snapshot!(
            "ref_properties_remove_subscribe",
            socket_packet.intern_and_redact(&mut redactions, ())
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "ref_properties_remove_all-2",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

/// When Ref properties were first implemented, a mistake was made that resulted
/// in Ref properties defined via attributes not being included in patch
/// computation, which resulted in subsequent patches setting those properties
/// to `nil`.
///
/// See: https://github.com/rojo-rbx/rojo/issues/929
#[test]
fn ref_properties_patch_update() {
    // Reusing ref_properties is fun and easy.
    run_serve_test("ref_properties", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!(
            "ref_properties_patch_update_info",
            redactions.redacted_yaml(info)
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "ref_properties_patch_update_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        let target_path = session.path().join("ModelTarget.model.json");

        // Inserting scale just to force the change processor to run
        fs::write(
            target_path,
            r#"{
            "id": "model target",
            "className": "Folder",
            "children": [
                {
                    "name": "ModelPointer",
                    "className": "Model",
                    "attributes": {
                        "Rojo_Target_PrimaryPart": "model target"
                    },
                    "properties": {
                        "Scale": 1
                    }
                }
            ]
        }"#,
        )
        .unwrap();

        let socket_packet = session
            .get_api_socket_packet(SocketPacketType::Messages, 0)
            .unwrap();
        assert_yaml_snapshot!(
            "ref_properties_patch_update_subscribe",
            socket_packet.intern_and_redact(&mut redactions, ())
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "ref_properties_patch_update_all-2",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn model_pivot_migration() {
    run_serve_test("pivot_migration", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("pivot_migration_info", redactions.redacted_yaml(info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "pivot_migration_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        let project_path = session.path().join("default.project.json");

        fs::write(
            project_path,
            r#"{
            "name": "pivot_migration",
            "tree": {
                "$className": "DataModel",
                "Workspace": {
                    "Model": {
                        "$className": "Model"
                    },
                    "Tool": {
                        "$path": "Tool.model.json"
                    },
                    "Actor": {
                        "$className": "Actor"
                    }
                }
            }
        }"#,
        )
        .unwrap();

        let socket_packet = session
            .get_api_socket_packet(SocketPacketType::Messages, 0)
            .unwrap();
        assert_yaml_snapshot!(
            "model_pivot_migration_all",
            socket_packet.intern_and_redact(&mut redactions, ())
        );

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "model_pivot_migration_all-2",
            read_response.intern_and_redact(&mut redactions, root_id)
        );
    });
}

#[test]
fn meshpart_with_id() {
    run_serve_test("meshpart_with_id", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("meshpart_with_id_info", redactions.redacted_yaml(&info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "meshpart_with_id_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        // This is a bit awkward, but it's fine.
        let (meshpart, _) = read_response
            .instances
            .iter()
            .find(|(_, inst)| inst.class_name == "MeshPart")
            .unwrap();
        let (objectvalue, _) = read_response
            .instances
            .iter()
            .find(|(_, inst)| inst.class_name == "ObjectValue")
            .unwrap();

        let body = session
            .post_api_serialize(&[*meshpart, *objectvalue], info.session_id)
            .unwrap()
            .bytes()
            .unwrap();
        let serialize_response: SerializeResponse =
            deserialize_msgpack(&body).expect("Server returned malformed response");

        // We don't assert a snapshot on the SerializeResponse because the model includes the
        // Refs from the DOM as names, which means it will obviously be different every time
        // this code runs. Still, we ensure that the SessionId is right at least.
        assert_eq!(serialize_response.session_id, info.session_id);

        let model = serialize_to_xml_model(&serialize_response, &redactions);
        assert_snapshot!("meshpart_with_id_serialize_model", model);
    });
}

#[test]
fn serialize_missing_id() {
    run_serve_test("empty", |session, _| {
        let info = session.get_api_rojo().unwrap();
        let missing_id = Ref::new();

        let response = session
            .post_api_serialize(&[missing_id], info.session_id)
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    });
}

#[test]
fn forced_parent() {
    run_serve_test("forced_parent", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();
        let root_id = info.root_instance_id;

        assert_yaml_snapshot!("forced_parent_info", redactions.redacted_yaml(&info));

        let read_response = session.get_api_read(root_id).unwrap();
        assert_yaml_snapshot!(
            "forced_parent_all",
            read_response.intern_and_redact(&mut redactions, root_id)
        );

        let body = session
            .post_api_serialize(&[root_id], info.session_id)
            .unwrap()
            .bytes()
            .unwrap();
        let serialize_response: SerializeResponse =
            deserialize_msgpack(&body).expect("Server returned malformed response");

        assert_eq!(serialize_response.session_id, info.session_id);

        let model = serialize_to_xml_model(&serialize_response, &redactions);
        assert_snapshot!("forced_parent_serialize_model", model);
    });
}
