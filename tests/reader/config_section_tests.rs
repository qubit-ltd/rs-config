// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// [`qubit_config::ConfigSection`] tests.

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigReader;
use qubit_config::ConfigSection;
use qubit_config::options::ReadPolicy;

#[test]
fn test_section_resolves_keys_strictly_relative() {
    let mut config = Config::new();
    config
        .set("http.host", "direct")
        .expect("the direct host should be set");
    config
        .set("http.http.host", "strict-relative")
        .expect("the nested host should be set");

    let section = config.section("http").unwrap();

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

    let section = config.section("http").unwrap();

    assert_eq!(section.len(), 1);
    assert_eq!(section.keys(), vec!["host".to_string()]);
    assert!(matches!(section.contains(""), Err(ConfigError::InvalidKey { .. })));
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
    let proxy = config.section("http").unwrap().section("proxy").unwrap();

    assert_eq!(proxy.path(), "http.proxy");
    assert_eq!(proxy.resolve_key("host").unwrap(), "http.proxy.host");
    assert_eq!(proxy.resolve_key("").unwrap(), "http.proxy");
}

#[test]
fn test_read_policy_view_is_borrowed_and_inherited_by_nested_sections() {
    let mut config = Config::new();
    config
        .set("service.child.values", "alpha,beta")
        .expect("the list should be configurable");
    let options = ReadPolicy::builder_from(&ReadPolicy::env_friendly())
        .max_interpolation_depth(7)
        .build();

    let service = config.section("service").unwrap().read_with(&options);
    let child = service.section("child").unwrap();

    assert_eq!(service.scope_path(), "service");
    assert_eq!(child.scope_path(), "service.child");
    assert_eq!(service.read_policy(), &options);
    assert_eq!(child.read_policy(), &options);
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
        .unwrap()
        .get_any::<u16>(["port", "PORT"])
        .expect_err("missing candidates should fail");

    assert_eq!(error.path(), None);
    assert_eq!(
        error.candidate_paths(),
        Some(["service.port".to_string(), "service.PORT".to_string()].as_slice(),),
    );
}

/// Verifies section prefix iteration excludes adjacent sibling prefixes.
#[test]
fn test_section_iter_prefix_stays_within_full_prefix_boundary() {
    let mut config = Config::new();
    config.set("http.proxy.host", "localhost").unwrap();
    config.set("http.proxy.port", 8080_u16).unwrap();
    config.set("http.proxy2.host", "sibling").unwrap();
    config.set("http.server.host", "outside").unwrap();
    let section = config.section("http").unwrap();

    let keys: Vec<_> = ConfigReader::iter_prefix(&section, "proxy.")
        .map(|(key, _)| key)
        .collect();

    assert_eq!(keys, ["proxy.host", "proxy.port"]);
}

#[test]
fn test_section_relative_keys_cannot_escape_or_be_reinterpreted_as_absolute() {
    let mut config = Config::new();
    config.set("service.child.value", "inside").unwrap();
    config.set("service2.child.value", "sibling").unwrap();

    let service = config.section("service").unwrap();
    assert_eq!(service.get::<String>("child.value").unwrap(), "inside");
    assert!(matches!(
        service.get::<String>("service.child.value"),
        Err(ConfigError::PropertyNotFound(path)) if path == "service.service.child.value"
    ));
    assert!(matches!(service.section(".."), Err(ConfigError::InvalidPath { .. })));
    assert!(matches!(
        service.section(".child"),
        Err(ConfigError::InvalidPath { .. })
    ));
    assert!(service.contains_section("child").unwrap());
    assert!(!service.contains_section("child.value").unwrap());
}

#[test]
fn section_path_and_presence_accessors_are_callable_as_functions() {
    let mut config = Config::new();
    config.set("service.child.value", true).unwrap();
    let section = config.section("service").unwrap();

    assert_eq!(std::hint::black_box(ConfigSection::path)(&section), "service");
    assert!(std::hint::black_box(ConfigSection::contains_section)(&section, "child").unwrap());
}

#[test]
fn test_section_empty_paths_keep_the_current_scope() {
    let mut config = Config::new();
    config.set("service.child.value", "visible").unwrap();
    let service = config.section("service").unwrap();

    let same_section = service.section("").unwrap();
    assert_eq!(same_section.path(), "service");
    assert_eq!(same_section.get::<String>("child.value").unwrap(), "visible");
    assert!(same_section.contains_section("").unwrap());
    assert!(same_section.contains_section("child").unwrap());
    assert!(!same_section.contains_section("missing").unwrap());

    let root = config.section("").unwrap();
    assert_eq!(root.section("service").unwrap().path(), "service");
    assert!(root.contains_section("").unwrap());
}

#[test]
fn test_section_strict_reads_and_property_lookup_are_relative() {
    let mut config = Config::new();
    config.set("service.port", "8080").unwrap();
    config.set("service.ports", vec!["80", "443"]).unwrap();
    let service = config.section("service").unwrap();

    assert!(service.get_property("port").unwrap().is_some());
    assert_eq!(service.get_strict::<String>("port").unwrap(), "8080");
    assert_eq!(service.get_list_strict::<String>("ports").unwrap(), ["80", "443"]);
    assert!(service.get_property("missing").unwrap().is_none());
}

#[test]
fn test_section_debug_shows_only_path_and_effective_read_policy() {
    let mut first = Config::builder().description("first root description").build();
    first.set("service.visible", "first secret value").unwrap();
    first.set("service.child.value", true).unwrap();

    let mut second = Config::builder().description("second root description").build();
    second.set("service.visible", "different secret value").unwrap();
    second.set("service.child.value", true).unwrap();
    second.set("service.sibling.marker", "sibling-only marker").unwrap();

    let first_section = first.section("service").unwrap();
    let second_section = second.section("service").unwrap();
    let first_debug = format!("{first_section:?}");
    let second_debug = format!("{second_section:?}");

    assert_eq!(first_debug, second_debug);
    assert!(first_debug.contains("path: \"service\""));
    assert!(first_debug.contains(&format!("read_policy: {:?}", first_section.read_policy())));
    assert!(!first_debug.contains("first root description"));
    assert!(!first_debug.contains("first secret value"));
    assert!(!first_debug.contains("second secret value"));
    assert!(!first_debug.contains("sibling-only marker"));

    let first_child = first_section.section("child").unwrap();
    let second_child = second_section.section("child").unwrap();
    assert_eq!(format!("{first_child:?}"), format!("{second_child:?}"));
    assert!(format!("{first_child:?}").contains("path: \"service.child\""));

    let configured_policy = ReadPolicy::builder().max_interpolation_depth(7).build();
    let configured = Config::builder().default_read_policy(configured_policy.clone()).build();
    let inherited = configured.section("service").unwrap().section("child").unwrap();
    let inherited_debug = format!("{inherited:?}");
    assert!(inherited_debug.contains(&format!("read_policy: {configured_policy:?}")));

    let override_policy = ReadPolicy::builder_from(&configured_policy)
        .max_interpolation_depth(3)
        .build();
    let overridden = configured
        .section("service")
        .unwrap()
        .read_with(&override_policy)
        .section("child")
        .unwrap();
    let overridden_debug = format!("{overridden:?}");
    assert!(overridden_debug.contains(&format!("read_policy: {override_policy:?}")));
}
