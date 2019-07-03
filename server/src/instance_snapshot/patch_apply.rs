use rbx_dom_weak::RbxTree;

use super::patch::PatchSet;

pub fn apply_patch(
    tree: &mut RbxTree,
    patch_set: &PatchSet,
) {
    for child_patch in &patch_set.children {
        for id in &child_patch.removed_children {
            tree.remove_instance(*id);
        }

        let instance = tree.get_instance_mut(child_patch.id)
            .expect("Instance referred to by patch does not exist");

        // TODO: Add children?
    }

    for prop_patch in &patch_set.properties {
        let instance = tree.get_instance_mut(prop_patch.id)
            .expect("Instance referred to by patch does not exist");

        if let Some(name) = &prop_patch.changed_name {
            instance.name = name.clone();
        }

        for (key, property_entry) in &prop_patch.changed_properties {
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