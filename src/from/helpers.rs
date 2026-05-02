/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use qubit_value::MultiValues;

use crate::config_reader::ConfigReader;
use crate::options::ConfigReadOptions;
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
/// Returns `Some(&str)` only when the property stores exactly one string.
pub(crate) fn first_scalar_string(property: &Property) -> Option<&str> {
    match property.value() {
        MultiValues::String(values) if values.len() == 1 => values.first().map(String::as_str),
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
/// Returns `true` when the property is empty or a scalar string normalized by
/// the active string options as missing.
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
    if property.is_empty() {
        return Ok(true);
    }
    let Some(value) = first_scalar_string(property) else {
        return Ok(false);
    };
    let substitute = |value: &str| {
        if reader.is_enable_variable_substitution() {
            utils::substitute_variables(value, reader, reader.max_substitution_depth())
        } else {
            Ok(value.to_string())
        }
    };
    let ctx = ConfigParseContext::new(name, options, &substitute);
    let value = ctx.substitute_string(value)?;
    match options.string.normalize(&value) {
        Ok(_) => Ok(false),
        Err(qubit_common::lang::DataConversionError::NoValue) => Ok(true),
        Err(_) => Ok(false),
    }
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
    let substitute = |value: &str| {
        if reader.is_enable_variable_substitution() {
            utils::substitute_variables(value, reader, reader.max_substitution_depth())
        } else {
            Ok(value.to_string())
        }
    };
    let ctx = ConfigParseContext::new(name, options, &substitute);
    T::from_config(property, &ctx)
}
