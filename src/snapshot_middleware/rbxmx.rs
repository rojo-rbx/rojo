use std::borrow::Cow;

use rbx_dom_weak::{RbxId, RbxTree};

use crate::{
    imfs::{Imfs, ImfsEntry, ImfsFetcher},
    snapshot::InstanceSnapshot,
};

use super::middleware::{SnapshotFileResult, SnapshotInstanceResult, SnapshotMiddleware};

pub struct SnapshotRbxmx;

impl SnapshotMiddleware for SnapshotRbxmx {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: &ImfsEntry,
    ) -> SnapshotInstanceResult<'static> {
        if entry.is_directory() {
            return Ok(None);
        }

        let file_name = entry.path().file_name().unwrap().to_string_lossy();

        if !file_name.ends_with(".rbxmx") {
            return Ok(None);
        }

        let instance_name = entry
            .path()
            .file_stem()
            .expect("Could not extract file stem")
            .to_string_lossy()
            .to_string();

        let options = rbx_xml::DecodeOptions::new()
            .property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown);

        let temp_tree = rbx_xml::from_reader(entry.contents(imfs)?, options)
            .expect("TODO: Handle rbx_xml errors");

        let root_instance = temp_tree.get_instance(temp_tree.get_root_id()).unwrap();
        let children = root_instance.get_children_ids();

        if children.len() == 1 {
            let mut snapshot = InstanceSnapshot::from_tree(&temp_tree, children[0]);
            snapshot.name = Cow::Owned(instance_name);
            snapshot.metadata.contributing_paths = vec![entry.path().to_path_buf()];

            Ok(Some(snapshot))
        } else {
            panic!("Rojo doesn't have support for model files with zero or more than one top-level instances yet.");
        }
    }

    fn from_instance(_tree: &RbxTree, _id: RbxId) -> SnapshotFileResult {
        unimplemented!("Snapshotting models");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::HashMap;

    use crate::imfs::{ImfsDebug, ImfsSnapshot, NoopFetcher};

    #[test]
    fn model_from_imfs() {
        let mut imfs = Imfs::new(NoopFetcher);
        let file = ImfsSnapshot::file(
            r#"
            <roblox version="4">
                <Item class="Folder" referent="0">
                    <Properties>
                        <string name="Name">THIS NAME IS IGNORED</string>
                    </Properties>
                </Item>
            </roblox>
        "#,
        );

        imfs.debug_load_snapshot("/foo.rbxmx", file);

        let entry = imfs.get("/foo.rbxmx").unwrap();
        let instance_snapshot = SnapshotRbxmx::from_imfs(&mut imfs, &entry)
            .unwrap()
            .unwrap();

        assert_eq!(instance_snapshot.name, "foo");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.properties, HashMap::new());
        assert_eq!(instance_snapshot.children, Vec::new());
    }
}
