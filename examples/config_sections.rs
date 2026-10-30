// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigReader;

fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("server.tls.enabled", true)?;
    let server = config.section("server")?;
    let tls = server.section("tls")?;
    assert!(tls.get::<bool>("enabled")?);
    Ok(())
}
