//! Hashing utility for a RojoTree

use blake3::{Hash, Hasher};
use rbx_dom_weak::{
    types::{Ref, Variant},
    Instance, WeakDom,
};

use crate::snapshot::RojoTree;

use std::collections::{HashMap, VecDeque};

pub fn hash_tree(tree: &RojoTree) -> HashMap<Ref, Hash> {
    let dom = tree.inner();
    let mut map: HashMap<Ref, Hash> = HashMap::new();
    let mut order = descendants(dom);

    let mut prop_list = Vec::with_capacity(2);

    // function get_hash_id(inst)
    // return hash({ sort(foreach(inst.properties, hash)), sort(foreach(inst.children, get_hash_id)) })
    // end
    while let Some(referent) = order.pop() {
        log::info!("processing {referent}");
        let inst = dom.get_by_ref(referent).unwrap();
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

    for (name, value) in &inst.properties {
        prop_list.push((name, value))
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

macro_rules! n_hash {
    ($hash:ident, $($num:expr),*) => {
        {$(
            $hash.update(&$num.to_le_bytes());
        )*}
    };
}

macro_rules! hash {
    ($hash:ident, $value:expr) => {{
        $hash.update($value);
    }};
}

fn hash_variant(hasher: &mut Hasher, value: &Variant) {
    // im da joker babeh
    match value {
        Variant::String(str) => hash!(hasher, str.as_bytes()),
        Variant::Bool(bool) => hash!(hasher, &[*bool as u8]),
        Variant::Float32(n) => n_hash!(hasher, n),
        Variant::Float64(n) => n_hash!(hasher, n),
        Variant::Int32(n) => n_hash!(hasher, n),
        Variant::Int64(n) => n_hash!(hasher, n),
        Variant::BinaryString(bytes) => hash!(hasher, bytes.as_ref()),
        Variant::Vector3(v3) => n_hash!(hasher, v3.x, v3.y, v3.z),
        Variant::Vector2(v2) => n_hash!(hasher, v2.x, v2.y),

        // TODO: Add the rest
        _ => (),
    }
}
