// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.set("host", "localhost")?;
    config.set("url", "http://${host}")?;

    assert_eq!(config.get::<String>("url")?, "http://${host}");
    assert_eq!(config.get_interpolated::<String>("url")?, "http://localhost",);
    Ok(())
}
