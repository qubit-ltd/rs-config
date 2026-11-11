// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests structured deserialization against the fuzz corpus seeds.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use qubit_config::Config;
use qubit_config::ConfigDeserializeOptions;
use qubit_config::UnknownFieldPolicy;
use qubit_config::conversion::ConfigSerdeExt;
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
        .join("tests")
        .join("data")
        .join("fuzz_corpus")
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
fn test_structured_deserialization_corpus_seeds_reach_their_expected_shapes() {
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
        let ordinary: Value = collections.deserialize("").expect("read the ordinary collections seed");
        assert_eq!(ordinary["items"], serde_json::json!(["${base}", "two"]));
        assert_eq!(ordinary["nested"], serde_json::json!({"array": [1, {"enabled": true}]}));
        let interpolated: Value = collections
            .deserialize_with("", options(true))
            .expect("read the interpolated collections seed");
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
            record
                .deserialize_with::<TypedConfig>("", options(false))
                .expect("read the ordinary typed record seed"),
            expected_record
        );
        assert_eq!(
            record
                .deserialize_with::<TypedConfig>("", options(true))
                .expect("read the interpolated typed record seed"),
            expected_record
        );

        let labeled = load_seed(target, "typed-labeled-interpolation.json");
        let ordinary = labeled
            .deserialize_with::<TypedConfig>("", options(false))
            .expect("read the ordinary labeled seed");
        assert_eq!(ordinary.label, Some(Label("${base}".to_owned())));
        assert_eq!(ordinary.mode, Some(Mode::Labeled(Label("tag".to_owned()))));
        assert_eq!(ordinary.signed, None);
        assert_eq!(ordinary.unsigned, None);
        assert_eq!(ordinary.timeout, None);

        let interpolated = labeled
            .deserialize_with::<TypedConfig>("", options(true))
            .expect("read the interpolated labeled seed");
        assert_eq!(interpolated.label, Some(Label("example.test".to_owned())));
        assert_eq!(interpolated.mode, Some(Mode::Labeled(Label("tag".to_owned()))));
        assert_eq!(interpolated.signed, None);
        assert_eq!(interpolated.unsigned, None);
        assert_eq!(interpolated.timeout, None);
    }
}

#[test]
fn test_structured_deserialization_accepts_equivalent_exact_and_dotted_shapes() {
    let expected_value = serde_json::json!({
        "signed": -7,
        "unsigned": 42,
        "timeout": {"secs": 2, "nanos": 3},
        "mode": "Fast",
        "label": "sample",
    });
    let expected_typed: TypedConfig =
        serde_json::from_value(expected_value.clone()).expect("typed expected value should deserialize");

    let mut dotted = Config::new();
    dotted.set("case.signed", -7_i64).expect("set signed value");
    dotted.set("case.unsigned", 42_u64).expect("set unsigned value");
    dotted
        .set("case.timeout", expected_value["timeout"].clone())
        .expect("set timeout object");
    dotted.set("case.mode", "Fast").expect("set mode value");
    dotted.set("case.label", "sample").expect("set label value");

    let mut root = Config::new();
    root.set("signed", -7_i64).expect("set root signed value");
    root.set("unsigned", 42_u64).expect("set root unsigned value");
    root.set("timeout", expected_value["timeout"].clone())
        .expect("set root timeout object");
    root.set("mode", "Fast").expect("set root mode value");
    root.set("label", "sample").expect("set root label value");

    let mut exact = Config::new();
    exact
        .set("case", expected_value.clone())
        .expect("set exact structured property");

    assert_eq!(
        dotted.deserialize::<Value>("case").expect("read dotted shape"),
        expected_value,
    );
    assert_eq!(
        dotted
            .section("case")
            .expect("select dotted section")
            .deserialize::<Value>("")
            .expect("read dotted section"),
        expected_value,
    );
    assert_eq!(
        dotted
            .deserialize_with::<Value>("case", options(true))
            .expect("read interpolated dotted shape"),
        expected_value,
    );
    assert_eq!(root.deserialize::<Value>("").expect("read root fields"), expected_value,);
    assert_eq!(
        root.deserialize_with::<Value>("", options(true))
            .expect("read interpolated root fields"),
        expected_value,
    );
    assert_eq!(
        dotted
            .deserialize_with::<TypedConfig>("case", options(false))
            .expect("read typed dotted shape"),
        expected_typed,
    );
    assert_eq!(
        exact.deserialize::<Value>("case").expect("read exact property"),
        expected_value,
    );
    assert_eq!(
        exact
            .deserialize_with::<TypedConfig>("case", options(false))
            .expect("read typed exact property"),
        expected_typed,
    );
}

#[test]
fn test_typed_semantic_cases_match_all_read_shapes() {
    let cases = [
        (
            serde_json::json!({"signed": i128::MIN.to_string(), "unsigned": u128::MAX.to_string()}),
            TypedConfig {
                signed: Some(i128::MIN),
                unsigned: Some(u128::MAX),
                timeout: None,
                mode: None,
                label: None,
            },
        ),
        (
            serde_json::json!({"signed": (i128::from(i64::MIN) - 1).to_string(), "unsigned": (u128::from(u64::MAX) + 1).to_string()}),
            TypedConfig {
                signed: Some(i128::from(i64::MIN) - 1),
                unsigned: Some(u128::from(u64::MAX) + 1),
                timeout: None,
                mode: None,
                label: None,
            },
        ),
        (
            serde_json::json!({"signed": (i128::from(i64::MAX) + 1).to_string(), "unsigned": u64::MAX.to_string()}),
            TypedConfig {
                signed: Some(i128::from(i64::MAX) + 1),
                unsigned: Some(u128::from(u64::MAX)),
                timeout: None,
                mode: None,
                label: None,
            },
        ),
        (
            serde_json::json!({"mode": {"Labeled": "text"}}),
            TypedConfig {
                signed: None,
                unsigned: None,
                timeout: None,
                mode: Some(Mode::Labeled(Label("text".to_owned()))),
                label: None,
            },
        ),
        (
            serde_json::json!({"mode": {"Record": {"value": 0}}}),
            TypedConfig {
                signed: None,
                unsigned: None,
                timeout: None,
                mode: Some(Mode::Record { value: Some(0) }),
                label: None,
            },
        ),
        (
            serde_json::json!({"mode": {"Record": {"value": null}}}),
            TypedConfig {
                signed: None,
                unsigned: None,
                timeout: None,
                mode: Some(Mode::Record { value: None }),
                label: None,
            },
        ),
        (
            serde_json::json!({"signed": null, "unsigned": "0", "mode": "Fast"}),
            TypedConfig {
                signed: None,
                unsigned: Some(0),
                timeout: None,
                mode: Some(Mode::Fast),
                label: None,
            },
        ),
    ];
    for (object, expected) in cases {
        let mut dotted = Config::new();
        let mut root = Config::new();
        for (key, value) in object.as_object().expect("case object") {
            dotted
                .set(format!("case.{key}"), value.clone())
                .expect("set the dotted case field");
            root.set(key, value.clone()).expect("set the root case field");
        }
        let mut exact = Config::new();
        exact.set("case", object).expect("set the exact case object");
        if dotted.get::<Value>("case").is_err() {
            dotted
                .set("case.timeout", Value::Null)
                .expect("add the nested timeout field to preserve the case object");
        }
        for interpolate in [false, true] {
            let read_options = options(interpolate);
            assert_eq!(
                dotted
                    .deserialize_with::<TypedConfig>("case", read_options)
                    .expect("read typed values from the dotted shape"),
                expected
            );
            assert_eq!(
                dotted
                    .section("case")
                    .expect("select the dotted case section")
                    .deserialize_with::<TypedConfig>("", read_options)
                    .expect("read typed values from the dotted section"),
                expected
            );
            assert_eq!(
                root.deserialize_with::<TypedConfig>("", read_options)
                    .expect("read typed values from the root shape"),
                expected
            );
            assert_eq!(
                exact
                    .deserialize_with::<TypedConfig>("case", read_options)
                    .expect("read typed values from the exact shape"),
                expected
            );
        }
    }
}
