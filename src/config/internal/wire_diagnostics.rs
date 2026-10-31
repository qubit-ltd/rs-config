// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Safe classification and path capture for persisted configuration fields.

use serde_json::Value;

use super::ConfigWireFields;
use crate::ConfigWireDecodeError;
use crate::ConfigWireRepresentationErrorKind;

/// Checks the stable envelope and property metadata, then decodes the full
/// representation while retaining a safe JSON Pointer for nested failures.
pub(in crate::config) fn decode_fields(value: Value) -> Result<ConfigWireFields, ConfigWireDecodeError> {
    check_wire_shape(&value)?;
    serde_path_to_error::deserialize(value).map_err(|error| ConfigWireDecodeError::Representation {
        path: wire_pointer(error.path()),
        reason: ConfigWireRepresentationErrorKind::InvalidValue,
    })
}

/// Checks only the envelope and Property metadata described by the wire
/// contract. Nested values remain owned by their existing value decoder.
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
    }
    Ok(())
}

/// Encodes Serde's path segments as an RFC 6901 JSON Pointer.
fn wire_pointer(path: &serde_path_to_error::Path) -> String {
    let mut pointer = String::new();
    for segment in path {
        match segment {
            serde_path_to_error::Segment::Map { key } => pointer = pointer_child(&pointer, key),
            serde_path_to_error::Segment::Seq { index } => pointer = pointer_child(&pointer, &index.to_string()),
            serde_path_to_error::Segment::Enum { variant } => pointer = pointer_child(&pointer, variant),
            serde_path_to_error::Segment::Unknown => break,
        }
    }
    pointer
}

/// Appends one escaped JSON Pointer segment.
fn pointer_child(parent: &str, segment: &str) -> String {
    let escaped = segment.replace('~', "~0").replace('/', "~1");
    format!("{parent}/{escaped}")
}

/// Builds a representation error without retaining rejected input values.
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
    fn classifies_invalid_wire_shapes_at_their_contract_paths() {
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
