// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::internal::InterpolationFrame;
use super::internal::InterpolationSession;
use super::map_value_error;
use crate::ConfigError;
use crate::ConfigReader;
use crate::ConfigResult;
use crate::helpers::is_effectively_missing;
use crate::options::InterpolationSources;
use crate::options::ReadPolicy;

/// Replaces variables using a primary reader and a fallback reader.
///
/// The primary reader is checked first. Absent or unset values fall back to
/// the fallback reader, then to environment variables only when the active
/// read policy permits environment fallback. Type and conversion errors in the
/// primary reader are returned directly.
/// # Type Parameters
///
/// * `P` - Primary reader consulted for interpolation variables.
/// * `F` - Fallback reader consulted when the primary reader has no value.
///
/// # Parameters
///
/// * `value` - Input value whose placeholders are being expanded.
/// * `primary` - Reader checked first for referenced configuration values.
/// * `fallback` - Fallback used when the primary lookup does not resolve.
/// * `options` - Options that control source key selection and normalization.
/// * `path` - Configuration path or filesystem path used by this operation.
/// * `session` - Per-load context used to enforce shared source limits.
pub(crate) fn substitute_variables_with_fallback<P: ConfigReader + ?Sized, F: ConfigReader + ?Sized>(
    value: &str,
    primary: &P,
    fallback: &F,
    options: &ReadPolicy,
    path: &str,
    session: &mut InterpolationSession,
) -> ConfigResult<String> {
    substitute_variables_by(value, options, path, session, |var_name| {
        find_variable_value_with_fallback(var_name, primary, fallback, options, path)
    })
}

/// Iteratively replaces variables in `value` using heap-owned interpolation
/// frames.
/// # Parameters
///
/// * `value` - Input value whose placeholders are being expanded.
/// * `options` - Options that control source key selection and normalization.
/// * `path` - Configuration path or filesystem path used by this operation.
/// * `session` - Per-load context used to enforce shared source limits.
/// * `resolve` - Callback that resolves one referenced variable.
fn substitute_variables_by(
    value: &str,
    options: &ReadPolicy,
    path: &str,
    session: &mut InterpolationSession,
    mut resolve: impl FnMut(&str) -> ConfigResult<String>,
) -> ConfigResult<String> {
    let max_depth = options.max_interpolation_depth();
    let max_output_bytes = session.max_output_bytes();
    if value.is_empty() || find_next_variable(value, 0).is_none() {
        session.check_output(value.len(), path)?;
        return Ok(value.to_string());
    }
    let mut active_variables: Vec<String> = Vec::new();
    let mut frames = vec![InterpolationFrame::new(value.to_string(), None)];

    loop {
        if !frames.last().expect("root interpolation frame remains present").entered {
            let is_plain_text = {
                let frame = frames.last().expect("root interpolation frame remains present");
                frame.text.is_empty() || find_next_variable(&frame.text, 0).is_none()
            };
            if is_plain_text {
                let completed = frames.pop().expect("plain interpolation frame exists");
                session.check_output(completed.text.len(), path)?;
                if let Some(expected_variable) = completed.entering_variable.as_deref() {
                    let popped_variable = active_variables.pop();
                    debug_assert_eq!(popped_variable.as_deref(), Some(expected_variable));
                }
                if let Some(parent) = frames.last_mut() {
                    push_substitution_fragment(&mut parent.output, &completed.text, session, path)?;
                    continue;
                }
                return Ok(completed.text);
            }
            if active_variables.len() >= max_depth {
                return Err(ConfigError::SubstitutionDepthExceeded {
                    path: path.to_string(),
                    max_depth,
                });
            }
            let frame = frames.last_mut().expect("root interpolation frame remains present");
            frame.output = String::with_capacity(frame.text.len().min(max_output_bytes));
            frame.entered = true;
            continue;
        }

        let next_variable = {
            let frame = frames.last().expect("entered interpolation frame remains present");
            find_next_variable(&frame.text, frame.search_from).map(|(start, end, name)| (start, end, name.to_string()))
        };
        if let Some((match_start, match_end, variable_name)) = next_variable {
            {
                let frame = frames.last_mut().expect("entered interpolation frame remains present");
                push_substitution_fragment(
                    &mut frame.output,
                    &frame.text[frame.last_end..match_start],
                    session,
                    path,
                )?;
            }

            if let Some(index) = active_variables.iter().position(|name| name == &variable_name) {
                let mut chain = active_variables[index..].to_vec();
                chain.push(variable_name);
                return Err(ConfigError::SubstitutionCycle {
                    path: path.to_string(),
                    chain,
                });
            }

            session.consume_expansion(path)?;
            active_variables.push(variable_name.clone());
            let raw_value = resolve(&variable_name)?;
            let frame = frames.last_mut().expect("parent interpolation frame remains present");
            frame.last_end = match_end;
            frame.search_from = match_end;
            frames.push(InterpolationFrame::new(raw_value, Some(variable_name)));
            continue;
        }

        {
            let frame = frames.last_mut().expect("entered interpolation frame remains present");
            push_substitution_fragment(&mut frame.output, &frame.text[frame.last_end..], session, path)?;
        }
        let completed = frames.pop().expect("completed interpolation frame exists");
        if let Some(expected_variable) = completed.entering_variable {
            let popped_variable = active_variables.pop();
            debug_assert_eq!(popped_variable.as_deref(), Some(expected_variable.as_str()));
        }
        if let Some(parent) = frames.last_mut() {
            push_substitution_fragment(&mut parent.output, &completed.output, session, path)?;
        } else {
            return Ok(completed.output);
        }
    }
}

/// Finds the next non-empty `${name}` placeholder in `value`.
/// # Parameters
///
/// * `value` - Input value whose placeholders are being expanded.
/// * `search_from` - Byte offset where the next placeholder search begins.
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
/// # Parameters
///
/// * `result` - Result returned by the value conversion to classify
///   missing-value errors.
/// * `fragment` - Text fragment appended to the current interpolation output.
/// * `session` - Per-load context used to enforce shared source limits.
/// * `path` - Configuration path or filesystem path used by this operation.
fn push_substitution_fragment(
    result: &mut String,
    fragment: &str,
    session: &InterpolationSession,
    path: &str,
) -> ConfigResult<()> {
    let Some(prospective_len) = result.len().checked_add(fragment.len()) else {
        return Err(ConfigError::SubstitutionOutputTooLarge {
            path: path.to_string(),
            max_output_bytes: session.max_output_bytes(),
        });
    };
    session.check_output(prospective_len, path)?;
    result.push_str(fragment);
    Ok(())
}

/// Finds the value of a variable.
/// # Type Parameters
///
/// * `R` - Reader used to resolve the requested configuration values.
///
/// # Parameters
///
/// * `var_name` - Name parsed from the interpolation placeholder.
/// * `config` - Configuration value or source configuration used by this
///   operation.
/// * `options` - Options that control source key selection and normalization.
/// * `path` - Configuration path or filesystem path used by this operation.
fn find_variable_value<R: ConfigReader + ?Sized>(
    var_name: &str,
    config: &R,
    options: &ReadPolicy,
    path: &str,
) -> ConfigResult<String> {
    match config.get_property(var_name)? {
        Some(property) if !property.is_unset() && is_effectively_missing(property, options) => Ok(String::new()),
        Some(property) if !property.is_unset() => match property
            .value()
            .to_first_with::<String>(options.conversion_policy(), options.conversion_limits())
        {
            Ok(value) => Ok(value),
            Err(error) => {
                let resolved = config.resolve_key(var_name)?;
                Err(map_value_error(&resolved, error))
            }
        },
        Some(_) | None if options.interpolation_sources() == InterpolationSources::ConfigThenEnv => {
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
/// # Type Parameters
///
/// * `P` - Primary reader consulted for interpolation variables.
/// * `F` - Fallback reader consulted when the primary reader has no value.
///
/// # Parameters
///
/// * `var_name` - Name parsed from the interpolation placeholder.
/// * `primary` - Reader checked first for referenced configuration values.
/// * `fallback` - Fallback used when the primary lookup does not resolve.
/// * `options` - Options that control source key selection and normalization.
/// * `path` - Configuration path or filesystem path used by this operation.
fn find_variable_value_with_fallback<P: ConfigReader + ?Sized, F: ConfigReader + ?Sized>(
    var_name: &str,
    primary: &P,
    fallback: &F,
    options: &ReadPolicy,
    path: &str,
) -> ConfigResult<String> {
    match primary.get_property(var_name)? {
        Some(property) if !property.is_unset() && is_effectively_missing(property, options) => Ok(String::new()),
        Some(property) if !property.is_unset() => match property
            .value()
            .to_first_with::<String>(options.conversion_policy(), options.conversion_limits())
        {
            Ok(value) => Ok(value),
            Err(error) => {
                let resolved = primary.resolve_key(var_name)?;
                Err(map_value_error(&resolved, error))
            }
        },
        Some(_) | None => find_variable_value(var_name, fallback, options, path),
    }
}
