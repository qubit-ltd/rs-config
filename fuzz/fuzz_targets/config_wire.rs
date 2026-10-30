// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![no_main]

//! Fuzzes bounded legacy and V1 `Config` JSON persistence decoding.

use libfuzzer_sys::fuzz_target;
use qubit_budget::BudgetError;
use qubit_budget::json::JsonResource;
use qubit_config::Config;
use qubit_config::ConfigWireEncodeError;
use qubit_config::ConfigWireLimitKind;
use qubit_config::ConfigWireLimits;

/// Bounds JSON parsing and nested wire allocation while retaining useful input.
const MAX_JSON_BYTES: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_JSON_BYTES {
        return;
    }

    let Ok(config) = Config::decode_json_slice(data) else {
        return;
    };

    let encoded = serde_json::to_vec(&config).expect("a decoded config must serialize through the V1 wire format");
    let limits = ConfigWireLimits::builder()
        .max_input_bytes(u64::try_from(encoded.len()).expect("encoded length must fit u64"))
        .build();
    let decoded = Config::decode_json_slice_with_limits(&encoded, limits)
        .expect("a serialized V1 config must deserialize within its exact input-byte limit");

    assert_eq!(decoded, config);

    if let Ok(encoded) = config.encode_json_vec() {
        let limits = ConfigWireLimits::builder()
            .max_input_bytes(u64::try_from(encoded.len()).expect("encoded length must fit u64"))
            .build();
        let restored = Config::decode_json_slice_with_limits(&encoded, limits)
            .expect("bounded V1 output must round-trip within matching decode limits");
        assert_eq!(restored, config);
    }

    if !config.is_empty() {
        let limits = ConfigWireLimits::builder().max_properties(0).build();
        assert!(matches!(
            config.encode_json_vec_with_limits(limits),
            Err(ConfigWireEncodeError::LimitExceeded {
                kind: ConfigWireLimitKind::Properties,
                ..
            })
        ));
    }

    let mut small_config = Config::new();
    small_config
        .set("bounded", true)
        .expect("the bounded-output oracle config should be valid");
    let default_limits = ConfigWireLimits::default();
    // Leave room for the value's preflight projection so the complete wire
    // envelope reaches the bounded JSON writer and exercises final output
    // accounting rather than the earlier value preflight path.
    let output_tiny = default_limits.json_encode().into_builder().max_output_bytes(1).build();
    let limits = ConfigWireLimits::builder_from(&default_limits)
        .json_encode(output_tiny)
        .build();
    let output_tiny_result = small_config.encode_json_vec_with_limits(limits);
    assert!(
        matches!(
            &output_tiny_result,
            Err(ConfigWireEncodeError::Budget(BudgetError::Insufficient {
                resource: JsonResource::OutputBytes,
                ..
            }))
        ),
        "unexpected tiny output-limit result: {output_tiny_result:?}"
    );

    let maximum = u64::try_from(data.len()).expect("input length must fit u64");
    let output_limit = default_limits
        .json_encode()
        .into_builder()
        .max_output_bytes(maximum)
        .build();
    let limits = ConfigWireLimits::builder_from(&default_limits)
        .json_encode(output_limit)
        .build();
    if let Ok(encoded) = config.encode_json_vec_with_limits(limits) {
        assert!(u64::try_from(encoded.len()).expect("encoded length must fit u64") <= maximum);
    }
});
