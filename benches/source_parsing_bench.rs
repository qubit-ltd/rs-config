// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.
// =============================================================================
//! Configuration source parsing benchmarks.

use std::hint::black_box;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_config::ConfigSource;
#[cfg(feature = "env-file")]
use qubit_config::source::EnvFileConfigSource;
use qubit_config::source::PropertiesConfigSource;
#[cfg(feature = "toml")]
use qubit_config::source::TomlConfigSource;
#[cfg(feature = "yaml")]
use qubit_config::source::YamlConfigSource;

const PROPERTY_COUNTS: [usize; 3] = [32, 1_024, 16_384];

fn properties_content(property_count: usize) -> String {
    (0..property_count)
        .map(|index| format!("service.endpoint_{index}={index}\n"))
        .collect()
}

#[cfg(feature = "env-file")]
fn env_file_content(property_count: usize) -> String {
    (0..property_count)
        .map(|index| format!("SERVICE_ENDPOINT_{index}={index}\n"))
        .collect()
}

#[cfg(feature = "toml")]
fn toml_content(property_count: usize) -> String {
    (0..property_count)
        .map(|index| format!("service.endpoint_{index} = {index}\n"))
        .collect()
}

#[cfg(feature = "yaml")]
fn yaml_content(property_count: usize) -> String {
    (0..property_count)
        .map(|index| format!("service.endpoint_{index}: {index}\n"))
        .collect()
}

/// Builds all source fixtures before Criterion starts timing iterations.
fn source_fixtures(property_count: usize) -> Vec<(&'static str, Box<dyn ConfigSource>)> {
    let mut fixtures: Vec<(&'static str, Box<dyn ConfigSource>)> = vec![(
        "properties",
        Box::new(PropertiesConfigSource::from_content(properties_content(
            property_count,
        ))),
    )];

    #[cfg(feature = "env-file")]
    fixtures.push((
        "env-file",
        Box::new(EnvFileConfigSource::from_content(env_file_content(
            property_count,
        ))),
    ));
    #[cfg(feature = "toml")]
    fixtures.push((
        "toml",
        Box::new(TomlConfigSource::from_content(toml_content(property_count))),
    ));
    #[cfg(feature = "yaml")]
    fixtures.push((
        "yaml",
        Box::new(YamlConfigSource::from_content(yaml_content(property_count))),
    ));

    fixtures
}

fn benchmark_source_parsing(criterion: &mut Criterion) {
    let fixtures = PROPERTY_COUNTS
        .into_iter()
        .map(|count| (count, source_fixtures(count)))
        .collect::<Vec<_>>();
    let mut group = criterion.benchmark_group("source_parsing");

    for (property_count, sources) in &fixtures {
        for (format, source) in sources {
            group.bench_with_input(
                BenchmarkId::new(*format, property_count),
                source,
                |bencher, source| {
                    bencher.iter(|| black_box(source.load()));
                },
            );
        }
    }
    group.finish();
}

criterion_group!(benches, benchmark_source_parsing);
criterion_main!(benches);
