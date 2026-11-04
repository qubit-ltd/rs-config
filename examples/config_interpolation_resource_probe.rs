// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::env;
use std::process;
use std::thread;

use qubit_config::Config;
use qubit_config::ConfigError;
use qubit_config::ConfigReader;
use qubit_config::ReadPolicy;

fn main() {
    let arguments: Vec<String> = env::args().collect();
    if let Err(error) = run(&arguments) {
        eprintln!("probe error: {error}");
        process::exit(2);
    }
}

fn run(arguments: &[String]) -> Result<(), String> {
    if arguments.len() != 5 {
        return Err(
            "usage: config_interpolation_resource_probe <chain|cycle> <references> <depth-limit> <stack-bytes>".into(),
        );
    }
    let mode = match arguments[1].as_str() {
        "chain" => "chain",
        "cycle" => "cycle",
        mode => return Err(format!("invalid mode: {mode}")),
    };
    let references = arguments[2]
        .parse::<usize>()
        .map_err(|error| format!("invalid references: {error}"))?;
    let depth_limit = arguments[3]
        .parse::<usize>()
        .map_err(|error| format!("invalid depth-limit: {error}"))?;
    let stack_bytes = arguments[4]
        .parse::<usize>()
        .map_err(|error| format!("invalid stack-bytes: {error}"))?;
    if stack_bytes == 0 {
        return Err("stack-bytes must be positive".into());
    }
    if mode == "cycle" && references < 2 {
        return Err("cycle mode requires at least two references".into());
    }

    thread::Builder::new()
        .name("interpolation-probe".into())
        .stack_size(stack_bytes)
        .spawn(move || probe(mode, references, depth_limit))
        .map_err(|error| format!("cannot spawn probe thread: {error}"))?
        .join()
        .map_err(|_| "probe thread panicked".to_string())??;
    Ok(())
}

fn probe(mode: &str, references: usize, depth_limit: usize) -> Result<(), String> {
    println!("phase=construct");
    let mut config = Config::new();
    if mode == "chain" && references == 0 {
        config.set("entry", "done").map_err(|error| error.to_string())?;
    } else {
        config.set("entry", "${v0}").map_err(|error| error.to_string())?;
        for index in 0..references {
            let value = if index + 1 < references {
                format!("${{v{}}}", index + 1)
            } else if mode == "cycle" {
                "${v0}".to_string()
            } else {
                "done".to_string()
            };
            config
                .set(format!("v{index}"), value)
                .map_err(|error| error.to_string())?;
        }
    }
    let policy = if depth_limit == 64 && mode == "chain" && references <= 65 {
        ReadPolicy::default()
    } else {
        ReadPolicy::builder()
            .max_interpolation_depth(depth_limit)
            .max_interpolation_expansions(16_384)
            .build()
    };
    config.set_default_read_policy(policy);

    println!("phase=read");
    let result = config
        .read_with(config.default_read_policy())
        .get_interpolated::<String>("entry");
    match (mode, result) {
        ("chain", Ok(value)) if value == "done" => println!("result=success value=done"),
        ("chain", Err(ConfigError::SubstitutionDepthExceeded { path, max_depth }))
            if references > depth_limit && path == "entry" && max_depth == depth_limit =>
        {
            println!("result=depth-exceeded path={path} max_depth={max_depth}");
        }
        ("cycle", Err(ConfigError::SubstitutionCycle { path, chain })) => {
            let expected: Vec<String> = (0..references)
                .map(|index| format!("v{index}"))
                .chain(std::iter::once("v0".into()))
                .collect();
            if path != "entry" || chain != expected {
                return Err(format!("unexpected cycle: path={path}, chain={chain:?}"));
            }
            println!("result=cycle path={path} chain-length={}", chain.len());
        }
        (_, Ok(value)) => return Err(format!("unexpected successful value: {value}")),
        (_, Err(error)) => return Err(format!("unexpected error: {error:?}")),
    }
    println!("phase=drop");
    drop(config);
    println!("result=complete");
    Ok(())
}
