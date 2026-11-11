// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Safe classification and path capture for persisted configuration fields.

use serde_json::Value;
use serde_path_to_error::Path;
use serde_path_to_error::Segment;
use serde_path_to_error::deserialize;

use super::ConfigWireFields;
use crate::ConfigWireDecodeError;
use crate::ConfigWireRepresentationErrorKind;

/// Checks the stable envelope and property metadata, then decodes the full
/// representation while retaining a safe JSON Pointer for nested failures.
///
/// # Parameters
/// - `value`: The admitted JSON value containing envelope and property fields.
///
/// # Returns
/// The decoded wire fields, or a safe representation error with a JSON Pointer.
///
/// # Errors
/// Returns an error when the envelope or property metadata has an invalid
/// shape, or when deserialization rejects a nested representation.
pub(in crate::config) fn decode_fields(value: Value) -> Result<ConfigWireFields, ConfigWireDecodeError> {
    check_wire_shape(&value)?;
    deserialize(value).map_err(|error| ConfigWireDecodeError::Representation {
        path: wire_pointer(error.path()),
        reason: ConfigWireRepresentationErrorKind::InvalidValue,
    })
}

/// Checks only the envelope and Property metadata described by the wire
/// contract. Nested values remain owned by their existing value decoder.
///
/// # Parameters
///
/// * `value` - Admitted JSON value whose envelope and property fields are
///   checked without inspecting nested payloads.
///
/// # Errors
///
/// Returns a classified representation error at the first invalid contract
/// path.
fn check_wire_shape(value: &Value) -> Result<(), ConfigWireDecodeError> {
    let Some(envelope) = value.as_object() else {
        return Err(representation("", ConfigWireRepresentationErrorKind::InvalidType));
    };

    for key in envelope.keys() {
        if !matches!(key.as_str(), "version" | "description" | "properties" | "read_options") {
            return Err(representation("", ConfigWireRepresentationErrorKind::UnknownField));
        }
    }

    if let Some(version) = envelope.get("version") {
        if !version.is_number() {
            return Err(representation(
                "/version",
                ConfigWireRepresentationErrorKind::InvalidType,
            ));
        }
        if version.as_u64().is_none_or(|number| number > u64::from(u8::MAX)) {
            return Err(representation(
                "/version",
                ConfigWireRepresentationErrorKind::InvalidValue,
            ));
        }
    }

    if let Some(description) = envelope.get("description")
        && !description.is_null()
        && !description.is_string()
    {
        return Err(representation(
            "/description",
            ConfigWireRepresentationErrorKind::InvalidType,
        ));
    }

    let Some(properties) = envelope.get("properties") else {
        return Ok(());
    };
    let Some(properties) = properties.as_object() else {
        return Err(representation(
            "/properties",
            ConfigWireRepresentationErrorKind::InvalidType,
        ));
    };

    for (key, property) in properties {
        check_property_wire_shape(key, property)?;
    }
    Ok(())
}

/// Checks one persisted property against the property wire contract.
///
/// # Parameters
///
/// * `key` - Persisted property name used to identify errors without exposing
///   the property's value.
/// * `property` - Admitted JSON value containing the property's wire fields.
///
/// # Errors
///
/// Returns a classified representation error at the first invalid property
/// field path.
fn check_property_wire_shape(key: &str, property: &Value) -> Result<(), ConfigWireDecodeError> {
    let property_path = pointer_child("/properties", key);
    let Some(fields) = property.as_object() else {
        return Err(representation(
            &property_path,
            ConfigWireRepresentationErrorKind::InvalidType,
        ));
    };

    for field in fields.keys() {
        if !matches!(field.as_str(), "name" | "value" | "description" | "is_final") {
            return Err(representation(
                &property_path,
                ConfigWireRepresentationErrorKind::UnknownField,
            ));
        }
    }

    for required in ["name", "value", "is_final"] {
        if !fields.contains_key(required) {
            return Err(representation(
                &pointer_child(&property_path, required),
                ConfigWireRepresentationErrorKind::MissingField,
            ));
        }
    }
    if !fields.get("name").is_some_and(Value::is_string) {
        return Err(representation(
            &pointer_child(&property_path, "name"),
            ConfigWireRepresentationErrorKind::InvalidType,
        ));
    }
    if !fields.get("is_final").is_some_and(Value::is_boolean) {
        return Err(representation(
            &pointer_child(&property_path, "is_final"),
            ConfigWireRepresentationErrorKind::InvalidType,
        ));
    }
    if let Some(description) = fields.get("description")
        && !description.is_null()
        && !description.is_string()
    {
        return Err(representation(
            &pointer_child(&property_path, "description"),
            ConfigWireRepresentationErrorKind::InvalidType,
        ));
    }
    Ok(())
}

/// Encodes Serde's path segments as an RFC 6901 JSON Pointer.
///
/// # Parameters
///
/// * `path` - Serde path identifying the rejected wire field.
///
/// # Returns
///
/// A JSON Pointer containing only recognized wire-schema segments.
#[must_use]
fn wire_pointer(path: &Path) -> String {
    let mut pointer = String::new();
    let mut first_segment = true;
    let mut expect_property_name = false;
    for segment in path {
        match segment {
            Segment::Map { key } => {
                if expect_property_name {
                    pointer = pointer_child(&pointer, key);
                    expect_property_name = false;
                    continue;
                }
                if !is_safe_wire_field(key) {
                    break;
                }
                pointer = pointer_child(&pointer, key);
                if first_segment && key == "properties" {
                    expect_property_name = true;
                }
                first_segment = false;
            }
            Segment::Seq { index } => pointer = pointer_child(&pointer, &index.to_string()),
            Segment::Enum { variant } => {
                if !is_safe_wire_field(variant) {
                    break;
                }
                pointer = pointer_child(&pointer, variant);
                first_segment = false;
            }
            Segment::Unknown => break,
        }
    }
    pointer
}

/// Returns whether a path segment names a field in the persisted wire schema.
///
/// Unknown segments can be keys from arbitrary JSON values and may contain
/// user data, so diagnostic paths stop before including them.
///
/// # Parameters
/// - `segment`: Path segment to check against the persisted wire fields.
///
/// # Returns
///
/// `true` when the segment is part of the persisted wire schema.
#[must_use]
fn is_safe_wire_field(segment: &str) -> bool {
    matches!(
        segment,
        "version"
            | "description"
            | "properties"
            | "read_options"
            | "name"
            | "value"
            | "is_final"
            | "scalar"
            | "collection"
            | "bool"
            | "char"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "int128"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "uint128"
            | "float32"
            | "float64"
            | "biginteger"
            | "bigdecimal"
            | "string"
            | "date"
            | "time"
            | "datetime"
            | "instant"
            | "duration"
            | "url"
            | "stringmap"
            | "json"
    )
}

/// Appends one escaped JSON Pointer segment.
///
/// # Parameters
///
/// * `parent` - Existing JSON Pointer prefix.
/// * `segment` - Raw path segment to escape and append.
///
/// # Returns
///
/// A new pointer with `segment` escaped according to RFC 6901.
#[must_use]
fn pointer_child(parent: &str, segment: &str) -> String {
    let escaped = segment.replace('~', "~0").replace('/', "~1");
    format!("{parent}/{escaped}")
}

/// Builds a representation error without retaining rejected input values.
///
/// # Parameters
///
/// * `path` - JSON Pointer locating the wire-contract violation.
/// * `reason` - Safe classification of the rejected value or field.
///
/// # Returns
///
/// A representation error that retains the safe path and reason.
fn representation(path: &str, reason: ConfigWireRepresentationErrorKind) -> ConfigWireDecodeError {
    ConfigWireDecodeError::Representation {
        path: path.to_owned(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::check_wire_shape;
    use crate::ConfigWireDecodeError;
    use crate::ConfigWireRepresentationErrorKind as Kind;

    #[test]
    fn test_classifies_invalid_wire_shapes_at_their_contract_paths() {
        let cases = [
            (json!(null), "", Kind::InvalidType),
            (json!({"unexpected": 1}), "", Kind::UnknownField),
            (json!({"version": "1"}), "/version", Kind::InvalidType),
            (json!({"version": 256}), "/version", Kind::InvalidValue),
            (json!({"description": false}), "/description", Kind::InvalidType),
            (json!({"properties": []}), "/properties", Kind::InvalidType),
            (json!({"properties": {"a": false}}), "/properties/a", Kind::InvalidType),
            (
                json!({"properties": {"a": {"name": "a", "value": null, "is_final": false, "extra": 1}}}),
                "/properties/a",
                Kind::UnknownField,
            ),
            (
                json!({"properties": {"a": {"name": "a", "is_final": false}}}),
                "/properties/a/value",
                Kind::MissingField,
            ),
            (
                json!({"properties": {"a": {"name": 1, "value": null, "is_final": false}}}),
                "/properties/a/name",
                Kind::InvalidType,
            ),
            (
                json!({"properties": {"a": {"name": "a", "value": null, "is_final": 1}}}),
                "/properties/a/is_final",
                Kind::InvalidType,
            ),
            (
                json!({"properties": {"a": {"name": "a", "value": null, "is_final": false, "description": 1}}}),
                "/properties/a/description",
                Kind::InvalidType,
            ),
        ];

        for (value, expected_path, expected_kind) in cases {
            let error = check_wire_shape(&value).expect_err("the malformed shape should be rejected");
            assert!(
                matches!(error, ConfigWireDecodeError::Representation { ref path, reason }
                    if path == expected_path && reason == expected_kind),
                "unexpected shape error: {error:?}"
            );
        }

        assert!(check_wire_shape(&json!({"description": null})).is_ok());
        assert!(
            check_wire_shape(&json!({"properties": {"a": {
                "name": "a", "value": null, "is_final": false, "description": null
            }}}))
            .is_ok()
        );
    }
}
