use anyhow::format_err;
use rbx_dom_weak::types::{
    Color3, Color3uint8, Content, Enum, Variant, VariantType, Vector2, Vector3,
};
use rbx_reflection::{DataType, PropertyDescriptor};
use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AmbiguousValue {
    Bool(bool),
    String(String),
    Number(f64),
    Array2([f64; 2]),
    Array3([f64; 3]),
    Array4([f64; 4]),
    Array16([f64; 16]),
}

impl AmbiguousValue {
    pub fn resolve(self, class_name: &str, prop_name: &str) -> anyhow::Result<Variant> {
        let property = find_descriptor(class_name, prop_name)
            .ok_or_else(|| format_err!("Unknown property {}.{}", class_name, prop_name))?;

        match &property.data_type {
            DataType::Enum(enum_name) => {
                let database = rbx_reflection_database::get();

                let enum_descriptor = database.enums.get(enum_name).ok_or_else(|| {
                    format_err!("Unknown enum {}. This is a Rojo bug!", enum_name)
                })?;

                let error = |what: &str| {
                    let sample_values = enum_descriptor
                        .items
                        .keys()
                        .take(3)
                        .map(|name| format!(r#""{}""#, name))
                        .collect::<Vec<_>>()
                        .join(", ");

                    format_err!(
                        "Invalid value for property {}.{}. Got {} but \
                         expected a member of the {} enum such as {}",
                        class_name,
                        prop_name,
                        what,
                        enum_name,
                        sample_values
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
                (VariantType::Content, AmbiguousValue::String(value)) => {
                    Ok(Content::from(value).into())
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
                (VariantType::Color3uint8, AmbiguousValue::Array3(value)) => {
                    let value = Color3uint8::new(
                        (value[0] / 255.0) as u8,
                        (value[1] / 255.0) as u8,
                        (value[2] / 255.0) as u8,
                    );

                    Ok(value.into())
                }

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

    fn describe(&self) -> &'static str {
        match self {
            AmbiguousValue::Bool(_) => "a bool",
            AmbiguousValue::String(_) => "a string",
            AmbiguousValue::Number(_) => "a number",
            AmbiguousValue::Array2(_) => "an array of two numbers",
            AmbiguousValue::Array3(_) => "an array of three numbers",
            AmbiguousValue::Array4(_) => "an array of four numbers",
            AmbiguousValue::Array16(_) => "an array of 16 numbers",
        }
    }
}

fn find_descriptor(
    class_name: &str,
    prop_name: &str,
) -> Option<&'static PropertyDescriptor<'static>> {
    let database = rbx_reflection_database::get();
    let mut current_class_name = class_name;

    loop {
        let class = database.classes.get(current_class_name)?;
        if let Some(descriptor) = class.properties.get(prop_name) {
            return Some(descriptor);
        }

        current_class_name = class.superclass.as_deref()?;
    }
}
