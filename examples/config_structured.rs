// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ReadPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Database {
    host: String,
    port: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let operation = ConversionOperationLimits::builder()
        .max_input_bytes(128)
        .build();
    let conversion = ConversionLimits::builder()
        .operation_limits(operation)
        .build();
    let policy = ReadPolicy::builder().conversion_limits(conversion).build();
    let mut config = Config::builder().default_read_policy(policy).build();
    config.set("db.host", "localhost")?;
    config.set("db.port", "5432")?;
    config.set("db.pool_size", 16)?;

    let error = config
        .deserialize::<Database>("db")
        .expect_err("strict reads reject unknown fields");
    assert!(matches!(error, ConfigError::UnknownProperties { .. }));

    let database = config.deserialize_lenient::<Database>("db")?;
    assert_eq!(
        database,
        Database {
            host: "localhost".to_owned(),
            port: 5432,
        },
    );
    Ok(())
}
