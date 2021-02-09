use anyhow::format_err;
use rbx_dom_weak::types::{Variant, VariantType, Vector2, Vector3};
use rbx_reflection::DataType;
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
        let database = rbx_reflection_database::get();
        let class = database
            .classes
            .get(class_name)
            .ok_or_else(|| format_err!("Unknown class {}", class_name))?;

        let property = class
            .properties
            .get(prop_name)
            .ok_or_else(|| format_err!("Unknown property {}.{}", class_name, prop_name))?;

        match &property.data_type {
            DataType::Enum(_enum_value) => todo!(),
            DataType::Value(variant_ty) => match (variant_ty, self) {
                (VariantType::Bool, AmbiguousValue::Bool(value)) => Ok(value.into()),

                (VariantType::Float32, AmbiguousValue::Number(value)) => Ok((value as f32).into()),
                (VariantType::Float64, AmbiguousValue::Number(value)) => Ok(value.into()),
                (VariantType::Int32, AmbiguousValue::Number(value)) => Ok((value as i32).into()),
                (VariantType::Int64, AmbiguousValue::Number(value)) => Ok((value as i64).into()),

                (VariantType::String, AmbiguousValue::String(value)) => Ok(value.into()),

                (VariantType::Vector2, AmbiguousValue::Array2(value)) => {
                    Ok(Vector2::new(value[0] as f32, value[1] as f32).into())
                }

                (VariantType::Vector3, AmbiguousValue::Array3(value)) => {
                    Ok(Vector3::new(value[0] as f32, value[1] as f32, value[2] as f32).into())
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
