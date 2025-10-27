use std::borrow::Borrow;

use anyhow::{bail, format_err};
use rbx_dom_weak::types::{
    Attributes, CFrame, Color3, Content, ContentId, Enum, Font, MaterialColors, Matrix3, Tags,
    Variant, VariantType, Vector2, Vector3,
};
use rbx_reflection::{DataType, PropertyDescriptor};
use serde::{Deserialize, Serialize};

use crate::REF_POINTER_ATTRIBUTE_PREFIX;

/// A user-friendly version of `Variant` that supports specifying ambiguous
/// values. Ambiguous values need a reflection database to be resolved to a
/// usable value.
///
/// This type is used in Rojo projects and JSON models to make specifying the
/// most common types of properties, like strings or vectors, much easier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UnresolvedValue {
    FullyQualified(Variant),
    Ambiguous(AmbiguousValue),
}

impl UnresolvedValue {
    pub fn resolve(self, class_name: &str, prop_name: &str) -> anyhow::Result<Variant> {
        match self {
            UnresolvedValue::FullyQualified(full) => Ok(full),
            UnresolvedValue::Ambiguous(partial) => partial.resolve(class_name, prop_name),
        }
    }

    pub fn resolve_unambiguous(self) -> anyhow::Result<Variant> {
        match self {
            UnresolvedValue::FullyQualified(full) => Ok(full),
            UnresolvedValue::Ambiguous(partial) => partial.resolve_unambiguous(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AmbiguousValue {
    Bool(bool),
    String(String),
    StringArray(Vec<String>),
    Number(f64),
    Array2([f64; 2]),
    Array3([f64; 3]),
    Array4([f64; 4]),
    Array12([f64; 12]),
    Attributes(Attributes),
    Font(Font),
    MaterialColors(MaterialColors),
}

impl AmbiguousValue {
    pub fn resolve(self, class_name: &str, prop_name: &str) -> anyhow::Result<Variant> {
        let property = find_descriptor(class_name, prop_name)
            .ok_or_else(|| format_err!("Unknown property {}.{}", class_name, prop_name))?;

        match &property.data_type {
            DataType::Enum(enum_name) => {
                let database = rbx_reflection_database::get().unwrap();

                let enum_descriptor = database.enums.get(enum_name).ok_or_else(|| {
                    format_err!("Unknown enum {}. This is a Rojo bug!", enum_name)
                })?;

                let error = |what: &str| {
                    let mut all_values = enum_descriptor
                        .items
                        .keys()
                        .map(|value| value.borrow())
                        .collect::<Vec<_>>();
                    all_values.sort();

                    let examples = nonexhaustive_list(&all_values);

                    format_err!(
                        "Invalid value for property {}.{}. Got {} but \
                         expected a member of the {} enum such as {}",
                        class_name,
                        prop_name,
                        what,
                        enum_name,
                        examples,
                    )
                };

                let value = match self {
                    AmbiguousValue::String(value) => value,
                    unresolved => return Err(error(unresolved.describe())),
                };

                let resolved = enum_descriptor
                    .items
                    .get(value.as_str())
                    .ok_or_else(|| error(value.as_str()))?;

                Ok(Enum::from_u32(*resolved).into())
            }
            DataType::Value(variant_ty) => match (variant_ty, self) {
                (VariantType::Bool, AmbiguousValue::Bool(value)) => Ok(value.into()),

                (VariantType::Float32, AmbiguousValue::Number(value)) => Ok((value as f32).into()),
                (VariantType::Float64, AmbiguousValue::Number(value)) => Ok(value.into()),
                (VariantType::Int32, AmbiguousValue::Number(value)) => Ok((value as i32).into()),
                (VariantType::Int64, AmbiguousValue::Number(value)) => Ok((value as i64).into()),

                (VariantType::String, AmbiguousValue::String(value)) => Ok(value.into()),
                (VariantType::Tags, AmbiguousValue::StringArray(value)) => {
                    Ok(Tags::from(value).into())
                }
                (VariantType::Content, AmbiguousValue::String(value)) => {
                    Ok(Content::from(value).into())
                }
                (VariantType::ContentId, AmbiguousValue::String(value)) => {
                    Ok(ContentId::from(value).into())
                }

                (VariantType::Vector2, AmbiguousValue::Array2(value)) => {
                    Ok(Vector2::new(value[0] as f32, value[1] as f32).into())
                }

                (VariantType::Vector3, AmbiguousValue::Array3(value)) => {
                    Ok(Vector3::new(value[0] as f32, value[1] as f32, value[2] as f32).into())
                }

                (VariantType::Color3, AmbiguousValue::Array3(value)) => {
                    Ok(Color3::new(value[0] as f32, value[1] as f32, value[2] as f32).into())
                }

                (VariantType::CFrame, AmbiguousValue::Array12(value)) => {
                    let value = value.map(|v| v as f32);
                    let pos = Vector3::new(value[0], value[1], value[2]);
                    let orientation = Matrix3::new(
                        Vector3::new(value[3], value[4], value[5]),
                        Vector3::new(value[6], value[7], value[8]),
                        Vector3::new(value[9], value[10], value[11]),
                    );

                    Ok(CFrame::new(pos, orientation).into())
                }

                (VariantType::Attributes, AmbiguousValue::Attributes(value)) => Ok(value.into()),

                (VariantType::Font, AmbiguousValue::Font(value)) => Ok(value.into()),

                (VariantType::MaterialColors, AmbiguousValue::MaterialColors(value)) => {
                    Ok(value.into())
                }

                (VariantType::Ref, AmbiguousValue::String(_)) => Err(format_err!(
                    "Cannot resolve Ref properties as a String.\
                    Use an attribute named `{REF_POINTER_ATTRIBUTE_PREFIX}{prop_name}"
                )),
                (_, unresolved) => Err(format_err!(
                    "Wrong type of value for property {}.{}. Expected {:?}, got {}",
                    class_name,
                    prop_name,
                    variant_ty,
                    unresolved.describe(),
                )),
            },
            _ => Err(format_err!(
                "Unknown data type for property {}.{}",
                class_name,
                prop_name
            )),
        }
    }

    pub fn resolve_unambiguous(self) -> anyhow::Result<Variant> {
        match self {
            AmbiguousValue::Bool(value) => Ok(value.into()),
            AmbiguousValue::Number(value) => Ok(value.into()),
            AmbiguousValue::String(value) => Ok(value.into()),

            other => bail!("Cannot unambiguously resolve the value {other:?}"),
        }
    }

    fn describe(&self) -> &'static str {
        match self {
            AmbiguousValue::Bool(_) => "a bool",
            AmbiguousValue::String(_) => "a string",
            AmbiguousValue::StringArray(_) => "an array of strings",
            AmbiguousValue::Number(_) => "a number",
            AmbiguousValue::Array2(_) => "an array of two numbers",
            AmbiguousValue::Array3(_) => "an array of three numbers",
            AmbiguousValue::Array4(_) => "an array of four numbers",
            AmbiguousValue::Array12(_) => "an array of twelve numbers",
            AmbiguousValue::Attributes(_) => "an object containing attributes",
            AmbiguousValue::Font(_) => "an object describing a Font",
            AmbiguousValue::MaterialColors(_) => "an object describing MaterialColors",
        }
    }
}

fn find_descriptor(
    class_name: &str,
    prop_name: &str,
) -> Option<&'static PropertyDescriptor<'static>> {
    let database = rbx_reflection_database::get().unwrap();
    let mut current_class_name = class_name;

    loop {
        let class = database.classes.get(current_class_name)?;
        if let Some(descriptor) = class.properties.get(prop_name) {
            return Some(descriptor);
        }

        current_class_name = class.superclass.as_deref()?;
    }
}

/// Outputs a string containing up to MAX_ITEMS entries from the given list. If
/// there are more than MAX_ITEMS items, the number of remaining items will be
/// listed.
fn nonexhaustive_list(values: &[&str]) -> String {
    use std::fmt::Write;

    const MAX_ITEMS: usize = 8;

    let mut output = String::new();

    let last_index = values.len() - 1;
    let main_length = last_index.min(9);

    let main_list = &values[..main_length];
    for value in main_list {
        output.push_str(value);
        output.push_str(", ");
    }

    if values.len() > MAX_ITEMS {
        write!(output, "or {} more", values.len() - main_length).unwrap();
    } else {
        output.push_str("or ");
        output.push_str(values[values.len() - 1]);
    }

    output
}

#[cfg(test)]
mod test {
    use super::*;

    fn resolve(class: &str, prop: &str, json_value: &str) -> Variant {
        let unresolved: UnresolvedValue = serde_json::from_str(json_value).unwrap();
        unresolved.resolve(class, prop).unwrap()
    }

    fn resolve_unambiguous(json_value: &str) -> Variant {
        let unresolved: UnresolvedValue = serde_json::from_str(json_value).unwrap();
        unresolved.resolve_unambiguous().unwrap()
    }

    #[test]
    fn bools() {
        assert_eq!(resolve("BoolValue", "Value", "false"), Variant::Bool(false));

        // Script.Disabled is inherited from BaseScript
        assert_eq!(resolve("Script", "Disabled", "true"), Variant::Bool(true));

        assert_eq!(resolve_unambiguous("false"), Variant::Bool(false));
        assert_eq!(resolve_unambiguous("true"), Variant::Bool(true));
    }

    #[test]
    fn strings() {
        // String literals can stay as strings
        assert_eq!(
            resolve("StringValue", "Value", "\"Hello!\""),
            Variant::String("Hello!".into()),
        );

        // String literals can also turn into ContentId
        assert_eq!(
            resolve("Sky", "MoonTextureId", "\"rbxassetid://12345\""),
            Variant::ContentId("rbxassetid://12345".into()),
        );

        // String literals can turn into Content!
        assert_eq!(
            resolve(
                "MeshPart",
                "MeshContent",
                "\"rbxasset://totally-a-real-uri.tiff\""
            ),
            Variant::Content("rbxasset://totally-a-real-uri.tiff".into())
        );

        // What about BinaryString values? For forward-compatibility reasons, we
        // don't support any shorthands for BinaryString.
        //
        // assert_eq!(
        //     resolve("Folder", "Tags", "\"a\\u0000b\\u0000c\""),
        //     Variant::BinaryString(b"a\0b\0c".to_vec().into()),
        // );

        assert_eq!(
            resolve_unambiguous("\"Hello world!\""),
            Variant::String("Hello world!".into()),
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(
            resolve("Part", "CollisionGroupId", "123"),
            Variant::Int32(123),
        );

        assert_eq!(
            resolve("IntValue", "Value", "532413"),
            Variant::Int64(532413),
        );

        assert_eq!(resolve("Part", "Transparency", "1"), Variant::Float32(1.0));
        assert_eq!(resolve("NumberValue", "Value", "1"), Variant::Float64(1.0));

        assert_eq!(resolve_unambiguous("12.5"), Variant::Float64(12.5));
    }

    #[test]
    fn vectors() {
        assert_eq!(
            resolve("ParticleEmitter", "SpreadAngle", "[1, 2]"),
            Variant::Vector2(Vector2::new(1.0, 2.0)),
        );

        assert_eq!(
            resolve("Part", "Position", "[4, 5, 6]"),
            Variant::Vector3(Vector3::new(4.0, 5.0, 6.0)),
        );
    }

    #[test]
    fn colors() {
        assert_eq!(
            resolve("Part", "Color", "[1, 1, 1]"),
            Variant::Color3(Color3::new(1.0, 1.0, 1.0)),
        );

        // There aren't any user-facing Color3uint8 properties. If there are
        // some, we should treat them the same in the future.
    }

    #[test]
    fn enums() {
        assert_eq!(
            resolve("Lighting", "Technology", "\"Voxel\""),
            Variant::Enum(Enum::from_u32(1)),
        );
    }

    #[test]
    fn font() {
        use rbx_dom_weak::types::{FontStyle, FontWeight};

        assert_eq!(
            resolve(
                "TextLabel",
                "FontFace",
                r#"{"family": "rbxasset://fonts/families/RobotoMono.json", "weight": "Thin", "style": "Normal"}"#
            ),
            Variant::Font(Font {
                family: "rbxasset://fonts/families/RobotoMono.json".into(),
                weight: FontWeight::Thin,
                style: FontStyle::Normal,
                cached_face_id: None,
            })
        )
    }

    #[test]
    fn material_colors() {
        use rbx_dom_weak::types::{Color3uint8, TerrainMaterials};

        let mut material_colors = MaterialColors::new();
        material_colors.set_color(TerrainMaterials::Grass, Color3uint8::new(10, 20, 30));
        material_colors.set_color(TerrainMaterials::Asphalt, Color3uint8::new(40, 50, 60));
        material_colors.set_color(TerrainMaterials::LeafyGrass, Color3uint8::new(255, 155, 55));

        assert_eq!(
            resolve(
                "Terrain",
                "MaterialColors",
                r#"{
                    "Grass": [10, 20, 30],
                    "Asphalt": [40, 50, 60],
                    "LeafyGrass": [255, 155, 55]
                }"#
            ),
            Variant::MaterialColors(material_colors)
        )
    }
}
