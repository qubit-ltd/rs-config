// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// # Configuration Utility Function Tests
//
// Integration tests for deserialize JSON building (`property_to_json_value` /
// dotted-key insertion) and variable substitution behavior.

use qubit_config::{
    Config, ConfigError, Property,
    options::{InterpolationSources, ReadPolicy},
};
use qubit_datatype::{
    DataConversionError, DataType, DurationConversionOptions, DurationRoundingPolicy, DurationUnit,
    InvalidValueReason, NumericConversionOptions,
};
use qubit_value::{MultiValues, Value, ValueContainer};
use serde::Deserialize;
use std::collections::HashMap;

#[test]
fn utility_test_modules_are_registered() {
    assert!(Config::new().is_empty());
}

fn set_max_interpolation_depth(config: &mut Config, max_depth: usize) {
    let options = config
        .default_read_policy()
        .clone()
        .with_max_interpolation_depth(max_depth);
    config.set_default_read_policy(options);
}

fn with_environment_fallback(options: ReadPolicy) -> ReadPolicy {
    options.with_interpolation_sources(InterpolationSources::ConfigThenEnv)
}

// ============================================================================
// Config::deserialize() (utils: property_to_json_value,
// insert_deserialize_value)
// ============================================================================

#[path = "utils/deserialize_tests.rs"]
mod test_deserialize;
#[path = "utils/structured_serde_tests.rs"]
mod test_property_to_json_value_deserialize_behavior;
#[path = "utils/interpolation_tests.rs"]
mod test_variable_substitution;
