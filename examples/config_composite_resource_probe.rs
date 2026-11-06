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

/// Parsed settings for one composite resource probe run.
struct ProbeOptions {
    shape: String,
    depth: usize,
    profile: String,
    label_bytes: Option<usize>,
    stack_bytes: Option<usize>,
    child: bool,
}

/// Selects a source-limit policy from the probe profile name.
fn limits(profile: &str) -> Result<SourceLimits, &'static str> {
    match profile {
        "default" => Ok(SourceLimits::default()),
        "raised" => Ok(SourceLimits::builder().max_sources(10_000).build()),
        "unbounded" => Ok(SourceLimits::unbounded()),
        _ => Err("unsupported limit profile"),
    }
}

/// Parses and validates the probe command-line arguments.
fn parse(args: &[String]) -> Result<ProbeOptions, &'static str> {
    if args.len() < 6 || args.len() > 11 {
        return Err(
            "usage: config_composite_resource_probe --shape chain|wide --depth N --limits default|raised|unbounded [--label-bytes 1..=4096] [--stack-bytes N] [--child]",
        );
    }
    let mut shape = None;
    let mut depth = None;
    let mut profile = None;
    let mut label_bytes = None;
    let mut stack_bytes = None;
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
            "--stack-bytes" if i + 1 < args.len() => {
                let bytes = args[i + 1].parse::<usize>().map_err(|_| "invalid stack bytes")?;
                if bytes == 0 {
                    return Err("invalid stack bytes");
                }
                stack_bytes = Some(bytes);
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
    Ok(ProbeOptions {
        shape,
        depth,
        profile,
        label_bytes,
        stack_bytes,
        child,
    })
}

/// Creates a fixed-width ASCII label whose suffix identifies its layer.
fn composite_label(index: usize, bytes: usize) -> String {
    let suffix = index.to_string();
    let mut label = "L".repeat(bytes);
    let used = suffix.len().min(bytes);
    label.replace_range(bytes - used.., &suffix[suffix.len() - used..]);
    label
}

/// Adapts a composite source so it can be nested as one child source.
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

/// Builds a chain or wide composite source under the selected limits.
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

/// Emits a flushed lifecycle marker that the parent probe can observe.
fn phase(message: &str) {
    println!("stage={message}");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

/// Runs source construction, load, and destruction in the isolated child.
fn child_work(shape: &str, depth: usize, profile: &str, label_bytes: Option<usize>) -> ExitCode {
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

/// Waits for the probe child and kills it if the fixed timeout expires.
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
fn child_args(
    shape: &str,
    depth: usize,
    profile: &str,
    label_bytes: Option<usize>,
    stack_bytes: Option<usize>,
) -> Vec<String> {
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
    if let Some(bytes) = stack_bytes {
        args.extend(["--stack-bytes".to_owned(), bytes.to_string()]);
    }
    args.push("--child".to_owned());
    args
}

/// Spawns the isolated probe child and maps its result to an exit code.
fn parent_run(
    shape: &str,
    depth: usize,
    profile: &str,
    label_bytes: Option<usize>,
    stack_bytes: Option<usize>,
) -> ExitCode {
    let mut command = Command::new(env::current_exe().expect("executable path"));
    command.args(child_args(shape, depth, profile, label_bytes, stack_bytes));
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

/// Dispatches this probe as a child worker or as its supervising parent.
fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match parse(&args) {
        Ok(options) => {
            println!(
                "shape={} depth={} edges={}",
                options.shape,
                options.depth,
                options.depth.saturating_add(1)
            );
            if options.child {
                if let Some(stack_bytes) = options.stack_bytes {
                    let worker = thread::Builder::new().stack_size(stack_bytes).spawn(move || {
                        child_work(&options.shape, options.depth, &options.profile, options.label_bytes)
                    });
                    match worker {
                        Ok(worker) => worker.join().unwrap_or(ExitCode::from(1)),
                        Err(error) => {
                            eprintln!("thread_spawn_error={error}");
                            ExitCode::from(2)
                        }
                    }
                } else {
                    child_work(&options.shape, options.depth, &options.profile, options.label_bytes)
                }
            } else {
                parent_run(
                    &options.shape,
                    options.depth,
                    &options.profile,
                    options.label_bytes,
                    options.stack_bytes,
                )
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
    fn test_label_bytes_is_optional_and_validated() {
        assert_eq!(parse(&base_args()).unwrap().label_bytes, None);
        let mut args = base_args();
        args.extend(["--label-bytes".into(), "1024".into()]);
        assert_eq!(parse(&args).unwrap().label_bytes, Some(1024));
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
    fn test_label_bytes_survives_parent_child_argument_forwarding() {
        let mut parent_args = base_args();
        parent_args.extend(["--label-bytes".into(), "1024".into()]);
        let parent_options = parse(&parent_args).unwrap();
        assert_eq!(parent_options.label_bytes, Some(1024));
        let args = child_args("chain", 1, "default", parent_options.label_bytes, None);
        let child_options = parse(&args).unwrap();
        assert_eq!(child_options.label_bytes, Some(1024));
        assert!(child_options.child);
    }

    #[test]
    fn test_stack_bytes_is_optional_and_validated() {
        assert_eq!(parse(&base_args()).unwrap().stack_bytes, None);
        let mut args = base_args();
        args.extend(["--stack-bytes".into(), "2097152".into()]);
        assert_eq!(parse(&args).unwrap().stack_bytes, Some(2 * 1024 * 1024));
        for invalid in ["0", "nope"] {
            let mut args = base_args();
            args.extend(["--stack-bytes".into(), invalid.into()]);
            assert!(parse(&args).is_err());
        }
        let mut args = base_args();
        args.push("--stack-bytes".into());
        assert!(parse(&args).is_err());
    }

    #[test]
    fn test_stack_bytes_survives_parent_child_argument_forwarding() {
        let bytes = 8 * 1024 * 1024;
        let mut parent_args = base_args();
        parent_args.extend(["--stack-bytes".into(), bytes.to_string()]);
        let parent_options = parse(&parent_args).unwrap();
        assert_eq!(parent_options.stack_bytes, Some(bytes));
        let args = child_args(
            "chain",
            1,
            "default",
            parent_options.label_bytes,
            parent_options.stack_bytes,
        );
        let child_options = parse(&args).unwrap();
        assert_eq!(child_options.stack_bytes, Some(bytes));
        assert!(child_options.child);
    }

    #[test]
    fn test_long_labels_have_exact_width_and_distinguish_layers() {
        assert_eq!(composite_label(12, 1024).len(), 1024);
        assert_ne!(composite_label(12, 1024), composite_label(13, 1024));
    }
}
