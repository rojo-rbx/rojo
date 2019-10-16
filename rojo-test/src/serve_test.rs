use std::fs;

use insta::assert_yaml_snapshot;

use crate::serve_util::run_serve_test;

#[test]
fn empty() {
    run_serve_test("empty", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();

        let root_id = info.root_instance_id;

        let info = redactions.redacted_yaml(info);

        assert_yaml_snapshot!(info);

        let read_result = session.get_api_read(root_id).unwrap();

        redactions.intern_iter(read_result.instances.keys().copied());

        let read_result = redactions.redacted_yaml(read_result);

        assert_yaml_snapshot!(read_result);
    });
}

#[test]
fn just_txt() {
    run_serve_test("just-txt.txt", |session, mut redactions| {
        let info = session.get_api_rojo().unwrap();

        let root_id = info.root_instance_id;
        let info = redactions.redacted_yaml(info);

        assert_yaml_snapshot!(info);

        let read_result = session.get_api_read(root_id).unwrap();
        redactions.intern_iter(read_result.instances.keys().copied());
        let read_result = redactions.redacted_yaml(read_result);

        assert_yaml_snapshot!(read_result);

        fs::write(session.path(), "Changed content!").unwrap();

        // TODO: Directly served files currently don't trigger changed events!

        // let subscribe_result = session.get_api_subscribe(0).unwrap();
        // let subscribe_result = redactions.redacted_yaml(subscribe_result);

        // assert_yaml_snapshot!(subscribe_result);
    });
}
