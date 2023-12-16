use std::collections::{HashMap, HashSet};

use rbx_dom_weak::{
    types::{Ref, Variant},
    Instance,
};

use crate::{
    resolution::UnresolvedValue,
    snapshot::{InstanceMetadata, InstanceSnapshot, InstanceWithMeta, InstigatingSource},
    snapshot_middleware::{Middleware, ScriptType},
    Project,
};

use super::{FsSnapshot, SyncbackSnapshot};

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
        Middleware::Project => syncback_project(snapshot),
        Middleware::ModuleScript => syncback_script(ScriptType::Module, snapshot),
        Middleware::ClientScript => syncback_script(ScriptType::Client, snapshot),
        Middleware::ServerScript => syncback_script(ScriptType::Server, snapshot),
        Middleware::Text => syncback_text(snapshot),
        Middleware::Rbxmx => syncback_rbxmx(snapshot),
        Middleware::Dir => syncback_dir(snapshot),
        Middleware::ModuleScriptDir => syncback_script_dir(ScriptType::Module, snapshot),
        Middleware::ClientScriptDir => syncback_script_dir(ScriptType::Client, snapshot),
        Middleware::ServerScriptDir => syncback_script_dir(ScriptType::Server, snapshot),
        _ => panic!("unsupported instance middleware {:?}", middleware),
    }
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

fn syncback_script_dir<'new, 'old>(
    script_type: ScriptType,
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> SyncbackReturn<'new, 'old> {
    let mut path = snapshot.parent_path.join("init");
    path.set_extension(match script_type {
        ScriptType::Module => "lua",
        ScriptType::Client => "client.lua",
        ScriptType::Server => "server.lua",
    });
    let contents =
        if let Some(Variant::String(source)) = snapshot.new_inst().properties.get("Source") {
            source.as_bytes().to_vec()
        } else {
            panic!("Source should be a string")
        };

    let dir_syncback = syncback_dir(snapshot);

    let mut fs_snapshot = FsSnapshot::new();
    fs_snapshot.push_file(path, contents);
    fs_snapshot.merge(dir_syncback.fs_snapshot);

    SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(snapshot.new_inst()),
        fs_snapshot,
        children: dir_syncback.children,
        removed_children: dir_syncback.removed_children,
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
    // TODO metadata, including classname

    SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(snapshot.new_inst()),
        fs_snapshot,
        children,
        removed_children,
    }
}

fn syncback_project<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> SyncbackReturn<'new, 'old> {
    let old_inst = snapshot
        .old_inst()
        .expect("project middleware shouldn't be used to make new files");
    // This can never be None.
    let source = old_inst.metadata().instigating_source.as_ref().unwrap();

    let project_path = match source {
        InstigatingSource::Path(path) => path.as_path(),
        InstigatingSource::ProjectNode { path, .. } => path.as_path(),
    };

    // We need to build a 'new' project and serialize it using an FsSnapshot.
    // It's convenient to start with the old one though, since it means we have
    // a thing to iterate through.
    let mut project =
        Project::load_from_slice(&snapshot.vfs().read(project_path).unwrap(), project_path)
            .unwrap();

    let mut children = Vec::new();
    let mut removed_children = Vec::new();

    // Projects are special. We won't be adding or removing things from them,
    // so we'll simply match Instances on a per-node basis and rebuild the tree
    // with the new instance's data. This matching will be done by class and name
    // to simplify things.
    let mut nodes = vec![(&mut project.tree, snapshot.new_inst(), old_inst)];

    // A map of referents from the new tree to the Path that created it,
    // if it exists. This is a roundabout way to locate the parents of
    // Instances.
    let mut ref_to_node = HashMap::new();

    while let Some((node, new_inst, old_inst)) = nodes.pop() {
        ref_to_node.insert(new_inst.referent(), node.path.as_ref());

        let mut old_child_map = HashMap::with_capacity(old_inst.children().len());
        for child_ref in old_inst.children() {
            let child = snapshot.get_old_instance(*child_ref).unwrap();
            old_child_map.insert(child.name(), child);
        }
        let mut new_child_map = HashMap::with_capacity(new_inst.children().len());
        for child_ref in new_inst.children() {
            let child = snapshot.get_new_instance(*child_ref).unwrap();
            new_child_map.insert(child.name.as_str(), child);
        }

        for (child_name, child_node) in &mut node.children {
            if let Some(new_child) = new_child_map.get(child_name.as_str()) {
                if let Some(old_child) = old_child_map.get(child_name.as_str()) {
                    // TODO verify class names
                    for (name, value) in &new_child.properties {
                        if child_node.properties.contains_key(name) {
                            child_node
                                .properties
                                .insert(name.clone(), UnresolvedValue::from(value.clone()));
                        }
                    }
                    nodes.push((child_node, new_child, *old_child));
                    new_child_map.remove(child_name.as_str());
                    old_child_map.remove(child_name.as_str());
                } else {
                    log::error!(
                        "Node {} was in new tree but not old. How did we get here?",
                        child_name
                    );
                }
            } else {
                panic!("Cannot add or remove children from a project")
            }
        }

        // From this point, both maps contain only children of the current
        // instance that aren't in the project. So, we just do some quick and
        // dirty matching to identify children that were added and removed.
        for (new_name, new_child) in new_child_map {
            let parent_path = match ref_to_node.get(&new_child.parent()) {
                Some(Some(path)) => path.path().to_path_buf(),
                Some(None) => {
                    log::debug!("{new_name} was visited but has no path");
                    continue;
                }
                None => {
                    log::debug!("{new_name} does not currently exist on FS");
                    continue;
                }
            };
            if let Some(old_inst) = old_child_map.get(new_name) {
                // All children are descendants of a node of a project
                // So we really just need to track which one is which.
                children.push(SyncbackSnapshot {
                    data: snapshot.data.clone(),
                    old: Some(old_inst.id()),
                    new: new_child.referent(),
                    name: new_name.to_string(),
                    parent_path,
                });
                old_child_map.remove(new_name);
            } else {
                // it's new
                children.push(SyncbackSnapshot {
                    data: snapshot.data.clone(),
                    old: None,
                    new: new_child.referent(),
                    name: new_name.to_string(),
                    parent_path,
                });
            }
        }
        removed_children.extend(old_child_map.into_values());
    }

    SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(snapshot.new_inst()),
        fs_snapshot: FsSnapshot::new().with_file(
            &project.file_location,
            serde_json::to_vec_pretty(&project).unwrap(),
        ),
        children,
        removed_children,
    }
}

pub fn syncback_rbxmx<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> SyncbackReturn<'new, 'old> {
    // If any of the children of this Instance are scripts, we don't want
    // include them in the model. So instead, we'll check and then serialize.

    let inst = snapshot.new_inst();
    let mut path = snapshot.parent_path.join(&inst.name);
    path.set_extension("rbxmx");
    // Long-term, anyway. Right now we stay silly.
    let mut serialized = Vec::new();
    rbx_xml::to_writer_default(&mut serialized, snapshot.new_tree(), &[inst.referent()]).unwrap();
    SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(inst),
        fs_snapshot: FsSnapshot::new().with_file(&path, serialized),
        children: Vec::new(),
        removed_children: Vec::new(),
    }
}

fn syncback_text<'new, 'old>(
    snapshot: &SyncbackSnapshot<'new, 'old>,
) -> SyncbackReturn<'new, 'old> {
    let inst = snapshot.new_inst();

    let mut path = snapshot.parent_path.clone();
    path.set_file_name(snapshot.name.clone());
    path.set_extension("txt");
    let contents = if let Some(Variant::String(source)) = inst.properties.get("Value") {
        source.as_bytes().to_vec()
    } else {
        panic!("Value should be a string")
    };

    SyncbackReturn {
        inst_snapshot: InstanceSnapshot::from_instance(inst).metadata(InstanceMetadata::new()),
        fs_snapshot: FsSnapshot::new().with_file(path, contents),
        children: Vec::new(),
        removed_children: Vec::new(),
    }
}
