/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Script Tests
//!
//! Tests project helper scripts.

use std::path::PathBuf;
use std::process::Command;

use regex::Regex;

#[test]
fn test_coverage_exclude_pattern_should_not_exclude_current_crate_files() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script_path = manifest_dir.join("coverage.sh");
    let output = Command::new("bash")
        .arg(&script_path)
        .arg("--print-exclude-pattern")
        .current_dir(&manifest_dir)
        .output()
        .expect("Failed to run coverage.sh");

    assert!(
        output.status.success(),
        "coverage.sh --print-exclude-pattern failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pattern = stdout
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .expect("Expected non-empty output from coverage.sh");
    let regex = Regex::new(pattern).expect("Invalid regex emitted by coverage.sh");
    let current_file = manifest_dir.join("src").join("config.rs");
    let current_file = current_file.to_string_lossy();

    assert!(
        !regex.is_match(&current_file),
        "Exclude regex unexpectedly matches current crate file. pattern={pattern}, file={current_file}"
    );
}
