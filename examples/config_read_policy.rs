// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigReader;
use qubit_config::options::ReadPolicy;

fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("HTTP_ENABLED", "yes")?;
    config.set("HTTP_PORTS", "8080, 8081,,8082")?;

    let policy = ReadPolicy::env_friendly();
    let reader = config.read_with(&policy);
    let enabled: bool = reader.get("HTTP_ENABLED")?;
    let ports: Vec<u16> = reader.get("HTTP_PORTS")?;
    assert!(enabled);
    assert_eq!(ports, [8080, 8081, 8082]);
    assert_eq!(config.default_read_policy(), &ReadPolicy::default());
    Ok(())
}
