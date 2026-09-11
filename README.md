# qubit-config

[![Rust CI](https://github.com/qubit-ltd/rs-config/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-config/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-config/coverage-badge.json)](https://qubit-ltd.github.io/rs-config/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-config.svg?color=blue)](https://crates.io/crates/qubit-config)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-config` is a type-safe configuration library for Rust applications that need to combine defaults, files, and environment variables without scattering string parsing throughout the application. It keeps configuration reads generic and explicit, while preserving source-layer, conversion, interpolation, and error context.

## Installation

```toml
[dependencies]
qubit-config = "0.17"
```

The default feature set is empty, so the core API does not enable optional file formats or rich value types. Enable only what the application needs, or use `full` for the complete optional surface:

```toml
qubit-config = { version = "0.17", features = ["toml", "env-file"] }
```

Or use the complete optional surface:

```toml
qubit-config = { version = "0.17", features = ["full"] }
```

| Feature | Adds |
| --- | --- |
| `bigdecimal` | `BigDecimal` values and conversion support |
| `chrono` | Chrono date/time values and conversion support |
| `num-bigint` | `BigInt` values and conversion support |
| `url` | URL values and conversion support |
| `env-file` | `.env` loading through `EnvFileConfigSource` and `Config::from_env_file` |
| `toml` | TOML loading through `TomlConfigSource` and `Config::from_toml_file` |
| `yaml` | YAML loading through `YamlConfigSource` and `Config::from_yaml_file` |
| `rich-types` | `bigdecimal`, `chrono`, `num-bigint`, and `url` |
| `formats` | `env-file`, `toml`, and `yaml` |
| `full` | `rich-types` and `formats` |

## Quick Start

The core workflow is a mutable `Config` with typed reads. The same generic API can read primitive values, collections, and types supported by `FromConfig`.

<!-- example: config_quickstart -->
```rust
use qubit_config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.set("server.host", "localhost")?;
    config.set("server.port", 8080)?;
    config.set("server.debug", true)?;

    let host: String = config.get("server.host")?;
    let port: u16 = config.get("server.port")?;
    let timeout: u64 = config.get_or("server.timeout", 30)?;

    assert_eq!(host, "localhost");
    assert_eq!(port, 8080);
    assert_eq!(timeout, 30);
    Ok(())
}
```

## A Real Configuration Scenario

An application can load a checked-in baseline and then apply a higher-priority environment layer. Sources are added in order; a later source overrides an earlier value for the same key unless the existing property is final.

<!-- example: config_sources -->
```rust
use qubit_config::Config;
use qubit_config::ConfigReader;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::EnvConfigSource;
use qubit_config::source::PropertiesConfigSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (host, port) = load_server_config()?;
    println!("server listening at {host}:{port}");
    Ok(())
}

// These sources are part of the default feature set, so this example compiles
// both with default features and with `--all-features`.
fn load_server_config() -> Result<(String, u16), Box<dyn std::error::Error>> {
    let mut sources = CompositeConfigSource::new();
    sources.add(PropertiesConfigSource::from_content(
        "server.host=localhost\nserver.port=8080\n",
    ));
    sources.add(EnvConfigSource::from_prefix("APP_"));

    let mut config = Config::new();
    config.merge_properties_from_source(&sources)?;
    let server = config.section("server")?;

    Ok((server.get("host")?, server.get("port")?))
}
```

With `APP_SERVER__HOST` and `APP_SERVER__PORT` set, the environment layer supplies the final values after prefix removal, lowercasing, and double-underscore-to-dot conversion. A single underscore remains part of a key segment. If normalization makes distinct environment names map to one key, loading returns `ConfigError::KeyConflict` and reports the conflicting names in lexicographic order; it never selects a winner based on process-environment iteration order. The same composition pattern can use `TomlConfigSource`, `YamlConfigSource`, or `EnvFileConfigSource` when their features are enabled.

## Structured Reads and Custom Policies

Use `Config::deserialize` when a subtree maps naturally to a Serde type:

<!-- example: config_structured -->
```rust
use qubit_config::Config;
use qubit_config::ConfigDeserializeOptions;
use qubit_config::ConfigError;
use qubit_config::ReadPolicy;
use qubit_config::UnknownFieldPolicy;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionOperationLimits;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Database {
    host: String,
    port: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let operation = ConversionOperationLimits::builder().max_input_bytes(128).build();
    let conversion = ConversionLimits::builder().operation_limits(operation).build();
    let policy = ReadPolicy::builder().conversion_limits(conversion).build();
    let mut config = Config::builder().default_read_policy(policy).build();
    config.set("db.host", "localhost")?;
    config.set("db.port", "5432")?;
    config.set("db.pool_size", 16)?;

    let error = config
        .deserialize::<Database>("db")
        .expect_err("strict reads reject unknown fields");
    assert!(matches!(error, ConfigError::UnknownProperties { .. }));

    let database = config.deserialize_with::<Database>(
        "db",
        ConfigDeserializeOptions {
            unknown_fields: UnknownFieldPolicy::Ignore,
            ..Default::default()
        },
    )?;
    assert_eq!(
        database,
        Database {
            host: "localhost".to_owned(),
            port: 5432,
        },
    );
    Ok(())
}
```

One `ConfigSerdeExt::deserialize::<T>` call constructs one
`ConversionSession` and threads it through every field, nested map, sequence,
enum, and variant in that materialization. The operation limits above therefore
accumulate across both fields. Separate ordinary `get` calls create fresh
sessions and do not share consumption. A failed field conversion keeps charges
accepted earlier in the same materialization; the rejected charge itself is
atomic.

Structured reads reject undeclared properties by default and report their
root-relative paths through `ConfigError::UnknownProperties`. Declare accepted
fields with the target's Serde shape (`rename`, `alias`, `default`, nested
types, maps, or `flatten`); choose `UnknownFieldPolicy::Ignore` only for
intentionally open sections. `deserialize(prefix)` uses default options: no
interpolation and unknown fields rejected. `deserialize_with(prefix, options)`
also accepts `interpolate: true`; these are the only two structured-read methods.
An empty prefix selects the complete visible scope.

Complete selected input, including ignored fields, is admitted before the
target visitor runs. Interpolation checks both the original and expanded input
with independent admission budgets. Leaf conversions then share one separate
`ConversionSession`. Native values and collections stay borrowed internally;
the result is still `DeserializeOwned`.

Add the direct Serde dependencies when using structured or JSON examples:

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

For direct customization of conversion options, depend on the owning `qubit-datatype` crate:

```toml
qubit-datatype = { version = "0.13", default-features = false, features = ["converter"] }
```

## Why This Project Exists

Configuration often arrives as strings, but application code needs typed values, defaults, lists, nested sections, and useful failure context. `qubit-config` keeps those concerns in one library:

- Sources produce independent configuration layers that can be inspected or merged transactionally.
- `ConfigReader` provides typed, optional, defaulted, multi-key, list, and strict reads for both `Config` and `ConfigSection`.
- Conversion rules are explicit through `ReadPolicy`; `read_with` applies a temporary borrowed policy.
- Interpolation is opt-in through ordinary `*_interpolated` reads or structured-read options. Environment fallback requires `InterpolationSources::ConfigThenEnv` explicitly.
- `ConfigError::kind()`, `path()`, `source_id()`, and `candidate_paths()` expose stable diagnostic context without requiring exhaustive matching on error variants.

## What It Provides—and What It Does Not

The stable core is `Config`, `ConfigReader`, `ConfigSection`, `ReadPolicy`,
and `ConfigSerdeExt`. Source adapters, persistence and wire decoding, and
low-level `Property` operations are separate layers for applications that need
those capabilities. The library provides generic type conversion, multi-value
properties, strict relative sections, source composition, optional TOML/YAML/
`.env` loaders, JSON persistence decoding, and redacted `Debug` output.
Persistence uses independent `JsonDecodeLimits`/`JsonDecodeSession` and
`JsonEncodeLimits`/`JsonEncodeSession` profiles, so input admission and output
limits cannot consume the wrong directional byte resource.
Ordinary `Deserialize` for `Config` applies the default decoded structure,
payload, property-count, and property-key limits. Because a general Serde
deserializer does not expose original bytes or lexical JSON tokens, untrusted
JSON must use `Config::decode_json_slice` for raw-input admission as well.

Built-in sources load through `SourceLoadContext` and the crate-owned executor.
Custom `ConfigSource` implementations must write through the context (for
example, `context.set(...)`) and explicitly report input bytes or parser nodes
before doing work. This is the boundary that lets rs-config enforce local and
composite aggregate budgets atomically for input bytes, assignments, parsed
nodes, child-source count, and nesting depth; unreported external I/O or parser
work cannot be inferred from the final layer. TOML and YAML are an explicit
exception at the parser boundary: their third-party parsers materialize an AST
before node, assignment, and depth accounting during flattening. Those limits
do not bound parser allocation or recursion; a future streaming parser is
required to provide that guarantee. Customize `SourceLimits` only when the
input boundary is understood.

It does not silently interpolate values during ordinary reads, expand process-environment placeholders while loading `.env` files, use defaults to hide a present but invalid value, permit third-party `ConfigReader` implementations, or support `dyn ConfigReader`: the trait is sealed and its generic methods make it non-object-safe. Detailed path rules, source failure behavior, structured deserialization, custom conversion, and troubleshooting are covered in the user guide; current component boundaries and compatibility commitments are recorded in the design document.

## Learn More

In 0.17, single-key parameters use `AsRef<str>` instead of `ConfigName`;
`ConfigKey` validation and multi-candidate `ConfigNames` remain. Default
parameters reuse `qubit_value::IntoValueDefault` instead of `IntoConfigDefault`,
with adaptation deferred until fallback. `Config::get` still converts according
to the reader policy. Value missing errors retain their original source and
item index through `ConfigError::value_missing()`; invalid collection items do
not trigger a default or a search for another candidate key.

`Config` still implements `serde::Serialize` and provides `encode_json_vec()`
for configuration persistence. Those encode the configuration envelope;
`deserialize` reads a business type from configuration. This release does not
add a reverse business-struct-to-configuration serializer. Wire V1 is unchanged.

- [English user guide](doc/user_guide.md)
- [中文用户手册](doc/user_guide.zh_CN.md)
- [English design document](doc/design.md)
- [中文设计说明](doc/design.zh_CN.md)
- [API documentation on docs.rs](https://docs.rs/qubit-config)
- [中文 README](README.zh_CN.md)
- [Repository](https://github.com/qubit-ltd/rs-config)

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./align-ci.sh` to format code and
`./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-config](https://github.com/qubit-ltd/rs-config)
