// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use qubit_config::ConfigReader;
use qubit_config::ReadPolicy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let policy = ReadPolicy::builder_from(&ReadPolicy::env_friendly())
        .max_interpolation_depth(7)
        .build();
    let mut original = Config::builder().default_read_policy(policy.clone()).build();
    original.set("enabled", "yes")?;
    assert!(original.get::<bool>("enabled")?);
    let mut restored = Config::decode_json_slice(&original.encode_json_vec()?)?;
    assert_eq!(restored, original);
    assert_eq!(restored.default_read_policy(), &ReadPolicy::default());
    assert!(restored.get::<bool>("enabled").is_err());
    assert!(restored.read_with(&policy).get::<bool>("enabled")?);
    restored.set_default_read_policy(policy.clone());
    assert_eq!(restored.default_read_policy(), &policy);
    assert!(restored.get::<bool>("enabled")?);
    Ok(())
}
