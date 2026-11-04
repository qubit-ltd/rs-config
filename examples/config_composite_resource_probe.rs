// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Isolated resource probe for composite source chains and wide trees.

use std::env;
use std::process::Child;
use std::process::Command;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use qubit_config::ConfigError;
use qubit_config::ConfigResult;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::ConfigSource;
use qubit_config::source::PropertiesConfigSource;
use qubit_config::source::SourceLimits;
use qubit_config::source::SourceLoadContext;

const TIMEOUT: Duration = Duration::from_secs(30);

fn limits(profile: &str) -> Result<SourceLimits, &'static str> {
    match profile {
        "default" => Ok(SourceLimits::default()),
        "raised" => Ok(SourceLimits::builder().max_sources(10_000).build()),
        "unbounded" => Ok(SourceLimits::unbounded()),
        _ => Err("unsupported limit profile"),
    }
}

fn parse(args: &[String]) -> Result<(String, usize, String, bool), &'static str> {
    if args.len() < 6 || args.len() > 7 {
        return Err(
            "usage: config_composite_resource_probe --shape chain|wide --depth N --limits default|raised|unbounded [--child]",
        );
    }
    let mut shape = None;
    let mut depth = None;
    let mut profile = None;
    let mut child = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--shape" if i + 1 < args.len() => {
                shape = Some(args[i + 1].clone());
                i += 2;
            }
            "--depth" if i + 1 < args.len() => {
                depth = Some(args[i + 1].parse().map_err(|_| "invalid depth")?);
                i += 2;
            }
            "--limits" if i + 1 < args.len() => {
                profile = Some(args[i + 1].clone());
                i += 2;
            }
            "--child" => {
                child = true;
                i += 1;
            }
            _ => return Err("invalid arguments"),
        }
    }
    let shape = shape.ok_or("missing shape")?;
    if !matches!(shape.as_str(), "chain" | "wide") {
        return Err("unsupported shape");
    }
    let depth = depth.ok_or("missing depth")?;
    let profile = profile.ok_or("missing limit profile")?;
    let _ = limits(&profile)?;
    Ok((shape, depth, profile, child))
}

struct CompositeLayer(CompositeConfigSource);

impl ConfigSource for CompositeLayer {
    fn source_id(&self) -> String {
        self.0.source_id()
    }
    fn limits(&self) -> SourceLimits {
        self.0.limits()
    }
    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        self.0.load_into(context)
    }
}

fn make_source(shape: &str, depth: usize, profile: &str) -> Result<CompositeConfigSource, &'static str> {
    let policy = limits(profile)?;
    if shape == "chain" {
        let mut source = CompositeConfigSource::builder()
            .limits(policy)
            .add_source(PropertiesConfigSource::from_content("a=1"))
            .build();
        for _ in 0..depth {
            source = CompositeConfigSource::builder()
                .limits(policy)
                .add_source(CompositeLayer(source))
                .build();
        }
        Ok(source)
    } else {
        let count = depth.checked_add(1).ok_or("depth overflow")?;
        let mut builder = CompositeConfigSource::builder().limits(policy);
        for _ in 0..count {
            builder = builder.add_source(PropertiesConfigSource::from_content("a=1"));
        }
        Ok(builder.build())
    }
}

fn phase(message: &str) {
    println!("stage={message}");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

fn child_run(shape: &str, depth: usize, profile: &str) -> ExitCode {
    phase("constructing");
    let source = match make_source(shape, depth, profile) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("construct_error={error}");
            return ExitCode::from(2);
        }
    };
    phase("constructed");
    phase("loading");
    let started = Instant::now();
    let result = source.load();
    println!("load_elapsed_ns={}", started.elapsed().as_nanos());
    let success = result.is_ok();
    let budget_rejection = matches!(result, Err(ConfigError::SourceLimitExceeded { .. }));
    match result {
        Ok(config) => println!("load_result=ok properties={}", config.len()),
        Err(error) => println!("load_result=error kind={:?} error={error}", error.kind()),
    }
    phase("loaded");
    phase("destroying");
    drop(source);
    phase("destroyed");
    if success || budget_rejection {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn wait_child(child: &mut Child) -> Result<std::process::ExitStatus, &'static str> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|_| "wait failed")? {
            return Ok(status);
        }
        if started.elapsed() >= TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err("timeout");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn parent_run(shape: &str, depth: usize, profile: &str) -> ExitCode {
    let mut command = Command::new(env::current_exe().expect("executable path"));
    command.args([
        "--shape",
        shape,
        "--depth",
        &depth.to_string(),
        "--limits",
        profile,
        "--child",
    ]);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            eprintln!("spawn_error={error}");
            return ExitCode::from(2);
        }
    };
    match wait_child(&mut child) {
        Ok(status) => {
            println!("child_exit={status}");
            if status.success() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            println!("child_exit={error}");
            ExitCode::from(124)
        }
    }
}

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match parse(&args) {
        Ok((shape, depth, profile, child)) => {
            println!("shape={shape} depth={depth} edges={}", depth.saturating_add(1));
            if child {
                child_run(&shape, depth, &profile)
            } else {
                parent_run(&shape, depth, &profile)
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
