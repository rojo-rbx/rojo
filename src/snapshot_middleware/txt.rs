use std::{path::Path, str};

use memofs::Vfs;
use rbx_dom_weak::ustr;

use crate::snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot};

use super::meta_file::AdjacentMetadata;

pub fn snapshot_txt(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    name: &str,
) -> anyhow::Result<Option<InstanceSnapshot>> {
    let contents = vfs.read_to_string(path)?;
    let contents_str = contents.as_str();

    let mut snapshot = InstanceSnapshot::new()
        .name(name)
        .class_name("StringValue")
        .property(ustr("Value"), contents_str)
        .metadata(
            InstanceMetadata::new()
                .instigating_source(path)
                .relevant_paths(vec![path.to_path_buf()])
                .context(context),
        );

    AdjacentMetadata::read_and_apply_all(vfs, path, name, &mut snapshot)?;

    Ok(Some(snapshot))
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn instance_from_vfs() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.txt", VfsSnapshot::file("Hello there!"))
            .unwrap();

        let vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_txt(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.txt"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }

    #[test]
    fn with_metadata() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot("/foo.txt", VfsSnapshot::file("Hello there!"))
            .unwrap();
        imfs.load_snapshot(
            "/foo.meta.json",
            VfsSnapshot::file(
                r#"{
                    "id": "manually specified"
                }"#,
            ),
        )
        .unwrap();
        let vfs = Vfs::new(imfs.clone());

        let instance_snapshot = snapshot_txt(
            &InstanceContext::default(),
            &vfs,
            Path::new("/foo.txt"),
            "foo",
        )
        .unwrap()
        .unwrap();

        insta::assert_yaml_snapshot!(instance_snapshot);
    }
}
