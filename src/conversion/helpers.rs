// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::RefCell;

use qubit_value::ValueRef;

use super::config_parse_context::ConfigParseContext;
use super::from_config::FromConfig;
use crate::ConfigResult;
use crate::Property;
use crate::config_reader::ConfigReader;
use crate::config_reader::root_config;
use crate::options::ReadPolicy;
use crate::utils;
use crate::utils::InterpolationSession;

/// Returns the property's scalar string when it has string scalar shape.
///
/// # Parameters
///
/// * `property` - Property whose scalar value is inspected without conversion.
///
/// # Returns
///
/// The scalar string when the property's value has string scalar shape;
/// otherwise, `None`.
pub(crate) fn first_scalar_string(property: &Property) -> Option<&str> {
    property.value().as_scalar().and_then(|value| match value.view() {
        ValueRef::String(value) => Some(value),
        _ => None,
    })
}

/// Checks missingness for a non-interpolated property.
///
/// # Parameters
///
/// * `property` - Property whose unset or normalized-empty state is checked.
/// * `options` - Read policy supplying optional-string normalization rules.
///
/// # Returns
///
/// `true` when the property is unset or its string scalar normalizes to no
/// value; otherwise, `false`.
#[must_use]
pub(crate) fn is_effectively_missing(property: &Property, options: &ReadPolicy) -> bool {
    property.is_unset()
        || first_scalar_string(property)
            .is_some_and(|value| matches!(options.conversion_policy().string().normalize_optional(value), Ok(None)))
}

/// Parses a property through a context created for the selected reader.
///
/// # Type Parameters
///
/// * `R` - Reader that supplies property lookup and interpolation sources.
/// * `T` - Parsed output type implementing [`FromConfig`].
///
/// # Parameters
///
/// * `reader` - Reader used to resolve interpolation references.
/// * `name` - Canonical key associated with the property.
/// * `property` - Property value to parse.
/// * `options` - Runtime policy for interpolation and conversion.
///
/// # Returns
///
/// The parsed value, or the conversion error produced for the property.
///
/// # Errors
///
/// Returns an error when the property's value cannot be converted to `T`.
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
    let session = RefCell::new(InterpolationSession::new(options));
    with_property_read_context(reader, name, property, options, false, &session, |context| {
        T::from_config(property, context)
    })
}

/// Runs one read using a shared interpolation session and a property-local
/// scalar cache.
///
/// # Type Parameters
///
/// * `R` - Reader used to resolve referenced configuration values.
/// * `T` - Result type produced by the read callback.
///
/// # Parameters
///
/// * `reader` - Active reader, including any section-relative lookup scope.
/// * `name` - Canonical key identifying the current property.
/// * `property` - Property being read and whose scalar text may be cached.
/// * `options` - Policy controlling substitution and conversion.
/// * `interpolate` - Whether this read should substitute string values.
/// * `session` - Shared interpolation state for this logical read.
/// * `read` - Callback that converts the property using the prepared context.
///
/// # Returns
///
/// The value returned by `read`, or its conversion or interpolation error.
///
/// # Errors
///
/// Returns an error propagated from interpolation or the `read` callback.
pub(crate) fn with_property_read_context<R, T>(
    reader: &R,
    name: &str,
    property: &Property,
    options: &ReadPolicy,
    interpolate: bool,
    session: &RefCell<InterpolationSession>,
    read: impl FnOnce(&ConfigParseContext<'_>) -> ConfigResult<T>,
) -> ConfigResult<T>
where
    R: ConfigReader + ?Sized,
{
    let substitute = |value: &str| {
        if interpolate {
            utils::substitute_variables_with_fallback(
                value,
                reader,
                root_config(reader),
                options,
                name,
                &mut session.borrow_mut(),
            )
        } else {
            Ok(value.to_string())
        }
    };
    let context = ConfigParseContext::new(name, options, &substitute, interpolate, first_scalar_string(property));
    read(&context)
}

/// Checks missingness from the same prepared scalar the converter will use.
///
/// # Parameters
///
/// * `property` - Property whose unset state is checked before text handling.
/// * `context` - Prepared context supplying normalized/interpolated text.
///
/// # Returns
///
/// Whether the property is effectively missing under the prepared scalar
/// value and active conversion policy.
///
/// # Errors
///
/// Returns an error if obtaining the prepared scalar text fails.
pub(crate) fn is_effectively_missing_with_context(
    property: &Property,
    context: &ConfigParseContext<'_>,
) -> ConfigResult<bool> {
    if property.is_unset() {
        return Ok(true);
    }
    let Some(value) = context.prepared_scalar_text()? else {
        return Ok(false);
    };
    Ok(matches!(
        context.options().conversion_policy().string().normalize_optional(value),
        Ok(None)
    ))
}
