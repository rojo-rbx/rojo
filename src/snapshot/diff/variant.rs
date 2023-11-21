use blake3::Hasher;
use float_cmp::approx_eq;
use rbx_dom_weak::types::{PhysicalProperties, Variant, Vector3};

use std::collections::HashMap;

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
        Variant::Attributes(attrs) => {
            let mut sorted: Vec<(&String, &Variant)> = attrs.iter().collect();
            sorted.sort_unstable_by_key(|(name, _)| *name);
            for (name, attribute) in sorted {
                hasher.update(name.as_bytes());
                hash_variant(hasher, attribute);
            }
        }
        Variant::Axes(a) => hash!(hasher, &[a.bits()]),
        Variant::BinaryString(bytes) => hash!(hasher, bytes.as_ref()),
        Variant::Bool(bool) => hash!(hasher, &[*bool as u8]),
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
        Variant::Float32(n) => n_hash!(hasher, round!(*n)),
        Variant::Float64(n) => n_hash!(hasher, round!(n)),
        Variant::Font(f) => {
            n_hash!(hasher, f.weight as u16);
            n_hash!(hasher, f.style as u8);
            hash!(hasher, f.family.as_bytes());
            if let Some(cache) = &f.cached_face_id {
                hash!(hasher, &[0x01]);
                hash!(hasher, cache.as_bytes());
            } else {
                hash!(hasher, &[0x00]);
            }
        }
        Variant::Int32(n) => n_hash!(hasher, n),
        Variant::Int64(n) => n_hash!(hasher, n),
        Variant::MaterialColors(n) => hash!(hasher, n.encode().as_slice()),
        Variant::NumberRange(nr) => n_hash!(hasher, round!(nr.max), round!(nr.min)),
        Variant::NumberSequence(seq) => {
            let mut new = Vec::with_capacity(seq.keypoints.len());
            for keypoint in &seq.keypoints {
                new.push(keypoint);
            }
            new.sort_unstable_by(|a, b| round!(a.time).partial_cmp(&round!(b.time)).unwrap());
            for keypoint in new {
                n_hash!(
                    hasher,
                    round!(keypoint.time),
                    round!(keypoint.value),
                    round!(keypoint.envelope)
                )
            }
        }
        Variant::OptionalCFrame(maybe_cf) => {
            if let Some(cf) = maybe_cf {
                hash!(hasher, &[0x01]);
                vector_hash(hasher, cf.position);
                vector_hash(hasher, cf.orientation.x);
                vector_hash(hasher, cf.orientation.y);
                vector_hash(hasher, cf.orientation.z);
            } else {
                hash!(hasher, &[0x00]);
            }
        }
        Variant::PhysicalProperties(properties) => match properties {
            PhysicalProperties::Default => hash!(hasher, &[0x00]),
            PhysicalProperties::Custom(custom) => {
                hash!(hasher, &[0x00]);
                n_hash!(
                    hasher,
                    round!(custom.density),
                    round!(custom.friction),
                    round!(custom.elasticity),
                    round!(custom.friction_weight),
                    round!(custom.elasticity_weight)
                )
            }
        },
        Variant::Ray(ray) => {
            vector_hash(hasher, ray.origin);
            vector_hash(hasher, ray.direction);
        }
        Variant::Rect(rect) => n_hash!(
            hasher,
            round!(rect.max.x),
            round!(rect.max.y),
            round!(rect.min.x),
            round!(rect.min.y)
        ),
        Variant::Ref(referent) => hash!(hasher, referent.to_string().as_bytes()),
        Variant::Region3(region) => {
            vector_hash(hasher, region.max);
            vector_hash(hasher, region.min);
        }
        Variant::Region3int16(region) => {
            n_hash!(
                hasher,
                region.max.x,
                region.max.y,
                region.max.z,
                region.min.x,
                region.min.y,
                region.min.z
            )
        }
        Variant::SecurityCapabilities(capabilities) => n_hash!(hasher, capabilities.bits()),
        Variant::SharedString(sstr) => hash!(hasher, sstr.hash().as_bytes()),
        Variant::String(str) => hash!(hasher, str.as_bytes()),
        Variant::Tags(tags) => {
            let mut dupe: Vec<&str> = tags.iter().collect();
            dupe.sort_unstable();
            for tag in dupe {
                hash!(hasher, tag.as_bytes())
            }
        }
        Variant::UDim(udim) => n_hash!(hasher, round!(udim.scale), udim.offset),
        Variant::UDim2(udim) => n_hash!(
            hasher,
            round!(udim.y.scale),
            udim.y.offset,
            round!(udim.x.scale),
            udim.x.offset
        ),
        Variant::Vector2(v2) => n_hash!(hasher, round!(v2.x), round!(v2.y)),
        Variant::Vector2int16(v2) => n_hash!(hasher, v2.x, v2.y),
        Variant::Vector3(v3) => vector_hash(hasher, *v3),
        Variant::Vector3int16(v3) => n_hash!(hasher, v3.x, v3.y, v3.z),

        // Hashing UniqueId properties doesn't make sense
        Variant::UniqueId(_) => (),

        unknown => {
            log::warn!(
                "Encountered unknown Variant {:?} while hashing",
                unknown.ty()
            )
        }
    }
}

fn vector_hash(hasher: &mut Hasher, vector: Vector3) {
    n_hash!(hasher, round!(vector.x), round!(vector.y), round!(vector.z))
}

/// Compares to variants to determine if they're equal. This correctly takes
/// float comparisons into account.
pub fn variant_eq(variant_a: &Variant, variant_b: &Variant) -> bool {
    if variant_a.ty() != variant_b.ty() {
        return false;
    }

    match (variant_a, variant_b) {
        (Variant::Attributes(a), Variant::Attributes(b)) => {
            // If they're not the same size, we can just abort
            if a.iter().count() != b.iter().count() {
                return false;
            }
            // Using a duplicated map, we can determine if we have
            // mismatched keys between A and B
            let mut b_dupe = HashMap::with_capacity(b.iter().count());
            for (name, value) in b.iter() {
                b_dupe.insert(name, value);
            }
            for (name, a_value) in a.iter() {
                if let Some(b_value) = b.get(name.as_str()) {
                    if variant_eq(a_value, b_value) {
                        b_dupe.remove(name);
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            b_dupe.is_empty()
        }
        (Variant::Axes(a), Variant::Axes(b)) => a == b,
        (Variant::BinaryString(a), Variant::BinaryString(b)) => a == b,
        (Variant::Bool(a), Variant::Bool(b)) => a == b,
        (Variant::BrickColor(a), Variant::BrickColor(b)) => a == b,
        (Variant::CFrame(a), Variant::CFrame(b)) => {
            vector_eq(&a.position, &b.position)
                && vector_eq(&a.orientation.x, &b.orientation.x)
                && vector_eq(&a.orientation.y, &b.orientation.y)
                && vector_eq(&a.orientation.z, &b.orientation.z)
        }
        (Variant::Color3(a), Variant::Color3(b)) => {
            approx_eq!(f32, a.r, b.r) && approx_eq!(f32, a.b, b.b) && approx_eq!(f32, a.g, b.g)
        }
        (Variant::Color3uint8(a), Variant::Color3uint8(b)) => a == b,
        (Variant::ColorSequence(a), Variant::ColorSequence(b)) => {
            if a.keypoints.len() != b.keypoints.len() {
                return false;
            }
            let mut a_keypoints = Vec::with_capacity(a.keypoints.len());
            let mut b_keypoints = Vec::with_capacity(b.keypoints.len());
            for keypoint in &a.keypoints {
                a_keypoints.push(keypoint)
            }
            for keypoint in &b.keypoints {
                b_keypoints.push(keypoint)
            }
            a_keypoints.sort_unstable_by(|k1, k2| k1.time.partial_cmp(&k2.time).unwrap());
            b_keypoints.sort_unstable_by(|k1, k2| k1.time.partial_cmp(&k2.time).unwrap());
            for (a_kp, b_kp) in a_keypoints.iter().zip(b_keypoints) {
                if !(approx_eq!(f32, a_kp.time, b_kp.time)
                    && approx_eq!(f32, a_kp.color.r, b_kp.color.r)
                    && approx_eq!(f32, a_kp.color.g, b_kp.color.g)
                    && approx_eq!(f32, a_kp.color.b, b_kp.color.b))
                {
                    return false;
                }
            }
            true
        }
        (Variant::Content(a), Variant::Content(b)) => a == b,
        (Variant::Enum(a), Variant::Enum(b)) => a == b,
        (Variant::Faces(a), Variant::Faces(b)) => a == b,
        (Variant::Float32(a), Variant::Float32(b)) => approx_eq!(f32, *a, *b),
        (Variant::Float64(a), Variant::Float64(b)) => approx_eq!(f64, *a, *b),
        (Variant::Font(a), Variant::Font(b)) => {
            a.weight == b.weight
                && a.style == b.style
                && a.family == b.family
                && a.cached_face_id == b.cached_face_id
        }
        (Variant::Int32(a), Variant::Int32(b)) => a == b,
        (Variant::Int64(a), Variant::Int64(b)) => a == b,
        (Variant::MaterialColors(a), Variant::MaterialColors(b)) => a.encode() == b.encode(),
        (Variant::NumberRange(a), Variant::NumberRange(b)) => {
            approx_eq!(f32, a.max, b.max) && approx_eq!(f32, a.min, a.max)
        }
        (Variant::NumberSequence(a), Variant::NumberSequence(b)) => {
            if a.keypoints.len() != b.keypoints.len() {
                return false;
            }
            let mut a_keypoints = Vec::with_capacity(a.keypoints.len());
            let mut b_keypoints = Vec::with_capacity(b.keypoints.len());
            for keypoint in &a.keypoints {
                a_keypoints.push(keypoint)
            }
            for keypoint in &b.keypoints {
                b_keypoints.push(keypoint)
            }
            a_keypoints.sort_unstable_by(|k1, k2| k1.time.partial_cmp(&k2.time).unwrap());
            b_keypoints.sort_unstable_by(|k1, k2| k1.time.partial_cmp(&k2.time).unwrap());
            for (a_kp, b_kp) in a_keypoints.iter().zip(b_keypoints) {
                if !(approx_eq!(f32, a_kp.time, b_kp.time)
                    && approx_eq!(f32, a_kp.value, b_kp.value)
                    && approx_eq!(f32, a_kp.envelope, b_kp.envelope))
                {
                    return false;
                }
            }
            true
        }
        (Variant::OptionalCFrame(a), Variant::OptionalCFrame(b)) => {
            if let (Some(a2), Some(b2)) = (a, b) {
                vector_eq(&a2.position, &b2.position)
                    && vector_eq(&a2.orientation.x, &b2.orientation.x)
                    && vector_eq(&a2.orientation.y, &b2.orientation.y)
                    && vector_eq(&a2.orientation.z, &b2.orientation.z)
            } else {
                false
            }
        }
        (Variant::PhysicalProperties(a), Variant::PhysicalProperties(b)) => match (a, b) {
            (PhysicalProperties::Default, PhysicalProperties::Default) => true,
            (PhysicalProperties::Custom(a2), PhysicalProperties::Custom(b2)) => {
                approx_eq!(f32, a2.density, b2.density)
                    && approx_eq!(f32, a2.elasticity, b2.elasticity)
                    && approx_eq!(f32, a2.friction, b2.friction)
                    && approx_eq!(f32, a2.elasticity_weight, b2.elasticity_weight)
                    && approx_eq!(f32, a2.friction_weight, b2.friction_weight)
            }
            (_, _) => false,
        },
        (Variant::Ray(a), Variant::Ray(b)) => {
            vector_eq(&a.direction, &b.direction) && vector_eq(&a.origin, &b.origin)
        }
        (Variant::Rect(a), Variant::Rect(b)) => {
            approx_eq!(f32, a.max.x, b.max.x)
                && approx_eq!(f32, a.max.y, b.max.y)
                && approx_eq!(f32, a.min.x, b.min.x)
                && approx_eq!(f32, a.min.y, b.min.y)
        }
        (Variant::Ref(a), Variant::Ref(b)) => a == b,
        (Variant::Region3(a), Variant::Region3(b)) => {
            vector_eq(&a.max, &b.max) && vector_eq(&a.min, &b.min)
        }
        (Variant::Region3int16(a), Variant::Region3int16(b)) => a == b,
        (Variant::SecurityCapabilities(a), Variant::SecurityCapabilities(b)) => a == b,
        (Variant::SharedString(a), Variant::SharedString(b)) => a == b,
        (Variant::Tags(a), Variant::Tags(b)) => {
            let mut a_sorted: Vec<&str> = a.iter().collect();
            let mut b_sorted: Vec<&str> = b.iter().collect();
            if a_sorted.len() == b_sorted.len() {
                a_sorted.sort_unstable();
                b_sorted.sort_unstable();
                for (a_tag, b_tag) in a_sorted.into_iter().zip(b_sorted) {
                    if a_tag != b_tag {
                        return false;
                    }
                }
                true
            } else {
                false
            }
        }
        (Variant::UDim(a), Variant::UDim(b)) => {
            approx_eq!(f32, a.scale, b.scale) && a.offset == b.offset
        }
        (Variant::UDim2(a), Variant::UDim2(b)) => {
            approx_eq!(f32, a.x.scale, b.x.scale)
                && a.x.offset == b.x.offset
                && approx_eq!(f32, a.y.scale, b.y.scale)
                && a.y.offset == b.y.offset
        }
        (Variant::UniqueId(a), Variant::UniqueId(b)) => a == b,
        (Variant::String(a), Variant::String(b)) => a == b,
        (Variant::Vector2(a), Variant::Vector2(b)) => {
            approx_eq!(f32, a.x, b.x) && approx_eq!(f32, a.y, b.y)
        }
        (Variant::Vector2int16(a), Variant::Vector2int16(b)) => a == b,
        (Variant::Vector3(a), Variant::Vector3(b)) => vector_eq(a, b),
        (Variant::Vector3int16(a), Variant::Vector3int16(b)) => a == b,
        (a, b) => panic!(
            "unsupport variant comparison: {:?} and {:?}",
            a.ty(),
            b.ty()
        ),
    }
}

#[inline(always)]
fn vector_eq(a: &Vector3, b: &Vector3) -> bool {
    approx_eq!(f32, a.x, b.x) && approx_eq!(f32, a.y, b.y) && approx_eq!(f32, a.z, b.z)
}
