// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(windows)]

use std::ffi::OsStr;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use super::EnvConfigSource;
use crate::source::SourceLimits;
use crate::source::SourceLoadContext;

fn key_with_units(units: &[u16]) -> OsString {
    OsString::from_wide(units)
}

#[test]
fn test_windows_prefix_matching_uses_native_units() {
    let invalid_key = key_with_units(&[b'A' as u16, b'P' as u16, b'P' as u16, b'_' as u16, 0xD800, b'X' as u16]);
    assert!(invalid_key.to_str().is_none());

    assert!(EnvConfigSource::env_key_matches_prefix(&invalid_key, "APP_"));
    assert!(!EnvConfigSource::env_key_matches_prefix(&invalid_key, "OTHER_"));
    assert!(EnvConfigSource::env_key_matches_prefix(&invalid_key, ""));
    assert!(EnvConfigSource::env_key_matches_prefix(&invalid_key, "APP"));
    assert!(!EnvConfigSource::env_key_matches_prefix(OsStr::new("AP"), "APP_"));
    let short_invalid_key = key_with_units(&[0xD800]);
    assert!(!EnvConfigSource::env_key_matches_prefix(&short_invalid_key, "APP_"));

    let non_ascii = key_with_units(&[0x00C5, b'_' as u16, 0xD800]);
    assert!(non_ascii.to_str().is_none());
    assert!(EnvConfigSource::env_key_matches_prefix(&non_ascii, "Å_"));

    assert!(EnvConfigSource::env_key_matches_prefix(OsStr::new("Å_APP"), "Å_"));
    assert!(!EnvConfigSource::env_key_matches_prefix(OsStr::new("å_APP"), "Å_"));
}

#[test]
fn test_matching_invalid_windows_key_errors_with_redacted_source_context() {
    const SECRET_VALUE: &str = "WINDOWS_ENV_KEY_SECRET_VALUE";
    let mut units = "APP_".encode_utf16().collect::<Vec<_>>();
    units.push(0xD800);
    units.extend(SECRET_VALUE.encode_utf16());
    let key = key_with_units(&units);
    assert!(key.to_str().is_none());

    let source = EnvConfigSource::from_prefix("APP_");
    let mut context = SourceLoadContext::new("process environment".to_owned(), SourceLimits::default());
    let error = source
        .load_environment_entries([(key, OsString::from(SECRET_VALUE))], &mut context)
        .expect_err("matching invalid Unicode key must fail");
    assert!(error.to_string().contains("process environment"));
    assert!(error.to_string().contains("<redacted>"));
    assert!(!error.to_string().contains(SECRET_VALUE));
    assert!(!format!("{error:?}").contains(SECRET_VALUE));
}
