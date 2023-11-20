use blake3::Hasher;
use rbx_dom_weak::types::{Variant, Vector3};

macro_rules! round {
    ($value:expr) => {
        (($value * 10.0).round() / 10.0)
    };
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
pub fn hash_variant(hasher: &mut Hasher, value: &Variant) {
    // We need to round floats, though I'm not sure to what degree we can
    // realistically do that.
    match value {
        Variant::String(str) => hash!(hasher, str.as_bytes()),
        Variant::Bool(bool) => hash!(hasher, &[*bool as u8]),
        Variant::Float32(n) => n_hash!(hasher, round!(*n)),
        Variant::Float64(n) => n_hash!(hasher, round!(n)),
        Variant::Int32(n) => n_hash!(hasher, n),
        Variant::Int64(n) => n_hash!(hasher, n),
        Variant::BinaryString(bytes) => hash!(hasher, bytes.as_ref()),
        Variant::Vector3(v3) => vector_hash(hasher, *v3),
        Variant::Vector2(v2) => n_hash!(hasher, round!(v2.x), round!(v2.y)),
        Variant::Axes(a) => hash!(hasher, &[a.bits()]),
        Variant::BrickColor(color) => n_hash!(hasher, *color as u16),
        Variant::CFrame(cf) => {
            vector_hash(hasher, cf.position);
            vector_hash(hasher, cf.orientation.x);
            vector_hash(hasher, cf.orientation.y);
            vector_hash(hasher, cf.orientation.z);
        }
        Variant::Color3(color) => {
            n_hash!(hasher, round!(color.r), round!(color.g), round!(color.b))
        }
        Variant::Color3uint8(color) => hash!(hasher, &[color.r, color.b, color.g]),
        Variant::ColorSequence(seq) => {
            let mut new = Vec::with_capacity(seq.keypoints.len());
            for keypoint in &seq.keypoints {
                new.push(keypoint);
            }
            new.sort_unstable_by(|a, b| round!(a.time).partial_cmp(&round!(b.time)).unwrap());
            for keypoint in new {
                n_hash!(
                    hasher,
                    round!(keypoint.time),
                    round!(keypoint.color.r),
                    round!(keypoint.color.g),
                    round!(keypoint.color.b)
                )
            }
        }
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
    n_hash!(hasher, round!(vector.x), round!(vector.y), round!(vector.z))
}
