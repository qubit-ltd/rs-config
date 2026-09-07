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
//! Variable interpolation benchmarks.

use std::hint::black_box;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_config::Config;
use qubit_config::ReadPolicy;

struct InterpolationFixture {
    name: &'static str,
    config: Config,
    key: &'static str,
}

/// Builds interpolation fixtures, including the near-budget output case,
/// before Criterion starts timing iterations.
fn interpolation_fixtures() -> Vec<InterpolationFixture> {
    let mut single = Config::new();
    single
        .set("host", "localhost")
        .expect("valid benchmark key");
    single
        .set("value", "https://${host}")
        .expect("valid benchmark key");

    let mut multiple = Config::new();
    multiple
        .set("scheme", "https")
        .expect("valid benchmark key");
    multiple
        .set("host", "config.example")
        .expect("valid benchmark key");
    multiple.set("port", 8443).expect("valid benchmark key");
    multiple
        .set("value", "${scheme}://${host}:${port}/api")
        .expect("valid benchmark key");

    let mut deep = Config::new();
    for index in (1..=32).rev() {
        let value = if index == 32 {
            "endpoint".to_owned()
        } else {
            format!("${{level_{}}}", index + 1)
        };
        deep.set(format!("level_{index}"), value)
            .expect("valid benchmark key");
    }
    deep.set("value", "${level_1}")
        .expect("valid benchmark key");

    let output_budget = ReadPolicy::default().max_interpolation_output_bytes();
    let near_budget_value = "x".repeat(output_budget.saturating_sub(1_024));
    let mut near_budget = Config::new();
    near_budget
        .set("payload", near_budget_value)
        .expect("valid benchmark key");
    near_budget
        .set("value", "${payload}")
        .expect("valid benchmark key");

    vec![
        InterpolationFixture {
            name: "single_variable",
            config: single,
            key: "value",
        },
        InterpolationFixture {
            name: "multiple_variables",
            config: multiple,
            key: "value",
        },
        InterpolationFixture {
            name: "deep_chain",
            config: deep,
            key: "value",
        },
        InterpolationFixture {
            name: "near_output_budget",
            config: near_budget,
            key: "value",
        },
    ]
}

fn benchmark_interpolation(criterion: &mut Criterion) {
    let fixtures = interpolation_fixtures();

    // Validate every prepared interpolation once before registering timed
    // iterations so errors cannot be treated as benchmark samples.
    for fixture in &fixtures {
        fixture
            .config
            .get_interpolated::<String>(fixture.key)
            .unwrap_or_else(|error| panic!("{} fixture should interpolate: {error}", fixture.name));
    }

    let mut group = criterion.benchmark_group("interpolation");

    for fixture in &fixtures {
        group.bench_function(fixture.name, |bencher| {
            bencher.iter(|| {
                black_box(
                    fixture
                        .config
                        .get_interpolated::<String>(fixture.key)
                        .unwrap(),
                )
            });
        });
    }
    group.finish();
}

criterion_group!(benches, benchmark_interpolation);
criterion_main!(benches);
