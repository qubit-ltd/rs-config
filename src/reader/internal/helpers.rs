// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::RefCell;

use crate::ConfigError;
use crate::ConfigNames;
use crate::ConfigReader;
use crate::ConfigResult;
use crate::config_path::ensure_config_key;
use crate::conversion::FromConfig;
use crate::helpers::is_effectively_missing_with_context;
use crate::helpers::with_property_read_context;
use crate::options::ReadPolicy;
use crate::utils::InterpolationSession;

/// Creates a structured error for an unsuccessful multi-key lookup.
///
/// # Type Parameters
///
/// * `R` - Reader used to resolve candidate configuration values.
///
/// # Parameters
///
/// * `reader` - Reader used to resolve candidate keys.
/// * `names` - Candidate keys checked in priority order.
///
/// # Returns
///
/// An error containing the resolved root-relative candidate paths.
///
/// # Errors
///
/// Returns an error if any candidate cannot be resolved as a valid key.
pub(in crate::config_reader) fn missing_candidates_error<R>(reader: &R, names: &[&str]) -> ConfigResult<ConfigError>
where
    R: ConfigReader + ?Sized,
{
    let paths = names
        .iter()
        .map(|name| reader.resolve_key(*name))
        .collect::<ConfigResult<Vec<_>>>()?;
    Ok(ConfigError::PropertyCandidatesNotFound { paths })
}

/// Reads the first candidate that is present under the selected read policy.
///
/// # Type Parameters
///
/// * `R` - Reader used for candidate property lookups.
/// * `T` - Type converted from a selected property.
///
/// # Parameters
///
/// * `reader` - Reader that supplies candidate properties.
/// * `names` - Candidate keys checked in priority order.
/// * `options` - Policy controlling missing-value normalization.
/// * `interpolate` - Whether string-backed values are interpolated.
///
/// # Returns
///
/// The first converted candidate, or `None` when every candidate is absent
/// or effectively missing.
///
/// # Errors
///
/// Returns key, interpolation, resource-limit, or conversion errors.
pub(in crate::config_reader) fn get_optional_any_with_options<R, T>(
    reader: &R,
    names: impl ConfigNames,
    options: &ReadPolicy,
    interpolate: bool,
) -> ConfigResult<Option<T>>
where
    R: ConfigReader + ?Sized,
    T: FromConfig,
{
    names.with_config_names(|names| {
        let session = RefCell::new(InterpolationSession::new(options));
        for name in names {
            ensure_config_key(name)?;
        }
        for name in names {
            let Some(property) = reader.get_property(*name)? else {
                continue;
            };
            let resolved = property.name();
            let parsed =
                with_property_read_context(reader, resolved, property, options, interpolate, &session, |context| {
                    if is_effectively_missing_with_context(property, context)? {
                        return Ok(None);
                    }
                    optional_converted_value(T::from_config(property, context))
                })?;
            if let Some(value) = parsed {
                return Ok(Some(value));
            }
        }
        Ok(None)
    })
}

/// Classifies defaultable missing conversions without hiding other failures.
///
/// # Type Parameters
///
/// * `T` - Converted value type.
///
/// # Parameters
///
/// * `result` - Conversion result being classified.
///
/// # Returns
///
/// `Some(value)` for a successful conversion, `None` for a defaultable missing
/// value, or the original non-defaultable error.
///
/// # Errors
///
/// Returns the original error when it is not a defaultable missing-value
/// error.
pub(in crate::config_reader) fn optional_converted_value<T>(result: ConfigResult<T>) -> ConfigResult<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error)
            if error
                .value_missing()
                .is_some_and(|missing| missing.is_defaultable_for_conversion()) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
