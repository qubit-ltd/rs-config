// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # Configuration Management Module
//!
//! Provides flexible configuration management with support for multiple data
//! types and variable substitution.

mod config;
#[path = "error/config_deserialize_error.rs"]
mod config_deserialize_error;
#[path = "error/config_error.rs"]
mod config_error;
#[path = "error/config_error_kind.rs"]
mod config_error_kind;
#[path = "key/config_key.rs"]
mod config_key;
#[path = "key/config_name.rs"]
mod config_name;
#[path = "key/config_names.rs"]
mod config_names;
#[path = "conversion/config_parse_context.rs"]
mod config_parse_context;
#[path = "key/config_path.rs"]
mod config_path;
#[path = "key/config_path_violation.rs"]
mod config_path_violation;
#[path = "property/property_mut.rs"]
mod config_property_mut;
#[path = "reader/config_reader.rs"]
mod config_reader;
#[path = "reader/config_section.rs"]
mod config_section;
#[path = "conversion/config_serde_ext.rs"]
mod config_serde_ext;
#[path = "conversion/config_value_deserializer.rs"]
mod config_value_deserializer;
mod config_wire_encode_error;
mod config_wire_limits;
mod constants;
pub mod conversion;
pub mod error;
#[path = "conversion/from_config.rs"]
mod from_config;
#[path = "conversion/helpers.rs"]
mod helpers;
#[path = "conversion/into_config_default.rs"]
mod into_config_default;
pub mod key;
pub mod options;
pub mod property;
#[path = "reader/read_policy.rs"]
mod read_policy;
pub mod reader;
pub mod source;
mod utils;

pub use config::Config;
pub use config_error_kind::ConfigErrorKind;
pub use config_key::ConfigKey;
pub use config_name::ConfigName;
pub use config_names::ConfigNames;
pub use config_path::ConfigPath;
pub use config_path_violation::ConfigPathViolation;
pub use config_property_mut::ConfigPropertyMut;
pub use config_reader::ConfigReader;
pub use config_section::ConfigSection;
pub use config_serde_ext::ConfigSerdeExt;
pub use config_wire_encode_error::ConfigWireEncodeError;
pub use config_wire_limits::ConfigWireDecodeError;
pub use config_wire_limits::ConfigWireLimitKind;
pub use config_wire_limits::ConfigWireLimits;
pub use error::ConfigError;
pub use error::ConfigResult;
pub use property::Property;
pub use read_policy::InterpolationSources;
pub use read_policy::ReadPolicy;
pub use source::ConfigSource;
pub use source::SourceLimitKind;
pub use source::SourceLimits;
