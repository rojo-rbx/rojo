mod fs_snapshot;
mod snapshot;

use memofs::Vfs;
use rbx_dom_weak::{
    types::{Ref, Variant},
    Instance, WeakDom,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

use crate::{
    resolution::UnresolvedValue,
    snapshot::{hash_tree, InstanceSnapshot, InstanceWithMeta, RojoTree},
    snapshot_middleware::Middleware,
    Project,
};

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

    let syncback_data = SyncbackData {
        vfs,
        old_tree,
        new_tree,
    };

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

        if let Some(syncback_rules) = &project.syncback_rules {
            if !syncback_rules.acceptable(new_tree, snapshot.new) {
                log::debug!(
                    "Path {} is blocked by project",
                    get_inst_path(new_tree, snapshot.new)
                );
                continue;
            }
        }

        let middleware = snapshot
            .old_inst()
            .and_then(|inst| inst.metadata().middleware)
            .unwrap_or_else(|| get_best_middleware(snapshot.new_inst()));
        log::trace!(
            "Middleware for {} is {:?}",
            get_inst_path(new_tree, snapshot.new),
            middleware
        );

        if matches!(middleware, Middleware::Json | Middleware::Toml) {
            log::warn!(
                "Cannot syncback {middleware:?} at {}, skipping",
                get_inst_path(new_tree, snapshot.new)
            );
            continue;
        }

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
        "Folder" | "Configuration" | "Tool" | "ScreenGui" => Middleware::Dir,
        "Sound"
        | "SoundGroup"
        | "Sky"
        | "Atmosphere"
        | "BloomEffect"
        | "BlurEffect"
        | "ColorCorrectionEffect"
        | "DepthOfFieldEffect"
        | "SunRaysEffect" => {
            if inst.children().is_empty() {
                Middleware::JsonModel
            } else {
                // This begs the question of an init.model.json but we'll leave
                // that for another day.
                Middleware::Dir
            }
        }
        "StringValue" => {
            if inst.children().is_empty() {
                Middleware::Text
            } else {
                Middleware::Dir
            }
        }
        "Script" => {
            if inst.children().is_empty() {
                Middleware::ServerScript
            } else {
                Middleware::ServerScriptDir
            }
        }
        "LocalScript" => {
            if inst.children().is_empty() {
                Middleware::ClientScript
            } else {
                Middleware::ClientScriptDir
            }
        }
        "ModuleScript" => {
            if inst.children().is_empty() {
                Middleware::ModuleScript
            } else {
                Middleware::ModuleScriptDir
            }
        }
        "LocalizationTable" => {
            if inst.children().is_empty() {
                Middleware::Csv
            } else {
                Middleware::CsvDir
            }
        }
        _ => Middleware::Rbxm,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncbackIgnoreRules {
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default, skip)]
    classes: HashMap<String, HashMap<String, UnresolvedValue>>,
}

impl SyncbackIgnoreRules {
    /// If possible, resolves all of the properties in the ignore rules so that
    /// they're Variants.
    pub fn resolve(&self) -> anyhow::Result<HashMap<&str, HashMap<&str, Variant>>> {
        let mut resolved = HashMap::with_capacity(self.classes.capacity());

        for (class_name, properties) in &self.classes {
            let mut resolved_props = HashMap::with_capacity(properties.capacity());
            for (prop_name, prop_value) in properties {
                resolved_props.insert(
                    prop_name.as_str(),
                    prop_value.clone().resolve(class_name, prop_name)?,
                );
            }

            resolved.insert(class_name.as_str(), resolved_props);
        }

        Ok(resolved)
    }

    /// Returns whether the provided Instance is allowed to be handled with
    /// syncback.
    #[inline]
    pub fn acceptable(&self, dom: &WeakDom, inst: Ref) -> bool {
        let path = get_inst_path(dom, inst);
        for ignored in &self.paths {
            if path.starts_with(ignored.as_str()) {
                return false;
            }
        }
        true
    }
}

fn get_inst_path(dom: &WeakDom, referent: Ref) -> String {
    let mut path: VecDeque<&str> = VecDeque::new();
    let mut inst = dom.get_by_ref(referent);
    while let Some(instance) = inst {
        path.push_front(&instance.name);
        inst = dom.get_by_ref(instance.parent());
    }
    path.into_iter().collect::<Vec<&str>>().join("/")
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
