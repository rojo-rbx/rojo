//! Hashing utility for a RojoTree

use blake3::{Hash, Hasher};
use rbx_dom_weak::{
    types::{Ref, Variant},
    Instance, WeakDom,
};

use std::collections::{HashMap, VecDeque};

use super::{hash_variant, variant_eq};

pub fn hash_tree(dom: &WeakDom) -> HashMap<Ref, Hash> {
    let mut map: HashMap<Ref, Hash> = HashMap::new();
    let mut order = descendants(dom);

    let mut prop_list = Vec::with_capacity(2);

    // function get_hash_id(inst)
    // return hash({ sort(foreach(inst.properties, hash)), sort(foreach(inst.children, get_hash_id)) })
    // end
    while let Some(referent) = order.pop() {
        let inst = dom.get_by_ref(referent).unwrap();
        // We don't really care about the equality of a DataModel.
        if inst.class == "DataModel" {
            continue;
        }
        let hash = hash_inst(&mut prop_list, &map, inst);

        map.insert(referent, hash);
    }

    map
}

fn hash_inst<'map, 'inst>(
    prop_list: &mut Vec<(&'inst str, &'inst Variant)>,
    map: &'map HashMap<Ref, Hash>,
    inst: &'inst Instance,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(inst.class.as_bytes());
    hasher.update(inst.name.as_bytes());

    let descriptor = rbx_reflection_database::get()
        .classes
        .get(inst.class.as_str())
        .expect("class should be known to Rojo");

    for (name, value) in &inst.properties {
        if let Some(default) = descriptor.default_properties.get(name.as_str()) {
            // TODO: Float comparison
            if variant_eq(default, value) {
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

    prop_list.clear();

    hasher.finalize()
}

fn descendants(dom: &WeakDom) -> Vec<Ref> {
    let mut queue = VecDeque::new();
    let mut ordered = Vec::new();
    queue.push_front(dom.root_ref());

    while let Some(referent) = queue.pop_front() {
        let inst = dom
            .get_by_ref(referent)
            .expect("Invariant: WeakDom had a Ref that wasn't inside it");
        ordered.push(referent);
        for child in inst.children() {
            queue.push_back(*child)
        }
    }

    ordered
}
