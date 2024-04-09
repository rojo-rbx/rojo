use std::collections::HashMap;

use rbx_dom_weak::{types::Variant, Instance};
use rbx_reflection::Scriptability;

use crate::{variant_eq::variant_eq, Project};

/// Returns a map of properties from `inst` that are both allowed under the
/// user-provided settings *and* not their default value.
pub fn filter_properties<'inst>(
    project: &Project,
    inst: &'inst Instance,
) -> HashMap<&'inst str, &'inst Variant> {
    let mut map = Vec::with_capacity(inst.properties.len());
    filter_properties_preallocated(project, inst, &mut map);

    map.into_iter().collect()
}

/// Fills `allocation` with a list of properties from `inst` that are both
/// allowed under the user-provided settings *and* not their default value.
pub fn filter_properties_preallocated<'inst>(
    project: &Project,
    inst: &'inst Instance,
    allocation: &mut Vec<(&'inst str, &'inst Variant)>,
) {
    let sync_unscriptable = project
        .syncback_rules
        .as_ref()
        .and_then(|s| s.sync_unscriptable)
        .unwrap_or_default();

    let class_data = rbx_reflection_database::get()
        .classes
        .get(inst.class.as_str());

    let predicate = |prop_name: &String, prop_value: &Variant| {
        // We don't want to serialize Ref or UniqueId properties in JSON files
        if matches!(prop_value, Variant::Ref(_) | Variant::UniqueId(_)) {
            return true;
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
                    allocation.push((name, value));
                }
            } else {
                allocation.push((name, value));
            }
        }
    } else {
        for (name, value) in &inst.properties {
            if predicate(name, value) {
                continue;
            }
            allocation.push((name, value));
        }
    }
}
