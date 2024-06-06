use std::collections::HashMap;

use rbx_dom_weak::types::{PhysicalProperties, Variant, Vector3};

/// Accepts three argumets: a float type and two values to compare.
///
/// Returns a bool indicating whether they're equal. This accounts for NaN such
/// that `approx_eq!(f32, f32::NAN, f32::NAN)` is `true`.
macro_rules! approx_eq {
    ($Ty:ty, $a:expr, $b:expr) => {
        float_cmp::approx_eq!($Ty, $a, $b) || $a.is_nan() && $b.is_nan()
    };
}

/// Compares two variants to determine if they're equal. This correctly takes
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
        (Variant::OptionalCFrame(a), Variant::OptionalCFrame(b)) => match (a, b) {
            (Some(a), Some(b)) => {
                vector_eq(&a.position, &b.position)
                    && vector_eq(&a.orientation.x, &b.orientation.x)
                    && vector_eq(&a.orientation.y, &b.orientation.y)
                    && vector_eq(&a.orientation.z, &b.orientation.z)
            }
            (None, None) => true,
            _ => false,
        },
        (Variant::PhysicalProperties(a), Variant::PhysicalProperties(b)) => match (a, b) {
            (PhysicalProperties::Default, PhysicalProperties::Default) => true,
            (PhysicalProperties::Custom(a2), PhysicalProperties::Custom(b2)) => {
                approx_eq!(f32, a2.density, b2.density)
                    && approx_eq!(f32, a2.elasticity, b2.elasticity)
                    && approx_eq!(f32, a2.friction, b2.friction)
                    && approx_eq!(f32, a2.elasticity_weight, b2.elasticity_weight)
                    && approx_eq!(f32, a2.friction_weight, b2.friction_weight)
            }
            _ => false,
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
