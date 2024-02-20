use std::collections::{HashMap, HashSet};

use rbx_dom_weak::{types::Variant, Instance};
use rbx_reflection::Scriptability;

use crate::variant_eq::variant_eq;

use super::SyncbackRules;

pub fn filter_properties(
    data: Option<&SyncbackRules>,
    inst: &Instance,
) -> HashMap<String, Variant> {
    let sync_unscriptable = data.and_then(|s| s.sync_unscriptable).unwrap_or_default();
    let mut properties: HashMap<String, Variant> =
        HashMap::with_capacity(inst.properties.capacity());

    let filter = get_property_filter(data, inst);
    let class_data = rbx_reflection_database::get()
        .classes
        .get(inst.class.as_str());

    let predicate = |prop_name: &String, prop_value: &Variant| {
        if matches!(prop_value, Variant::Ref(_) | Variant::SharedString(_)) {
            return true;
        }
        if let Some(list) = &filter {
            if list.contains(prop_name) {
                return true;
            }
        }
        if !sync_unscriptable {
            if let Some(data) = class_data {
                if let Some(prop_data) = data.properties.get(prop_name.as_str()) {
                    if matches!(prop_data.scriptability, Scriptability::None) {
                        return true;
                    }
                }
            }
        }
        false
    };

    if let Some(class_data) = class_data {
        let defaults = &class_data.default_properties;
        for (name, value) in &inst.properties {
            if predicate(name, value) {
                continue;
            }
            if let Some(default) = defaults.get(name.as_str()) {
                if !variant_eq(value, default) {
                    properties.insert(name.clone(), value.clone());
                }
            } else {
                properties.insert(name.clone(), value.clone());
            }
        }
    } else {
        for (name, value) in &inst.properties {
            if predicate(name, value) {
                continue;
            }
            properties.insert(name.clone(), value.clone());
        }
    }

    properties
}

/// Returns a set of properties that should not be written with syncback if
/// one exists.
fn get_property_filter<'rules>(
    rules: Option<&'rules SyncbackRules>,
    new_inst: &Instance,
) -> Option<HashSet<&'rules String>> {
    let filter = &rules?.ignore_properties;
    let mut set = HashSet::new();

    let database = rbx_reflection_database::get();
    let mut current_class_name = new_inst.class.as_str();

    loop {
        if let Some(list) = filter.get(current_class_name) {
            set.extend(list)
        }

        let class = database.classes.get(current_class_name)?;
        if let Some(super_class) = class.superclass.as_ref() {
            current_class_name = &super_class;
        } else {
            break;
        }
    }

    Some(set)
}
