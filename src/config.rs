// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # Configuration Manager
//!
//! Provides storage, retrieval, and management of configurations.

#![allow(private_bounds)]

mod internal;

use serde::de::Error as _;
use serde::{
    Deserialize,
    Deserializer,
    Serialize,
    Serializer,
};
use std::collections::BTreeMap;

use self::internal::{
    ConfigSerdeRepr,
    ConfigWire,
    ConfigWireV1,
    ConfigWireV1Ref,
};
use crate::options::ReadPolicy;
use crate::{
    ConfigError,
    ConfigResult,
    ConfigWireDecodeError,
    ConfigWireLimitKind,
    ConfigWireLimits,
    Property,
};
use qubit_datatype::DataConversionTarget;
use qubit_utils::Transient;
use qubit_value::{
    Value as QubitValue,
    ValueWireDecodeError,
    WireBudget,
};

mod access;
mod mutation;
mod read;
mod source_loading;
mod structured_serde;
mod traversal;

/// Converts deserialized numeric text with the configured conversion policy.
///
/// # Type Parameters
///
/// * `T` - Target numeric type.
///
/// # Parameters
///
/// * `key` - Configuration path used for error context.
/// * `options` - Read policy controlling conversion.
/// * `value` - Numeric text to convert.
///
/// # Returns
///
/// The converted number.
///
/// # Errors
///
/// Returns a keyed conversion error when `value` cannot be converted to `T`.
pub(crate) fn convert_deserialize_number<T>(
    key: &str,
    options: &ReadPolicy,
    value: String,
) -> ConfigResult<T>
where
    T: DataConversionTarget,
{
    match QubitValue::String(value).to_with::<T>(options.conversion_options()) {
        Ok(value) => Ok(value),
        Err(error) => Err(ConfigError::from((key, error))),
    }
}

/// Configuration Manager
///
/// Manages a set of configuration properties with type-safe read/write
/// interfaces.
///
/// # Features
///
/// - Supports multiple data types
/// - Supports variable substitution (`${var_name}` format)
/// - Supports configuration merging
/// - Supports final value protection
/// - Thread-safe (when wrapped in `Arc<RwLock<Config>>`)
///
/// # Persistence
///
/// Serde serialization emits the stable V1 JSON persistence format with an
/// explicit `version` field and lexically ordered property keys.
/// Deserialization accepts both V1 and the unversioned top-level form emitted
/// before V1.
///
/// # Examples
///
/// ```rust
/// use qubit_config::Config;
///
/// let mut config = Config::new();
///
/// // Set configuration values (type inference)
/// config.set("port", 8080).unwrap();                    // inferred as i32
/// config.set("host", "localhost").unwrap();
/// // &str is converted to String
/// config.set("debug", true).unwrap();                   // inferred as bool
/// config.set("timeout", 30.5).unwrap();                 // inferred as f64
/// config.set("code", 42u8).unwrap();                    // inferred as u8
///
/// // Set multiple values (type inference)
/// config.set("ports", vec![8080, 8081, 8082]).unwrap(); // inferred as i32
/// config.set("hosts", vec!["host1", "host2"]).unwrap();
/// // &str elements are converted
///
/// // Read configuration values (type inference)
/// let port: i32 = config.get("port").unwrap();
/// let host: String = config.get("host").unwrap();
/// let debug: bool = config.get("debug").unwrap();
/// let code: u8 = config.get("code").unwrap();
///
/// // Read configuration values (turbofish)
/// let port = config.get::<i32>("port").unwrap();
///
/// // Read configuration value or use default
/// let timeout: f64 = config.get_or("timeout", 30.0).unwrap();
/// ```
#[must_use]
#[derive(Debug, Clone)]
pub struct Config {
    /// Configuration description
    description: Option<String>,
    /// Configuration property mapping
    pub(crate) properties: BTreeMap<String, Property>,
    /// Runtime policy used by direct reads on this configuration.
    default_read_policy: Transient<ReadPolicy>,
}

impl PartialEq for Config {
    /// Compares only the persisted configuration data.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.description == other.description
            && self.properties == other.properties
    }
}

impl Serialize for Config {
    /// Serializes this configuration through the stable V1 persistence format.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ConfigWireV1Ref::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Config {
    /// Deserializes either the stable V1 format or the legacy unversioned form.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ConfigWire::deserialize(deserializer)
            .and_then(|wire| Self::try_from(wire).map_err(D::Error::custom))
    }
}

impl TryFrom<ConfigSerdeRepr> for Config {
    type Error = String;

    /// Builds a configuration only when every map key matches its property.
    ///
    /// # Parameters
    ///
    /// * `value` - Deserialized wire representation to validate.
    ///
    /// # Returns
    ///
    /// A configuration whose property-name invariant is satisfied.
    ///
    /// # Errors
    ///
    /// Returns an error naming the first mismatched map key and property name.
    fn try_from(value: ConfigSerdeRepr) -> Result<Self, Self::Error> {
        let ConfigSerdeRepr {
            description,
            properties,
            read_options: _,
        } = value;
        Self::from_wire_parts(description, properties)
    }
}

impl TryFrom<ConfigWireV1> for Config {
    type Error = String;

    /// Builds a configuration from the stable V1 persistence format.
    ///
    /// # Errors
    ///
    /// Returns an error when the wire version is unsupported or a map key does
    /// not match its embedded property name.
    fn try_from(value: ConfigWireV1) -> Result<Self, Self::Error> {
        if value.version != 1 {
            return Err(format!(
                "unsupported config wire version {}; expected 1",
                value.version,
            ));
        }
        Self::from_wire_parts(
            value.description,
            value.properties.into_iter().collect(),
        )
    }
}

impl TryFrom<ConfigWire> for Config {
    type Error = String;

    /// Builds a configuration from any accepted persisted representation.
    ///
    /// # Errors
    ///
    /// Returns the validation failure for the selected wire representation.
    fn try_from(value: ConfigWire) -> Result<Self, Self::Error> {
        match value {
            ConfigWire::V1(value) => Self::try_from(value),
            ConfigWire::Legacy(value) => Self::try_from(value),
        }
    }
}

impl Config {
    /// Decodes a complete configuration JSON wire document with default
    /// structural limits.
    ///
    /// # Parameters
    ///
    /// * `input` - Complete untrusted configuration JSON document.
    ///
    /// # Returns
    ///
    /// The decoded configuration.
    ///
    /// # Errors
    ///
    /// Returns a wire-limit, JSON, or configuration-invariant error.
    pub fn decode_json_slice(
        input: &[u8],
    ) -> Result<Self, ConfigWireDecodeError> {
        Self::decode_json_slice_with_limits(input, ConfigWireLimits::default())
    }

    /// Decodes a complete configuration JSON wire document with shared Value
    /// and configuration-specific structural limits.
    ///
    /// # Parameters
    ///
    /// * `input` - Complete untrusted configuration JSON document.
    /// * `limits` - Shared and configuration-specific resource limits.
    ///
    /// # Returns
    ///
    /// The decoded configuration.
    ///
    /// # Errors
    ///
    /// Returns a wire-limit error before or after decoding, a JSON error, or a
    /// configuration-invariant error.
    pub fn decode_json_slice_with_limits(
        input: &[u8],
        limits: ConfigWireLimits,
    ) -> Result<Self, ConfigWireDecodeError> {
        let mut budget =
            limits
                .wire()
                .begin(input.len())
                .map_err(|error| match error {
                    ValueWireDecodeError::InvalidJson(error) => {
                        ConfigWireDecodeError::InvalidJson(error)
                    }
                    error => ConfigWireDecodeError::from(error),
                })?;
        let config: Self = serde_json::from_slice(input)
            .map_err(ConfigWireDecodeError::InvalidJson)?;
        config.check_wire_budget(&mut budget, limits)?;
        Ok(config)
    }

    /// Charges decoded configuration resources against one shared budget.
    fn check_wire_budget(
        &self,
        budget: &mut WireBudget,
        limits: ConfigWireLimits,
    ) -> Result<(), ConfigWireDecodeError> {
        budget.check_node().map_err(ConfigWireDecodeError::from)?;
        budget
            .check_map_entries(self.properties.len())
            .map_err(ConfigWireDecodeError::from)?;
        if self.properties.len() > limits.max_properties() {
            return Err(ConfigWireDecodeError::LimitExceeded {
                kind: ConfigWireLimitKind::Properties,
                value: self.properties.len(),
                maximum: limits.max_properties(),
            });
        }
        if let Some(description) = &self.description {
            budget
                .check_string_bytes(description.len())
                .map_err(ConfigWireDecodeError::from)?;
        }
        for (key, property) in &self.properties {
            budget.check_node().map_err(ConfigWireDecodeError::from)?;
            if key.len() > limits.max_property_key_bytes() {
                return Err(ConfigWireDecodeError::LimitExceeded {
                    kind: ConfigWireLimitKind::PropertyKeyBytes,
                    value: key.len(),
                    maximum: limits.max_property_key_bytes(),
                });
            }
            budget
                .check_string_bytes(key.len())
                .map_err(ConfigWireDecodeError::from)?;
            budget
                .check_string_bytes(property.name().len())
                .map_err(ConfigWireDecodeError::from)?;
            if let Some(description) = property.description() {
                budget
                    .check_string_bytes(description.len())
                    .map_err(ConfigWireDecodeError::from)?;
            }
            budget
                .check_container_at(property.value(), 2)
                .map_err(ConfigWireDecodeError::from)?;
        }
        Ok(())
    }

    /// Validates shared fields decoded from an accepted persisted wire format.
    fn from_wire_parts(
        description: Option<String>,
        properties: BTreeMap<String, Property>,
    ) -> Result<Self, String> {
        if let Some((key, violation)) = properties
            .keys()
            .filter_map(|key| {
                crate::config_path::validate_config_key(key)
                    .err()
                    .map(|violation| (key, violation))
            })
            .min_by_key(|(key, _)| *key)
        {
            return Err(ConfigError::InvalidKey {
                key: key.clone(),
                violation,
            }
            .to_string());
        }
        if let Some((key, property)) = properties
            .iter()
            .filter(|(key, property)| key.as_str() != property.name())
            .min_by_key(|(key, _)| *key)
        {
            return Err(format!(
                "configuration map key '{key}' does not match property name '{}'",
                property.name(),
            ));
        }
        Ok(Self {
            description,
            properties,
            default_read_policy: Transient::new(ReadPolicy::default()),
        })
    }
    /// Creates a new empty configuration
    ///
    /// # Returns
    ///
    /// Returns a new configuration instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let mut config = Config::new();
    /// assert!(config.is_empty());
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            description: None,
            properties: BTreeMap::new(),
            default_read_policy: Transient::new(ReadPolicy::default()),
        }
    }

    /// Creates a configuration with description
    ///
    /// # Parameters
    ///
    /// * `description` - Configuration description
    ///
    /// # Returns
    ///
    /// Returns a new configuration instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let config = Config::with_description("Server Configuration");
    /// assert_eq!(config.description(), Some("Server Configuration"));
    /// ```
    #[inline]
    pub fn with_description(description: &str) -> Self {
        Self {
            description: Some(description.to_string()),
            properties: BTreeMap::new(),
            default_read_policy: Transient::new(ReadPolicy::default()),
        }
    }
}

impl Default for Config {
    /// Creates a new default configuration
    ///
    /// # Returns
    ///
    /// Returns a new configuration instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Config;
    ///
    /// let config = Config::default();
    /// assert!(config.is_empty());
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
