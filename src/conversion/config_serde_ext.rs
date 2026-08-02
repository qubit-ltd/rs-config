// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Serde-based structured reads for configuration readers.

use serde::de::DeserializeOwned;
use serde_json::{
    Map,
    Value as JsonValue,
};

use crate::config_reader::{
    ConfigReader,
    root_config,
};
use crate::config_value_deserializer::ConfigValueDeserializer;
use crate::options::ReadPolicy;
use crate::utils;
use crate::{
    ConfigError,
    ConfigResult,
    Property,
};

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
    /// Returns lookup, conversion, key-conflict, or sanitized Serde errors
    /// with root-relative configuration paths.
    fn deserialize<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        deserialize_by(self, prefix, false)
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
    /// key-conflict, or sanitized Serde errors with root-relative paths.
    fn deserialize_interpolated<T>(&self, prefix: &str) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        deserialize_by(self, prefix, true)
    }
}

impl<R> ConfigSerdeExt for R where R: ConfigReader + ?Sized {}

/// Deserializes a reader-selected value with explicit interpolation behavior.
///
/// # Errors
///
/// Returns structured configuration or sanitized Serde errors.
fn deserialize_by<R, T>(
    reader: &R,
    prefix: &str,
    interpolate: bool,
) -> ConfigResult<T>
where
    R: ConfigReader + ?Sized,
    T: DeserializeOwned,
{
    let path = reader.resolve_key(prefix)?;
    let value = deserialize_root_value(reader, prefix, interpolate)?;
    match T::deserialize(ConfigValueDeserializer::new(
        value,
        path.clone(),
        reader.read_policy(),
    )) {
        Ok(value) => Ok(value),
        Err(error) => Err(error.into_config_error(&path)),
    }
}

/// Selects an exact property or constructs a subtree root for deserialization.
///
/// # Errors
///
/// Returns a key conflict when an exact property and descendants coexist, or
/// propagates conversion and interpolation errors.
fn deserialize_root_value<R>(
    reader: &R,
    prefix: &str,
    interpolate: bool,
) -> ConfigResult<JsonValue>
where
    R: ConfigReader + ?Sized,
{
    if prefix.is_empty() {
        return deserialize_subtree_value(reader, prefix, interpolate);
    }

    let exact = reader.get_property(prefix)?;
    let has_children = reader.iter().any(|(key, _)| is_child_key(key, prefix));
    match (exact, has_children) {
        (Some(_), true) => Err(ConfigError::KeyConflict {
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
    if scalar_string_is_missing_for_deserialize(
        primary,
        fallback,
        path,
        property,
        primary.read_policy(),
        interpolate,
    )? {
        return Ok(JsonValue::Null);
    }

    let mut value =
        utils::property_to_json_value(property, path, primary.read_policy())?;
    if interpolate {
        utils::substitute_json_strings_with_fallback(
            &mut value, path, primary, fallback,
        )?;
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
    let mut properties = subtree.iter().collect::<Vec<_>>();
    properties.sort_by_key(|(key, _)| *key);

    let mut map = Map::new();
    for (key, property) in properties {
        let path = property.name();
        if scalar_string_is_missing_for_deserialize(
            &subtree,
            fallback,
            path,
            property,
            subtree.read_policy(),
            interpolate,
        )? {
            continue;
        }

        let mut value = utils::property_to_json_value(
            property,
            path,
            subtree.read_policy(),
        )?;
        if interpolate {
            utils::substitute_json_strings_with_fallback(
                &mut value, path, &subtree, fallback,
            )?;
        }
        utils::insert_deserialize_value(&mut map, key, value)?;
    }
    Ok(JsonValue::Object(map))
}

/// Returns whether a scalar string is missing under deserialization options.
///
/// # Errors
///
/// Returns interpolation or string-normalization errors with `path` context.
fn scalar_string_is_missing_for_deserialize<P, F>(
    primary: &P,
    fallback: &F,
    path: &str,
    property: &Property,
    options: &ReadPolicy,
    interpolate: bool,
) -> ConfigResult<bool>
where
    P: ConfigReader + ?Sized,
    F: ConfigReader + ?Sized,
{
    let Some(value) = property.value().as_scalar() else {
        return Ok(false);
    };
    let qubit_value::ValueRef::String(value) = value.view() else {
        return Ok(false);
    };
    let value = if interpolate {
        utils::substitute_variables_with_fallback(
            value, primary, fallback, options, path,
        )?
    } else {
        value.to_string()
    };
    match options
        .conversion_options()
        .string()
        .normalize_optional(&value)
    {
        Ok(Some(_)) => Ok(false),
        Ok(None) => Ok(true),
        Err(error) => Err(ConfigError::from_data_conversion_error(
            path,
            error.into_data_conversion_error(qubit_datatype::DataType::String),
        )),
    }
}

/// Returns `true` when `key` is strictly below `prefix`.
fn is_child_key(key: &str, prefix: &str) -> bool {
    key.len() > prefix.len()
        && key.starts_with(prefix)
        && key.as_bytes().get(prefix.len()) == Some(&b'.')
}
