/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for field-level configuration read declarations.

use qubit_config::{Config, field::ConfigField, options::ConfigReadOptions};

#[test]
fn test_config_field_builder_reads_alias_with_env_friendly_bool() {
    let mut config = Config::new();
    config
        .set("MIME_DETECTOR_ENABLE_PRECISE_DETECTION", "yes")
        .expect("setting test config should succeed");

    let enabled = config
        .read(
            ConfigField::<bool>::builder()
                .name("mime.enable_precise_detection")
                .alias("MIME_DETECTOR_ENABLE_PRECISE_DETECTION")
                .alias("ANOTHER_MIME_DETECTOR_ENABLE_PRECISE_DETECTION_PROPERTY")
                .default(false)
                .read_options(ConfigReadOptions::env_friendly())
                .build(),
        )
        .expect("env-friendly boolean alias should parse");

    assert!(enabled);
}
