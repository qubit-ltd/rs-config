// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::BTreeMap;

use qubit_config::Config;

/// Converts string keys while rejecting normalized numeric duplicates.
fn numeric_ports(named: BTreeMap<String, u16>) -> Result<BTreeMap<u16, u16>, Box<dyn std::error::Error>> {
    let mut ports = BTreeMap::new();
    for (name, value) in named {
        let number = name.parse::<u16>()?;
        if ports.insert(number, value).is_some() {
            return Err("duplicate numeric port key".into());
        }
    }
    Ok(ports)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.set("ports.80", 8080_u16)?;
    config.set("ports.443", 8443_u16)?;
    let named: BTreeMap<String, u16> = config.deserialize("ports")?;
    let ports = numeric_ports(named)?;
    assert_eq!(ports.get(&80), Some(&8080));
    assert_eq!(ports.get(&443), Some(&8443));
    config.set("ports.080", 8081_u16)?;
    let duplicate: BTreeMap<String, u16> = config.deserialize("ports")?;
    assert!(numeric_ports(duplicate).is_err());
    Ok(())
}
