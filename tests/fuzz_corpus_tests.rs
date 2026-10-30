// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// # Structured deserialization fuzz corpus tests

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use qubit_config::Config;
use qubit_config::ConfigDeserializeOptions;
use qubit_config::UnknownFieldPolicy;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, PartialEq)]
struct TypedConfig {
    signed: Option<i128>,
    unsigned: Option<u128>,
    timeout: Option<Duration>,
    mode: Option<Mode>,
    label: Option<Label>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Label(String);

#[derive(Debug, Deserialize, PartialEq)]
enum Mode {
    Fast,
    Slow,
    Labeled(Label),
    Record { value: Option<u8> },
}

fn load_seed(target: &str, name: &str) -> Config {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fuzz")
        .join("corpus")
        .join(target)
        .join(name);
    let bytes = fs::read(path).expect("fuzz corpus seed should be readable");
    assert!(bytes.len() <= 4096, "seed {target}/{name} exceeds max_len");
    Config::decode_json_slice(&bytes).expect("fuzz corpus seed should decode as V1 wire")
}

fn options(interpolate: bool) -> ConfigDeserializeOptions {
    ConfigDeserializeOptions {
        interpolate,
        unknown_fields: UnknownFieldPolicy::Ignore,
    }
}

#[test]
fn structured_deserialization_corpus_seeds_reach_their_expected_shapes() {
    let targets = ["structured_deserialize", "structured_deserialize_typed"];
    let names = [
        "collections-nested.json",
        "typed-record.json",
        "typed-labeled-interpolation.json",
    ];

    for target in targets {
        for name in names {
            let config = load_seed(target, name);
            let _: Value = config.deserialize("").expect("seed should project to generic Value");
            let _: Value = config
                .deserialize_with("", options(true))
                .expect("seed should project to interpolated generic Value");
        }

        let collections = load_seed(target, "collections-nested.json");
        let ordinary: Value = collections.deserialize("").unwrap();
        assert_eq!(ordinary["items"], serde_json::json!(["${base}", "two"]));
        assert_eq!(ordinary["nested"], serde_json::json!({"array": [1, {"enabled": true}]}));
        let interpolated: Value = collections.deserialize_with("", options(true)).unwrap();
        assert_eq!(interpolated["items"], serde_json::json!(["example.test", "two"]));
        assert_eq!(
            interpolated["nested"],
            serde_json::json!({"array": [1, {"enabled": true}]})
        );

        let record = load_seed(target, "typed-record.json");
        let expected_record = TypedConfig {
            signed: Some(i128::MIN),
            unsigned: Some(u128::MAX),
            timeout: Some(Duration::new(2, 3)),
            mode: Some(Mode::Record { value: Some(3) }),
            label: Some(Label("hello".to_owned())),
        };
        assert_eq!(
            record.deserialize_with::<TypedConfig>("", options(false)).unwrap(),
            expected_record
        );
        assert_eq!(
            record.deserialize_with::<TypedConfig>("", options(true)).unwrap(),
            expected_record
        );

        let labeled = load_seed(target, "typed-labeled-interpolation.json");
        let ordinary = labeled.deserialize_with::<TypedConfig>("", options(false)).unwrap();
        assert_eq!(ordinary.label, Some(Label("${base}".to_owned())));
        assert_eq!(ordinary.mode, Some(Mode::Labeled(Label("tag".to_owned()))));
        assert_eq!(ordinary.signed, None);
        assert_eq!(ordinary.unsigned, None);
        assert_eq!(ordinary.timeout, None);

        let interpolated = labeled.deserialize_with::<TypedConfig>("", options(true)).unwrap();
        assert_eq!(interpolated.label, Some(Label("example.test".to_owned())));
        assert_eq!(interpolated.mode, Some(Mode::Labeled(Label("tag".to_owned()))));
        assert_eq!(interpolated.signed, None);
        assert_eq!(interpolated.unsigned, None);
        assert_eq!(interpolated.timeout, None);
    }
}
