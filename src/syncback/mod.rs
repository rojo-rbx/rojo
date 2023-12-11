mod fs_snapshot;
mod middleware;

use crate::{
    snapshot::{hash_tree, InstanceSnapshot, InstanceWithMeta, RojoTree},
    snapshot_middleware::Middleware,
    Project,
};
use memofs::Vfs;
use rbx_dom_weak::{types::Ref, Instance, WeakDom};
use std::path::{Path, PathBuf};

pub use fs_snapshot::FsSnapshot;

use self::middleware::syncback_middleware;

#[derive(Debug)]
pub struct SyncbackSnapshot<'new, 'old> {
    old_tree: &'old RojoTree,
    new_tree: &'new WeakDom,
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
            old_tree: self.old_tree,
            new_tree: self.new_tree,
            old: old_ref,
            new: new_ref,
            parent_path: self.parent_path.join(extension.as_ref()),
            name: new_name,
        }
    }

    /// The 'old' Instance this snapshot is for, if it exists.
    #[inline]
    pub fn old_inst(&self) -> Option<InstanceWithMeta<'old>> {
        self.old.and_then(|old| self.old_tree.get_instance(old))
    }

    /// The 'new' Instance this snapshot is for.
    #[inline]
    pub fn new_inst(&self) -> &'new Instance {
        self.new_tree
            .get_by_ref(self.new)
            .expect("SyncbackSnapshot should not contain invalid referents")
    }

    /// Returns an Instance from the old tree with the provided referent, if it
    /// exists.
    #[inline]
    pub fn get_old_instance(&self, referent: Ref) -> Option<InstanceWithMeta<'old>> {
        self.old_tree.get_instance(referent)
    }

    /// Returns an Instance from the new tree with the provided referent, if it
    /// exists.
    #[inline]
    pub fn get_new_instance(&self, referent: Ref) -> Option<&'new Instance> {
        self.new_tree.get_by_ref(referent)
    }
}

pub fn syncback_loop(
    vfs: &Vfs,
    old_tree: &RojoTree,
    new_tree: &WeakDom,
    project: &Project,
) -> anyhow::Result<Vec<(Ref, InstanceSnapshot)>> {
    let old_hashes = hash_tree(old_tree.inner());
    let new_hashes = hash_tree(new_tree);
    let mut snapshots = vec![SyncbackSnapshot {
        old_tree,
        new_tree,
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
                continue;
            }
        }

        let middleware = snapshot
            .old_inst()
            .and_then(|inst| inst.metadata().middleware)
            .unwrap_or_else(|| get_best_middleware(snapshot.new_inst()));

        let syncback = syncback_middleware(&snapshot, middleware);

        if let Some(old_inst) = snapshot.old_inst() {
            replacements.push((old_inst.parent(), syncback.inst_snapshot));
        }

        syncback.fs_snapshot.write_to_vfs(vfs)?;

        // TODO handle children
    }

    Ok(replacements)
}

fn get_best_middleware(inst: &Instance) -> Middleware {
    match inst.class.as_str() {
        "Folder" => Middleware::Dir,
        // TODO this should probably just be rbxm
        "Model" => Middleware::Rbxmx,
        "Script" => {
            if inst.children().len() == 0 {
                Middleware::ServerScript
            } else {
                Middleware::ServerScriptDir
            }
        }
        "LocalScript" => {
            if inst.children().len() == 0 {
                Middleware::ClientScript
            } else {
                Middleware::ClientScriptDir
            }
        }
        "ModuleScript" => {
            if inst.children().len() == 0 {
                Middleware::ModuleScript
            } else {
                Middleware::ModuleScriptDir
            }
        }
        _ => Middleware::Rbxmx,
    }
}
