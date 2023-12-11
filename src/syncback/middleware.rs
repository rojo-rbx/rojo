use std::collections::{HashMap, HashSet};

use rbx_dom_weak::{
    types::{Ref, Variant},
    Instance,
};

use crate::{
    resolution::UnresolvedValue,
    snapshot::{InstanceMetadata, InstanceSnapshot, InstanceWithMeta},
    snapshot_middleware::{DirectoryMetadata, Middleware, ScriptType},
};

use super::{FsSnapshot, SyncbackSnapshot};

#[derive(Debug)]
pub struct SyncbackReturn<'new, 'old> {
    pub inst_snapshot: InstanceSnapshot,
    pub fs_snapshot: FsSnapshot,
    pub children: Vec<SyncbackSnapshot<'new, 'old>>,
    pub removed_children: Vec<InstanceWithMeta<'old>>,
}

pub fn syncback_middleware<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
    middleware: Middleware,
) -> SyncbackReturn<'new, 'old> {
    match middleware {
        Middleware::ModuleScript => syncback_script(ScriptType::Module, snapshot),
        Middleware::ClientScript => syncback_script(ScriptType::Client, snapshot),
        Middleware::ServerScript => syncback_script(ScriptType::Server, snapshot),
        Middleware::Dir => syncback_dir(snapshot),
        _ => panic!("unsupported instance middleware {:?}", middleware),
    }
}

fn syncback_script<'new, 'old>(
    script_type: ScriptType,
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> SyncbackReturn<'new, 'old> {
    let inst = snapshot.new_inst();

    let mut path = snapshot.parent_path.clone();
    path.set_file_name(snapshot.name.clone());
    path.set_extension(match script_type {
        ScriptType::Module => "lua",
        ScriptType::Client => "client.lua",
        ScriptType::Server => "server.lua",
    });
    let contents = if let Some(Variant::String(source)) = inst.properties.get("Source") {
        source.as_bytes().to_vec()
    } else {
        panic!("Source should be a string")
    };

    SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(inst).metadata(InstanceMetadata::new()),
        fs_snapshot: FsSnapshot::new().with_file(path, contents),
        // Scripts don't have a child!
        children: Vec::new(),
        removed_children: Vec::new(),
    }
}

fn syncback_dir<'new, 'old>(snapshot: &SyncbackSnapshot<'new, 'old>) -> SyncbackReturn<'new, 'old> {
    let path = snapshot.parent_path.join(snapshot.name.clone());

    let mut removed_children = Vec::new();
    let mut children = Vec::new();

    if let Some(old_inst) = snapshot.old_inst() {
        let old_children: HashMap<&str, Ref> = old_inst
            .children()
            .iter()
            .map(|old_ref| {
                (
                    snapshot.get_old_instance(*old_ref).unwrap().name(),
                    *old_ref,
                )
            })
            .collect();
        let new_children: HashSet<&str> = snapshot
            .new_inst()
            .children()
            .iter()
            .map(|new_ref| snapshot.get_new_instance(*new_ref).unwrap().name.as_str())
            .collect();

        for child_ref in old_inst.children() {
            let old_child = snapshot.get_old_instance(*child_ref).unwrap();
            // If it exists in the old tree but not the new one, it was removed.
            if !new_children.contains(old_child.name()) {
                removed_children.push(old_child);
            }
        }

        for child_ref in snapshot.new_inst().children() {
            let new_child = snapshot.get_new_instance(*child_ref).unwrap();
            // If it exists in the new tree but not the old one, it was added.
            match old_children.get(new_child.name.as_str()) {
                None => children.push(snapshot.from_parent(
                    &new_child.name,
                    new_child.name.clone(),
                    *child_ref,
                    None,
                )),
                Some(old_ref) => children.push(snapshot.from_parent(
                    &new_child.name,
                    new_child.name.clone(),
                    *child_ref,
                    Some(*old_ref),
                )),
            }
        }
    } else {
        for child_ref in snapshot.new_inst().children() {
            let child = snapshot.get_new_instance(*child_ref).unwrap();
            children.push(snapshot.from_parent(&child.name, child.name.clone(), *child_ref, None))
        }
    }
    let mut fs_snapshot = FsSnapshot::new().with_dir(&path);
    // TODO metadata

    SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(snapshot.new_inst()),
        fs_snapshot,
        children,
        removed_children,
    }
}
