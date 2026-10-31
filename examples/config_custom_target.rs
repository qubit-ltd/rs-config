// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigReader;
use qubit_config::ConfigResult;
use qubit_config::Property;
use qubit_config::conversion::ConfigParseContext;
use qubit_config::conversion::FromConfig;

#[derive(Debug, PartialEq)]
struct ServicePort(u16);

impl FromConfig for ServicePort {
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        u16::from_config(property, ctx).map(Self)
    }
}

fn main() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("server.port", "8080")?;
    assert_eq!(config.get::<ServicePort>("server.port")?, ServicePort(8080));

    config.set("server.default_port", "9090")?;
    config.set("server.port", "${default_port}")?;
    let server = config.section("server")?;
    assert_eq!(server.get_interpolated::<ServicePort>("port")?, ServicePort(9090));
    assert!(server.get::<ServicePort>("port").is_err());

    config.set("server.port", "invalid")?;
    let error = config.get::<ServicePort>("server.port").unwrap_err();
    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("server.port"));
    Ok(())
}
