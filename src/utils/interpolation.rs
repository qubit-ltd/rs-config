// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::options::{InterpolationSources, ReadPolicy};
use crate::{ConfigError, ConfigReader, ConfigResult};

use super::map_value_error;

/// Replaces variables using a primary reader and a fallback reader.
///
/// The primary reader is checked first. Absent or unset values fall back to
/// the fallback reader, then to environment variables only when the active
/// read policy permits environment fallback. Type and conversion errors in the
/// primary reader are returned directly.
pub(crate) fn substitute_variables_with_fallback<
    P: ConfigReader + ?Sized,
    F: ConfigReader + ?Sized,
>(
    value: &str,
    primary: &P,
    fallback: &F,
    options: &ReadPolicy,
    path: &str,
) -> ConfigResult<String> {
    substitute_variables_by(value, options, path, |var_name| {
        find_variable_value_with_fallback(var_name, primary, fallback, options, path)
    })
}

/// Replaces variables in `value` by repeatedly applying `resolve`.
fn substitute_variables_by(
    value: &str,
    options: &ReadPolicy,
    path: &str,
    mut resolve: impl FnMut(&str) -> ConfigResult<String>,
) -> ConfigResult<String> {
    let mut stack = Vec::new();
    let mut expansions = 0;
    substitute_variables_recursive(
        value,
        options,
        path,
        &mut stack,
        &mut expansions,
        &mut resolve,
    )
}

/// Recursively expands variables while tracking the active variable chain.
fn substitute_variables_recursive(
    value: &str,
    options: &ReadPolicy,
    path: &str,
    stack: &mut Vec<String>,
    expansions: &mut usize,
    resolve: &mut impl FnMut(&str) -> ConfigResult<String>,
) -> ConfigResult<String> {
    let max_depth = options.max_interpolation_depth();
    let max_expansions = options.max_interpolation_expansions();
    let max_output_bytes = options.max_interpolation_output_bytes();
    if value.is_empty() || find_next_variable(value, 0).is_none() {
        ensure_substitution_output_fits(value.len(), max_output_bytes, path)?;
        return Ok(value.to_string());
    }
    if stack.len() >= max_depth {
        return Err(ConfigError::SubstitutionDepthExceeded {
            path: path.to_string(),
            max_depth,
        });
    }

    let mut result = String::with_capacity(value.len().min(max_output_bytes));
    let mut last_end = 0;
    let mut search_from = 0;
    while let Some((match_start, match_end, var_name)) = find_next_variable(value, search_from) {
        push_substitution_fragment(
            &mut result,
            &value[last_end..match_start],
            max_output_bytes,
            path,
        )?;

        if let Some(index) = stack.iter().position(|name| name == var_name) {
            let mut chain = stack[index..].to_vec();
            chain.push(var_name.to_string());
            return Err(ConfigError::SubstitutionCycle {
                path: path.to_string(),
                chain,
            });
        }

        if *expansions >= max_expansions {
            return Err(ConfigError::SubstitutionExpansionLimitExceeded {
                path: path.to_string(),
                max_expansions,
            });
        }
        *expansions += 1;

        stack.push(var_name.to_string());
        let raw_value = resolve(var_name)?;
        let expanded =
            substitute_variables_recursive(&raw_value, options, path, stack, expansions, resolve)?;
        stack.pop();
        push_substitution_fragment(&mut result, &expanded, max_output_bytes, path)?;
        last_end = match_end;
        search_from = match_end;
    }
    push_substitution_fragment(&mut result, &value[last_end..], max_output_bytes, path)?;
    Ok(result)
}

/// Finds the next non-empty `${name}` placeholder in `value`.
fn find_next_variable(value: &str, mut search_from: usize) -> Option<(usize, usize, &str)> {
    while let Some(relative_start) = value.get(search_from..)?.find("${") {
        let match_start = search_from + relative_start;
        let name_start = match_start + 2;
        let relative_end = value.get(name_start..)?.find('}')?;
        let name_end = name_start + relative_end;
        if name_end > name_start {
            return Some((match_start, name_end + 1, &value[name_start..name_end]));
        }
        search_from = name_start;
    }
    None
}

/// Appends one substitution fragment after enforcing the output byte limit.
fn push_substitution_fragment(
    result: &mut String,
    fragment: &str,
    max_output_bytes: usize,
    path: &str,
) -> ConfigResult<()> {
    let output_bytes = result.len().checked_add(fragment.len()).ok_or_else(|| {
        ConfigError::SubstitutionOutputTooLarge {
            path: path.to_string(),
            max_output_bytes,
        }
    })?;
    ensure_substitution_output_fits(output_bytes, max_output_bytes, path)?;
    result.push_str(fragment);
    Ok(())
}

/// Rejects a substitution output length above the configured byte limit.
fn ensure_substitution_output_fits(
    output_bytes: usize,
    max_output_bytes: usize,
    path: &str,
) -> ConfigResult<()> {
    if output_bytes > max_output_bytes {
        Err(ConfigError::SubstitutionOutputTooLarge {
            path: path.to_string(),
            max_output_bytes,
        })
    } else {
        Ok(())
    }
}

/// Finds the value of a variable.
fn find_variable_value<R: ConfigReader + ?Sized>(
    var_name: &str,
    config: &R,
    options: &ReadPolicy,
    path: &str,
) -> ConfigResult<String> {
    match config.get_property(var_name)? {
        Some(property) if !property.is_unset() => match property.value().to_first::<String>() {
            Ok(value) => Ok(value),
            Err(error) => {
                let resolved = config.resolve_key(var_name)?;
                Err(map_value_error(&resolved, error))
            }
        },
        Some(_) | None
            if options.interpolation_sources() == InterpolationSources::ConfigThenEnv =>
        {
            std::env::var(var_name).map_err(|_| ConfigError::SubstitutionError {
                path: path.to_string(),
                message: format!("Cannot resolve variable: {var_name}"),
            })
        }
        Some(_) | None => Err(ConfigError::SubstitutionError {
            path: path.to_string(),
            message: format!("Cannot resolve variable from config: {var_name}"),
        }),
    }
}

/// Finds a variable value from `primary`, then `fallback`.
fn find_variable_value_with_fallback<P: ConfigReader + ?Sized, F: ConfigReader + ?Sized>(
    var_name: &str,
    primary: &P,
    fallback: &F,
    options: &ReadPolicy,
    path: &str,
) -> ConfigResult<String> {
    match primary.get_property(var_name)? {
        Some(property) if !property.is_unset() => match property.value().to_first::<String>() {
            Ok(value) => Ok(value),
            Err(error) => {
                let resolved = primary.resolve_key(var_name)?;
                Err(map_value_error(&resolved, error))
            }
        },
        Some(_) | None => find_variable_value(var_name, fallback, options, path),
    }
}
