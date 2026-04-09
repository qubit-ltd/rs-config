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
