// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::process::Command;

use qubit_config::Config;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigReader;
use qubit_config::ConfigResult;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::EnvConfigSource;
use qubit_config::source::PropertiesConfigSource;

/// Loads a baseline and an explicit deployment environment layer.
fn load_server() -> ConfigResult<(String, u16, u64)> {
    let sources = CompositeConfigSource::builder()
        .add_source(PropertiesConfigSource::from_content(
            "server.host=localhost\nserver.port=8080\n",
        ))
        .add_source(EnvConfigSource::from_prefix("QUBIT_CONFIG_GUIDE_APP_"))
        .build();
    let config = Config::from_source(&sources)?;
    let server = config.section("server")?;
    Ok((server.get("host")?, server.get("port")?, server.get_or("timeout", 30)?))
}

/// Checks one deployment state inside its own environment.
fn check_child(mode: &str) -> Result<(), Box<dyn std::error::Error>> {
    match mode {
        "baseline" => assert_eq!(load_server()?, ("localhost".to_owned(), 8080, 30)),
        "override" => assert_eq!(load_server()?, ("deployed.test".to_owned(), 9090, 30)),
        "invalid" => {
            let error = load_server().expect_err("invalid port must fail");
            assert_eq!(error.kind(), ConfigErrorKind::Conversion);
            assert_eq!(error.path(), Some("server.port"));
        }
        _ => return Err("unexpected example child mode".into()),
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(mode) = std::env::args().nth(1) {
        return check_child(&mode);
    }
    for mode in ["baseline", "override", "invalid"] {
        let mut command = Command::new(std::env::current_exe()?);
        command.env_clear().arg(mode);
        if mode == "override" {
            command.env("QUBIT_CONFIG_GUIDE_APP_SERVER__HOST", "deployed.test");
            command.env("QUBIT_CONFIG_GUIDE_APP_SERVER__PORT", "9090");
        } else if mode == "invalid" {
            command.env("QUBIT_CONFIG_GUIDE_APP_SERVER__PORT", "not-a-number");
        }
        assert!(command.status()?.success(), "example child failed: {mode}");
    }
    Ok(())
}
