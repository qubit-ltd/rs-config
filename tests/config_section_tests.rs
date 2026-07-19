// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! [`qubit_config::ConfigSection`] tests.

use qubit_config::{
    Config,
    ConfigReader,
};

#[test]
fn test_section_resolves_keys_strictly_relative() {
    let mut config = Config::new();
    config
        .set("http.host", "direct")
        .expect("the direct host should be set");
    config
        .set("http.http.host", "strict-relative")
        .expect("the nested host should be set");

    let section = config.section("http");

    assert_eq!(section.path(), "http");
    assert_eq!(
        section
            .get::<String>("host")
            .expect("the direct host should be readable"),
        "direct",
    );
    assert_eq!(
        section
            .get::<String>("http.host")
            .expect("the qualified-looking relative key should be readable"),
        "strict-relative",
    );
}

#[test]
fn test_section_excludes_exact_root_property() {
    let mut config = Config::new();
    config
        .set("http", "root")
        .expect("the section root scalar should be set");
    config
        .set("http.host", "localhost")
        .expect("the section child should be set");

    let section = config.section("http");

    assert_eq!(section.len(), 1);
    assert_eq!(section.keys(), vec!["host".to_string()]);
    assert!(!section.contains(""));
    assert_eq!(
        config
            .get::<String>("http")
            .expect("the root config should still expose the scalar"),
        "root",
    );
}

#[test]
fn test_section_nests_and_reports_root_paths() {
    let config = Config::new();
    let proxy = config.section(".http.").section(".proxy.");

    assert_eq!(proxy.path(), "http.proxy");
    assert_eq!(proxy.resolve_key("host"), "http.proxy.host");
    assert_eq!(proxy.resolve_key(""), "http.proxy");
}
