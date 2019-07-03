use rbx_dom_weak::{RbxTree, RbxId, RbxInstanceProperties};

use super::{
    patch::{PatchSet, PatchUpdateInstance},
    snapshot::InstanceSnapshot,
};

pub fn apply_patch(
    tree: &mut RbxTree,
    patch_set: &PatchSet,
) {
    for removed_id in &patch_set.removed_instances {
        tree.remove_instance(*removed_id);
    }

    for add_patch in &patch_set.added_instances {
        apply_add_child(tree, add_patch.parent_id, &add_patch.instance);
    }

    for update_patch in &patch_set.updated_instances {
        apply_update_child(tree, update_patch);
    }
}

fn apply_add_child(
    tree: &mut RbxTree,
    parent_id: RbxId,
    snapshot: &InstanceSnapshot,
) {
    let properties = RbxInstanceProperties {
        name: snapshot.name.clone().into_owned(),
        class_name: snapshot.class_name.clone().into_owned(),
        properties: snapshot.properties.clone(),
    };

    let id = tree.insert_instance(properties, parent_id);

    for child_snapshot in &snapshot.children {
        apply_add_child(tree, id, child_snapshot);
    }
}

fn apply_update_child(
    tree: &mut RbxTree,
    patch: &PatchUpdateInstance,
) {
    let instance = tree.get_instance_mut(patch.id)
        .expect("Instance referred to by patch does not exist");

    if let Some(name) = &patch.changed_name {
        instance.name = name.clone();
    }

    for (key, property_entry) in &patch.changed_properties {
        match property_entry {
            Some(value) => {
                instance.properties.insert(key.clone(), value.clone());
            }
            None => {
                instance.properties.remove(key);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::HashMap;

    use rbx_dom_weak::{RbxTree, RbxId, RbxInstanceProperties};

    fn new_tree() -> (RbxTree, RbxId) {
        let tree = RbxTree::new(RbxInstanceProperties {
            name: "Folder".to_owned(),
            class_name: "Folder".to_owned(),
            properties: HashMap::new(),
        });

        let root_id = tree.get_root_id();

        (tree, root_id)
    }

    #[test]
    fn add_from_empty() {
        let (tree, root_id) = new_tree();
    }
}