//! Utilities for parsing JSON with comments (JSONC) and deserializing to Rust types.
//!
//! This module provides convenient wrappers around `jsonc_parser` and `serde_json`
//! to reduce boilerplate and improve ergonomics when working with JSONC files.

use anyhow::Context as _;
use serde::de::DeserializeOwned;

/// Parse JSONC text into a `serde_json::Value`.
///
/// This handles the common pattern of calling `jsonc_parser::parse_to_serde_value`
/// and unwrapping the `Option` with a clear error message.
///
/// # Errors
///
/// Returns an error if:
/// - The text is not valid JSONC
/// - The text contains no JSON value
pub fn parse_value(text: &str) -> anyhow::Result<serde_json::Value> {
    jsonc_parser::parse_to_serde_value(text, &Default::default())
        .context("Failed to parse JSONC")?
        .ok_or_else(|| anyhow::anyhow!("File contains no JSON value"))
}

/// Parse JSONC text into a `serde_json::Value` with a custom context message.
///
/// This is useful when you want to provide a specific error message that includes
/// additional information like the file path.
///
/// # Errors
///
/// Returns an error if:
/// - The text is not valid JSONC
/// - The text contains no JSON value
pub fn parse_value_with_context(
    text: &str,
    context: impl Fn() -> String,
) -> anyhow::Result<serde_json::Value> {
    jsonc_parser::parse_to_serde_value(text, &Default::default())
        .with_context(|| format!("{}: JSONC parse error", context()))?
        .ok_or_else(|| anyhow::anyhow!("{}: File contains no JSON value", context()))
}

/// Parse JSONC text and deserialize it into a specific type.
///
/// This combines parsing JSONC and deserializing into a single operation,
/// eliminating the need to manually chain `parse_to_serde_value` and `from_value`.
///
/// # Errors
///
/// Returns an error if:
/// - The text is not valid JSONC
/// - The text contains no JSON value
/// - The value cannot be deserialized into type `T`
pub fn from_str<T: DeserializeOwned>(text: &str) -> anyhow::Result<T> {
    let value = parse_value(text)?;
    serde_json::from_value(value).context("Failed to deserialize JSON")
}

/// Parse JSONC text and deserialize it into a specific type with a custom context message.
///
/// This is useful when you want to provide a specific error message that includes
/// additional information like the file path.
///
/// # Errors
///
/// Returns an error if:
/// - The text is not valid JSONC
/// - The text contains no JSON value
/// - The value cannot be deserialized into type `T`
pub fn from_str_with_context<T: DeserializeOwned>(
    text: &str,
    context: impl Fn() -> String,
) -> anyhow::Result<T> {
    let value = parse_value_with_context(text, &context)?;
    serde_json::from_value(value).with_context(|| format!("{}: Invalid JSON structure", context()))
}

/// Parse JSONC bytes into a `serde_json::Value` with a custom context message.
///
/// This handles UTF-8 conversion and JSONC parsing in one step.
///
/// # Errors
///
/// Returns an error if:
/// - The bytes are not valid UTF-8
/// - The text is not valid JSONC
/// - The text contains no JSON value
pub fn parse_value_from_slice_with_context(
    slice: &[u8],
    context: impl Fn() -> String,
) -> anyhow::Result<serde_json::Value> {
    let text = std::str::from_utf8(slice)
        .with_context(|| format!("{}: File is not valid UTF-8", context()))?;
    parse_value_with_context(text, context)
}

/// Parse JSONC bytes and deserialize it into a specific type.
///
/// This handles UTF-8 conversion, JSONC parsing, and deserialization in one step.
///
/// # Errors
///
/// Returns an error if:
/// - The bytes are not valid UTF-8
/// - The text is not valid JSONC
/// - The text contains no JSON value
/// - The value cannot be deserialized into type `T`
pub fn from_slice<T: DeserializeOwned>(slice: &[u8]) -> anyhow::Result<T> {
    let text = std::str::from_utf8(slice).context("File is not valid UTF-8")?;
    from_str(text)
}

/// Parse JSONC bytes and deserialize it into a specific type with a custom context message.
///
/// This handles UTF-8 conversion, JSONC parsing, and deserialization in one step.
///
/// # Errors
///
/// Returns an error if:
/// - The bytes are not valid UTF-8
/// - The text is not valid JSONC
/// - The text contains no JSON value
/// - The value cannot be deserialized into type `T`
pub fn from_slice_with_context<T: DeserializeOwned>(
    slice: &[u8],
    context: impl Fn() -> String,
) -> anyhow::Result<T> {
    let text = std::str::from_utf8(slice)
        .with_context(|| format!("{}: File is not valid UTF-8", context()))?;
    from_str_with_context(text, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_parse_value() {
        let value = parse_value(r#"{"foo": "bar"}"#).unwrap();
        assert_eq!(value["foo"], "bar");
    }

    #[test]
    fn test_parse_value_with_comments() {
        let value = parse_value(
            r#"{
            // This is a comment
            "foo": "bar" // Inline comment
        }"#,
        )
        .unwrap();
        assert_eq!(value["foo"], "bar");
    }

    #[test]
    fn test_parse_value_with_trailing_comma() {
        let value = parse_value(
            r#"{
            "foo": "bar",
            "baz": 123,
        }"#,
        )
        .unwrap();
        assert_eq!(value["foo"], "bar");
        assert_eq!(value["baz"], 123);
    }

    #[test]
    fn test_parse_value_empty() {
        let err = parse_value("").unwrap_err();
        assert!(err.to_string().contains("no JSON value"));
    }

    #[test]
    fn test_parse_value_invalid() {
        let err = parse_value("{invalid}").unwrap_err();
        assert!(err.to_string().contains("parse"));
    }

    #[test]
    fn test_parse_value_with_context() {
        let err = parse_value_with_context("{invalid}", || "test.json".to_string()).unwrap_err();
        assert!(err.to_string().contains("test.json"));
        assert!(err.to_string().contains("parse"));
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestStruct {
        foo: String,
        bar: i32,
    }

    #[test]
    fn test_from_str() {
        let result: TestStruct = from_str(r#"{"foo": "hello", "bar": 42}"#).unwrap();
        assert_eq!(
            result,
            TestStruct {
                foo: "hello".to_string(),
                bar: 42
            }
        );
    }

    #[test]
    fn test_from_str_with_comments() {
        let result: TestStruct = from_str(
            r#"{
            // Comment
            "foo": "hello",
            "bar": 42, // Trailing comma is fine
        }"#,
        )
        .unwrap();
        assert_eq!(
            result,
            TestStruct {
                foo: "hello".to_string(),
                bar: 42
            }
        );
    }

    #[test]
    fn test_from_str_invalid_type() {
        let err = from_str::<TestStruct>(r#"{"foo": "hello"}"#).unwrap_err();
        assert!(err.to_string().contains("deserialize"));
    }

    #[test]
    fn test_from_str_with_context() {
        let err = from_str_with_context::<TestStruct>(r#"{"foo": "hello"}"#, || {
            "config.json".to_string()
        })
        .unwrap_err();
        assert!(err.to_string().contains("config.json"));
        assert!(err.to_string().contains("Invalid JSON structure"));
    }

    #[test]
    fn test_parse_value_from_slice_with_context() {
        let err = parse_value_from_slice_with_context(b"{invalid}", || "test.json".to_string())
            .unwrap_err();
        assert!(err.to_string().contains("test.json"));
        assert!(err.to_string().contains("parse"));
    }

    #[test]
    fn test_parse_value_from_slice_with_context_invalid_utf8() {
        let err = parse_value_from_slice_with_context(&[0xFF, 0xFF], || "test.json".to_string())
            .unwrap_err();
        assert!(err.to_string().contains("test.json"));
        assert!(err.to_string().contains("UTF-8"));
    }

    #[test]
    fn test_from_slice() {
        let result: TestStruct = from_slice(br#"{"foo": "hello", "bar": 42}"#).unwrap();
        assert_eq!(
            result,
            TestStruct {
                foo: "hello".to_string(),
                bar: 42
            }
        );
    }

    #[test]
    fn test_from_slice_with_comments() {
        let result: TestStruct = from_slice(
            br#"{
            // Comment
            "foo": "hello",
            "bar": 42, // Trailing comma is fine
        }"#,
        )
        .unwrap();
        assert_eq!(
            result,
            TestStruct {
                foo: "hello".to_string(),
                bar: 42
            }
        );
    }

    #[test]
    fn test_from_slice_invalid_utf8() {
        let err = from_slice::<TestStruct>(&[0xFF, 0xFF]).unwrap_err();
        assert!(err.to_string().contains("UTF-8"));
    }

    #[test]
    fn test_from_slice_with_context() {
        let err = from_slice_with_context::<TestStruct>(br#"{"foo": "hello"}"#, || {
            "config.json".to_string()
        })
        .unwrap_err();
        assert!(err.to_string().contains("config.json"));
        assert!(err.to_string().contains("Invalid JSON structure"));
    }

    #[test]
    fn test_from_slice_with_context_invalid_utf8() {
        let err =
            from_slice_with_context::<TestStruct>(&[0xFF, 0xFF], || "config.json".to_string())
                .unwrap_err();
        assert!(err.to_string().contains("config.json"));
        assert!(err.to_string().contains("UTF-8"));
    }
}
