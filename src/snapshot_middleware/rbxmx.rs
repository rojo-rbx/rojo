use std::path::Path;

use memofs::Vfs;

use crate::snapshot::{InstanceContext, InstanceMetadata, InstanceSnapshot};

use super::middleware::SnapshotInstanceResult;

pub fn snapshot_rbxmx(
    context: &InstanceContext,
    vfs: &Vfs,
    path: &Path,
    instance_name: &str,
) -> SnapshotInstanceResult {
    let options = rbx_xml::DecodeOptions::new()
        .property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown);

    let temp_tree = rbx_xml::from_reader(vfs.read(path)?.as_slice(), options)
        .expect("TODO: Handle rbx_xml errors");

    let root_instance = temp_tree.get_instance(temp_tree.get_root_id()).unwrap();
    let children = root_instance.get_children_ids();

    if children.len() == 1 {
        let snapshot = InstanceSnapshot::from_tree(&temp_tree, children[0])
            .name(instance_name)
            .metadata(
                InstanceMetadata::new()
                    .instigating_source(path)
                    .relevant_paths(vec![path.to_path_buf()])
                    .context(context),
            );

        Ok(Some(snapshot))
    } else {
        panic!("Rojo doesn't have support for model files with zero or more than one top-level instances yet.");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use memofs::{InMemoryFs, VfsSnapshot};

    #[test]
    fn plain_folder() {
        let mut imfs = InMemoryFs::new();
        imfs.load_snapshot(
            "/foo.rbxmx",
            VfsSnapshot::file(
                r#"
                    <roblox version="4">
                        <Item class="Folder" referent="0">
                            <Properties>
                                <string name="Name">THIS NAME IS IGNORED</string>
                            </Properties>
                        </Item>
                    </roblox>
                "#,
            ),
        )
        .unwrap();

        let mut vfs = Vfs::new(imfs);

        let instance_snapshot = snapshot_rbxmx(
            &InstanceContext::default(),
            &mut vfs,
            Path::new("/foo.rbxmx"),
            "foo",
        )
        .unwrap()
        .unwrap();

        assert_eq!(instance_snapshot.name, "foo");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.properties, Default::default());
        assert_eq!(instance_snapshot.children, Vec::new());
    }
}
