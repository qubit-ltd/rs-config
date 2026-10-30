// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::Config;
use serde_json::Value as JsonValue;

#[test]
fn test_dependency_uses_standard_json_numbers() {
    let number: JsonValue = serde_json::from_str("1.2300").unwrap();
    assert_eq!(number.to_string(), "1.23");
}

#[test]
fn test_standard_numbers_survive_both_wire_paths() {
    let mut config = Config::new();
    config.set("native", 1.25_f64).unwrap();
    let object: JsonValue = serde_json::from_str(r#"{"small":7,"value":1.25}"#).unwrap();
    config.set("payload", object.clone()).unwrap();

    let bounded = Config::decode_json_slice(&config.encode_json_vec().unwrap()).unwrap();
    let generic: Config = serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
    for current in [&config, &bounded, &generic] {
        assert_eq!(current.get::<f64>("native").unwrap(), 1.25);
        assert_eq!(current.deserialize::<f64>("native").unwrap(), 1.25);
        assert_eq!(current.deserialize::<JsonValue>("payload").unwrap(), object);
    }
}
