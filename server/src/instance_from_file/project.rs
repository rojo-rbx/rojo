use std::{
    borrow::Cow,
    str,
};

use maplit::hashmap;
use rbx_dom_weak::{RbxTree, RbxValue, RbxId};

use crate::{
    imfs::{
        FsErrorKind,
        new::{Imfs, ImfsSnapshot, FileSnapshot, ImfsFetcher, ImfsEntry},
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

        if !entry.path().ends_with(".project.json") {
            return Ok(None)
        }

        Ok(None)
    }

    fn from_instance(
        tree: &RbxTree,
        id: RbxId,
    ) -> SnapshotFileResult {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use maplit::hashmap;
    use rbx_dom_weak::{RbxInstanceProperties};

    use crate::imfs::new::NoopFetcher;

    #[test]
    fn instance_from_imfs() {
        let mut imfs = Imfs::new(NoopFetcher);
        let dir = ImfsSnapshot::dir(hashmap! {
            "default.project.json" => ImfsSnapshot::file("{}"),
        });

        imfs.load_from_snapshot("/foo", dir);

        let entry = imfs.get("/foo").unwrap();
        let instance_snapshot = SnapshotProject::from_imfs(&mut imfs, entry).unwrap().unwrap();
    }
}