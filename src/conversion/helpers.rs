// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_value::ValueRef;

use crate::config_reader::{ConfigReader, root_config};
use crate::options::ReadPolicy;
use crate::{ConfigResult, Property, utils};

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
    property
        .value()
        .as_scalar()
        .and_then(|value| match value.view() {
            ValueRef::String(value) => Some(value),
            _ => None,
        })
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
/// * `options` - Active read policy.
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
    options: &ReadPolicy,
) -> ConfigResult<bool> {
    is_effectively_missing_by(reader, name, property, options, false)
}

/// Checks whether an interpolated property should be treated as missing.
///
/// # Type Parameters
///
/// * `R` - Reader used to resolve placeholders.
///
/// # Parameters
///
/// * `reader` - Reader that owns the interpolation context.
/// * `name` - Root-relative property name used in diagnostics.
/// * `property` - Property to inspect.
/// * `options` - Active read policy and interpolation limits.
///
/// # Returns
///
/// `true` when the property is unset or its interpolated scalar string is
/// normalized as missing.
///
/// # Errors
///
/// Returns interpolation or string-normalization errors with key context.
pub(crate) fn is_effectively_missing_interpolated<R: ConfigReader + ?Sized>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ReadPolicy,
) -> ConfigResult<bool> {
    is_effectively_missing_by(reader, name, property, options, true)
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
/// * `options` - Active read policy.
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
    options: &ReadPolicy,
) -> ConfigResult<T>
where
    R: ConfigReader + ?Sized,
    T: FromConfig,
{
    parse_property_from_reader_by(reader, name, property, options, false)
}

/// Parses a property after interpolating its string values.
///
/// # Type Parameters
///
/// * `R` - Reader used to resolve placeholders.
/// * `T` - Target type parsed from the interpolated property.
///
/// # Parameters
///
/// * `reader` - Reader that owns the interpolation context.
/// * `name` - Root-relative property name used in diagnostics.
/// * `property` - Property to interpolate and parse.
/// * `options` - Active conversion options and interpolation limits.
///
/// # Returns
///
/// Parsed value after interpolation.
///
/// # Errors
///
/// Returns interpolation, missing-value, or conversion errors with key
/// context.
pub(crate) fn parse_property_from_reader_interpolated<R, T>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ReadPolicy,
) -> ConfigResult<T>
where
    R: ConfigReader + ?Sized,
    T: FromConfig,
{
    parse_property_from_reader_by(reader, name, property, options, true)
}

/// Checks whether a property is effectively missing after applying the active
/// substitution policy.
fn is_effectively_missing_by<R: ConfigReader + ?Sized>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ReadPolicy,
    interpolate: bool,
) -> ConfigResult<bool> {
    if property.is_unset() {
        return Ok(true);
    }
    let Some(value) = first_scalar_string(property) else {
        return Ok(false);
    };
    if !interpolate {
        return Ok(matches!(
            options
                .conversion_options()
                .string()
                .normalize_optional(value),
            Ok(None)
        ));
    }
    let substitute = |value: &str| substitute_for_reader(reader, name, value, options, interpolate);
    let ctx = ConfigParseContext::new(name, options, &substitute, interpolate);
    let value = ctx.substitute_string(value)?;
    match options
        .conversion_options()
        .string()
        .normalize_optional(&value)
    {
        Ok(Some(_)) | Err(_) => Ok(false),
        Ok(None) => Ok(true),
    }
}

/// Parses a property through a reader context using the active substitution
/// policy.
fn parse_property_from_reader_by<R, T>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ReadPolicy,
    interpolate: bool,
) -> ConfigResult<T>
where
    R: ConfigReader + ?Sized,
    T: FromConfig,
{
    let substitute = |value: &str| substitute_for_reader(reader, name, value, options, interpolate);
    let ctx = ConfigParseContext::new(name, options, &substitute, interpolate);
    T::from_config(property, &ctx)
}

/// Applies the active variable-substitution policy before typed conversion.
///
/// Placeholder lookup checks the current reader scope first, then the root
/// configuration, and finally the process environment when the policy permits
/// it. This matches structured deserialization for scoped readers.
///
/// # Parameters
///
/// * `reader` - Reader used to resolve variables.
/// * `path` - Configuration path whose value is being expanded.
/// * `value` - Source text.
/// * `options` - Active read and substitution options.
/// * `interpolate` - Whether placeholders should be resolved.
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
    options: &ReadPolicy,
    interpolate: bool,
) -> ConfigResult<String> {
    if interpolate {
        utils::substitute_variables_with_fallback(value, reader, root_config(reader), options, path)
    } else {
        Ok(value.to_string())
    }
}
