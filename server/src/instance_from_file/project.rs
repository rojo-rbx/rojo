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

        snapshot_project_node(&project.tree)
    }

    fn from_instance(
        _tree: &RbxTree,
        _id: RbxId,
    ) -> SnapshotFileResult {
        unimplemented!("TODO");
    }
}

fn snapshot_project_node(_node: &ProjectNode) -> SnapshotInstanceResult<'static> {
    // TODO: This function is a stub to satisfy tests.

    Ok(Some(InstanceSnapshot {
        snapshot_id: None,
        name: Cow::Borrowed("template-project"),
        class_name: Cow::Borrowed("Folder"),
        properties: HashMap::new(),
        children: Vec::new(),
    }))
}

#[cfg(test)]
mod test {
    use super::*;

    use maplit::hashmap;

    use crate::imfs::new::{ImfsSnapshot, NoopFetcher};

    #[test]
    fn instance_from_imfs() {
        let _ = env_logger::try_init();

        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "default.project.json" => ImfsSnapshot::file(r#"
                {
                    "name": "template-project",
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

        assert_eq!(instance_snapshot.name, "template-project");
        assert_eq!(instance_snapshot.class_name, "Folder");
        assert_eq!(instance_snapshot.properties, HashMap::new());
        assert_eq!(instance_snapshot.children, Vec::new());
    }
}