// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Typed collection conversion benchmarks.

use std::collections::HashMap;
use std::hint::black_box;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
#[cfg(feature = "rich-types")]
use num_bigint::BigInt;
use qubit_config::Config;

const COLLECTION_SIZES: [usize; 3] = [32, 1_024, 16_384];

fn collection_fixtures() -> Vec<(usize, Config)> {
    COLLECTION_SIZES
        .into_iter()
        .map(|size| {
            let mut config = Config::new();
            config
                .set("i64_values", (0..size).map(|value| value as i64).collect::<Vec<_>>())
                .expect("valid benchmark key");
            config
                .set(
                    "string_values",
                    (0..size).map(|value| format!("value_{value}")).collect::<Vec<_>>(),
                )
                .expect("valid benchmark key");
            let map = (0..size)
                .map(|value| (format!("key_{value}"), format!("value_{value}")))
                .collect::<HashMap<_, _>>();
            config.set("string_map", map).expect("valid benchmark key");

            #[cfg(feature = "rich-types")]
            config
                .set(
                    "big_integers",
                    (0..size).map(|value| BigInt::from(value as i64)).collect::<Vec<_>>(),
                )
                .expect("valid benchmark key");

            (size, config)
        })
        .collect()
}

fn benchmark_typed_collections(criterion: &mut Criterion) {
    let fixtures = collection_fixtures();

    // Validate every prepared conversion once before registering timed
    // iterations so errors cannot be treated as benchmark samples.
    for (size, config) in &fixtures {
        assert_eq!(config.get::<Vec<i64>>("i64_values").unwrap().len(), *size);
        assert_eq!(config.get::<Vec<String>>("string_values").unwrap().len(), *size);
        assert_eq!(
            config.get::<HashMap<String, String>>("string_map").unwrap().len(),
            *size
        );

        #[cfg(feature = "rich-types")]
        assert_eq!(config.get::<Vec<BigInt>>("big_integers").unwrap().len(), *size);
    }

    let mut group = criterion.benchmark_group("typed_collections");

    for (size, config) in &fixtures {
        group.bench_with_input(BenchmarkId::new("vec_i64", size), config, |bencher, config| {
            bencher.iter(|| black_box(config.get::<Vec<i64>>("i64_values").unwrap()));
        });
        group.bench_with_input(BenchmarkId::new("vec_string", size), config, |bencher, config| {
            bencher.iter(|| black_box(config.get::<Vec<String>>("string_values").unwrap()));
        });
        group.bench_with_input(BenchmarkId::new("string_map", size), config, |bencher, config| {
            bencher.iter(|| black_box(config.get::<HashMap<String, String>>("string_map").unwrap()));
        });

        #[cfg(feature = "rich-types")]
        group.bench_with_input(BenchmarkId::new("vec_big_int", size), config, |bencher, config| {
            bencher.iter(|| black_box(config.get::<Vec<BigInt>>("big_integers").unwrap()));
        });
    }
    group.finish();
}

criterion_group!(benches, benchmark_typed_collections);
criterion_main!(benches);
