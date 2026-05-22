use std::fs;

use insta::{assert_snapshot, assert_yaml_snapshot, with_settings};
use tempfile::tempdir;

use crate::rojo_test::{
    internable::InternAndRedact,
    serve_util::{run_serve_test, serialize_to_xml_model},
};

use librojo::web_api::SocketPacketType;

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

        let serialize_response = session
            .get_api_serialize(&[*meshpart, *objectvalue], info.session_id)
            .unwrap();

        // We don't assert a snapshot on the SerializeResponse because the model includes the
        // Refs from the DOM as names, which means it will obviously be different every time
        // this code runs. Still, we ensure that the SessionId is right at least.
        assert_eq!(serialize_response.session_id, info.session_id);

        let model = serialize_to_xml_model(&serialize_response, &redactions);
        assert_snapshot!("meshpart_with_id_serialize_model", model);
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

        let serialize_response = session
            .get_api_serialize(&[root_id], info.session_id)
            .unwrap();

        assert_eq!(serialize_response.session_id, info.session_id);

        let model = serialize_to_xml_model(&serialize_response, &redactions);
        assert_snapshot!("forced_parent_serialize_model", model);
    });
}
