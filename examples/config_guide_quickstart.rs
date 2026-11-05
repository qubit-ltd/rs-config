// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shows the smallest Config write and typed-read workflow.

use qubit_config::Config;

/// Writes a server port and reads it back as an unsigned integer.
fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("server.port", 8080)?;
    let port: u16 = config.get("server.port")?;
    assert_eq!(port, 8080);
    Ok(())
}
