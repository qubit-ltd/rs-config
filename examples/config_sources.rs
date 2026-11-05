// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Demonstrates merging property and environment configuration sources.

use qubit_config::Config;
use qubit_config::ConfigReader;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::EnvConfigSource;
use qubit_config::source::PropertiesConfigSource;

/// Loads and prints a server endpoint from layered configuration sources.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (host, port) = load_server_config()?;
    println!("server listening at {host}:{port}");
    Ok(())
}

/// Merges property defaults with environment overrides and reads server fields.
fn load_server_config() -> Result<(String, u16), Box<dyn std::error::Error>> {
    let mut sources = CompositeConfigSource::new();
    sources.add(PropertiesConfigSource::from_content(
        "server.host=localhost\nserver.port=8080\n",
    ));
    sources.add(EnvConfigSource::from_prefix("APP_"));

    let mut config = Config::new();
    config.merge_properties_from_source(&sources)?;
    let server = config.section("server")?;

    Ok((server.get("host")?, server.get("port")?))
}
