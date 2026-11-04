// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ReadPolicy;

const CHILD_PROBE_ENV: &str = "QUBIT_CONFIG_INTERPOLATION_STACK_PROBE_CHILD";
const REFERENCES: usize = 1024;
const STACK_BYTES: usize = 2 * 1024 * 1024;

#[cfg(unix)]
#[repr(C)]
struct CoreLimit {
    current: usize,
    maximum: usize,
}

#[cfg(unix)]
unsafe extern "C" {
    fn setrlimit(resource: i32, limit: *const CoreLimit) -> i32;
}

#[cfg(target_os = "haiku")]
const RLIMIT_CORE: i32 = 0;
#[cfg(all(unix, not(target_os = "haiku")))]
const RLIMIT_CORE: i32 = 4;

#[test]
fn interpolation_1024_deep_chain_fits_2mib_stack() {
    run_isolated_probe("interpolation_1024_deep_chain_fits_2mib_stack", "chain");
}

#[test]
fn interpolation_1024_node_cycle_reports_closed_chain_on_2mib_stack() {
    run_isolated_probe(
        "interpolation_1024_node_cycle_reports_closed_chain_on_2mib_stack",
        "cycle",
    );
}

fn run_isolated_probe(test_name: &str, mode: &str) {
    if std::env::var(CHILD_PROBE_ENV).as_deref() == Ok(mode) {
        let core_limit_status = if disable_core_dumps() {
            "core_limit=applied"
        } else {
            "core_limit=unavailable; running probe"
        };
        println!("{core_limit_status}");
        run_probe_on_small_stack(mode);
        return;
    }

    let executable = std::env::current_exe().expect("test executable path is available");
    let mut child = Command::new(executable)
        .args(["--exact", test_name, "--nocapture"])
        .env_remove(CHILD_PROBE_ENV)
        .env(CHILD_PROBE_ENV, mode)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("isolated interpolation child starts");

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().expect("child status is readable").is_some() {
            break;
        }
        if Instant::now() >= deadline {
            child.kill().expect("timed out child can be killed");
            let output = child.wait_with_output().expect("timed out child can be collected");
            panic!(
                "isolated interpolation probe timed out after 10 seconds; stdout={}; stderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        thread::sleep(Duration::from_millis(10));
    }

    let output = child.wait_with_output().expect("child output is readable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let core_limit_status = stdout
        .lines()
        .find(|line| line.starts_with("core_limit="))
        .expect("child reports its core-dump limit status");
    #[cfg(target_os = "linux")]
    assert_eq!(core_limit_status, "core_limit=applied");
    #[cfg(not(target_os = "linux"))]
    println!("child {core_limit_status}");
    assert!(
        output.status.success(),
        "isolated interpolation probe failed with {:?}; stdout={}; stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn disable_core_dumps() -> bool {
    #[cfg(unix)]
    {
        let limit = CoreLimit { current: 0, maximum: 0 };
        // SAFETY: `limit` points to a valid C-compatible rlimit structure.
        unsafe { setrlimit(RLIMIT_CORE, &limit) == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn run_probe_on_small_stack(mode: &str) {
    let mode = mode.to_owned();
    thread::Builder::new()
        .name("interpolation-stack-regression".into())
        .stack_size(STACK_BYTES)
        .spawn(move || {
            println!("phase=construct");
            let mut config = Config::new();
            config.set("entry", "${v0}").expect("valid entry key");
            for index in 0..REFERENCES {
                let value = if index + 1 < REFERENCES {
                    format!("${{v{}}}", index + 1)
                } else if mode == "cycle" {
                    "${v0}".to_string()
                } else {
                    "done".to_string()
                };
                config.set(format!("v{index}"), value).expect("valid variable key");
            }
            config.set_default_read_policy(
                ReadPolicy::builder()
                    .max_interpolation_depth(if mode == "cycle" { REFERENCES + 1 } else { REFERENCES })
                    .max_interpolation_expansions(16_384)
                    .build(),
            );

            println!("phase=read");
            let result = config.get_interpolated::<String>("entry");
            if mode == "cycle" {
                let error = result.expect_err("closed ring is reported as a cycle");
                let ConfigError::SubstitutionCycle { path, chain } = error else {
                    panic!("unexpected cycle result: {error:?}");
                };
                let expected_chain: Vec<String> = (0..REFERENCES)
                    .map(|index| format!("v{index}"))
                    .chain(std::iter::once("v0".into()))
                    .collect();
                assert_eq!(path, "entry");
                assert_eq!(chain, expected_chain);
            } else {
                assert_eq!(result.expect("depth-limited chain resolves"), "done");
            }
            println!("phase=drop");
            drop(config);
        })
        .expect("small-stack worker starts")
        .join()
        .expect("small-stack worker completes");
}
