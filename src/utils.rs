/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Utility Functions
//!
//! Provides configuration-related utility functions, such as variable
//! substitution.
//!
//! # Author
//!
//! Haixing Hu

use regex::Regex;
use std::sync::OnceLock;

use super::{ConfigError, ConfigReader, ConfigResult};

/// Regular expression pattern for variables
///
/// Matches variables in `${variable_name}` format
///
/// # Author
///
/// Haixing Hu
///
static VARIABLE_PATTERN: OnceLock<Regex> = OnceLock::new();

/// Gets the regular expression pattern for variables
///
/// # Author
///
/// Haixing Hu
///
#[inline]
fn get_variable_pattern() -> &'static Regex {
    VARIABLE_PATTERN.get_or_init(|| {
        Regex::new(r"\$\{([^}]+)\}").expect("Failed to compile variable pattern regex")
    })
}

/// Replaces variables in a string
///
/// Replaces all variables in `${var_name}` format in the string. Variable
/// values are first looked up in the configuration, and if not found, then in
/// environment variables.
///
/// # Parameters
///
/// * `value` - The string to replace variables in
/// * `config` - Configuration object
/// * `max_depth` - Maximum substitution depth (prevents circular references)
///
/// # Returns
///
/// Returns the replaced string on success, or an error on failure
///
/// # Errors
///
/// - Returns `ConfigError::SubstitutionDepthExceeded` if substitution depth
///   exceeds maximum
/// - Returns `ConfigError::SubstitutionError` if a variable cannot be resolved
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_config::Config;
///
/// let mut config = Config::new();
/// config.set("host", "localhost")?;
/// config.set("port", 8080)?;
/// config.set("url", "http://${host}:${port}")?;
///
/// let url = config.get_string("url")?;
/// assert_eq!(url, "http://localhost:8080");
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub fn substitute_variables<R: ConfigReader + ?Sized>(
    value: &str,
    config: &R,
    max_depth: usize,
) -> ConfigResult<String> {
    if value.is_empty() {
        return Ok(value.to_string());
    }

    let pattern = get_variable_pattern();
    let mut result = value.to_string();
    let mut depth = 0;

    loop {
        // Find all variables and collect replacement information
        let replacements: Vec<(String, String)> = pattern
            .captures_iter(&result)
            .map(|cap| {
                let full_match = cap.get(0).unwrap().as_str().to_string();
                let var_name = cap.get(1).unwrap().as_str();
                let var_value = find_variable_value(var_name, config)?;
                Ok((full_match, var_value))
            })
            .collect::<ConfigResult<Vec<_>>>()?;

        if replacements.is_empty() {
            // No more variables to replace
            break;
        }

        if depth >= max_depth {
            return Err(ConfigError::SubstitutionDepthExceeded(max_depth));
        }

        // Perform all replacements
        for (full_match, var_value) in replacements {
            result = result.replace(&full_match, &var_value);
        }

        depth += 1;
    }

    Ok(result)
}

/// Finds the value of a variable
///
/// First looks in the configuration. It falls back to environment variables
/// only when the key is missing or explicitly empty/null in config.
///
/// # Parameters
///
/// * `var_name` - Variable name
/// * `config` - Configuration object
///
/// # Returns
///
/// Returns the variable value on success, or an error on failure
///
/// # Author
///
/// Haixing Hu
///
fn find_variable_value<R: ConfigReader + ?Sized>(
    var_name: &str,
    config: &R,
) -> ConfigResult<String> {
    // 1. Try configuration first.
    match config.get::<String>(var_name) {
        Ok(value) => Ok(value),
        // Only missing or empty values can fall back to env vars.
        Err(ConfigError::PropertyNotFound(_)) | Err(ConfigError::PropertyHasNoValue(_)) => {
            std::env::var(var_name).map_err(|_| {
                ConfigError::SubstitutionError(format!("Cannot resolve variable: {}", var_name))
            })
        }
        // Type/conversion errors in config should surface directly instead of
        // being silently masked by environment values.
        Err(err) => Err(err),
    }
}
