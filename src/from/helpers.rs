// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_value::Value as QubitValue;

use crate::config_reader::ConfigReader;
use crate::options::ConfigReadOptions;
use crate::{
    ConfigResult,
    Property,
    utils,
};

use super::config_parse_context::ConfigParseContext;
use super::from_config::FromConfig;

/// Gets the property's single string value when it is a scalar string source.
///
/// # Parameters
///
/// * `property` - Property to inspect.
///
/// # Returns
///
/// Returns `Some(&str)` only when the property has scalar string shape.
pub(crate) fn first_scalar_string(property: &Property) -> Option<&str> {
    match property.value().as_scalar() {
        Some(QubitValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

/// Checks whether a property should be treated as missing for read operations.
///
/// # Type Parameters
///
/// * `R` - Reader used for variable substitution.
///
/// # Parameters
///
/// * `reader` - Reader that owns the substitution context.
/// * `name` - Root-relative property name used in diagnostics.
/// * `property` - Property to inspect.
/// * `options` - Active read options.
///
/// # Returns
///
/// Returns `true` when the property is unset or a scalar string is normalized
/// by the active string options as missing. A concrete empty collection is not
/// missing.
///
/// # Errors
///
/// Returns a keyed error when variable substitution fails or the active string
/// options reject the scalar string.
pub(crate) fn is_effectively_missing<R: ConfigReader + ?Sized>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ConfigReadOptions,
) -> ConfigResult<bool> {
    is_effectively_missing_by(reader, name, property, options, false)
}

/// Parses a property through a reader-created parsing context.
///
/// # Type Parameters
///
/// * `R` - Reader used for variable substitution.
/// * `T` - Target type parsed from the property.
///
/// # Parameters
///
/// * `reader` - Reader that owns the substitution context.
/// * `name` - Root-relative property name used in diagnostics.
/// * `property` - Property to parse.
/// * `options` - Active read options.
///
/// # Returns
///
/// Parsed value.
///
/// # Errors
///
/// Returns conversion, missing-value, or substitution errors with key context.
pub(crate) fn parse_property_from_reader<R, T>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ConfigReadOptions,
) -> ConfigResult<T>
where
    R: ConfigReader + ?Sized,
    T: FromConfig,
{
    parse_property_from_reader_by(reader, name, property, options, false)
}

/// Checks whether a property is missing after applying variable substitution.
pub(crate) fn is_effectively_missing_with_substitution<
    R: ConfigReader + ?Sized,
>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ConfigReadOptions,
) -> ConfigResult<bool> {
    is_effectively_missing_by(reader, name, property, options, true)
}

/// Parses a property and applies variable substitution to string values.
pub(crate) fn parse_property_from_reader_with_substitution<R, T>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ConfigReadOptions,
) -> ConfigResult<T>
where
    R: ConfigReader + ?Sized,
    T: FromConfig,
{
    parse_property_from_reader_by(reader, name, property, options, true)
}

/// Checks whether a property is effectively missing, optionally substituting
/// strings first.
fn is_effectively_missing_by<R: ConfigReader + ?Sized>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ConfigReadOptions,
    apply_substitution: bool,
) -> ConfigResult<bool> {
    if property.is_unset() {
        return Ok(true);
    }
    let Some(value) = first_scalar_string(property) else {
        return Ok(false);
    };
    let substitute = |value: &str| {
        substitute_for_reader(reader, name, value, apply_substitution)
    };
    let ctx = ConfigParseContext::new(name, options, &substitute);
    let value = ctx.substitute_string(value)?;
    match options.conversion_options().string().normalize(&value) {
        Ok(_) => Ok(false),
        Err(error) => Ok(error.is_missing()),
    }
}

/// Parses a property through a reader context, optionally substituting strings
/// first.
fn parse_property_from_reader_by<R, T>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ConfigReadOptions,
    apply_substitution: bool,
) -> ConfigResult<T>
where
    R: ConfigReader + ?Sized,
    T: FromConfig,
{
    let substitute = |value: &str| {
        substitute_for_reader(reader, name, value, apply_substitution)
    };
    let ctx = ConfigParseContext::new(name, options, &substitute);
    T::from_config(property, &ctx)
}

/// Applies variable substitution for explicit string reads.
///
/// # Parameters
///
/// * `reader` - Reader used to resolve variables.
/// * `path` - Configuration path whose value is being expanded.
/// * `value` - Source text.
/// * `apply_substitution` - Whether this read path requests substitution.
///
/// # Returns
///
/// Expanded or unchanged owned text.
///
/// # Errors
///
/// Returns keyed lookup, conversion, depth, or cycle errors.
#[inline]
fn substitute_for_reader<R: ConfigReader + ?Sized>(
    reader: &R,
    path: &str,
    value: &str,
    apply_substitution: bool,
) -> ConfigResult<String> {
    if apply_substitution && reader.is_enable_variable_substitution() {
        utils::substitute_variables(
            value,
            reader,
            reader.max_substitution_depth(),
            path,
        )
    } else {
        no_substitution(value)
    }
}

/// Returns `value` unchanged as an owned string.
///
/// # Parameters
///
/// * `value` - Text to clone.
///
/// # Returns
///
/// An owned copy of `value`.
#[inline(always)]
fn no_substitution(value: &str) -> ConfigResult<String> {
    Ok(value.to_string())
}
