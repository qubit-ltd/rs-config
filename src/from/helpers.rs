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
    is_effectively_missing_by(reader, name, property, options)
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
    parse_property_from_reader_by(reader, name, property, options)
}

/// Checks whether a property is effectively missing after applying the active
/// substitution policy.
fn is_effectively_missing_by<R: ConfigReader + ?Sized>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ConfigReadOptions,
) -> ConfigResult<bool> {
    if property.is_unset() {
        return Ok(true);
    }
    let Some(value) = first_scalar_string(property) else {
        return Ok(false);
    };
    let substitute =
        |value: &str| substitute_for_reader(reader, name, value, options);
    let ctx = ConfigParseContext::new(name, options, &substitute);
    let value = ctx.substitute_string(value)?;
    match options.conversion_options().string().normalize(&value) {
        Ok(_) => Ok(false),
        Err(error) => Ok(error.is_missing()),
    }
}

/// Parses a property through a reader context using the active substitution
/// policy.
fn parse_property_from_reader_by<R, T>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ConfigReadOptions,
) -> ConfigResult<T>
where
    R: ConfigReader + ?Sized,
    T: FromConfig,
{
    let substitute =
        |value: &str| substitute_for_reader(reader, name, value, options);
    let ctx = ConfigParseContext::new(name, options, &substitute);
    T::from_config(property, &ctx)
}

/// Applies the active variable-substitution policy before typed conversion.
///
/// # Parameters
///
/// * `reader` - Reader used to resolve variables.
/// * `path` - Configuration path whose value is being expanded.
/// * `value` - Source text.
/// * `options` - Active read and substitution options.
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
    options: &ConfigReadOptions,
) -> ConfigResult<String> {
    let substitution = options.substitution();
    if substitution.is_enabled() {
        utils::substitute_variables(value, reader, substitution, path)
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
