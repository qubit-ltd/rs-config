// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Measures bounded Config wire encoding with fixtures built outside timing.

use std::hint::black_box;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_config::Config;
use qubit_config::ConfigWireLimits;
use serde_json::Value;

const LARGE_VALUE_COUNT: usize = 100_000;
const MULTI_PROPERTY_COUNT: usize = 100;
const VALUES_PER_PROPERTY: usize = 1_000;
const MAX_BENCH_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_BENCH_NODES: u64 = 200_000;
const MAX_BENCH_SEQUENCE_ITEMS: u64 = LARGE_VALUE_COUNT as u64;
const MAX_BENCH_PAYLOAD_BYTES: u64 = 4 * 1024 * 1024;

/// Holds a named configuration used by one bounded wire-encoding benchmark.
struct ConfigFixture {
    /// Criterion identifier for this input shape.
    name: &'static str,
    /// Configuration prepared before timed iterations.
    config: Config,
}

/// Builds the shared limits and input fixtures before timed iterations.
fn config_fixtures() -> (ConfigWireLimits, Vec<ConfigFixture>) {
    let json_encode = ConfigWireLimits::default()
        .json_encode()
        .into_builder()
        .max_output_bytes(MAX_BENCH_OUTPUT_BYTES)
        .max_nodes(MAX_BENCH_NODES)
        .max_sequence_items(MAX_BENCH_SEQUENCE_ITEMS)
        .max_payload_bytes(MAX_BENCH_PAYLOAD_BYTES)
        .build();
    let limits = ConfigWireLimits::builder().json_encode(json_encode).build();

    let mut small = Config::new();
    for index in 0..4 {
        small
            .set(format!("integer_{index}"), index as i64)
            .expect("valid benchmark property");
    }

    let mut one_large_json = Config::new();
    one_large_json
        .set("payload", integer_array(LARGE_VALUE_COUNT))
        .expect("valid benchmark property");

    let mut many_json_properties = Config::new();
    for index in 0..MULTI_PROPERTY_COUNT {
        many_json_properties
            .set(format!("payload_{index:03}"), integer_array(VALUES_PER_PROPERTY))
            .expect("valid benchmark property");
    }

    let mut float_collection = Config::new();
    float_collection
        .set(
            "values",
            (0..LARGE_VALUE_COUNT)
                .map(|index| index as f64 / 10.0)
                .collect::<Vec<_>>(),
        )
        .expect("valid benchmark property");

    let fixtures = vec![
        ConfigFixture {
            name: "small_config_4_i64_properties",
            config: small,
        },
        ConfigFixture {
            name: "one_json_100000_integer_nodes",
            config: one_large_json,
        },
        ConfigFixture {
            name: "100_json_properties_1000_integer_nodes_each",
            config: many_json_properties,
        },
        ConfigFixture {
            name: "one_f64_collection_100000_values",
            config: float_collection,
        },
    ];

    (limits, fixtures)
}

/// Builds a JSON array with the requested number of integer values.
fn integer_array(size: usize) -> Value {
    Value::Array((0..size).map(|index| Value::from(index as u64)).collect())
}

/// Benchmarks the existing bounded aggregate Config V1 wire encoder.
fn benchmark_config_wire_encode(criterion: &mut Criterion) {
    let (limits, fixtures) = config_fixtures();

    for fixture in &fixtures {
        let encoded = fixture
            .config
            .encode_json_vec_with_limits(limits)
            .expect("benchmark fixture must fit the shared wire limits");
        eprintln!("fixture={} encoded_bytes={}", fixture.name, encoded.len());
        criterion.bench_function(&format!("config_wire_encode/{}", fixture.name), |bencher| {
            bencher.iter(|| {
                black_box(
                    fixture
                        .config
                        .encode_json_vec_with_limits(limits)
                        .expect("benchmark fixture must encode successfully"),
                )
            });
        });
    }
}

criterion_group!(benches, benchmark_config_wire_encode);
criterion_main!(benches);
