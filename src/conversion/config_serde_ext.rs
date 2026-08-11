// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Serde-based structured reads for configuration readers.

use qubit_datatype::ConversionSession;
use qubit_datatype::DataType;
use serde::de::DeserializeOwned;
use serde_json::Map;
use serde_json::Value as JsonValue;

use crate::ConfigError;
use crate::ConfigResult;
use crate::Property;
use crate::config_reader::ConfigReader;
use crate::config_reader::root_config;
use crate::config_value_deserializer::ConfigValueDeserializer;
use crate::options::ReadPolicy;
use crate::utils;

/// Adds Serde-based structured reads to every supported configuration reader.
///
/// The trait is implemented for all [`ConfigReader`] values, including
/// [`crate::Config`] and [`crate::ConfigSection`]. Prefixes are relative to the
/// current reader scope. Import this trait when calling its methods through a
/// generic reader or a section.
pub trait ConfigSerdeExt: ConfigReader {
    /// Deserializes an exact property or subtree without interpolation.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Owned target type accepted by Serde.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Exact property key or dotted subtree path relative to this
    ///   reader. An empty prefix selects all properties visible to the reader.
    ///
    /// # Returns
    ///
    /// The deserialized value.
    ///
    /// # Errors
    ///
    /// Returns lookup, conversion, key-conflict, unknown-property, or
    /// sanitized Serde errors with root-relative configuration paths. Unknown
    /// fields are rejected; declare them on `T` or call
    /// [`Self::deserialize_lenient`] explicitly.
    fn deserialize<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        deserialize_by(self, prefix, false, UnknownPropertyMode::Reject)
    }

    /// Deserializes an exact property or subtree after interpolating strings.
    ///
    /// Placeholder lookup first uses the selected scope, then the root
    /// configuration, and finally the process environment when enabled by the
    /// active [`ReadPolicy`]'s [`crate::options::InterpolationSources`].
    ///
    /// # Type Parameters
    ///
    /// * `T` - Owned target type accepted by Serde.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Exact property key or dotted subtree path relative to this
    ///   reader. An empty prefix selects all properties visible to the reader.
    ///
    /// # Returns
    ///
    /// The interpolated and deserialized value.
    ///
    /// # Errors
    ///
    /// Returns lookup, interpolation, resource-limit, conversion,
    /// key-conflict, unknown-property, or sanitized Serde errors with
    /// root-relative paths. Unknown fields are rejected by default.
    fn deserialize_interpolated<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        deserialize_by(self, prefix, true, UnknownPropertyMode::Reject)
    }

    /// Deserializes an exact property or subtree without interpolation while
    /// ignoring fields not consumed by the target type.
    ///
    /// This method is intended for explicitly open or partially consumed
    /// configuration sections. Use [`Self::deserialize`] when unknown fields
    /// should be rejected.
    fn deserialize_lenient<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        deserialize_by(self, prefix, false, UnknownPropertyMode::Ignore)
    }

    /// Deserializes an exact property or subtree after interpolation while
    /// ignoring fields not consumed by the target type.
    ///
    /// Use this only when the target intentionally permits additional
    /// configuration fields; strict reads are the default contract.
    fn deserialize_interpolated_lenient<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        deserialize_by(self, prefix, true, UnknownPropertyMode::Ignore)
    }
}

impl<R> ConfigSerdeExt for R where R: ConfigReader + ?Sized {}

#[derive(Clone, Copy)]
enum UnknownPropertyMode {
    Reject,
    Ignore,
}

/// Deserializes a reader-selected value with explicit interpolation behavior.
///
/// # Errors
///
/// Returns structured configuration or sanitized Serde errors.
fn deserialize_by<R, T>(
    reader: &R,
    prefix: &str,
    interpolate: bool,
    unknown_mode: UnknownPropertyMode,
) -> ConfigResult<T>
where
    R: ConfigReader + ?Sized,
    T: DeserializeOwned,
{
    let path = reader.resolve_key(prefix)?;
    let value = deserialize_root_value(reader, prefix, interpolate)?;
    let options = reader.read_policy();
    let mut session =
        ConversionSession::new(options.conversion_policy(), options.conversion_limits());
    let deserializer = ConfigValueDeserializer::new(value, path.clone(), options, &mut session);
    let mut ignored = Vec::new();
    let result = match unknown_mode {
        UnknownPropertyMode::Reject => serde_ignored::deserialize(deserializer, |ignored_path| {
            ignored.push(ignored_path.to_string())
        }),
        UnknownPropertyMode::Ignore => T::deserialize(deserializer),
    };
    match result {
        Ok(value) if ignored.is_empty() => Ok(value),
        Ok(_) => {
            ignored.sort();
            ignored.dedup();
            let paths = ignored
                .into_iter()
                .map(|ignored_path| join_ignored_path(&path, &ignored_path))
                .collect();
            Err(ConfigError::UnknownProperties { paths })
        }
        Err(error) => Err(error.into_config_error(&path)),
    }
}

/// Joins a Serde ignored path to the reader's root-relative path.
fn join_ignored_path(prefix: &str, ignored_path: &str) -> String {
    let ignored_path = ignored_path.trim_start_matches('.');
    if prefix.is_empty() {
        ignored_path.to_string()
    } else if ignored_path.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}.{ignored_path}")
    }
}

/// Selects an exact property or constructs a subtree root for deserialization.
///
/// # Errors
///
/// Returns a key conflict when an exact property and descendants coexist, or
/// propagates conversion and interpolation errors.
fn deserialize_root_value<R>(reader: &R, prefix: &str, interpolate: bool) -> ConfigResult<JsonValue>
where
    R: ConfigReader + ?Sized,
{
    if prefix.is_empty() {
        return deserialize_subtree_value(reader, prefix, interpolate);
    }

    let exact = reader.get_property(prefix)?;
    let has_children = reader.contains_section(prefix)?;
    match (exact, has_children) {
        (Some(_), true) => Err(ConfigError::KeyConflict {
            source_id: None,
            path: reader.resolve_key(prefix)?,
            existing: "exact value".to_string(),
            incoming: "nested child keys".to_string(),
        }),
        (Some(property), false) => deserialize_exact_value(
            reader,
            root_config(reader),
            property.name(),
            property,
            interpolate,
        ),
        (None, _) => deserialize_subtree_value(reader, prefix, interpolate),
    }
}

/// Projects one exact property into the JSON-like Serde representation.
///
/// # Errors
///
/// Returns conversion, normalization, or interpolation errors for `property`.
fn deserialize_exact_value<P, F>(
    primary: &P,
    fallback: &F,
    path: &str,
    property: &Property,
    interpolate: bool,
) -> ConfigResult<JsonValue>
where
    P: ConfigReader + ?Sized,
    F: ConfigReader + ?Sized,
{
    let value = utils::prepare_deserialize_value(property, path, primary, fallback, interpolate)?;
    if prepared_scalar_string_is_missing_for_deserialize(
        &value,
        path,
        property,
        primary.read_policy(),
    )? {
        return Ok(JsonValue::Null);
    }
    Ok(value)
}

/// Builds a JSON object from the descendants visible below `prefix`.
///
/// # Errors
///
/// Returns dotted-key conflicts or propagates conversion and interpolation
/// errors from the active reader.
fn deserialize_subtree_value<R>(
    reader: &R,
    prefix: &str,
    interpolate: bool,
) -> ConfigResult<JsonValue>
where
    R: ConfigReader + ?Sized,
{
    let subtree = reader.section(prefix)?;
    let fallback = root_config(reader);
    let mut map = Map::new();
    for (key, property) in subtree.iter() {
        let path = property.name();
        let value =
            utils::prepare_deserialize_value(property, path, &subtree, fallback, interpolate)?;
        if prepared_scalar_string_is_missing_for_deserialize(
            &value,
            path,
            property,
            subtree.read_policy(),
        )? {
            continue;
        }
        utils::insert_deserialize_value(&mut map, key, value)?;
    }
    Ok(JsonValue::Object(map))
}

/// Returns whether a prepared scalar string is missing under read options.
///
/// # Errors
///
/// Returns string-normalization errors with `path` context.
fn prepared_scalar_string_is_missing_for_deserialize(
    value: &JsonValue,
    path: &str,
    property: &Property,
    options: &ReadPolicy,
) -> ConfigResult<bool> {
    if property.value().as_scalar().is_none() {
        return Ok(false);
    }
    let JsonValue::String(value) = value else {
        return Ok(false);
    };
    match options
        .conversion_policy()
        .string()
        .normalize_optional(value)
    {
        Ok(Some(_)) => Ok(false),
        Ok(None) => Ok(true),
        Err(error) => Err(ConfigError::from_data_conversion_error(
            path,
            error.into_data_conversion_error(DataType::String),
        )),
    }
}
