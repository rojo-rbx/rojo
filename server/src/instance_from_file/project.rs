use std::{
    borrow::Cow,
    collections::HashMap,
};

use rbx_dom_weak::{RbxTree, RbxId};

use crate::{
    project::{Project, ProjectNode},
    imfs::{
        FsErrorKind,
        new::{Imfs, ImfsFetcher, ImfsEntry},
    },
    snapshot::InstanceSnapshot,
};

use super::{
    middleware::{SnapshotMiddleware, SnapshotInstanceResult, SnapshotFileResult},
};

pub struct SnapshotProject;

impl SnapshotMiddleware for SnapshotProject {
    fn from_imfs<F: ImfsFetcher>(
        imfs: &mut Imfs<F>,
        entry: ImfsEntry,
    ) -> SnapshotInstanceResult {
        if entry.is_directory() {
            let project_path = entry.path().join("default.project.json");

            match imfs.get(project_path) {
                Err(ref err) if err.kind() == FsErrorKind::NotFound => {}
                Err(err) => return Err(err),
                Ok(entry) => return SnapshotProject::from_imfs(imfs, entry),
            }
        }

        if !entry.path().to_string_lossy().ends_with(".project.json") {
            return Ok(None)
        }

        let project = Project::load_from_slice(entry.contents(imfs)?, entry.path())
            .expect("Invalid project file");

        snapshot_project_node(&project.name, &project.tree)
    }

    fn from_instance(
        _tree: &RbxTree,
        _id: RbxId,
    ) -> SnapshotFileResult {
        unimplemented!("TODO: Supporting turning instances into projects");
    }
}

fn snapshot_project_node(instance_name: &str, node: &ProjectNode) -> SnapshotInstanceResult<'static> {
    assert!(node.path.is_none(), "TODO: Support $path");
    assert!(node.properties.is_empty(), "TODO: Support $properties");
    assert!(node.children.is_empty(), "TODO: Support children");
    assert!(node.ignore_unknown_instances.is_none(), "TODO: Support $ignoreUnknownInstances");

    let name = Cow::Owned(instance_name.to_owned());
    let class_name = node.class_name
        .as_ref()
        .map(|name| Cow::Owned(name.clone()));
    let properties = HashMap::new();
    let children = Vec::new();

    // TODO: Load instance from $path if it's set.
    // TODO: Load properties and children from project node

    let class_name = class_name
        .expect("TODO: Support omitting $className in projects");

    Ok(Some(InstanceSnapshot {
        snapshot_id: None,
        name,
        class_name,
        properties,
        children,
    }))
}

#[cfg(test)]
mod test {
    use super::*;

    use maplit::hashmap;

    use crate::imfs::new::{ImfsSnapshot, NoopFetcher};

    #[test]
    fn project_from_folder() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "default.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "indirect-project",
                    "tree": {
                        "$className": "Folder"
                    }
                }
            "#),
        });

        imfs.load_from_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, entry)
            .expect("snapshot error")
            .expect("snapshot returned no instances");

        assert_eq!(instance_snapshot.name, "indirect-project");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.properties, HashMap::new());
        assert_eq!(instance_snapshot.children, Vec::new());
    }

    #[test]
    fn project_from_direct_file() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "hello.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "direct-project",
                    "tree": {
                        "$className": "Model"
                    }
                }
            "#),
        });

        imfs.load_from_snapshot("/foo", dir);

        let entry = imfs.get("/foo/hello.project.json").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, entry)
            .expect("snapshot error")
            .expect("snapshot returned no instances");

        assert_eq!(instance_snapshot.name, "direct-project");
        assert_eq!(instance_snapshot.class_name, "Model");
        assert_eq!(instance_snapshot.properties, HashMap::new());
        assert_eq!(instance_snapshot.children, Vec::new());
    }
}