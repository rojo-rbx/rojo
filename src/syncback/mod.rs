mod fs_snapshot;
mod middleware;
mod snapshot;

use crate::{
    snapshot::{hash_tree, InstanceSnapshot, RojoTree},
    Project,
};
use memofs::Vfs;
use rbx_dom_weak::{types::Ref, WeakDom};
use std::{collections::VecDeque, rc::Rc};

pub use fs_snapshot::FsSnapshot;
pub use middleware::{get_best_middleware, syncback_middleware};
pub use snapshot::{SyncbackData, SyncbackSnapshot};

pub fn syncback_loop<'old, 'new>(
    vfs: &'old Vfs,
    old_tree: &'old RojoTree,
    new_tree: &'new WeakDom,
    project: &Project,
) -> anyhow::Result<Vec<(Ref, InstanceSnapshot)>> {
    log::debug!("Hashing project DOM");
    let old_hashes = hash_tree(old_tree.inner());
    log::debug!("Hashing file DOM");
    let new_hashes = hash_tree(new_tree);

    let syncback_data = Rc::new(SyncbackData {
        vfs,
        old_tree,
        new_tree,
    });

    let mut snapshots = vec![SyncbackSnapshot {
        data: syncback_data,
        old: Some(old_tree.get_root_id()),
        new: new_tree.root_ref(),
        parent_path: project.folder_location().to_path_buf(),
        name: project.name.clone(),
    }];

    let mut replacements = Vec::new();
    let mut fs_snapshot = FsSnapshot::new();

    while let Some(snapshot) = snapshots.pop() {
        log::debug!(
            "instance {} parent is {}",
            snapshot.name,
            snapshot.parent_path.display()
        );
        // We can quickly check that two subtrees are identical and if they are,
        // skip reconciling them.
        if let Some(old_ref) = snapshot.old {
            if old_hashes.get(&old_ref) == new_hashes.get(&snapshot.new) {
                log::trace!(
                    "Skipping {} due to it being identically hashed as {:?}",
                    get_inst_path(new_tree, snapshot.new),
                    old_hashes.get(&old_ref)
                );
                continue;
            }
        }

        let middleware = snapshot
            .old_inst()
            .and_then(|inst| inst.metadata().middleware)
            .unwrap_or_else(|| get_best_middleware(snapshot.new_inst()));
        log::trace!(
            "Middleware for {}: {:?}",
            get_inst_path(new_tree, snapshot.new),
            middleware
        );

        let syncback = syncback_middleware(&snapshot, middleware);

        if let Some(old_inst) = snapshot.old_inst() {
            replacements.push((old_inst.parent(), syncback.inst_snapshot));
        }

        // TODO: Check if file names are valid files
        fs_snapshot.merge(syncback.fs_snapshot);

        snapshots.extend(syncback.children);
    }

    fs_snapshot.write_to_vfs(project.folder_location(), vfs)?;

    Ok(replacements)
}

fn get_inst_path(dom: &WeakDom, referent: Ref) -> String {
    let mut path: VecDeque<&str> = VecDeque::new();
    let mut inst = dom.get_by_ref(referent);
    while let Some(instance) = inst {
        path.push_front(&instance.name);
        inst = dom.get_by_ref(instance.parent());
    }
    path.into_iter().collect::<Vec<&str>>().join(".")
}
