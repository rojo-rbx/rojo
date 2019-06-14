use rbx_dom_weak::RbxTree;

use super::patch::PatchSet;

pub fn apply_patch(
    tree: &mut RbxTree,
    patch_set: &PatchSet,
) {
    for child_patch in &patch_set.children {
        // TODO
    }

    for prop_patch in &patch_set.properties {
        let instance = tree.get_instance_mut(prop_patch.id)
            .expect("Instance referred to by patch does not exist.");

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