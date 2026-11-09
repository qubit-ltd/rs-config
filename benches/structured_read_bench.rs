// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Measures real structured configuration reads with fixtures outside timing.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::hint::black_box;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_config::Config;
use qubit_config::ConfigSerdeExt;
use serde::Deserialize;

/// Deserializes only the port field for unknown-field policy benchmarks.
#[derive(Debug, Deserialize)]
struct Selected {
    /// Port value selected from the server subtree.
    port: u16,
}

/// Measures exact text, property trees, native collections and interpolation.
fn benchmark_structured_reads(c: &mut Criterion) {
    for size in [16, 65_536] {
        let mut config = Config::new();
        config.set("payload", "x".repeat(size)).unwrap();
        assert_eq!(
            ConfigSerdeExt::deserialize::<String>(&config, "payload").unwrap().len(),
            size
        );
        c.bench_function(&format!("structured_read/exact_string/{size}"), |b| {
            b.iter(|| black_box(config.deserialize::<String>(black_box("payload")).unwrap()));
        });
    }
    for count in [1, 32, 1024] {
        let mut config = Config::new();
        for index in 0..count {
            config.set(format!("settings.field{index}"), index as i32).unwrap();
        }
        assert_eq!(
            config.deserialize::<BTreeMap<String, i32>>("settings").unwrap().len(),
            count
        );
        c.bench_function(&format!("structured_read/properties/{count}"), |b| {
            b.iter(|| {
                black_box(
                    config
                        .deserialize::<BTreeMap<String, i32>>(black_box("settings"))
                        .unwrap(),
                )
            });
        });
    }
    let mut config = Config::new();
    config.set("numbers", (0..1024_i32).collect::<Vec<_>>()).unwrap();
    assert_eq!(config.deserialize::<Vec<i32>>("numbers").unwrap().len(), 1024);
    c.bench_function("structured_read/numeric_collection/1024", |b| {
        b.iter(|| black_box(config.deserialize::<Vec<i32>>(black_box("numbers")).unwrap()));
    });
    for count in [32_usize, 1024] {
        let mut natural = Config::new();
        let input = serde_json::Value::Array((0..count).map(|index| serde_json::Value::from(index as u64)).collect());
        natural.set("numbers", input.clone()).unwrap();
        assert_eq!(natural.deserialize::<serde_json::Value>("numbers").unwrap(), input);
        c.bench_function(&format!("structured_read/json_natural_numbers/{count}"), |b| {
            b.iter(|| black_box(natural.deserialize::<serde_json::Value>(black_box("numbers")).unwrap()));
        });
    }
    config
        .set(
            "nested",
            serde_json::json!({"child": {"items": [1, 2, 3], "label": "abc"}}),
        )
        .unwrap();
    assert_eq!(
        config.deserialize::<serde_json::Value>("nested").unwrap()["child"]["label"],
        "abc"
    );
    c.bench_function("structured_read/nested_json", |b| {
        b.iter(|| black_box(config.deserialize::<serde_json::Value>(black_box("nested")).unwrap()));
    });
    config.set("server.port", 8080_u16).unwrap();
    config.set("server.unused", "x".repeat(65_536)).unwrap();
    assert_eq!(
        config
            .deserialize_with::<Selected>(
                "server",
                qubit_config::ConfigDeserializeOptions {
                    interpolate: false,
                    unknown_fields: qubit_config::UnknownFieldPolicy::Ignore
                }
            )
            .unwrap()
            .port,
        8080
    );
    assert!(config.deserialize::<Selected>("server").is_err());
    c.bench_function("structured_read/unknown/lenient", |b| {
        b.iter(|| {
            black_box(
                config
                    .deserialize_with::<Selected>(
                        black_box("server"),
                        qubit_config::ConfigDeserializeOptions {
                            interpolate: false,
                            unknown_fields: qubit_config::UnknownFieldPolicy::Ignore,
                        },
                    )
                    .unwrap(),
            )
        });
    });
    c.bench_function("structured_read/unknown/reject", |b| {
        b.iter(|| black_box(config.deserialize::<Selected>(black_box("server")).unwrap_err()));
    });
    config.set("base", "localhost").unwrap();
    config.set("endpoint", "http://${base}:8080").unwrap();
    assert_eq!(
        config
            .deserialize_with::<String>(
                "endpoint",
                qubit_config::ConfigDeserializeOptions {
                    interpolate: true,
                    unknown_fields: qubit_config::UnknownFieldPolicy::Reject
                }
            )
            .unwrap(),
        "http://localhost:8080"
    );
    assert_eq!(config.deserialize::<String>("endpoint").unwrap(), "http://${base}:8080");
    c.bench_function("structured_read/interpolation/enabled", |b| {
        b.iter(|| {
            black_box(
                config
                    .deserialize_with::<String>(
                        black_box("endpoint"),
                        qubit_config::ConfigDeserializeOptions {
                            interpolate: true,
                            unknown_fields: qubit_config::UnknownFieldPolicy::Reject,
                        },
                    )
                    .unwrap(),
            )
        });
    });
    c.bench_function("structured_read/interpolation/disabled", |b| {
        b.iter(|| black_box(config.deserialize::<String>(black_box("endpoint")).unwrap()));
    });
    for count in [0_usize, 1, 16, 1024, 16_384] {
        let mut string_map = Config::new();
        let entries = (0..count)
            .map(|index| (format!("key{index:05}"), format!("value{index:05}")))
            .collect::<HashMap<_, _>>();
        string_map.set("map", entries).unwrap();
        assert_eq!(
            string_map.deserialize::<HashMap<String, String>>("map").unwrap().len(),
            count
        );
        for (label, interpolate) in [("default", false), ("interpolation", true)] {
            c.bench_function(&format!("structured_read/string_map/{label}/{count}"), |b| {
                b.iter(|| {
                    black_box(
                        string_map
                            .deserialize_with::<HashMap<String, String>>(
                                black_box("map"),
                                qubit_config::ConfigDeserializeOptions {
                                    interpolate,
                                    unknown_fields: qubit_config::UnknownFieldPolicy::Reject,
                                },
                            )
                            .unwrap(),
                    )
                });
            });
        }
    }
}

/// Measures reads of a fixed subtree as unrelated reader properties grow.
fn benchmark_fixed_subtree(c: &mut Criterion) {
    for unrelated in [0, 1024, 16384] {
        let mut config = Config::new();
        config.set("settings.a", 1_i32).unwrap();
        config.set("settings.b", 2_i32).unwrap();
        for index in 0..unrelated {
            config.set(format!("unrelated.field{index}"), index).unwrap();
        }
        assert_eq!(
            config.deserialize::<BTreeMap<String, i32>>("settings").unwrap().len(),
            2
        );
        c.bench_function(&format!("structured_read/fixed_subtree/{unrelated}"), |b| {
            b.iter(|| {
                black_box(
                    config
                        .deserialize::<BTreeMap<String, i32>>(black_box("settings"))
                        .unwrap(),
                )
            });
        });
    }
}

/// Measures rejection of a single property with a deeply dotted key.
fn benchmark_deep_key_rejection(c: &mut Criterion) {
    for segment_count in [64, 256, 1024, 4096] {
        let key = std::iter::repeat_n("a", segment_count).collect::<Vec<_>>().join(".");
        let limits = qubit_datatype::ConversionLimits::builder()
            .operation_limits(
                qubit_datatype::ConversionOperationLimits::builder()
                    .max_input_bytes(0)
                    .build(),
            )
            .build();
        let policy = qubit_config::ReadPolicy::builder().conversion_limits(limits).build();
        let mut config = Config::builder().default_read_policy(policy).build();
        config.set(key, "value").unwrap();
        assert!(config.deserialize::<serde_json::Value>("").is_err());

        c.bench_function(&format!("structured_read/deep_key_rejection/{segment_count}"), |b| {
            b.iter(|| black_box(config.deserialize::<serde_json::Value>(black_box("")).unwrap_err()));
        });
    }
}

criterion_group!(
    benches,
    benchmark_structured_reads,
    benchmark_fixed_subtree,
    benchmark_deep_key_rejection
);
criterion_main!(benches);
