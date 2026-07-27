# qubit-config

[![Rust CI](https://github.com/qubit-ltd/rs-config/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-config/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-config/coverage-badge.json)](https://qubit-ltd.github.io/rs-config/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-config.svg?color=blue)](https://crates.io/crates/qubit-config)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

A powerful, type-safe configuration management system for Rust, providing flexible configuration management with support for multiple data types, variable substitution, multi-value properties, and pluggable **configuration sources** (files, environment, and composites).

## Features

- ✅ **Pure Generic API** - Use `get<T>()`, `read(ConfigField<T>)`, and `set<T>()` generic methods with full type inference support
- ✅ **Rich Data Types** - Supports Rust primitives plus feature-gated temporal, URL, and arbitrary-precision value types
- ✅ **Multi-Value Properties** - Each configuration property can contain multiple values with list operations
- ✅ **Explicit Interpolation** - `*_interpolated` reads resolve `${var_name}` from config and fall back to process environment variables by default
- ✅ **Type-aware API** - Generic target types are checked at compile time; missing, malformed, or incompatible configuration data is reported at runtime through `ConfigError`
- ✅ **Stable Persistence Wire** - `Config` serialization emits a deterministic, versioned V1 JSON contract and continues to read legacy unversioned payloads
- ✅ **Extensible** - Trait-based design for easy custom type support
- ✅ **Configuration sources** - [`ConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/trait.ConfigSource.html) trait with built-in loaders: TOML, YAML, Java-style `.properties`, `.env` files, process environment variables (with optional prefix / key normalization), and [`CompositeConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.CompositeConfigSource.html) to merge several sources in order (later entries override earlier ones for the same key); built-in sources load transactionally, validate ambiguous normalized keys, and reject duplicate flattened TOML/YAML keys
- ✅ **Read-only API** - The sealed [`ConfigReader`](https://docs.rs/qubit-config/latest/qubit_config/trait.ConfigReader.html) trait provides typed, multi-key, and field-declaration reads for [`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html) and [`ConfigSection`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigSection.html)
- ✅ **Configurable parsing** - [`ReadOptions`](https://docs.rs/qubit-config/latest/qubit_config/options/struct.ReadOptions.html) controls string trimming, blank handling, boolean literals, and scalar-string collection splitting globally or per field
- ✅ **Strict sections** - [`Config::section`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.section) returns a [`ConfigSection`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigSection.html) with strictly relative keys; nest with [`ConfigSection::section`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigSection.html#method.section)
- ✅ **Safe diagnostics** - `Debug` output preserves property metadata while redacting every stored configuration value through `qubit-redact`
- ✅ **Structured errors** - [`ConfigError::kind`](https://docs.rs/qubit-config/latest/qubit_config/enum.ConfigError.html#method.kind) and [`ConfigError::path`](https://docs.rs/qubit-config/latest/qubit_config/enum.ConfigError.html#method.path) expose stable machine-readable context without downstream exhaustive variant matching
- ✅ **Efficient core representation** - Uses enum-backed values and staged source loading; pluggable sources can still use trait objects where dynamic composition is useful

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
qubit-config = "0.14"
```

The default feature set is intentionally empty, so core configuration reads do
not pull in optional formats or rich value types. Enable the complete optional
surface with `full`:

```toml
qubit-config = { version = "0.14", features = ["full"] }
```

Enable only the capabilities you need:

```toml
qubit-config = { version = "0.14", features = ["toml"] }
```

Available feature flags:

| Feature | Enables |
|---------|---------|
| `bigdecimal` | `BigDecimal` values and direct `FromConfig` support |
| `chrono` | Chrono date/time values and direct `FromConfig` support |
| `num-bigint` | `BigInt` values and direct `FromConfig` support |
| `url` | URL values and direct `FromConfig` support |
| `env-file` | `EnvFileConfigSource` and `Config::from_env_file` |
| `toml` | `TomlConfigSource` and `Config::from_toml_file` |
| `yaml` | `YamlConfigSource` and `Config::from_yaml_file` |
| `rich-types` | All four rich-value features |
| `formats` | `env-file`, `toml`, and `yaml` |
| `full` | `rich-types` and `formats` |

## Quick Start

```rust
use qubit_config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();

    // Set configuration values
    config.set("port", 8080)?;
    config.set("host", "localhost")?;
    config.set("debug", true)?;

    // Read configuration values (with type inference)
    let port: i32 = config.get("port")?;
    let host: String = config.get("host")?;
    let debug: bool = config.get("debug")?;

    // Use turbofish syntax
    let port = config.get::<i32>("port")?;

    // Use default values
    let timeout: u64 = config.get_or("timeout", 30)?;

    println!("Server running on {}:{}", host, port);
    Ok(())
}
```

## Core Concepts

### Config

The `Config` struct is the central configuration manager that stores and manages all configuration properties.

```rust
let mut config = Config::new();
config.set("database.host", "localhost")?;
config.set("database.port", 5432)?;
```

### Property

Each configuration item is represented by a `Property` that contains:
- Name (key)
- Scalar-or-collection value container
- Optional description
- Final flag (immutable after set)

### ValueContainer

A type-safe container that preserves whether a source supplied one scalar value
or an explicit collection. Scalar strings may be split by configured collection
conversion rules; collection elements are converted individually and are never
split again.

### ConfigReader

[`ConfigReader`](https://docs.rs/qubit-config/latest/qubit_config/trait.ConfigReader.html) is the read-only configuration surface. Functions or types that only need settings can take `&impl ConfigReader` (or a generic `R: ConfigReader`) instead of `&Config`; the same API works for [`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html) and [`ConfigSection`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigSection.html). `ConfigReader` has generic typed methods, so it is not object-safe and should not be used as `dyn ConfigReader`.

For new integrations, prefer `ConfigReader`, `ConfigSection`, `ReadOptions`, and `ConfigSerdeExt`. `ConfigField`, `Configured`, `Configurable`, and custom `ConfigSource` implementations remain supported specialized APIs, but are not the recommended starting point for ordinary application reads.

The main read APIs are:

| API | Behavior |
|-----|----------|
| `get<T>(name)` | Read a required value through `FromConfig` without interpolation. |
| `get_optional<T>(name)` | Return `Ok(None)` when the key is absent or effectively missing. |
| `get_or<T>(name, default)` | Use `default` only when the key is absent or effectively missing. |
| `get_any<T>(&[names])` | Read the first key whose value is not effectively missing. |
| `get_optional_any<T>(&[names])` | Multi-key optional read. |
| `get_any_or<T>(&[names], default)` | Multi-key defaulted read. |
| `get_interpolated<T>` / `get_optional_interpolated<T>` / `get_interpolated_or<T>` | Explicit single-key interpolation before conversion. |
| `get_any_interpolated<T>` / `get_optional_any_interpolated<T>` / `get_any_interpolated_or<T>` | Explicit multi-key interpolation before conversion. |
| `read(ConfigField<T>)` | Field declaration with name, aliases, default, and field-level read options. |
| `read_interpolated(ConfigField<T>)` / `read_optional_interpolated(ConfigField<T>)` | Explicit field interpolation. |
| `get_strict` / `get_list_strict` | Exact stored-type reads without cross-type conversion. |

Defaults do not hide bad configuration. If a key exists and its value fails parsing or type conversion, the error is returned immediately instead of falling back to a default or later alias. Explicit interpolated reads likewise return interpolation errors.

An unset property is effectively missing. A scalar string can also be treated
as missing by the active string policy. A concrete empty collection is present:
`get_optional_list` returns `Some(Vec::new())`, and defaults are not used.

```rust
use qubit_config::{Config, ConfigError};

let mut config = Config::new();
config.set("worker.threads", "abc")?;

let missing = config.get_or("missing.threads", 4u16)?;
assert_eq!(missing, 4);

let invalid = config.get_or("worker.threads", 4u16);
assert!(matches!(invalid, Err(ConfigError::ConversionError { .. })));
```

Defaulted reads such as `get_or`, `get_any_or`, `get_interpolated_or`, and `get_any_interpolated_or` accept convenient fallback values. Scalar defaults still use the target type directly, while string defaults can use borrowed literals and string-list defaults can use arrays, slices, or borrowed vectors. Single-key and multi-key arguments also accept direct arrays, slices, vectors, and borrowed vectors.

```rust
let host = config.get_or::<String>("server.host", "localhost")?;
let paths = config.get_or::<Vec<String>>("server.paths", ["bin", "lib"])?;

let paths = config.get_any_or::<Vec<String>>(
    ["server.paths", "SERVER_PATHS"],
    ["cache", "tmp"],
)?;
```

### ConfigSection

[`ConfigSection`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigSection.html) is a zero-copy, strictly relative view of a `Config`. Use [`Config::section`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.section) to create it; every key is resolved below the section path, so section `db` and key `host` read `db.host`. The exact scalar at `db` is not part of the section; only descendants such as `db.host` are visible. Use [`ConfigSection::section`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigSection.html#method.section) for nested sections.

Use `contains_section("db")` for dotted section membership. Use
`contains_key_prefix("db")` only when raw character-prefix matching is
intentional; it also matches sibling names such as `db2`.

```rust
use qubit_config::{Config, ConfigReader};

let mut config = Config::new();
config.set("db.host", "localhost")?;
config.set("db.port", 5432i32)?;

let db = config.section("db");
let host: String = db.get("host")?;
let port: i32 = db.get("port")?;
```

### ReadOptions

`ReadOptions` controls how configured values are parsed. It can be set globally on a `Config`, or attached to a single `ConfigField<T>`.

| Option group | Controls |
|--------------|----------|
| `StringConversionOptions` | Trimming and blank-string handling: preserve, treat as missing, or reject. |
| `BooleanConversionOptions` | Accepted boolean literals and case sensitivity. |
| `CollectionConversionOptions` | Splitting scalar strings into lists, delimiters, per-item trimming, and empty-item policy. |
| `NumericConversionOptions` | Fractional-to-integer, numeric-to-float, and text-to-float policies, plus numeric text and `BigInt` materialization limits. |
| `DurationConversionOptions` | Numeric input unit, text suffix rules, output unit and suffix, and independent Duration rounding. |
| Interpolation settings on `ReadOptions` | Environment fallback plus recursion depth, expansion count, and output byte limits for explicit interpolated reads. |

`ReadOptions::env_friendly()` is useful for environment-variable style values: it trims strings, treats blank scalar strings as missing, accepts `true/false`, `1/0`, `yes/no`, and `on/off`, and splits scalar strings on commas for `Vec<T>` reads while skipping empty items. It permits nearest-even text-to-float rounding, but keeps fractional-to-integer and existing-numeric-to-float conversions exact.

Ordinary reads never interpolate `${...}`. Explicit interpolated reads resolve configuration keys first and allow environment-variable fallback by default; `ReadOptions` can disable that fallback and adjust recursion depth, placeholder expansion, and output byte limits. Treat an interpolated configuration as trusted when environment fallback is enabled: it can select the names of process environment variables to read. Use `ReadOptions::config_only()` for untrusted configuration content, or explicitly disable fallback with `with_environment_fallback_enabled(false)`.

```rust
use qubit_config::{Config, options::ReadOptions};

let mut config = Config::new().with_read_options(ReadOptions::env_friendly());
config.set("HTTP_ENABLED", "yes")?;
config.set("HTTP_PORTS", "8080, 8081,,8082")?;

let enabled: bool = config.get("HTTP_ENABLED")?;
let ports: Vec<u16> = config.get("HTTP_PORTS")?;

assert!(enabled);
assert_eq!(ports, vec![8080, 8081, 8082]);
```

You can build stricter or domain-specific options with builder-style methods:

The conversion policy types are owned by `qubit-datatype`. Applications that
customize them should depend on that crate directly:

```toml
[dependencies]
qubit-config = "0.14"
qubit-datatype = { version = "0.9", default-features = false, features = ["converter"] }
```

```rust
use qubit_config::{Config, options::ReadOptions};
use qubit_datatype::{
    BlankStringPolicy,
    BooleanConversionOptions,
    CollectionConversionOptions,
    EmptyItemPolicy,
    NumericConversionLimits,
    NumericConversionOptions,
    StringConversionOptions,
};

let options = ReadOptions::default()
    .with_numeric_options(
        NumericConversionOptions::strict().with_limits(
            NumericConversionLimits::default().with_max_text_bytes(4096),
        ),
    )
    .with_string_options(
        StringConversionOptions::default()
            .with_trim(true)
            .with_blank_string_policy(BlankStringPolicy::Reject),
    )
    .with_boolean_options(
        BooleanConversionOptions::strict()
            .with_true_literal("enabled")?
            .with_false_literal("disabled")?,
    )
    .with_collection_options(
        CollectionConversionOptions::default()
            .with_split_scalar_strings(true)
            .with_delimiters([',', ';'])
            .with_trim_items(true)
            .with_empty_item_policy(EmptyItemPolicy::Reject),
    );

let mut config = Config::new().with_read_options(options);
config.set("feature", "enabled")?;
config.set("ports", "8080; 8081")?;

let feature: bool = config.get("feature")?;
let ports: Vec<u16> = config.get("ports")?;
```

### ConfigField

Use `ConfigField<T>` when a logical setting has aliases, a default, or field-specific parsing rules. This keeps migration keys, legacy names, and environment-style keys out of application parsing code.

```rust
use qubit_config::{Config, field::ConfigField, options::ReadOptions};

let mut config = Config::new();
config.set("MIME_DETECTOR_ENABLE_PRECISE_DETECTION", "yes")?;

let enabled = config.read(
    ConfigField::<bool>::builder()
        .name("mime.enable_precise_detection")
        .alias("MIME_DETECTOR_ENABLE_PRECISE_DETECTION")
        .alias("ANOTHER_MIME_DETECTOR_ENABLE_PRECISE_DETECTION_PROPERTY")
        .default(false)
        .read_options(ReadOptions::env_friendly())
        .build(),
)?;

assert!(enabled);
```

The builder makes the primary name explicit: `build()` is available only after `name(...)` has been supplied.

### Multi-Key Reads

Use `get_any`, `get_optional_any`, and `get_any_or` for ordinary lightweight alias reads. Use the corresponding `*_interpolated` methods when placeholders must be resolved.

```rust
use qubit_config::{Config, options::ReadOptions};

let mut config = Config::new().with_read_options(ReadOptions::env_friendly());
config.set("SERVICE_URL", "http://localhost:8080")?;
config.set("SERVER_TIMEOUT", "30")?;

let url = config.get_any::<String>(["service.url", "SERVICE_URL"])?;
let timeout = config.get_any_or(["server.timeout", "SERVER_TIMEOUT"], 10u64)?;
let optional_port = config.get_optional_any::<u16>(["server.port", "SERVER_PORT"])?;
let retries = config.get_any_or(
    ["server.retries", "SERVER_RETRIES"],
    3u8,
)?;

assert_eq!(url, "http://localhost:8080");
assert_eq!(timeout, 30);
assert_eq!(optional_port, None);
assert_eq!(retries, 3);
```

Multi-key reads scan keys in order. Absent and effectively missing values are
skipped; a concrete empty collection is still present. If the first selected
value is invalid, the error is returned and later keys are not tried.

### Configuration sources

Implementations of [`ConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/trait.ConfigSource.html) load external settings into a [`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html). Call [`merge_from_source`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.merge_from_source) (or `load` on the source with a `&mut Config`) to apply them. When no pre-load customization is needed, use the convenience constructors such as [`Config::from_toml_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_toml_file), [`Config::from_yaml_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_yaml_file), [`Config::from_properties_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_properties_file), [`Config::from_env_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env_file), [`Config::from_env`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env), or [`Config::from_env_prefix`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env_prefix). TOML, YAML, and `.env` convenience constructors require the `toml`, `yaml`, and `env-file` features respectively.

Built-in sources and `Config::merge_from_source` are transactional: if parsing or merging fails, the target `Config` keeps its previous state.

Environment sources that normalize keys reject empty or malformed dotted paths such as `APP_`, `APP__DB`, and `APP_DB__HOST`. TOML and YAML sources also reject duplicate flattened keys inside one document, for example a literal `"server.port"` key colliding with a nested `server.port` mapping.

TOML and YAML loaders intentionally support a flattened configuration subset. Nested maps/tables become dotted keys and homogeneous scalar sequences become multi-value properties. Nested arrays, arrays/tables/mappings inside a sequence, and YAML tagged sequence elements are rejected because flattening would lose structure; mixed scalar sequences are represented as strings.

| Type | Role |
|------|------|
| [`TomlConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.TomlConfigSource.html) | TOML files; nested tables are flattened to dot-separated keys |
| [`YamlConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.YamlConfigSource.html) | YAML files; nested mappings flattened similarly |
| [`PropertiesConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.PropertiesConfigSource.html) | Java `.properties` files |
| [`EnvFileConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.EnvFileConfigSource.html) | `.env`-style files |
| [`EnvConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.EnvConfigSource.html) | Process environment; optional prefix filtering and key normalization (e.g. `APP_SERVER_HOST` → `server.host`) |
| [`CompositeConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.CompositeConfigSource.html) | Chains multiple sources in order; later sources win on duplicate keys (subject to `Property` final semantics) |

```rust
use qubit_config::{Config, source::{
    CompositeConfigSource, ConfigSource, EnvConfigSource, TomlConfigSource,
}};

let mut config = Config::new();
let mut composite = CompositeConfigSource::new();
composite
    .add(TomlConfigSource::from_file("config.toml"))
    .add(EnvConfigSource::with_prefix("APP_"));
config.merge_from_source(&composite)?;
```

```rust
use qubit_config::Config;

let config = Config::from_toml_file("config.toml")?;
let env_config = Config::from_env_prefix("APP_")?;
```

## Usage Examples

### Basic Configuration

```rust
use qubit_config::Config;

let mut config = Config::new();

// Set various types
config.set("port", 8080)?;
config.set("host", "localhost")?;
config.set("debug", true)?;
config.set("timeout", 30.5)?;
config.set("is_use_prefix", "0")?;

// Get values with type inference and conversion
let port: i32 = config.get("port")?;
let host: String = config.get("host")?;
let debug: bool = config.get("debug")?;
let is_use_prefix: bool = config.get("is_use_prefix")?;

// Exact stored-type reads remain available when needed
assert!(config.get_strict::<bool>("is_use_prefix").is_err());
```

### Multi-Value Configuration

```rust
// Set multiple values
config.set("ports", vec![8080, 8081, 8082])?;

// Get all values
let ports: Vec<i32> = config.get_list("ports")?;

// Add values incrementally
config.set("server", "server1")?;
config.add("server", "server2")?;
config.add("server", "server3")?;

let servers: Vec<String> = config.get_list("server")?;
```

### Variable Substitution

```rust
config.set("host", "localhost")?;
config.set("port", "8080")?;
config.set("url", "http://${host}:${port}/api")?;

// Ordinary reads preserve placeholders.
let raw_url: String = config.get("url")?;
assert_eq!(raw_url, "http://${host}:${port}/api");

// Interpolation is explicit and may convert the result to any supported type.
let url: String = config.get_interpolated("url")?;
// Result: "http://localhost:8080/api"

// Configuration keys are resolved before the optional environment fallback.
config.set("APP_ENV", "production")?;
config.set("env", "${APP_ENV}")?;
let env: String = config.get_interpolated("env")?;
// Result: "production"
```

### Structured Configuration

`deserialize()` exposes a JSON-like Serde view containing mappings, sequences, booleans, strings, numbers, and null values without interpolating placeholders. Use `deserialize_interpolated()` when string leaves must be resolved first. Both methods apply the conversion rules in `ReadOptions`; for example, `ReadOptions::env_friendly()` can parse numeric strings, boolean aliases, comma-separated scalar string lists, and blank strings treated as missing while building a serde struct.

Lookup and conversion failures retain their original `ConfigError` kind, leaf path, and source. A mismatch raised only by the target type's Serde implementation returns a sanitized `DeserializeError` at the requested prefix.

### Persistence Wire Format

`serde_json::to_string(&config)` emits the stable V1 JSON persistence format:
`{ "version": 1, "description": ..., "properties": ..., "read_options": ... }`.
Property keys are emitted in lexical order, so equivalent configurations have the
same JSON bytes. V1 preserves property value shapes through `ValueWireV1` and
is the cross-version persistence contract for JSON. Future incompatible wire
changes will use a new version; readers continue to accept the legacy
unversioned top-level format emitted before V1.

When `prefix` is non-empty, `deserialize(prefix)` uses strict root selection:
an exact `prefix` property is deserialized as the root value, otherwise
`prefix.*` child keys form the root object. Defining both `prefix` and
`prefix.*` is a key conflict. Dotted keys must form an unambiguous object tree;
for example, `a` and `a.b` cannot both appear in the same deserialized object.

```rust
use qubit_config::Config;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct DatabaseConfig {
    host: String,
    port: i32,
    username: String,
    password: String,
}

let mut config = Config::new();
config.set("db.host", "localhost")?;
config.set("db.port", 5432)?;
config.set("db.username", "admin")?;
config.set("db.password", "secret")?;

let db_config: DatabaseConfig = config.deserialize("db")?;
assert_eq!(db_config.host, "localhost");
assert_eq!(db_config.port, 5432);
```

### Configurable Objects

```rust
use qubit_config::{Configurable, Configured};

// Use the Configured base class
let mut configured = Configured::new();
configured.config_mut().set("port", 3000)?;
configured.update_config(|config| {
    config.set("host", "localhost")?;
    config.set("workers", 4)?;
    Ok(())
})?;

// Custom configurable object
struct Application {
    configured: Configured,
}

impl Application {
    fn new() -> Self {
        Self {
            configured: Configured::new(),
        }
    }

    fn config(&self) -> &Config {
        self.configured.config()
    }

    fn config_mut(&mut self) -> &mut Config {
        self.configured.config_mut()
    }
}

let mut app = Application::new();
app.config_mut().set("port", 3000)?;
```

`config_mut()` gives direct mutable access and does not trigger
`on_config_changed()`. Use `update_config()` when changes should trigger that
callback once after a successful closure.

## Supported Data Types

| Rust Type | Description | Example |
|-----------|-------------|---------|
| `bool` | Boolean value; string reads accept `true` / `false` and `1` / `0` by default; `ReadOptions::env_friendly()` also accepts `yes` / `no` and `on` / `off` | `true`, `false`, `"0"`, `"yes"` |
| `char` | Character | `'a'`, `'中'` |
| `i8`, `i16`, `i32`, `i64`, `i128` | Signed integers | `42`, `-100` |
| `u8`, `u16`, `u32`, `u64`, `u128` | Unsigned integers | `255`, `1000` |
| `f32`, `f64` | Floating point | `3.14`, `2.718` |
| `String` | String | `"hello"`, `"世界"` |
| `Vec<T>` | List values; with collection read options, scalar strings can be split into list items | `[1, 2, 3]`, `"a,b,c"` |
| `chrono::NaiveDate` | Date | `2025-01-01` |
| `chrono::NaiveTime` | Time | `12:30:45` |
| `chrono::NaiveDateTime` | Date and time | `2025-01-01 12:30:45` |
| `chrono::DateTime<Utc>` | Timestamped datetime | `2025-01-01T12:30:45Z` |

## Extending with Custom Types

To support domain-specific reads, implement `FromConfig` for the target type. The implementation can reuse built-in `FromConfig` parsers and add validation, so call sites still use `config.get::<T>()`, `config.get_or::<T>()`, or `config.read(ConfigField::<T>)` without hand-written parse code.

```rust
use qubit_config::{Config, ConfigError, ConfigResult, Property};
use qubit_config::from::{ConfigParseContext, FromConfig};

#[derive(Debug, Clone, PartialEq)]
struct Port(u16);

impl Port {
    fn new(value: u16) -> Result<Self, String> {
        if value < 1024 {
            Err("Port must be >= 1024".to_string())
        } else {
            Ok(Port(value))
        }
    }

    fn value(&self) -> u16 {
        self.0
    }
}

impl FromConfig for Port {
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        let value = u16::from_config(property, ctx)?;
        Port::new(value).map_err(|message| {
            ConfigError::Other(format!("{}: {message}", ctx.key()))
        })
    }
}

let mut config = Config::new();
config.set("port", "8080")?;

let port: Port = config.get("port")?;
let fallback = config.get_or("fallback_port", Port::new(8080).unwrap())?;
```

Implement lower-level `qubit_value` traits only when you also need to store the custom type directly or use exact stored-type reads through `get_strict` / `get_list_strict`.

## API Design Philosophy

### Why Pure Generic API?

Typed reads use a generic approach (`get<T>()`, `set<T>()`, `get_or<T>()`, `read(ConfigField<T>)`) instead of a separate method for every supported type (like `get_i32()`, `get_bool()`, etc.) because:

1. **Universal** - Generic methods work with any type that implements the required traits, including custom types
2. **Concise** - Avoids repetitive type-specific method definitions
3. **Maintainable** - Adding new types only requires trait implementation, no modification to Config struct
4. **Idiomatic Rust** - Leverages Rust's type system and type inference capabilities

### Three Ways of Type Inference

```rust
// 1. Variable type annotation (recommended, most clear)
let port: i32 = config.get("port")?;

// 2. Turbofish syntax (use when needed)
let port = config.get::<i32>("port")?;

// 3. Context inference (most concise)
struct Server {
    port: i32,
}
let server = Server {
    port: config.get("port")?,  // Inferred from field type
};
```

## Error Handling

The configuration system uses the non-exhaustive `ConfigResult<T>` and
`ConfigError` types. Consumers should branch on `ConfigError::kind()`, read
single-key context through `ConfigError::path()`, and use
`ConfigError::candidate_paths()` for ordered multi-key lookup failures such as
`PropertyCandidatesNotFound`. Converting a value-layer error requires explicit
key context via `ConfigError::from((path, value_error))`; pathless conversion is
not supported.

## Performance Considerations

- **Enum-backed values** - Core property values use enums for predictable storage and conversion paths
- **Variable Substitution Optimization** - Uses `OnceLock` to cache regex patterns, avoiding repeated compilation
- **Efficient Storage** - Exact property lookup uses `HashMap` with O(1) expected complexity; prefix and section enumeration scan the stored properties and are O(n)
- **Staged Source Loading** - Built-in source loaders write into an already staged `Config` during composite and merge operations, preserving transaction semantics without repeated full-config clones

## Documentation

For detailed API documentation, visit [docs.rs/qubit-config](https://docs.rs/qubit-config).

## Dependencies

- `qubit-datatype` - Core utilities and data type definitions
- `qubit-value` - Value handling framework
- `qubit-redact` - Fixed-marker redaction for configuration diagnostics
- `serde` - Serialization framework
- `regex` - Regular expression support
- `chrono`, `url`, `num-bigint`, `bigdecimal` - optional rich-value support behind their matching atomic features
- `toml` - TOML parsing behind `toml`
- `serde_norway` - YAML parsing behind `yaml`
- `dotenvy` - `.env` file parsing behind `env-file`

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Run representative exact-key, prefix, section, and wire benchmarks
cargo bench --bench config_lookup_bench

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
