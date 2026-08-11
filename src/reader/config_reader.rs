// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(private_bounds)]

mod internal;

use qubit_datatype::DataConversionTarget;
use qubit_value::StrictValueRead;

use crate::Config;
use crate::ConfigError;
use crate::ConfigName;
use crate::ConfigNames;
use crate::ConfigResult;
use crate::Property;
use crate::config_path::ensure_config_key;
use crate::config_path::ensure_config_path;
use crate::config_section::ConfigSection;
use crate::conversion::FromConfig;
use crate::conversion::IntoConfigDefault;
use crate::helpers::is_effectively_missing;
use crate::helpers::is_effectively_missing_interpolated;
use crate::helpers::parse_property_from_reader;
use crate::helpers::parse_property_from_reader_interpolated;
use crate::options::ReadPolicy;

/// Read-only configuration interface.
///
/// This trait allows consumers to read configuration values without requiring
/// ownership of a [`crate::Config`]. Both [`crate::Config`] and
/// [`crate::ConfigSection`] implement it.
///
/// The trait is sealed because its default methods rely on invariants shared by
/// [`Config`] and [`ConfigSection`]. Consumers can use it as a generic bound
/// but cannot provide third-party implementations.
pub trait ConfigReader: internal::Sealed {
    /// Returns a reference to the raw [`Property`] for `name`, if present.
    ///
    /// For a [`ConfigSection`], `name` is resolved relative to the view
    /// prefix (same rules as [`Self::get`]).
    fn get_property(&self, name: impl ConfigName) -> ConfigResult<Option<&Property>>;

    /// Number of configuration entries visible to this reader (all keys for
    /// [`crate::Config`]; relative keys only for a [`ConfigSection`]).
    fn len(&self) -> usize;

    /// Returns `true` when [`Self::len`] is zero.
    fn is_empty(&self) -> bool;

    /// All keys visible to this reader (relative keys for a section).
    fn keys(&self) -> Vec<String>;

    /// Returns whether a property exists for the given key.
    ///
    /// # Parameters
    ///
    /// * `name` - Full configuration key (for [`crate::ConfigSection`],
    ///   relative keys are resolved against the view prefix).
    ///
    /// # Returns
    ///
    /// `true` if the key is present.
    fn contains(&self, name: impl ConfigName) -> ConfigResult<bool>;

    /// Reads the first stored value for `name` and converts it to `T`.
    ///
    /// # Type parameters
    ///
    /// * `T` - Target type parsed by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// The converted value on success, or a [`crate::ConfigError`] if the key
    /// is absent, effectively missing, or not convertible.
    fn get<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        name.with_config_name(|name| {
            let property = match self.get_property(name)? {
                Some(property) => property,
                None => {
                    return Err(ConfigError::PropertyNotFound(self.resolve_key(name)?));
                }
            };
            let resolved = property.name();
            if !property.is_unset()
                && is_effectively_missing(self, resolved, property, self.read_policy())?
            {
                return Err(ConfigError::PropertyHasNoValue(resolved.to_owned()));
            }
            parse_property_from_reader(self, resolved, property, self.read_policy())
        })
    }

    /// Reads and interpolates the first stored value for `name`, then converts
    /// it to `T`.
    ///
    /// String-backed values resolve `${...}` placeholders through this reader,
    /// then the root configuration, and, when enabled by
    /// [`crate::options::ReadPolicy`], the process environment.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type parsed by [`FromConfig`] after interpolation.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// The interpolated and converted value.
    ///
    /// # Errors
    ///
    /// Returns missing-value, interpolation, resource-limit, or conversion
    /// errors with key context.
    fn get_interpolated<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        name.with_config_name(|name| {
            let property = match self.get_property(name)? {
                Some(property) => property,
                None => {
                    return Err(ConfigError::PropertyNotFound(self.resolve_key(name)?));
                }
            };
            let resolved = property.name();
            if !property.is_unset()
                && is_effectively_missing_interpolated(
                    self,
                    resolved,
                    property,
                    self.read_policy(),
                )?
            {
                return Err(ConfigError::PropertyHasNoValue(resolved.to_owned()));
            }
            parse_property_from_reader_interpolated(self, resolved, property, self.read_policy())
        })
    }

    /// Reads the first stored value for `name` without cross-type conversion.
    ///
    /// # Type parameters
    ///
    /// * `T` - Exact target type; requires `T` to implement strict reads from
    ///   both scalar and collection storage.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// The exact stored value on success, or a [`crate::ConfigError`] if the
    /// key is absent, unset, or has a different stored type.
    fn get_strict<T>(&self, name: impl ConfigName) -> ConfigResult<T>
    where
        T: StrictValueRead;

    /// Reads all stored values for `name` and converts each element to `T`.
    ///
    /// # Type parameters
    ///
    /// * `T` - Element type supported by the shared conversion layer.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// A vector of values on success, or a [`crate::ConfigError`] on failure.
    fn get_list<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        T: DataConversionTarget,
    {
        self.get::<Vec<T>>(name)
    }

    /// Reads all stored values for `name` without cross-type conversion.
    ///
    /// # Type parameters
    ///
    /// * `T` - Exact element type supported by both value shapes.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// A vector of exact stored values on success, or a
    /// [`crate::ConfigError`] on failure.
    fn get_list_strict<T>(&self, name: impl ConfigName) -> ConfigResult<Vec<T>>
    where
        T: StrictValueRead;

    /// Gets a value or `default` if the key is absent or effectively missing.
    ///
    /// Conversion errors are returned instead of being hidden by the default.
    #[inline]
    fn get_or<T>(
        &self,
        name: impl ConfigName,
        default: impl IntoConfigDefault<T>,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        self.get_optional(name)
            .map(|value| value.unwrap_or_else(|| default.into_config_default()))
    }

    /// Gets an interpolated value or `default` when the key is missing.
    ///
    /// The typed default is returned directly and is never interpolated.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type parsed by [`FromConfig`] after interpolation.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    /// * `default` - Fallback used only for an absent or effectively missing
    ///   value.
    ///
    /// # Returns
    ///
    /// The interpolated value or the supplied default.
    ///
    /// # Errors
    ///
    /// Returns interpolation and conversion errors instead of hiding them
    /// behind the default.
    #[inline]
    fn get_interpolated_or<T>(
        &self,
        name: impl ConfigName,
        default: impl IntoConfigDefault<T>,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        self.get_optional_interpolated(name)
            .map(|value| value.unwrap_or_else(|| default.into_config_default()))
    }

    /// Gets an optional value with the same semantics as
    /// [`crate::Config::get_optional`].
    ///
    /// # Type parameters
    ///
    /// * `T` - Target type parsed by [`FromConfig`].
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key (relative for a section).
    ///
    /// # Returns
    ///
    /// `Ok(Some(v))`, `Ok(None)` when absent or effectively missing, or `Err`
    /// on conversion failure. Concrete empty collections are present values.
    fn get_optional<T>(&self, name: impl ConfigName) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        name.with_config_name(|name| match self.get_property(name)? {
            None => Ok(None),
            Some(property) => {
                let resolved = property.name();
                if is_effectively_missing(self, resolved, property, self.read_policy())? {
                    Ok(None)
                } else {
                    parse_property_from_reader(self, resolved, property, self.read_policy())
                        .map(Some)
                }
            }
        })
    }

    /// Gets an optional value after interpolating string-backed values.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type parsed by [`FromConfig`] after interpolation.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// `Ok(Some(value))` for a configured value, `Ok(None)` when absent or
    /// effectively missing after interpolation, or `Err` on failure. For a
    /// section, placeholders use the section first and the root configuration
    /// as fallback.
    ///
    /// # Errors
    ///
    /// Returns interpolation, resource-limit, or conversion errors with key
    /// context.
    fn get_optional_interpolated<T>(&self, name: impl ConfigName) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        name.with_config_name(|name| match self.get_property(name)? {
            None => Ok(None),
            Some(property) => {
                let resolved = property.name();
                if is_effectively_missing_interpolated(
                    self,
                    resolved,
                    property,
                    self.read_policy(),
                )? {
                    Ok(None)
                } else {
                    parse_property_from_reader_interpolated(
                        self,
                        resolved,
                        property,
                        self.read_policy(),
                    )
                    .map(Some)
                }
            }
        })
    }

    /// Gets the read policy active for this reader.
    ///
    /// # Returns
    ///
    /// The policy inherited by ordinary reads.
    fn read_policy(&self) -> &ReadPolicy;

    /// Creates a borrowed reader view using `policy` for typed reads.
    ///
    /// # Parameters
    ///
    /// * `policy` - Read policy borrowed by the returned view.
    ///
    /// # Returns
    ///
    /// A view preserving this reader's current scope.
    #[inline]
    fn read_with<'a>(&'a self, policy: &'a ReadPolicy) -> ConfigSection<'a> {
        self.section("")
            .expect("the root configuration section must be valid")
            .with_read_policy_override(policy)
    }

    /// Returns this reader's normalized root-relative scope path.
    ///
    /// # Returns
    ///
    /// The root reader returns an empty string.
    #[inline(always)]
    fn scope_path(&self) -> &str {
        ""
    }

    /// Reads a value from the first present and non-empty key in `names`.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    ///
    /// # Returns
    ///
    /// Parsed value from the first configured key. Conversion errors stop the
    /// search and are returned directly.
    fn get_any<T>(&self, names: impl ConfigNames) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            self.get_optional_any(names)?
                .ok_or(missing_candidates_error(self, names)?)
        })
    }

    /// Reads and interpolates the first configured value from `names`.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type parsed by [`FromConfig`] after interpolation.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    ///
    /// # Returns
    ///
    /// The interpolated value from the first present, non-empty key.
    ///
    /// # Errors
    ///
    /// Returns missing-value, interpolation, resource-limit, or conversion
    /// errors. An error from a selected key stops the search.
    fn get_any_interpolated<T>(&self, names: impl ConfigNames) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            self.get_optional_any_interpolated(names)?
                .ok_or(missing_candidates_error(self, names)?)
        })
    }

    /// Reads an optional value from the first present and non-empty key.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    ///
    /// # Returns
    ///
    /// `Ok(None)` only when every key is absent or effectively missing.
    fn get_optional_any<T>(&self, names: impl ConfigNames) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            get_optional_any_with_options(self, names, self.read_policy(), false)
        })
    }

    /// Reads an optional interpolated value from the first configured key.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type parsed by [`FromConfig`] after interpolation.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    ///
    /// # Returns
    ///
    /// `Ok(Some(value))` for the first present, non-empty key or `Ok(None)`
    /// when every candidate is absent or effectively missing.
    ///
    /// # Errors
    ///
    /// Returns interpolation, resource-limit, or conversion errors from the
    /// selected key.
    fn get_optional_any_interpolated<T>(&self, names: impl ConfigNames) -> ConfigResult<Option<T>>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            get_optional_any_with_options(self, names, self.read_policy(), true)
        })
    }

    /// Reads a value from any key, using `default` only when every key is
    /// absent or effectively missing.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    /// * `default` - Fallback when no candidate is configured.
    ///
    /// # Returns
    ///
    /// Parsed value or `default`; parsing errors are never swallowed.
    fn get_any_or<T>(
        &self,
        names: impl ConfigNames,
        default: impl IntoConfigDefault<T>,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            self.get_optional_any(names)
                .map(|value| value.unwrap_or_else(|| default.into_config_default()))
        })
    }

    /// Reads an interpolated value from any key, using `default` only when all
    /// candidates are absent or effectively missing.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Target type parsed by [`FromConfig`] after interpolation.
    ///
    /// # Parameters
    ///
    /// * `names` - Candidate keys in priority order.
    /// * `default` - Fallback when no candidate is configured.
    ///
    /// # Returns
    ///
    /// Interpolated value or `default`; parsing errors are never swallowed.
    ///
    /// # Errors
    ///
    /// Returns interpolation, resource-limit, or conversion errors from the
    /// selected key.
    fn get_any_interpolated_or<T>(
        &self,
        names: impl ConfigNames,
        default: impl IntoConfigDefault<T>,
    ) -> ConfigResult<T>
    where
        T: FromConfig,
    {
        names.with_config_names(|names| {
            self.get_optional_any_interpolated(names)
                .map(|value| value.unwrap_or_else(|| default.into_config_default()))
        })
    }

    /// Gets an optional list with the same semantics as
    /// [`crate::Config::get_optional_list`].
    ///
    /// # Type parameters
    ///
    /// * `T` - Element type supported by the shared conversion layer.
    ///
    /// # Parameters
    ///
    /// * `name` - Configuration key.
    ///
    /// # Returns
    ///
    /// `Ok(Some(vec))`, including `Some(Vec::new())` for a concrete empty
    /// collection; `Ok(None)` only when absent or effectively missing.
    fn get_optional_list<T>(&self, name: impl ConfigName) -> ConfigResult<Option<Vec<T>>>
    where
        T: DataConversionTarget,
    {
        self.get_optional::<Vec<T>>(name)
    }

    /// Returns whether any key visible to this reader starts with `prefix`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Key prefix to test (for a section, keys are relative to
    ///   that view).
    ///
    /// # Returns
    ///
    /// `true` if at least one matching key exists.
    fn contains_key_prefix(&self, prefix: &str) -> bool;

    /// Returns whether a dotted section visible to this reader has children.
    ///
    /// # Parameters
    ///
    /// * `path` - Section path relative to this reader.
    ///
    /// # Returns
    ///
    /// `true` when at least one descendant belongs to that exact dotted path.
    fn contains_section(&self, path: &str) -> ConfigResult<bool>;

    /// Iterates `(key, property)` pairs for keys that start with `prefix`.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Key prefix filter.
    ///
    /// # Returns
    ///
    /// An iterator over matching entries without dynamic dispatch.
    fn iter_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a Property)> + 'a;

    /// Iterates all `(key, property)` pairs visible to this reader (same scope
    /// as [`Self::keys`]).
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&'a str, &'a Property)> + 'a>;

    /// Returns whether `name` exists as an unset property.
    ///
    /// A missing key and a concrete empty collection both return `false`.
    /// Only a property backed by an unset [`qubit_value::ValueContainer`]
    /// returns `true`.
    fn is_unset(&self, name: impl ConfigName) -> ConfigResult<bool> {
        name.with_config_name(|name| {
            Ok(self
                .get_property(name)?
                .map(|property| property.is_unset())
                .unwrap_or(false))
        })
    }

    /// Creates a read-only section; property keys resolve strictly relative to
    /// `path`.
    ///
    /// Semantics match [`crate::Config::section`] and
    /// [`crate::ConfigSection::section`]. Calling this method on a section
    /// creates a nested section.
    ///
    /// # Arguments
    ///
    /// * `path` - Relative section path; an empty path keeps the current scope.
    ///
    /// # Returns
    ///
    /// A [`ConfigSection`] borrowing this reader's underlying
    /// [`crate::Config`].
    fn section(&self, path: &str) -> ConfigResult<ConfigSection<'_>>;

    /// Creates a read-only section only when it has visible descendant
    /// properties.
    ///
    /// An exact scalar at `path` does not make the section present. The
    /// returned section uses the same strict relative-key and read-policy
    /// semantics as [`Self::section`].
    ///
    /// # Arguments
    ///
    /// * `path` - Relative section path; an empty path checks the current
    ///   reader scope.
    ///
    /// # Returns
    ///
    /// `Some` with the section when at least one descendant property is
    /// visible, or `None` when the section has no visible properties.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidPath`] when `path` is not canonical.
    #[inline]
    fn section_if_present(&self, path: &str) -> ConfigResult<Option<ConfigSection<'_>>> {
        let section = self.section(path)?;
        if section.is_empty() {
            Ok(None)
        } else {
            Ok(Some(section))
        }
    }

    /// Resolves `name` into the canonical key path against the root
    /// [`crate::Config`].
    ///
    /// For a root [`crate::Config`], this returns `name` unchanged. For a
    /// [`crate::ConfigSection`], this prepends the effective section path so
    /// callers can report root-relative key paths in diagnostics.
    ///
    /// # Parameters
    ///
    /// * `name` - Key relative to the current reader scope.
    ///
    /// # Returns
    ///
    /// Root-relative key path string.
    #[inline]
    fn resolve_key(&self, name: impl ConfigName) -> ConfigResult<String> {
        name.with_config_name(|name| {
            ensure_config_path(name)?;
            Ok(name.to_string())
        })
    }
}

/// Returns the root configuration backing a supported reader.
#[inline(always)]
pub(crate) fn root_config<R>(reader: &R) -> &Config
where
    R: ConfigReader + ?Sized,
{
    internal::Sealed::root_config(reader)
}

/// Creates a structured error for an unsuccessful multi-key lookup.
fn missing_candidates_error<R>(reader: &R, names: &[&str]) -> ConfigResult<ConfigError>
where
    R: ConfigReader + ?Sized,
{
    let paths = names
        .iter()
        .map(|name| reader.resolve_key(*name))
        .collect::<ConfigResult<Vec<_>>>()?;
    Ok(ConfigError::PropertyCandidatesNotFound { paths })
}

/// Shared implementation for field-level and global multi-key reads.
fn get_optional_any_with_options<R, T>(
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
        for name in names {
            ensure_config_key(name)?;
        }
        for name in names {
            let Some(property) = reader.get_property(*name)? else {
                continue;
            };
            let resolved = property.name();
            let missing = if interpolate {
                is_effectively_missing_interpolated(reader, resolved, property, options)?
            } else {
                is_effectively_missing(reader, resolved, property, options)?
            };
            if missing {
                continue;
            }
            return if interpolate {
                parse_property_from_reader_interpolated(reader, resolved, property, options)
                    .map(Some)
            } else {
                parse_property_from_reader(reader, resolved, property, options).map(Some)
            };
        }
        Ok(None)
    })
}

impl internal::Sealed for Config {
    #[inline(always)]
    fn root_config(&self) -> &Config {
        self
    }
}

impl internal::Sealed for ConfigSection<'_> {
    #[inline(always)]
    fn root_config(&self) -> &Config {
        ConfigSection::root_config(self)
    }
}
