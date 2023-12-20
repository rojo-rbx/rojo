use blake3::Hasher;
use rbx_dom_weak::types::{PhysicalProperties, Variant, Vector3};

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
