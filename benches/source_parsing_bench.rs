// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Configuration source parsing benchmarks.

use std::hint::black_box;

use criterion::BatchSize;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_config::Config;
use qubit_config::ConfigSource;
use qubit_config::source::CompositeConfigSource;
#[cfg(feature = "env-file")]
use qubit_config::source::EnvFileConfigSource;
use qubit_config::source::PropertiesConfigSource;
#[cfg(feature = "toml")]
use qubit_config::source::TomlConfigSource;
#[cfg(feature = "yaml")]
use qubit_config::source::YamlConfigSource;

const PROPERTY_COUNTS: [usize; 3] = [32, 1_024, 16_384];

/// Builds properties-format input with the requested number of assignments.
fn properties_content(property_count: usize) -> String {
    (0..property_count)
        .map(|index| format!("service.endpoint_{index}={index}\n"))
        .collect()
}

#[cfg(feature = "env-file")]
/// Builds environment-file input with the requested number of assignments.
fn env_file_content(property_count: usize) -> String {
    (0..property_count)
        .map(|index| format!("SERVICE_ENDPOINT_{index}={index}\n"))
        .collect()
}

#[cfg(feature = "toml")]
/// Builds TOML input with the requested number of assignments.
fn toml_content(property_count: usize) -> String {
    (0..property_count)
        .map(|index| format!("service.endpoint_{index} = {index}\n"))
        .collect()
}

#[cfg(feature = "yaml")]
/// Builds YAML input with the requested number of assignments.
fn yaml_content(property_count: usize) -> String {
    (0..property_count)
        .map(|index| format!("service.endpoint_{index}: {index}\n"))
        .collect()
}

/// Builds all source fixtures before Criterion starts timing iterations.
fn source_fixtures(property_count: usize) -> Vec<(&'static str, Box<dyn ConfigSource>)> {
    let fixtures: Vec<(&'static str, Box<dyn ConfigSource>)> = vec![
        (
            "properties",
            Box::new(PropertiesConfigSource::from_content(properties_content(property_count))),
        ),
        #[cfg(feature = "env-file")]
        (
            "env-file",
            Box::new(EnvFileConfigSource::from_content(env_file_content(property_count))),
        ),
        #[cfg(feature = "toml")]
        (
            "toml",
            Box::new(TomlConfigSource::from_content(toml_content(property_count))),
        ),
        #[cfg(feature = "yaml")]
        (
            "yaml",
            Box::new(YamlConfigSource::from_content(yaml_content(property_count))),
        ),
    ];
    fixtures
}

/// Benchmarks loading prepared source formats across configuration sizes.
fn benchmark_source_parsing(criterion: &mut Criterion) {
    let fixtures = PROPERTY_COUNTS
        .into_iter()
        .map(|count| (count, source_fixtures(count)))
        .collect::<Vec<_>>();

    // Validate every prepared source once before registering timed iterations.
    for (property_count, sources) in &fixtures {
        for (format, source) in sources {
            let config = source
                .load()
                .unwrap_or_else(|error| panic!("{format} fixture should load: {error}"));
            assert_eq!(config.len(), *property_count);
        }
    }

    let mut group = criterion.benchmark_group("source_parsing");

    for (property_count, sources) in &fixtures {
        for (format, source) in sources {
            group.bench_with_input(BenchmarkId::new(*format, property_count), source, |bencher, source| {
                bencher.iter(|| black_box(source.load().unwrap()));
            });
        }
    }
    group.finish();
}

/// Benchmarks parsing properties values continued across physical lines.
fn benchmark_continuation_backslashes(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("properties_continuation_backslashes");
    for count in [256_usize, 1_024, 4_096] {
        let physical_line = format!("{}\n", "\\".repeat(3));
        let input = format!("key={}end", physical_line.repeat(count));
        let expected = format!("{}end", "\\".repeat(count));
        assert_eq!(PropertiesConfigSource::parse_content(&input).unwrap()[0].1, expected);
        group.bench_with_input(BenchmarkId::from_parameter(count), &input, |b, input| {
            b.iter(|| PropertiesConfigSource::parse_content(black_box(input)).unwrap());
        });
    }
    group.finish();
}

/// Benchmarks the public collecting parser and the direct source load path.
fn benchmark_properties_parse_and_load(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("properties");
    for count in [32_usize, 1_024, 16_384] {
        for repeated in [false, true] {
            let content = (0..count)
                .map(|index| {
                    let key = if repeated {
                        "service.shared".to_string()
                    } else {
                        format!("service.key_{index}")
                    };
                    format!("{key}={index}\n")
                })
                .collect::<String>();
            let source = PropertiesConfigSource::from_content(content.clone());
            let label = format!("{count}/{}", if repeated { "repeated" } else { "distinct" });
            group.bench_function(BenchmarkId::new("parse", &label), |b| {
                b.iter(|| black_box(PropertiesConfigSource::parse_content(black_box(&content)).unwrap()));
            });
            group.bench_function(BenchmarkId::new("load", &label), |b| {
                b.iter(|| black_box(source.load().unwrap()));
            });
            group.bench_function(BenchmarkId::new("legacy_collect_and_set", &label), |b| {
                b.iter(|| {
                    let pairs = PropertiesConfigSource::parse_content(black_box(&content)).unwrap();
                    let mut config = Config::new();
                    for (key, value) in pairs {
                        config.set(&key, value).unwrap();
                    }
                    black_box(config)
                });
            });
        }
    }
    group.finish();
}

/// Loads a properties configuration with generated keys and values.
fn config_with_properties(property_count: usize, key_prefix: &str, value_prefix: &str) -> Config {
    let content = (0..property_count)
        .map(|index| format!("{key_prefix}{index}={value_prefix}{index}\n"))
        .collect::<String>();
    PropertiesConfigSource::from_content(content)
        .load()
        .expect("prepared properties should load")
}

/// Benchmarks merging overlapping and disjoint configurations of varied sizes.
fn benchmark_config_merge(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("config_merge");
    for target_size in [1_024_usize, 16_384] {
        for incoming_size in [1_usize, 32, 1_024] {
            for overlap in [false, true] {
                let target = config_with_properties(target_size, "target.key_", "old_");
                let incoming = if overlap {
                    config_with_properties(incoming_size, "target.key_", "new_")
                } else {
                    config_with_properties(incoming_size, "incoming.key_", "new_")
                };
                let expected_size = if overlap {
                    target_size
                } else {
                    target_size + incoming_size
                };
                let checked = {
                    let mut merged = target.clone();
                    merged
                        .merge_properties(incoming.clone())
                        .expect("prepared merge should succeed");
                    merged
                };
                assert_eq!(checked.len(), expected_size);
                assert_eq!(
                    checked
                        .get::<String>(if overlap { "target.key_0" } else { "incoming.key_0" })
                        .unwrap(),
                    "new_0"
                );

                let id = format!(
                    "target_{target_size}/incoming_{incoming_size}/{}",
                    if overlap { "overlap" } else { "disjoint" }
                );
                group.bench_function(BenchmarkId::new("merge", id), |bencher| {
                    bencher.iter_batched(
                        || (target.clone(), incoming.clone()),
                        |(mut target, incoming)| {
                            target.merge_properties(incoming).unwrap();
                            black_box(target)
                        },
                        BatchSize::SmallInput,
                    );
                });
            }
        }
    }
    group.finish();
}

/// Benchmarks loading composite sources with overlapping or distinct keys.
fn benchmark_composite_layers(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("composite_layers");
    for layer_count in [1_usize, 8, 64] {
        for overlap in [false, true] {
            let mut composite = CompositeConfigSource::new();
            for layer in 0..layer_count {
                let content = (0..32_usize)
                    .map(|assignment| {
                        let key = if overlap {
                            format!("service.endpoint_{assignment}")
                        } else {
                            format!("service.layer_{layer}_endpoint_{assignment}")
                        };
                        format!("{key}=layer_{layer}_{assignment}\n")
                    })
                    .collect::<String>();
                composite.add(PropertiesConfigSource::from_content(content));
            }

            let loaded = composite.load().expect("prepared composite should load");
            let expected_size = if overlap { 32 } else { layer_count * 32 };
            assert_eq!(loaded.len(), expected_size);
            let expected_key = if overlap {
                "service.endpoint_0".to_string()
            } else {
                format!("service.layer_{}_endpoint_0", layer_count - 1)
            };
            assert_eq!(
                loaded.get::<String>(&expected_key).unwrap(),
                format!("layer_{}_0", layer_count - 1)
            );
            if !overlap {
                assert_eq!(
                    loaded
                        .get::<String>(&format!("service.layer_{}_endpoint_31", layer_count - 1))
                        .unwrap(),
                    format!("layer_{}_31", layer_count - 1)
                );
            }

            let id = format!(
                "layers_{layer_count}/{}",
                if overlap { "same_keys" } else { "disjoint" }
            );
            group.bench_function(BenchmarkId::new("load", id), |bencher| {
                bencher.iter(|| black_box(composite.load().unwrap()));
            });
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    benchmark_source_parsing,
    benchmark_properties_parse_and_load,
    benchmark_continuation_backslashes,
    benchmark_config_merge,
    benchmark_composite_layers
);
criterion_main!(benches);
