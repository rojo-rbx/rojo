use std::{
    io::{self, Write},
    net::IpAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{anyhow, bail, Context};
use clap::Parser;
use reqwest::{
    blocking::Client,
    header::{ACCEPT, CONTENT_TYPE},
    StatusCode,
};
use uuid::Uuid;

#[cfg(test)]
use serde::de::DeserializeOwned;

use crate::{
    exec::MAX_SOURCE_SIZE_BYTES,
    web::{
        interface::{
            ExecJobResponse, ExecJobState, ExecJobSubmissionRequest, ExecLog, ExecLogLevel,
            ExecValue,
        },
        serialize_msgpack,
    },
};

#[cfg(test)]
use crate::web::interface::ErrorResponse;

use super::{
    automation::{
        build_client, decode_response, poll_status, send_request, server_url, verify_rojo_server,
        PollOptions, DEFAULT_ADDRESS, DEFAULT_PORT, MSGPACK_CONTENT_TYPE,
    },
    resolve_path,
};

#[cfg(test)]
use super::automation::{decode_buffered_response, BufferedResponse};

const POLL_INTERVAL: Duration = Duration::from_millis(250);
const LOCAL_TIMEOUT: Duration = Duration::from_secs(70);

/// Runs a trusted Luau file through a connected Prism Studio plugin.
#[derive(Debug, Parser)]
pub struct ExecCommand {
    /// Path to the Luau file to execute.
    pub file: PathBuf,

    /// IP address of the running Rojo server.
    #[clap(long, default_value = DEFAULT_ADDRESS)]
    pub address: IpAddr,

    /// Port of the running Rojo server.
    #[clap(long, default_value_t = DEFAULT_PORT)]
    pub port: u16,
}

impl ExecCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let script = read_exec_script(&self.file)?;
        let server_url = server_url(self.address, self.port);
        let client = build_client("Prism exec")?;

        verify_rojo_server(&client, &server_url, "Prism exec client")?;
        let submitted = submit_job(&client, &server_url, &script)?;
        let job_id = parse_response_job_id(&submitted.job_id, "submission")?;
        log::debug!("Submitted exec job {}", job_id);

        let completed = poll_job(&client, &server_url, job_id, LOCAL_TIMEOUT)?;
        let stdout = io::stdout();
        let stderr = io::stderr();
        finish_job(&completed, &mut stdout.lock(), &mut stderr.lock())
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ExecScript {
    script_name: String,
    source: String,
}

fn read_exec_script(path: &Path) -> anyhow::Result<ExecScript> {
    let path = resolve_path(path)?;
    let metadata = std::fs::metadata(path.as_ref()).with_context(|| {
        format!(
            "Could not inspect exec script '{}'. Ensure the file exists and is readable.",
            path.display()
        )
    })?;

    if !metadata.is_file() {
        bail!("Exec script '{}' is not a regular file.", path.display());
    }

    if metadata.len() > (MAX_SOURCE_SIZE_BYTES + '\u{FEFF}'.len_utf8()) as u64 {
        bail!(
            "Exec script '{}' is {} bytes, exceeding the {}-byte source limit.",
            path.display(),
            metadata.len(),
            MAX_SOURCE_SIZE_BYTES
        );
    }

    let script_name = script_name_from_path(path.as_ref())?;
    let source = std::fs::read(path.as_ref())
        .with_context(|| format!("Could not read exec script '{}'.", path.display()))?;

    let mut source = String::from_utf8(source)
        .with_context(|| format!("Exec script '{}' is not valid UTF-8.", path.display()))?;
    normalize_source(&mut source);

    if source.len() > MAX_SOURCE_SIZE_BYTES {
        bail!(
            "Exec script '{}' is {} bytes, exceeding the {}-byte source limit.",
            path.display(),
            source.len(),
            MAX_SOURCE_SIZE_BYTES
        );
    }

    Ok(ExecScript {
        script_name,
        source,
    })
}

fn normalize_source(source: &mut String) {
    if source.starts_with('\u{FEFF}') {
        source.replace_range(..'\u{FEFF}'.len_utf8(), "");
    }
}

fn script_name_from_path(path: &Path) -> anyhow::Result<String> {
    let file_name = path
        .file_name()
        .context("Exec script path has no file name.")?;
    let file_name = file_name
        .to_str()
        .context("Exec script file name is not valid UTF-8.")?;

    if file_name.is_empty() {
        bail!("Exec script file name is empty.");
    }

    Ok(file_name.to_owned())
}

fn submit_job(
    client: &Client,
    server_url: &str,
    script: &ExecScript,
) -> anyhow::Result<ExecJobResponse> {
    let body = serialize_msgpack(ExecJobSubmissionRequest {
        script_name: script.script_name.clone(),
        source: script.source.clone(),
    })
    .context("Could not encode the exec job submission as MessagePack")?;

    let response = send_request(
        client
            .post(format!("{server_url}/api/exec/jobs"))
            .header(ACCEPT, MSGPACK_CONTENT_TYPE)
            .header(CONTENT_TYPE, MSGPACK_CONTENT_TYPE)
            .body(body),
        server_url,
        "submitting the exec job",
    )?;
    let job: ExecJobResponse = decode_response(
        response,
        StatusCode::CREATED,
        server_url,
        "submitting the exec job",
        exec_http_summary,
    )?;

    if job.state != ExecJobState::Pending {
        bail!(
            "The Rojo server at {server_url} returned unexpected state {:?} for a newly submitted exec job.",
            job.state
        );
    }

    Ok(job)
}

fn poll_job(
    client: &Client,
    server_url: &str,
    job_id: Uuid,
    timeout: Duration,
) -> anyhow::Result<ExecJobResponse> {
    let path = format!("/api/exec/jobs/{job_id}");
    poll_status(
        client,
        PollOptions {
            server_url,
            path: &path,
            timeout,
            interval: POLL_INTERVAL,
            operation: "polling the exec job",
        },
        |response| {
            decode_response(
                response,
                StatusCode::OK,
                server_url,
                "polling the exec job",
                exec_http_summary,
            )
        },
        |job: &ExecJobResponse| {
            let response_job_id = parse_response_job_id(&job.job_id, "status")?;
            if response_job_id != job_id {
                bail!(
                    "The Rojo server at {server_url} returned status for exec job {response_job_id} while job {job_id} was requested."
                );
            }
            Ok(matches!(
                job.state,
                ExecJobState::Succeeded | ExecJobState::Failed | ExecJobState::TimedOut
            ))
        },
        || local_timeout_error(job_id, timeout),
    )
}

fn parse_response_job_id(job_id: &str, response_kind: &str) -> anyhow::Result<Uuid> {
    Uuid::parse_str(job_id).with_context(|| {
        format!("The Rojo server returned a malformed job ID in its {response_kind} response")
    })
}

fn local_timeout_error(job_id: Uuid, timeout: Duration) -> anyhow::Error {
    anyhow!(
        "Timed out after {} seconds waiting for exec job {job_id}. The server may still retain the job briefly.",
        timeout.as_secs()
    )
}

#[cfg(test)]
fn decode_buffered_exec_response<T: DeserializeOwned>(
    response: &BufferedResponse,
    expected_status: StatusCode,
    server_url: &str,
    operation: &str,
) -> anyhow::Result<T> {
    decode_buffered_response(
        response,
        expected_status,
        server_url,
        operation,
        exec_http_summary,
    )
}

fn exec_http_summary(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "Rojo server rejected the request as malformed",
        StatusCode::FORBIDDEN => "Prism exec API is not available from this peer",
        StatusCode::NOT_FOUND => "Prism exec job disappeared or expired",
        StatusCode::CONFLICT => "Prism exec job has an unexpected state conflict",
        StatusCode::PAYLOAD_TOO_LARGE => "Prism exec source or output is too large",
        StatusCode::TOO_MANY_REQUESTS => "Prism exec queue is full",
        StatusCode::INTERNAL_SERVER_ERROR => "Rojo server reported an internal error",
        _ => "Rojo server returned an unexpected HTTP status",
    }
}

fn finish_job(
    job: &ExecJobResponse,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> anyhow::Result<()> {
    replay_logs(job.logs.as_deref().unwrap_or_default(), stdout, stderr)?;

    match job.state {
        ExecJobState::Succeeded => {
            if let Some(result) = job
                .result
                .as_ref()
                .map(render_result)
                .transpose()?
                .flatten()
            {
                writeln!(stdout, "{result}")
                    .context("Could not write the exec result to stdout")?;
            }
            Ok(())
        }
        ExecJobState::Failed => Err(anyhow!(terminal_failure_message(job, false))),
        ExecJobState::TimedOut => Err(anyhow!(terminal_failure_message(job, true))),
        ExecJobState::Pending | ExecJobState::Claimed => bail!(
            "The Rojo server returned non-terminal state {:?} after exec polling finished.",
            job.state
        ),
    }
}

fn replay_logs(
    logs: &[ExecLog],
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> anyhow::Result<()> {
    for log in logs {
        match log.level {
            ExecLogLevel::Print => {
                writeln!(stdout, "{}", log.message)
                    .context("Could not write an exec print log to stdout")?;
            }
            ExecLogLevel::Warn => {
                writeln!(stderr, "{}", log.message)
                    .context("Could not write an exec warning log to stderr")?;
            }
        }
    }

    Ok(())
}

fn terminal_failure_message(job: &ExecJobResponse, timed_out: bool) -> String {
    let fallback = if timed_out {
        "Execution timed out without an error message"
    } else {
        "Execution failed without an error message"
    };
    let error = job
        .error
        .as_deref()
        .filter(|error| !error.is_empty())
        .unwrap_or(fallback);
    let mut message = if timed_out {
        format!("Prism exec timed out: {error}")
    } else {
        format!("Prism exec failed: {error}")
    };

    if let Some(traceback) = job
        .traceback
        .as_deref()
        .filter(|traceback| !traceback.is_empty())
    {
        message.push('\n');
        message.push_str(traceback);
    }

    message
}

fn render_result(value: &ExecValue) -> anyhow::Result<Option<String>> {
    match value {
        ExecValue::Nil => Ok(None),
        ExecValue::String { value } => Ok(Some(value.clone())),
        ExecValue::Number { value } => Ok(Some(render_number(*value)?)),
        ExecValue::Boolean { value } => Ok(Some(value.to_string())),
        ExecValue::Array { .. } | ExecValue::Table { .. } => Ok(Some(render_json_like(value, 0)?)),
    }
}

fn render_json_like(value: &ExecValue, indent: usize) -> anyhow::Result<String> {
    match value {
        ExecValue::Nil => Ok("null".to_owned()),
        ExecValue::String { value } => {
            serde_json::to_string(value).context("Could not quote a string in the exec result")
        }
        ExecValue::Number { value } => render_number(*value),
        ExecValue::Boolean { value } => Ok(value.to_string()),
        ExecValue::Array { value } => {
            if value.is_empty() {
                return Ok("[]".to_owned());
            }

            let child_indent = indent + 2;
            let padding = " ".repeat(child_indent);
            let closing_padding = " ".repeat(indent);
            let rendered = value
                .iter()
                .map(|value| render_json_like(value, child_indent))
                .collect::<anyhow::Result<Vec<_>>>()?;

            Ok(format!(
                "[\n{padding}{}\n{closing_padding}]",
                rendered.join(&format!(",\n{padding}"))
            ))
        }
        ExecValue::Table { value } => {
            if value.is_empty() {
                return Ok("{}".to_owned());
            }

            let mut entries: Vec<_> = value.iter().collect();
            entries.sort_by(|left, right| left.key.cmp(&right.key));
            if let Some(duplicate) = entries.windows(2).find(|pair| pair[0].key == pair[1].key) {
                bail!(
                    "Malformed exec result: table contains duplicate key {:?}.",
                    duplicate[0].key
                );
            }

            let child_indent = indent + 2;
            let padding = " ".repeat(child_indent);
            let closing_padding = " ".repeat(indent);
            let rendered = entries
                .into_iter()
                .map(|entry| {
                    let key = serde_json::to_string(&entry.key)
                        .context("Could not quote a table key in the exec result")?;
                    let value = render_json_like(&entry.value, child_indent)?;
                    Ok(format!("{key}: {value}"))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;

            Ok(format!(
                "{{\n{padding}{}\n{closing_padding}}}",
                rendered.join(&format!(",\n{padding}"))
            ))
        }
    }
}

fn render_number(value: f64) -> anyhow::Result<String> {
    if !value.is_finite() {
        bail!("Malformed exec result: non-finite number {value}.");
    }

    Ok(value.to_string())
}

#[cfg(test)]
mod test {
    use std::{fs, net::Ipv4Addr};

    use super::*;
    use crate::{
        cli::{Options, Subcommand},
        web::interface::ExecTableEntry,
    };

    fn parse(args: &[&str]) -> Result<Options, clap::Error> {
        Options::try_parse_from(args)
    }

    fn response_with_state(state: ExecJobState) -> ExecJobResponse {
        ExecJobResponse {
            job_id: Uuid::nil().to_string(),
            script_name: "test.lua".to_owned(),
            state,
            result: None,
            logs: None,
            error: None,
            traceback: None,
        }
    }

    fn buffered_response(
        status: StatusCode,
        content_type: Option<&str>,
        is_msgpack: bool,
        body: Vec<u8>,
    ) -> BufferedResponse {
        BufferedResponse {
            status,
            content_type: content_type.map(str::to_owned),
            is_msgpack,
            body,
        }
    }

    #[test]
    fn clap_parses_exec_file() {
        let options = parse(&["prism", "exec", "file.lua"]).unwrap();
        let Subcommand::Exec(command) = options.subcommand else {
            panic!("expected exec command");
        };

        assert_eq!(command.file, PathBuf::from("file.lua"));
    }

    #[test]
    fn clap_uses_default_address_and_port() {
        let options = parse(&["prism", "exec", "file.lua"]).unwrap();
        let Subcommand::Exec(command) = options.subcommand else {
            panic!("expected exec command");
        };

        assert_eq!(command.address, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(command.port, DEFAULT_PORT);
    }

    #[test]
    fn clap_accepts_custom_address_and_port() {
        let options = parse(&[
            "prism",
            "exec",
            "file.lua",
            "--address",
            "192.0.2.10",
            "--port",
            "4567",
        ])
        .unwrap();
        let Subcommand::Exec(command) = options.subcommand else {
            panic!("expected exec command");
        };

        assert_eq!(command.address, "192.0.2.10".parse::<IpAddr>().unwrap());
        assert_eq!(command.port, 4567);
    }

    #[test]
    fn clap_rejects_missing_file() {
        assert!(parse(&["prism", "exec"]).is_err());
    }

    #[test]
    fn clap_rejects_inline_source() {
        assert!(parse(&["prism", "exec", "-e", "return true"]).is_err());
    }

    #[test]
    fn reads_utf8_source_exactly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unicode.luau");
        let source = "-- café\r\nreturn \"雪\"\n";
        fs::write(&path, source.as_bytes()).unwrap();

        assert_eq!(
            read_exec_script(&path).unwrap(),
            ExecScript {
                script_name: "unicode.luau".to_owned(),
                source: source.to_owned(),
            }
        );
    }

    #[test]
    fn normalization_preserves_source_without_bom() {
        let mut source = "return true".to_owned();

        normalize_source(&mut source);

        assert_eq!(source, "return true");
    }

    #[test]
    fn normalization_removes_one_leading_bom() {
        let mut source = "\u{FEFF}return true".to_owned();

        normalize_source(&mut source);

        assert_eq!(source, "return true");
    }

    #[test]
    fn normalization_preserves_interior_bom() {
        let mut source = "print(\"\u{FEFF}\")".to_owned();

        normalize_source(&mut source);

        assert_eq!(source, "print(\"\u{FEFF}\")");
    }

    #[test]
    fn normalization_turns_single_bom_into_empty_source() {
        let mut source = "\u{FEFF}".to_owned();

        normalize_source(&mut source);

        assert_eq!(source, "");
    }

    #[test]
    fn normalization_removes_only_first_of_two_leading_boms() {
        let mut source = "\u{FEFF}\u{FEFF}return true".to_owned();

        normalize_source(&mut source);

        assert_eq!(source, "\u{FEFF}return true");
    }

    #[test]
    fn source_limit_applies_after_bom_normalization() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("maximum-size.lua");
        let mut source = "\u{FEFF}".as_bytes().to_vec();
        source.extend(std::iter::repeat_n(b'x', MAX_SOURCE_SIZE_BYTES));
        fs::write(&path, source).unwrap();

        let script = read_exec_script(&path).unwrap();

        assert_eq!(script.source.len(), MAX_SOURCE_SIZE_BYTES);
        assert!(!script.source.starts_with('\u{FEFF}'));
    }

    #[test]
    fn rejects_invalid_utf8_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.lua");
        fs::write(&path, [0xff, 0xfe]).unwrap();

        let error = read_exec_script(&path).unwrap_err();
        assert!(error.to_string().contains("not valid UTF-8"));
    }

    #[test]
    fn reports_missing_source_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.lua");

        let error = read_exec_script(&path).unwrap_err();
        assert!(error.to_string().contains("Could not inspect exec script"));
        assert!(error.to_string().contains("exists and is readable"));
    }

    #[test]
    fn rejects_directory() {
        let dir = tempfile::tempdir().unwrap();

        let error = read_exec_script(dir.path()).unwrap_err();
        assert!(error.to_string().contains("not a regular file"));
    }

    #[test]
    fn rejects_oversized_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.lua");
        fs::write(&path, vec![b'x'; MAX_SOURCE_SIZE_BYTES + 1]).unwrap();

        let error = read_exec_script(&path).unwrap_err();
        assert!(error.to_string().contains("exceeding"));
        assert!(error.to_string().contains("source limit"));
    }

    #[test]
    fn extracts_script_basename() {
        assert_eq!(
            script_name_from_path(Path::new("some/project/create-part.lua")).unwrap(),
            "create-part.lua"
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_non_utf8_script_basename() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};

        let path = PathBuf::from(OsString::from_vec(vec![0xff]));
        assert!(script_name_from_path(&path)
            .unwrap_err()
            .to_string()
            .contains("not valid UTF-8"));
    }

    #[cfg(windows)]
    #[test]
    fn rejects_non_utf8_script_basename() {
        use std::{ffi::OsString, os::windows::ffi::OsStringExt};

        let path = PathBuf::from(OsString::from_wide(&[0xd800]));
        assert!(script_name_from_path(&path)
            .unwrap_err()
            .to_string()
            .contains("not valid UTF-8"));
    }

    #[test]
    fn renders_nil_as_no_output() {
        assert_eq!(render_result(&ExecValue::Nil).unwrap(), None);
    }

    #[test]
    fn renders_string_without_debug_wrappers() {
        assert_eq!(
            render_result(&ExecValue::String {
                value: "hello\nworld".to_owned(),
            })
            .unwrap(),
            Some("hello\nworld".to_owned())
        );
    }

    #[test]
    fn renders_number() {
        assert_eq!(
            render_result(&ExecValue::Number { value: 42.5 }).unwrap(),
            Some("42.5".to_owned())
        );
    }

    #[test]
    fn renders_boolean() {
        assert_eq!(
            render_result(&ExecValue::Boolean { value: true }).unwrap(),
            Some("true".to_owned())
        );
    }

    #[test]
    fn renders_arrays_in_order() {
        let value = ExecValue::Array {
            value: vec![
                ExecValue::String {
                    value: "first".to_owned(),
                },
                ExecValue::Number { value: 2.0 },
                ExecValue::Nil,
            ],
        };

        assert_eq!(
            render_result(&value).unwrap(),
            Some("[\n  \"first\",\n  2,\n  null\n]".to_owned())
        );
    }

    #[test]
    fn renders_tables_with_sorted_and_escaped_keys() {
        let value = ExecValue::Table {
            value: vec![
                ExecTableEntry {
                    key: "z".to_owned(),
                    value: ExecValue::String {
                        value: "last\nline".to_owned(),
                    },
                },
                ExecTableEntry {
                    key: "a\"key".to_owned(),
                    value: ExecValue::Boolean { value: true },
                },
            ],
        };

        assert_eq!(
            render_result(&value).unwrap(),
            Some("{\n  \"a\\\"key\": true,\n  \"z\": \"last\\nline\"\n}".to_owned())
        );
    }

    #[test]
    fn routes_print_and_warn_logs_to_separate_streams() {
        let logs = [
            ExecLog {
                level: ExecLogLevel::Print,
                message: "hello".to_owned(),
            },
            ExecLog {
                level: ExecLogLevel::Warn,
                message: "careful".to_owned(),
            },
            ExecLog {
                level: ExecLogLevel::Print,
                message: "done".to_owned(),
            },
        ];
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        replay_logs(&logs, &mut stdout, &mut stderr).unwrap();

        assert_eq!(String::from_utf8(stdout).unwrap(), "hello\ndone\n");
        assert_eq!(String::from_utf8(stderr).unwrap(), "careful\n");
    }

    #[test]
    fn formats_failed_state_with_traceback() {
        let mut response = response_with_state(ExecJobState::Failed);
        response.error = Some("attempt to index nil".to_owned());
        response.traceback = Some("stack traceback:\n  test.lua:1".to_owned());

        let error = finish_job(&response, &mut Vec::new(), &mut Vec::new()).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Prism exec failed: attempt to index nil\nstack traceback:\n  test.lua:1"
        );
    }

    #[test]
    fn formats_timed_out_state() {
        let mut response = response_with_state(ExecJobState::TimedOut);
        response.error = Some("execution exceeded its deadline".to_owned());

        let error = finish_job(&response, &mut Vec::new(), &mut Vec::new()).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Prism exec timed out: execution exceeded its deadline"
        );
    }

    #[test]
    fn malformed_result_returns_an_error_without_panicking() {
        let result =
            std::panic::catch_unwind(|| render_result(&ExecValue::Number { value: f64::NAN }));

        assert!(result.is_ok());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn duplicate_table_keys_are_rejected() {
        let value = ExecValue::Table {
            value: vec![
                ExecTableEntry {
                    key: "same".to_owned(),
                    value: ExecValue::Nil,
                },
                ExecTableEntry {
                    key: "same".to_owned(),
                    value: ExecValue::Boolean { value: true },
                },
            ],
        };

        assert!(render_result(&value)
            .unwrap_err()
            .to_string()
            .contains("duplicate key"));
    }

    #[test]
    fn local_timeout_message_retains_job_id() {
        let job_id = Uuid::new_v4();
        let message = local_timeout_error(job_id, LOCAL_TIMEOUT).to_string();

        assert!(message.contains(&job_id.to_string()));
        assert!(message.contains("may still retain the job briefly"));
    }

    #[test]
    fn decodes_a_valid_messagepack_exec_response() {
        let expected = response_with_state(ExecJobState::Pending);
        let response = buffered_response(
            StatusCode::OK,
            Some(MSGPACK_CONTENT_TYPE),
            true,
            serialize_msgpack(&expected).unwrap(),
        );

        let actual: ExecJobResponse = decode_buffered_exec_response(
            &response,
            StatusCode::OK,
            "http://127.0.0.1:34872",
            "polling the exec job",
        )
        .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn rejects_wrong_response_content_type() {
        let response = buffered_response(StatusCode::OK, Some("text/plain"), false, Vec::new());

        let error = decode_buffered_exec_response::<ExecJobResponse>(
            &response,
            StatusCode::OK,
            "http://127.0.0.1:34872",
            "polling the exec job",
        )
        .unwrap_err();

        assert!(error.to_string().contains("content type 'text/plain'"));
        assert!(error.to_string().contains(MSGPACK_CONTENT_TYPE));
    }

    #[test]
    fn rejects_malformed_messagepack_response() {
        let response =
            buffered_response(StatusCode::OK, Some(MSGPACK_CONTENT_TYPE), true, vec![0xc1]);

        let error = decode_buffered_exec_response::<ExecJobResponse>(
            &response,
            StatusCode::OK,
            "http://127.0.0.1:34872",
            "polling the exec job",
        )
        .unwrap_err();

        assert!(error.to_string().contains("malformed MessagePack"));
    }

    #[test]
    fn maps_http_errors_and_decodes_the_error_envelope() {
        let response = buffered_response(
            StatusCode::TOO_MANY_REQUESTS,
            Some(MSGPACK_CONTENT_TYPE),
            true,
            serialize_msgpack(ErrorResponse::too_many_requests("pending queue is full")).unwrap(),
        );

        let error = decode_buffered_exec_response::<ExecJobResponse>(
            &response,
            StatusCode::CREATED,
            "http://127.0.0.1:34872",
            "submitting the exec job",
        )
        .unwrap_err();
        let message = error.to_string();

        assert!(message.contains("exec queue is full"));
        assert!(message.contains("TooManyRequests: pending queue is full"));
    }

    #[test]
    fn formats_ipv6_server_url_with_brackets() {
        assert_eq!(
            server_url("::1".parse().unwrap(), 34872),
            "http://[::1]:34872"
        );
    }
}
