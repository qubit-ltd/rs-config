// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Demonstrates bounded structured reads and unknown-field policy choices.

use qubit_config::Config;
use qubit_config::ConfigDeserializeOptions;
use qubit_config::ConfigError;
use qubit_config::ReadPolicy;
use qubit_config::UnknownFieldPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use serde::Deserialize;

/// Fields selected from the database section in this structured-read example.
#[derive(Debug, Deserialize, PartialEq)]
struct Database {
    /// Database host name.
    host: String,
    /// Database service port.
    port: u16,
}

/// Compares strict and lenient reads under a conversion input budget.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let operation = ConversionOperationLimits::builder().max_input_bytes(128).build();
    let conversion = ConversionLimits::builder().operation_limits(operation).build();
    let policy = ReadPolicy::builder().conversion_limits(conversion).build();
    let mut config = Config::builder().default_read_policy(policy).build();
    config.set("db.host", "localhost")?;
    config.set("db.port", "5432")?;
    config.set("db.pool_size", 16)?;

    let error = config
        .deserialize::<Database>("db")
        .expect_err("strict reads reject unknown fields");
    assert!(matches!(error, ConfigError::UnknownProperties { .. }));

    let database = config.deserialize_with::<Database>(
        "db",
        ConfigDeserializeOptions {
            unknown_fields: UnknownFieldPolicy::Ignore,
            ..Default::default()
        },
    )?;
    assert_eq!(
        database,
        Database {
            host: "localhost".to_owned(),
            port: 5432,
        },
    );
    Ok(())
}
