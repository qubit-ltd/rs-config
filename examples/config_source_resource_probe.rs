// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Manually measure TOML/YAML source loading under configured source limits.

use std::env;
use std::process::ExitCode;
#[cfg(any(feature = "toml", feature = "yaml"))]
use std::time::Instant;

#[cfg(any(feature = "toml", feature = "yaml"))]
use qubit_config::ConfigError;
#[cfg(any(feature = "toml", feature = "yaml"))]
use qubit_config::source::ConfigSource;
use qubit_config::source::SourceLimits;

const USAGE: &str = "usage: config_source_resource_probe FORMAT SHAPE SIZE MAX_NODES MAX_DEPTH MODE\nFORMAT: toml | yaml; SHAPE: wide | deep; MODE: load | generate-only";

fn parse_usize(value: &str) -> Result<usize, &'static str> {
    value.parse().map_err(|_| "invalid numeric argument")
}

fn generate(format: &str, shape: &str, size: usize) -> Result<String, &'static str> {
    if size == 0 {
        return Err("SIZE must be greater than zero");
    }
    let max_input_bytes = SourceLimits::default().max_input_bytes();
    if size > max_input_bytes {
        return Err("generated input exceeds the default source byte limit");
    }
    match (format, shape) {
        ("toml", "wide") => {
            if size > max_input_bytes.saturating_sub(8) / 2 {
                return Err("generated input exceeds the default source byte limit");
            }
            Ok(format!("items=[{}]", vec!["0"; size].join(",")))
        }
        ("yaml", "wide") => {
            if size > max_input_bytes.saturating_sub(8) / 2 {
                return Err("generated input exceeds the default source byte limit");
            }
            Ok(format!("items: [{}]", vec!["0"; size].join(",")))
        }
        ("toml", "deep") => {
            let estimated_bytes = size
                .checked_mul(4)
                .and_then(|bytes| bytes.checked_add(14))
                .ok_or("generated input exceeds the default source byte limit")?;
            if estimated_bytes > max_input_bytes {
                return Err("generated input exceeds the default source byte limit");
            }
            let mut input = String::from("items={");
            for _ in 0..size {
                input.push_str("k={");
            }
            input.push_str("leaf=0");
            input.extend(std::iter::repeat_n('}', size + 1));
            Ok(input)
        }
        ("yaml", "deep") => {
            let estimated_bytes = size
                .checked_mul(size)
                .and_then(|bytes| bytes.checked_add(size.checked_mul(6)?))
                .and_then(|bytes| bytes.checked_add(17))
                .ok_or("generated input exceeds the default source byte limit")?;
            if estimated_bytes > max_input_bytes {
                return Err("generated input exceeds the default source byte limit");
            }
            let mut input = String::from("items:\n");
            for level in 0..size {
                input.push_str(&" ".repeat((level + 1) * 2));
                input.push_str("k:\n");
            }
            input.push_str(&" ".repeat((size + 1) * 2));
            input.push_str("leaf: 0\n");
            Ok(input)
        }
        _ => Err("unsupported format or shape"),
    }
}

#[cfg(any(feature = "toml", feature = "yaml"))]
fn report_result(result: Result<qubit_config::Config, ConfigError>, elapsed: u128) -> ExitCode {
    match result {
        Ok(config) => {
            println!("result=ok properties={}", config.len());
            println!("load_elapsed_ns={elapsed}");
            ExitCode::SUCCESS
        }
        Err(ConfigError::SourceLimitExceeded { kind, .. }) => {
            println!("result=error error_kind=SourceLimitExceeded source_limit_kind={kind}");
            println!("load_elapsed_ns={elapsed}");
            ExitCode::from(1)
        }
        Err(ConfigError::SourceParseError { path, source_index, .. }) => {
            let parse_stage = if path.is_some() || source_index.is_some() {
                "source-shape"
            } else {
                "parser-or-scanner"
            };
            println!("result=error error_kind=Parse parse_stage={parse_stage}");
            println!("load_elapsed_ns={elapsed}");
            ExitCode::from(1)
        }
        Err(error) => {
            println!("result=error error_kind={:?}", error.kind());
            println!("load_elapsed_ns={elapsed}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, &'static str> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 6 {
        return Err("invalid argument count");
    }
    let format = args[0].as_str();
    let shape = args[1].as_str();
    let size = parse_usize(&args[2])?;
    let _max_nodes = parse_usize(&args[3])?;
    let _max_depth = parse_usize(&args[4])?;
    let mode = args[5].as_str();
    if !matches!(format, "toml" | "yaml")
        || !matches!(shape, "wide" | "deep")
        || !matches!(mode, "load" | "generate-only")
    {
        return Err("unsupported format, shape, or mode");
    }

    let content = generate(format, shape, size)?;
    if content.len() > SourceLimits::default().max_input_bytes() {
        return Err("generated input exceeds the default source byte limit");
    }
    println!(
        "format={format} shape={shape} size={size} input_bytes={}",
        content.len()
    );
    println!("generated=complete");
    if mode == "generate-only" {
        println!("result=generated-only");
        return Ok(ExitCode::SUCCESS);
    }

    #[cfg(any(feature = "toml", feature = "yaml"))]
    let limits = SourceLimits::builder()
        .max_nodes(_max_nodes)
        .max_nesting_depth(_max_depth)
        .build();
    #[cfg(any(feature = "toml", feature = "yaml"))]
    let started = Instant::now();
    #[cfg(feature = "toml")]
    if format == "toml" {
        let source = qubit_config::source::TomlConfigSource::builder()
            .content(content)
            .limits(limits)
            .build();
        return Ok(report_result(source.load(), started.elapsed().as_nanos()));
    }
    #[cfg(feature = "yaml")]
    if format == "yaml" {
        let source = qubit_config::source::YamlConfigSource::builder()
            .content(content)
            .limits(limits)
            .build();
        return Ok(report_result(source.load(), started.elapsed().as_nanos()));
    }
    #[cfg(not(feature = "toml"))]
    if format == "toml" {
        println!("result=format-not-enabled format=toml");
        return Ok(ExitCode::from(2));
    }
    #[cfg(not(feature = "yaml"))]
    if format == "yaml" {
        println!("result=format-not-enabled format=yaml");
        return Ok(ExitCode::from(2));
    }
    unreachable!("format validation and feature branches cover TOML and YAML")
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(_) => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
