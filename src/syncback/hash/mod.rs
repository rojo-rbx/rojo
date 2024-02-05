//! Hashing utilities for a WeakDom.
mod variant;

pub use variant::*;

use blake3::{Hash, Hasher};
use rbx_dom_weak::{
    types::{Ref, Variant},
    Instance, WeakDom,
};
use std::collections::HashMap;

use super::descendants;
use crate::variant_eq::variant_eq;

/// Returns a map of every `Ref` in the `WeakDom` to a hashed version of the
/// `Instance` it points to, including the properties but not including the
/// descendants of the Instance.
///
/// The hashes **do not** include the descendants of the Instances in them,
/// so they should only be used for comparing Instances directly. To compare a
/// subtree, use `hash_tree`.
pub fn hash_tree_no_descendants(dom: &WeakDom) -> HashMap<Ref, Hash> {
    let mut map: HashMap<Ref, Hash> = HashMap::new();
    let mut order = descendants(dom);

    let mut prop_list = Vec::with_capacity(2);

    while let Some(referent) = order.pop() {
        let inst = dom.get_by_ref(referent).unwrap();
        let hash = hash_inst_no_descendants(inst, &mut prop_list);

        map.insert(referent, hash.finalize());
    }

    map
}

/// Returns a map of every `Ref` in the `WeakDom` to a hashed version of the
/// `Instance` it points to, including the properties and descendants of the
/// `Instance`.
///
/// The hashes **do** include the descendants of the Instances in them,
/// so they should only be used for comparing subtrees directly. To compare an
/// `Instance` directly, use `hash_tree_no_descendants`.
pub fn hash_tree(dom: &WeakDom) -> HashMap<Ref, Hash> {
    let mut map: HashMap<Ref, Hash> = HashMap::new();
    let mut order = descendants(dom);

    let mut prop_list = Vec::with_capacity(2);

    while let Some(referent) = order.pop() {
        let inst = dom.get_by_ref(referent).unwrap();
        let hash = hash_inst(inst, &mut prop_list, &map);

        map.insert(referent, hash);
    }

    map
}

/// Hashes an Instance using its class, name, and properties. The passed
/// `prop_list` is used to sort properties before hashing them.
fn hash_inst_no_descendants<'inst>(
    inst: &'inst Instance,
    prop_list: &mut Vec<(&'inst str, &'inst Variant)>,
) -> Hasher {
    let mut hasher = Hasher::new();
    hasher.update(inst.class.as_bytes());
    hasher.update(inst.name.as_bytes());

    let descriptor = rbx_reflection_database::get()
        .classes
        .get(inst.class.as_str())
        .expect("class should be known to Rojo");

    for (name, value) in &inst.properties {
        if let Some(default) = descriptor.default_properties.get(name.as_str()) {
            if !variant_eq(default, value) {
                prop_list.push((name, value))
            }
        } else {
            prop_list.push((name, value))
        }
    }

    prop_list.sort_unstable_by_key(|(key, _)| *key);
    for (name, value) in prop_list.iter() {
        hasher.update(name.as_bytes());
        hash_variant(&mut hasher, value)
    }

    prop_list.clear();

    hasher
}

/// Hashes an Instance using its class, name, properties, and descendants.
/// The passed `prop_list` is used to sort properties before hashing them.
///
/// # Panics
/// If any children of the Instance are inside `map`, this function will panic.
fn hash_inst<'inst>(
    inst: &'inst Instance,
    prop_list: &mut Vec<(&'inst str, &'inst Variant)>,
    map: &HashMap<Ref, Hash>,
) -> Hash {
    let mut hasher = hash_inst_no_descendants(inst, prop_list);
    let mut child_list = Vec::with_capacity(inst.children().len());

    for child in inst.children() {
        if let Some(hash) = map.get(child) {
            child_list.push(hash.as_bytes())
        } else {
            panic!("Invariant: child {} not hashed before its parent", child);
        }
    }
    child_list.sort_unstable();
    for hash in child_list {
        hasher.update(hash);
    }

    hasher.finalize()
}
