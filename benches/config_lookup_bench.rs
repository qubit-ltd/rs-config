// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Representative lookup and persistence benchmarks for `Config`.

use std::hint::black_box;

use criterion::{
    Criterion,
    criterion_group,
    criterion_main,
};
use qubit_config::{
    Config,
    ConfigReader,
};

/// Number of keys used to make prefix and section scans representative.
const PROPERTY_COUNT: usize = 1_024;

/// Builds a fixed configuration outside benchmark timing loops.
fn build_config() -> Config {
    let mut config = Config::new();
    for index in 0..PROPERTY_COUNT {
        let key = if index % 2 == 0 {
            format!("service.endpoint_{index}")
        } else {
            format!("other.setting_{index}")
        };
        config
            .set(key, index as u64)
            .expect("benchmark fixture keys should be valid");
    }
    config
}

/// Benchmarks exact-key, prefix, section, and persistence operations.
fn benchmark_config_lookup(c: &mut Criterion) {
    let config = build_config();
    let section = config.section("service");
    let mut group = c.benchmark_group("config_lookup");

    group.bench_function("exact_key", |b| {
        b.iter(|| {
            black_box(
                config
                    .get::<u64>(black_box("service.endpoint_512"))
                    .expect("benchmark fixture value should be readable"),
            );
        });
    });
    group.bench_function("prefix_scan", |b| {
        b.iter(|| {
            black_box(config.iter_prefix(black_box("service.")).count());
        });
    });
    group.bench_function("section_scan", |b| {
        b.iter(|| {
            black_box(section.iter().count());
        });
    });
    group.bench_function("serialize_v1", |b| {
        b.iter(|| {
            black_box(
                serde_json::to_vec(black_box(&config))
                    .expect("benchmark config should serialize"),
            );
        });
    });
    group.finish();
}

criterion_group!(benches, benchmark_config_lookup);
criterion_main!(benches);
