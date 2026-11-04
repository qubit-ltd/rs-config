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

fn parse(args: &[String]) -> Result<(String, usize, String, Option<usize>, bool), &'static str> {
    if args.len() < 6 || args.len() > 9 {
        return Err(
            "usage: config_composite_resource_probe --shape chain|wide --depth N --limits default|raised|unbounded [--label-bytes 1..=4096] [--child]",
        );
    }
    let mut shape = None;
    let mut depth = None;
    let mut profile = None;
    let mut label_bytes = None;
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
            "--label-bytes" if i + 1 < args.len() => {
                let bytes = args[i + 1].parse::<usize>().map_err(|_| "invalid label bytes")?;
                if !(1..=4096).contains(&bytes) {
                    return Err("invalid label bytes");
                }
                label_bytes = Some(bytes);
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
    Ok((shape, depth, profile, label_bytes, child))
}

/// Creates a fixed-width ASCII label whose suffix identifies its layer.
fn composite_label(index: usize, bytes: usize) -> String {
    let suffix = index.to_string();
    let mut label = "L".repeat(bytes);
    let used = suffix.len().min(bytes);
    label.replace_range(bytes - used.., &suffix[suffix.len() - used..]);
    label
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

fn make_source(
    shape: &str,
    depth: usize,
    profile: &str,
    label_bytes: Option<usize>,
) -> Result<CompositeConfigSource, &'static str> {
    let policy = limits(profile)?;
    if shape == "chain" {
        let mut source = CompositeConfigSource::builder()
            .limits(policy)
            .add_source(PropertiesConfigSource::from_content("a=1"))
            .build();
        for index in 0..depth {
            let mut builder = CompositeConfigSource::builder().limits(policy);
            if let Some(bytes) = label_bytes {
                builder = builder.label(composite_label(index, bytes));
            }
            source = builder.add_source(CompositeLayer(source)).build();
        }
        Ok(source)
    } else {
        let count = depth.checked_add(1).ok_or("depth overflow")?;
        let mut builder = CompositeConfigSource::builder().limits(policy);
        if let Some(bytes) = label_bytes {
            builder = builder.label(composite_label(0, bytes));
        }
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

fn child_run(shape: &str, depth: usize, profile: &str, label_bytes: Option<usize>) -> ExitCode {
    phase("constructing");
    let source = match make_source(shape, depth, profile, label_bytes) {
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

/// Builds the arguments that the parent passes to its probe child.
fn child_args(shape: &str, depth: usize, profile: &str, label_bytes: Option<usize>) -> Vec<String> {
    let mut args = vec![
        "--shape".to_owned(),
        shape.to_owned(),
        "--depth".to_owned(),
        depth.to_string(),
        "--limits".to_owned(),
        profile.to_owned(),
    ];
    if let Some(bytes) = label_bytes {
        args.extend(["--label-bytes".to_owned(), bytes.to_string()]);
    }
    args.push("--child".to_owned());
    args
}

fn parent_run(shape: &str, depth: usize, profile: &str, label_bytes: Option<usize>) -> ExitCode {
    let mut command = Command::new(env::current_exe().expect("executable path"));
    command.args(child_args(shape, depth, profile, label_bytes));
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
        Ok((shape, depth, profile, label_bytes, child)) => {
            println!("shape={shape} depth={depth} edges={}", depth.saturating_add(1));
            if child {
                child_run(&shape, depth, &profile, label_bytes)
            } else {
                parent_run(&shape, depth, &profile, label_bytes)
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::child_args;
    use super::composite_label;
    use super::parse;

    fn base_args() -> Vec<String> {
        ["--shape", "chain", "--depth", "1", "--limits", "default"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn label_bytes_is_optional_and_validated() {
        assert_eq!(parse(&base_args()).unwrap().3, None);
        let mut args = base_args();
        args.extend(["--label-bytes".into(), "1024".into()]);
        assert_eq!(parse(&args).unwrap().3, Some(1024));
        for invalid in ["0", "4097", "nope"] {
            let mut args = base_args();
            args.extend(["--label-bytes".into(), invalid.into()]);
            assert!(parse(&args).is_err());
        }
        let mut args = base_args();
        args.push("--label-bytes".into());
        assert!(parse(&args).is_err());
    }

    #[test]
    fn label_bytes_survives_parent_child_argument_forwarding() {
        let mut parent_args = base_args();
        parent_args.extend(["--label-bytes".into(), "1024".into()]);
        let (_, _, _, label_bytes, _) = parse(&parent_args).unwrap();
        assert_eq!(label_bytes, Some(1024));
        let args = child_args("chain", 1, "default", label_bytes);
        let (_, _, _, child_label_bytes, is_child) = parse(&args).unwrap();
        assert_eq!(child_label_bytes, Some(1024));
        assert!(is_child);
    }

    #[test]
    fn long_labels_have_exact_width_and_distinguish_layers() {
        assert_eq!(composite_label(12, 1024).len(), 1024);
        assert_ne!(composite_label(12, 1024), composite_label(13, 1024));
    }
}
