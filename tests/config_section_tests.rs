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
    options::ReadOptions,
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

#[test]
fn test_read_options_view_is_borrowed_and_inherited_by_nested_sections() {
    let mut config = Config::new();
    config
        .set("service.child.values", "alpha,beta")
        .expect("the list should be configurable");
    let options = ReadOptions::env_friendly().with_max_interpolation_depth(7);

    let service = config.section("service").with_read_options_view(&options);
    let child = service.section("child");

    assert_eq!(service.scope_path(), "service");
    assert_eq!(child.scope_path(), "service.child");
    assert_eq!(service.read_options(), &options);
    assert_eq!(child.read_options(), &options);
    assert_eq!(
        child
            .get::<Vec<String>>("values")
            .expect("the borrowed options should split the list"),
        ["alpha", "beta"],
    );
}

#[test]
fn test_root_reader_scope_path_is_empty() {
    let config = Config::new();

    assert_eq!(config.scope_path(), "");
}

#[test]
fn test_section_missing_candidates_report_root_relative_paths() {
    let config = Config::new();

    let error = config
        .section("service")
        .get_any::<u16>(["port", "PORT"])
        .expect_err("missing candidates should fail");

    assert_eq!(error.path(), None);
    assert_eq!(
        error.candidate_paths(),
        Some(
            ["service.port".to_string(), "service.PORT".to_string()].as_slice(),
        ),
    );
}
