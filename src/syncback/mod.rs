mod fs_snapshot;
mod middleware;

use crate::{
    snapshot::{hash_tree, InstanceSnapshot, InstanceWithMeta, RojoTree},
    Project,
};
use blake3::Hash;
use memofs::Vfs;
use rbx_dom_weak::{types::Ref, Instance, WeakDom};
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    rc::Rc,
};

pub use fs_snapshot::FsSnapshot;

use self::middleware::{get_best_middleware, syncback_middleware};

struct SyncbackData<'new, 'old> {
    vfs: &'old Vfs,
    old_tree: &'old RojoTree,
    new_tree: &'new WeakDom,

    old_hashes: Rc<HashMap<Ref, Hash>>,
    new_hashes: Rc<HashMap<Ref, Hash>>,
}
pub struct SyncbackSnapshot<'new, 'old> {
    data: Rc<SyncbackData<'new, 'old>>,
    old: Option<Ref>,
    new: Ref,
    parent_path: PathBuf,
    name: String,
}

impl<'new, 'old> SyncbackSnapshot<'new, 'old> {
    /// Constructs a SyncbackSnapshot from the provided refs
    /// while inheriting the parent's trees and path
    #[inline]
    pub fn from_parent<P: AsRef<Path>>(
        &self,
        extension: P,
        new_name: String,
        new_ref: Ref,
        old_ref: Option<Ref>,
    ) -> Self {
        Self {
            data: Rc::clone(&self.data),
            old: old_ref,
            new: new_ref,
            parent_path: self.parent_path.join(extension.as_ref()),
            name: new_name,
        }
    }

    /// Returns an Instance from the old tree with the provided referent, if it
    /// exists.
    #[inline]
    pub fn get_old_instance(&self, referent: Ref) -> Option<InstanceWithMeta<'old>> {
        self.data.old_tree.get_instance(referent)
    }

    /// Returns an Instance from the new tree with the provided referent, if it
    /// exists.
    #[inline]
    pub fn get_new_instance(&self, referent: Ref) -> Option<&'new Instance> {
        self.data.new_tree.get_by_ref(referent)
    }

    /// The 'old' Instance this snapshot is for, if it exists.
    #[inline]
    pub fn old_inst(&self) -> Option<InstanceWithMeta<'old>> {
        self.old
            .and_then(|old| self.data.old_tree.get_instance(old))
    }

    /// The 'new' Instance this snapshot is for.
    #[inline]
    pub fn new_inst(&self) -> &'new Instance {
        self.data
            .new_tree
            .get_by_ref(self.new)
            .expect("SyncbackSnapshot should not contain invalid referents")
    }

    /// Returns the underlying VFS being used for syncback.
    #[inline]
    pub fn vfs(&self) -> &Vfs {
        self.data.vfs
    }
}

pub fn syncback_loop<'old, 'new>(
    vfs: &'old Vfs,
    old_tree: &'old RojoTree,
    new_tree: &'new WeakDom,
    project: &Project,
) -> anyhow::Result<Vec<(Ref, InstanceSnapshot)>> {
    log::debug!("Hashing project DOM");
    let old_hashes = Rc::new(hash_tree(old_tree.inner()));
    log::debug!("Hashing file DOM");
    let new_hashes = Rc::new(hash_tree(new_tree));

    let syncback_data = Rc::new(SyncbackData {
        vfs,
        old_tree,
        new_tree,
        old_hashes: Rc::clone(&old_hashes),
        new_hashes: Rc::clone(&new_hashes),
    });

    let mut snapshots = vec![SyncbackSnapshot {
        data: syncback_data,
        old: Some(old_tree.get_root_id()),
        new: new_tree.root_ref(),
        parent_path: project.file_location.clone(),
        name: project.name.clone(),
    }];

    let mut replacements = Vec::new();

    while let Some(snapshot) = snapshots.pop() {
        // We can quickly check that two subtrees are identical and if they are,
        // skip reconciling them.
        if let Some(old_ref) = snapshot.old {
            if old_hashes.get(&old_ref) == new_hashes.get(&snapshot.new) {
                log::debug!(
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
        log::debug!(
            "Middleware for {}: {:?}",
            get_inst_path(new_tree, snapshot.new),
            middleware
        );

        let syncback = syncback_middleware(&snapshot, middleware);

        if let Some(old_inst) = snapshot.old_inst() {
            replacements.push((old_inst.parent(), syncback.inst_snapshot));
        }

        log::debug!("Writing {} to vfs", get_inst_path(new_tree, snapshot.new));
        syncback.fs_snapshot.write_to_vfs(vfs)?;

        snapshots.extend(syncback.children);
    }

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
