mod fs_snapshot;
mod snapshot;

use crate::{
    snapshot::{hash_tree, InstanceSnapshot, InstanceWithMeta, RojoTree},
    snapshot_middleware::Middleware,
    Project,
};
use memofs::Vfs;
use rbx_dom_weak::{types::Ref, Instance, WeakDom};
use std::{collections::VecDeque, rc::Rc};

pub use fs_snapshot::FsSnapshot;
pub use snapshot::{SyncbackData, SyncbackSnapshot};

pub fn syncback_loop<'old, 'new>(
    vfs: &'old Vfs,
    old_tree: &'old RojoTree,
    new_tree: &'new WeakDom,
    project: &'old Project,
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
        log::debug!(
            "instance {} parent is {} (using middleware {:?})",
            get_inst_path(new_tree, snapshot.new),
            snapshot.parent_path.display(),
            middleware
        );

        let syncback = middleware.syncback(&snapshot)?;

        if let Some(old_inst) = snapshot.old_inst() {
            replacements.push((old_inst.parent(), syncback.inst_snapshot));
        }

        fs_snapshot.merge(syncback.fs_snapshot);

        snapshots.extend(syncback.children);
    }

    fs_snapshot.write_to_vfs(project.folder_location(), vfs)?;

    Ok(replacements)
}

pub struct SyncbackReturn<'new, 'old> {
    pub inst_snapshot: InstanceSnapshot,
    pub fs_snapshot: FsSnapshot,
    pub children: Vec<SyncbackSnapshot<'new, 'old>>,
    pub removed_children: Vec<InstanceWithMeta<'old>>,
}

pub fn get_best_middleware(inst: &Instance) -> Middleware {
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

fn get_inst_path(dom: &WeakDom, referent: Ref) -> String {
    let mut path: VecDeque<&str> = VecDeque::new();
    let mut inst = dom.get_by_ref(referent);
    while let Some(instance) = inst {
        path.push_front(&instance.name);
        inst = dom.get_by_ref(instance.parent());
    }
    path.into_iter().collect::<Vec<&str>>().join(".")
}

/// A list of file names that are not valid on Windows.
const INVALID_WINDOWS_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// A list of all characters that are outright forbidden to be included
/// in a file's name.
const FORBIDDEN_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '|', '?', '*', '\\'];

/// Returns whether a given name is a valid file name. This takes into account
/// rules for Windows, MacOS, and Linux.
///
/// In practice however, these broadly overlap so the only unexpected behavior
/// is Windows, where there are 22 reserved names.
pub fn is_valid_file_name<S: AsRef<str>>(name: S) -> bool {
    let str = name.as_ref();

    if str.ends_with(' ') || str.ends_with('.') {
        return false;
    }
    // TODO check control characters
    for forbidden in FORBIDDEN_CHARS {
        if str.contains(forbidden) {
            return false;
        }
    }
    for forbidden in INVALID_WINDOWS_NAMES {
        if str == forbidden {
            return false;
        }
    }
    true
}
