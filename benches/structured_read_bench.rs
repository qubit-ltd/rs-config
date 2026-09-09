// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Measures real structured configuration reads with fixtures outside timing.

use std::collections::BTreeMap;
use std::hint::black_box;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_config::Config;
use qubit_config::ConfigSerdeExt;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Selected {
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
}

criterion_group!(benches, benchmark_structured_reads);
criterion_main!(benches);
