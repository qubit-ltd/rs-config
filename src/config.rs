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
// qubit-style: allow multiple-public-types

#![allow(private_bounds)]

mod internal;

use std::collections::BTreeMap;

use qubit_budget::MeasuredBudgetError;
use qubit_budget::json::JsonDecodeSession;
use qubit_budget::json::JsonEncodeSession;
use qubit_budget::json::JsonResource;
use qubit_json::decode::JsonDecodeError;
use qubit_json::decode::JsonDecodeErrorKind;
use qubit_json::decode::JsonDecoder;
use qubit_json::encode::JsonEncodeError;
use qubit_json::encode::JsonEncoder;
use qubit_utils::Transient;
use qubit_value::ValueWireRefV1;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::de::Error as _;

use self::internal::ConfigSerdeRepr;
use self::internal::ConfigWire;
use self::internal::ConfigWireV1;
use self::internal::ConfigWireV1Ref;
use self::internal::JsonAdmittedConfigWireSeed;
use crate::ConfigError;
use crate::ConfigWireDecodeError;
use crate::ConfigWireEncodeError;
use crate::ConfigWireLimitKind;
use crate::ConfigWireLimits;
use crate::Property;
use crate::options::ReadPolicy;

mod access;
mod mutation;
mod read;
mod source_loading;
mod structured_serde;
mod traversal;

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
    ///
    /// This entry point applies [`ConfigWireLimits::default`] to decoded Serde
    /// structure, payload, properties, and property keys. A general
    /// deserializer does not expose its original bytes or lexical tokens, so
    /// this implementation cannot enforce the raw-input limit. Use
    /// [`Config::decode_json_slice`] for untrusted JSON bytes.
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
    /// Creates a builder initialized with an empty configuration.
    #[inline]
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// Encodes this configuration as a V1 JSON wire document with the
    /// configuration crate's default JSON budget profile.
    ///
    /// # Returns
    ///
    /// The complete encoded JSON document.
    ///
    /// # Errors
    ///
    /// Returns a value-representation, resource-limit, or JSON serialization
    /// error. Ordinary [`serde::Serialize`] behavior is unchanged.
    pub fn encode_json_vec(&self) -> Result<Vec<u8>, ConfigWireEncodeError> {
        self.encode_json_vec_with_limits(ConfigWireLimits::default())
    }

    /// Encodes this configuration as a V1 JSON wire document under an
    /// explicit configuration profile.
    ///
    /// Generic JSON costs are charged by rs-budget while configuration
    /// property invariants remain local to this crate.
    ///
    /// # Parameters
    ///
    /// * `limits` - Shared and configuration-specific resource limits.
    ///
    /// # Returns
    ///
    /// The complete encoded JSON document.
    ///
    /// # Errors
    ///
    /// Returns a value-representation error, a shared budget error, a JSON
    /// serialization error, or a configuration-specific limit error.
    pub fn encode_json_vec_with_limits(
        &self,
        limits: ConfigWireLimits,
    ) -> Result<Vec<u8>, ConfigWireEncodeError> {
        self.check_config_limits_encode(limits)?;
        for property in self.properties.values() {
            let _ = ValueWireRefV1::try_from(property.value())?;
        }
        let session = JsonEncodeSession::from_limits(limits.json_encode());
        JsonEncoder::new(session)
            .to_vec(&ConfigWireV1Ref::from(self))
            .map_err(map_encode_json_error)
    }

    /// Decodes a complete configuration JSON wire document with the
    /// configuration crate's default JSON budget profile.
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
    /// Returns a shared budget, JSON, or configuration-invariant error.
    /// Unlike ordinary [`Deserialize`], this API also admits the original JSON
    /// slice against its raw-input limit.
    pub fn decode_json_slice(
        input: &[u8],
    ) -> Result<Self, ConfigWireDecodeError> {
        Self::decode_json_slice_with_limits(input, ConfigWireLimits::default())
    }

    /// Decodes a complete configuration JSON wire document with an explicit
    /// configuration profile.
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
    /// Returns a shared budget error, a JSON error, or a configuration
    /// invariant/limit error.
    pub fn decode_json_slice_with_limits(
        input: &[u8],
        limits: ConfigWireLimits,
    ) -> Result<Self, ConfigWireDecodeError> {
        let session = JsonDecodeSession::from_limits(limits.json_decode());
        let wire = JsonDecoder::new(session)
            .decode_seed_utf8(JsonAdmittedConfigWireSeed::new(limits), input)
            .map_err(map_decode_json_error)??;
        let config = Self::try_from(wire)
            .map_err(ConfigWireDecodeError::InvalidConfig)?;
        Ok(config)
    }

    /// Checks configuration-specific limits for an encoding operation.
    fn check_config_limits_encode(
        &self,
        limits: ConfigWireLimits,
    ) -> Result<(), ConfigWireEncodeError> {
        let property_count = u64::try_from(self.properties.len())
            .expect("property count must fit in u64");
        limits
            .properties_limit()
            .check(property_count)
            .map_err(|error| ConfigWireEncodeError::LimitExceeded {
                kind: ConfigWireLimitKind::Properties,
                value: error
                    .exact_observed()
                    .expect("point failure carries an exact value"),
                maximum: error.maximum(),
            })?;
        for key in self.properties.keys() {
            let key_bytes = u64::try_from(key.len())
                .expect("property key length must fit in u64");
            limits.property_key_bytes_limit().check(key_bytes).map_err(
                |error| ConfigWireEncodeError::LimitExceeded {
                    kind: ConfigWireLimitKind::PropertyKeyBytes,
                    value: error
                        .exact_observed()
                        .expect("point failure carries an exact value"),
                    maximum: error.maximum(),
                },
            )?;
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
        Self::builder().build()
    }
}

/// Builder for [`Config`].
#[must_use]
#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    description: Option<String>,
    default_read_policy: ReadPolicy,
}

impl ConfigBuilder {
    /// Creates a builder initialized with an empty configuration.
    #[inline]
    pub fn new() -> Self {
        Self {
            description: None,
            default_read_policy: ReadPolicy::default(),
        }
    }

    /// Sets the configuration description.
    #[inline]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Sets the default runtime read policy.
    #[inline]
    pub fn default_read_policy(mut self, policy: ReadPolicy) -> Self {
        self.default_read_policy = policy;
        self
    }

    /// Builds the configured empty configuration.
    #[inline]
    pub fn build(self) -> Config {
        Config {
            description: self.description,
            properties: BTreeMap::new(),
            default_read_policy: Transient::new(self.default_read_policy),
        }
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
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

/// Maps a budget-aware JSON decode failure to the configuration wire error.
fn map_decode_json_error(
    error: JsonDecodeError<JsonResource, u64>,
) -> ConfigWireDecodeError {
    if let Some(error) = error.budget_error().cloned() {
        return match error {
            MeasuredBudgetError::Budget(error) => {
                ConfigWireDecodeError::Budget(error)
            }
            MeasuredBudgetError::Quantity { resource, source } => {
                ConfigWireDecodeError::Quantity { resource, source }
            }
        };
    }
    if let Some(error) = error.syntax_error() {
        return ConfigWireDecodeError::Syntax(*error);
    }
    if error.kind() == JsonDecodeErrorKind::Deserialize {
        ConfigWireDecodeError::Json {
            category: serde_json::error::Category::Data,
            line: error.line().unwrap_or(0),
            column: error.column().unwrap_or(0),
        }
    } else {
        ConfigWireDecodeError::Adapter(error.to_string())
    }
}

/// Maps a budget-aware JSON encode failure to the configuration wire error.
fn map_encode_json_error(
    error: JsonEncodeError<JsonResource, u64>,
) -> ConfigWireEncodeError {
    match error {
        JsonEncodeError::Budget(error) => match error {
            MeasuredBudgetError::Budget(error) => {
                ConfigWireEncodeError::Budget(error)
            }
            MeasuredBudgetError::Quantity { resource, source } => {
                ConfigWireEncodeError::Quantity { resource, source }
            }
        },
        JsonEncodeError::InvalidRawJson(error) => {
            ConfigWireEncodeError::Syntax(error)
        }
        JsonEncodeError::Serialize(error) => ConfigWireEncodeError::Json(error),
        JsonEncodeError::Write(_) => {
            ConfigWireEncodeError::Adapter(String::from(
                "unexpected writer failure while buffering configuration JSON",
            ))
        }
    }
}
