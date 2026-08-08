// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Configuration lookup and structured-read scaling benchmarks.

use std::hint::black_box;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_config::Config;
use qubit_config::ConfigReader;

const PROPERTY_COUNTS: [usize; 3] = [32, 1_024, 16_384];

/// Builds a fixed configuration outside benchmark timing loops.
fn build_config(property_count: usize) -> Config {
    let mut config = Config::new();
    for index in 0..property_count {
        let key = match index {
            0 => "sparse.target".to_owned(),
            1 => "deep.region.zone.endpoint".to_owned(),
            _ if index % 2 == 0 => format!("service.endpoint_{index}"),
            _ => format!("other.setting_{index}"),
        };
        config
            .set(key, index as u64)
            .expect("benchmark fixture keys should be canonical");
    }
    config
}

/// Benchmarks operations whose cost may depend on total property count.
fn benchmark_config_lookup(criterion: &mut Criterion) {
    for property_count in PROPERTY_COUNTS {
        let config = build_config(property_count);
        let exact_index = property_count.saturating_sub(2) & !1;
        let exact_key = format!("service.endpoint_{exact_index}");
        let section = config
            .section("service")
            .expect("benchmark section path should be canonical");
        let deep_section = config
            .section("deep.region.zone")
            .expect("benchmark deep section path should be canonical");

        assert_eq!(config.len(), property_count);
        assert!(config.contains_section("service").unwrap());
        assert!(config.contains_section("sparse").unwrap());
        assert!(config.contains_section("deep.region.zone").unwrap());
        assert!(!config.contains_section("missing").unwrap());
        assert_eq!(config.iter_prefix("sparse.").count(), 1);
        assert_eq!(config.iter_prefix("missing.").count(), 0);
        assert_eq!(deep_section.get::<u64>("endpoint").unwrap(), 1);

        let mut exact_get = criterion.benchmark_group("exact_get");
        exact_get.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        config
                            .get::<u64>(black_box(exact_key.as_str()))
                            .unwrap(),
                    );
                });
            },
        );
        exact_get.finish();

        let mut get_property = criterion.benchmark_group("get_property");
        get_property.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        config
                            .get_property(black_box(exact_key.as_str()))
                            .unwrap()
                            .unwrap(),
                    );
                });
            },
        );
        get_property.finish();

        let mut contains = criterion.benchmark_group("contains");
        contains.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        config.contains(black_box(exact_key.as_str())).unwrap(),
                    );
                });
            },
        );
        contains.finish();

        let mut contains_section =
            criterion.benchmark_group("contains_section");
        contains_section.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        config.contains_section(black_box("service")).unwrap(),
                    );
                });
            },
        );
        contains_section.finish();

        let mut contains_section_sparse =
            criterion.benchmark_group("contains_section_sparse");
        contains_section_sparse.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        config.contains_section(black_box("sparse")).unwrap(),
                    );
                });
            },
        );
        contains_section_sparse.finish();

        let mut contains_section_absent =
            criterion.benchmark_group("contains_section_absent");
        contains_section_absent.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        config.contains_section(black_box("missing")).unwrap(),
                    );
                });
            },
        );
        contains_section_absent.finish();

        let mut section_get = criterion.benchmark_group("section_get");
        section_get.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                let relative_key = format!("endpoint_{exact_index}");
                bencher.iter(|| {
                    black_box(
                        section
                            .get::<u64>(black_box(relative_key.as_str()))
                            .unwrap(),
                    );
                });
            },
        );
        section_get.finish();

        let mut deep_section_get =
            criterion.benchmark_group("deep_section_get");
        deep_section_get.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        deep_section.get::<u64>(black_box("endpoint")).unwrap(),
                    );
                });
            },
        );
        deep_section_get.finish();

        let mut iter_prefix = criterion.benchmark_group("iter_prefix");
        iter_prefix.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        config.iter_prefix(black_box("service.")).count(),
                    );
                });
            },
        );
        iter_prefix.finish();

        let mut iter_prefix_sparse =
            criterion.benchmark_group("iter_prefix_sparse");
        iter_prefix_sparse.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(config.iter_prefix(black_box("sparse.")).count());
                });
            },
        );
        iter_prefix_sparse.finish();

        let mut iter_prefix_absent =
            criterion.benchmark_group("iter_prefix_absent");
        iter_prefix_absent.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        config.iter_prefix(black_box("missing.")).count(),
                    );
                });
            },
        );
        iter_prefix_absent.finish();

        let mut structured_deserialize =
            criterion.benchmark_group("structured_deserialize");
        structured_deserialize.bench_with_input(
            BenchmarkId::from_parameter(property_count),
            &property_count,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(
                        config
                            .deserialize::<serde_json::Value>("service")
                            .unwrap(),
                    );
                });
            },
        );
        structured_deserialize.finish();
    }
}

criterion_group!(benches, benchmark_config_lookup);
criterion_main!(benches);
