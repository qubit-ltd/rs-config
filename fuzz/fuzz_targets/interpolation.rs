// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

//! Fuzzes bounded explicit interpolation without process-environment input.

use libfuzzer_sys::fuzz_target;
use qubit_config::Config;
use qubit_config::ConfigDeserializeOptions;
use qubit_config::ConfigReader;
use qubit_config::ConfigSerdeExt;
use qubit_config::UnknownFieldPolicy;
use qubit_config::options::ReadPolicy;
use serde::Deserialize;

/// Bounds interpolation source length before configuration allocation.
const MAX_INPUT_BYTES: usize = 32 * 1024;
const MAX_GRAPH_NODES: usize = 16;
const MAX_VALUE_BYTES: usize = 1024;

#[derive(Deserialize)]
struct ReferenceGraph {
    values: Vec<String>,
    entry: usize,
    depth: usize,
    expansions: usize,
    output_bytes: usize,
}

fn fuzz_reference_graph(graph: ReferenceGraph) {
    if graph.values.is_empty()
        || graph.values.len() > MAX_GRAPH_NODES
        || graph.values.iter().any(|value| value.len() > MAX_VALUE_BYTES)
    {
        return;
    }
    let mut config = Config::builder()
        .default_read_policy(
            ReadPolicy::builder()
                .max_interpolation_depth(graph.depth.min(16))
                .max_interpolation_expansions(graph.expansions.min(128))
                .max_interpolation_output_bytes(graph.output_bytes.min(4096))
                .build(),
        )
        .build();
    for (index, value) in graph.values.iter().enumerate() {
        config
            .set(format!("k{index}"), value.as_str())
            .expect("fixed key should be valid");
        if index + 1 < graph.values.len() || graph.values.len() == 1 {
            config
                .set(format!("scope.k{index}"), value.as_str())
                .expect("fixed scope key should be valid");
        }
    }
    let entry = graph.entry % graph.values.len();
    let key = format!("k{entry}");
    let _ = config.get_interpolated::<String>(&key);
    let scope = config.section("scope");
    if let Ok(scope) = scope {
        let local_key = format!("k{entry}");
        let _ = scope.get_interpolated::<String>(&local_key);
        let _ = scope.deserialize_with::<serde_json::Value>(
            "",
            ConfigDeserializeOptions {
                interpolate: true,
                unknown_fields: UnknownFieldPolicy::Ignore,
            },
        );
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    if let Ok(graph) = serde_json::from_slice::<ReferenceGraph>(data) {
        fuzz_reference_graph(graph);
        return;
    }
    let Ok(value) = std::str::from_utf8(data) else {
        return;
    };

    let mut config = Config::builder().default_read_policy(ReadPolicy::config_only()).build();
    config
        .set("value", value)
        .expect("the fixed fuzzing key should always be valid");
    let _ = config.get_interpolated::<String>("value");
});
