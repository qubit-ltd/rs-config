/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Crate root re-exports (`lib.rs`) smoke test.

#[test]
fn crate_public_api_is_reachable() {
    let _ = qubit_config::Config::new();
}

#[test]
fn crate_public_modules_are_reachable() {
    fn assert_from_config<T: qubit_config::from::FromConfig>() {}

    assert_from_config::<u16>();
    let _ = qubit_config::options::ConfigReadOptions::default();
    let _ = qubit_config::field::ConfigField::<bool>::builder()
        .name("enabled")
        .build();
}
