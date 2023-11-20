//! Hashing utility for a RojoTree

use blake3::{Hash, Hasher};
use rbx_dom_weak::{
    types::{Ref, Variant, Vector3},
    Instance, WeakDom,
};

use std::collections::{HashMap, VecDeque};

pub fn hash_tree(dom: &WeakDom) -> HashMap<Ref, Hash> {
    let mut map: HashMap<Ref, Hash> = HashMap::new();
    let mut order = descendants(dom);

    let mut prop_list = Vec::with_capacity(2);

    // function get_hash_id(inst)
    // return hash({ sort(foreach(inst.properties, hash)), sort(foreach(inst.children, get_hash_id)) })
    // end
    while let Some(referent) = order.pop() {
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
            $hash.update(&($num).to_le_bytes());
        )*}
    };
}

macro_rules! hash {
    ($hash:ident, $value:expr) => {{
        $hash.update($value);
    }};
}

/// Places `value` into the provided hasher.
fn hash_variant(hasher: &mut Hasher, value: &Variant) {
    // We need to round floats, though I'm not sure to what degree we can
    // realistically do that.
    match value {
        Variant::String(str) => hash!(hasher, str.as_bytes()),
        Variant::Bool(bool) => hash!(hasher, &[*bool as u8]),
        Variant::Float32(n) => n_hash!(hasher, round(*n)),
        Variant::Float64(n) => n_hash!(hasher, n),
        Variant::Int32(n) => n_hash!(hasher, n),
        Variant::Int64(n) => n_hash!(hasher, n),
        Variant::BinaryString(bytes) => hash!(hasher, bytes.as_ref()),
        Variant::Vector3(v3) => vector_hash(hasher, *v3),
        Variant::Vector2(v2) => n_hash!(hasher, round(v2.x), round(v2.y)),
        Variant::Axes(a) => hash!(hasher, &[a.bits()]),
        Variant::BrickColor(color) => n_hash!(hasher, *color as u16),
        Variant::CFrame(cf) => {
            vector_hash(hasher, cf.position);
            vector_hash(hasher, cf.orientation.x);
            vector_hash(hasher, cf.orientation.y);
            vector_hash(hasher, cf.orientation.z);
        }
        Variant::Color3(color) => n_hash!(hasher, round(color.r), round(color.g), round(color.b)),
        Variant::Color3uint8(color) => hash!(hasher, &[color.r, color.b, color.g]),
        Variant::ColorSequence(seq) => {
            let mut new = Vec::with_capacity(seq.keypoints.len());
            for keypoint in &seq.keypoints {
                new.push(keypoint);
            }
            new.sort_unstable_by(|a, b| round(a.time).partial_cmp(&round(b.time)).unwrap());
            for keypoint in new {
                n_hash!(
                    hasher,
                    round(keypoint.time),
                    round(keypoint.color.r),
                    round(keypoint.color.g),
                    round(keypoint.color.b)
                )
            }
        }
        // TODO: Make this more ergonomic
        Variant::Content(content) => {
            let s: &str = content.as_ref();
            hash!(hasher, s.as_bytes())
        }
        Variant::Enum(e) => n_hash!(hasher, e.to_u32()),
        Variant::Faces(f) => hash!(hasher, &[f.bits()]),

        // TODO: Add the rest
        // Hashing UniqueId properties doesn't make sense
        Variant::UniqueId(_) | _ => (),
    }
}

fn vector_hash(hasher: &mut Hasher, vector: Vector3) {
    n_hash!(hasher, round(vector.x), round(vector.y), round(vector.z))
}

fn round(float: f32) -> f32 {
    (float * 10.0).round() / 10.0
}
