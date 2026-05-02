# qubit-config

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-config.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-config)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-config/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-config?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-config.svg?color=blue)](https://crates.io/crates/qubit-config)
[![Rust](https://img.shields.io/badge/rust-1.94%2B-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

A powerful, type-safe configuration management system for Rust, providing flexible configuration management with support for multiple data types, variable substitution, multi-value properties, and pluggable **configuration sources** (files, environment, and composites).

## Features

- ✅ **Pure Generic API** - Use `get<T>()`, `read(ConfigField<T>)`, and `set<T>()` generic methods with full type inference support
- ✅ **Rich Data Types** - Support for all primitive types, temporal types, strings, byte arrays, and more
- ✅ **Multi-Value Properties** - Each configuration property can contain multiple values with list operations
- ✅ **Variable Substitution** - Support for `${var_name}` style variable substitution from config or environment
- ✅ **Type Safety** - Compile-time type checking to prevent runtime type errors
- ✅ **Serialization Support** - Full serde support for serialization and deserialization
- ✅ **Extensible** - Trait-based design for easy custom type support
- ✅ **Configuration sources** - [`ConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/trait.ConfigSource.html) trait with built-in loaders: TOML, YAML, Java-style `.properties`, `.env` files, process environment variables (with optional prefix / key normalization), and [`CompositeConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/struct.CompositeConfigSource.html) to merge several sources in order (later entries override earlier ones for the same key); use [`Config::merge_from_source`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.merge_from_source) to populate a `Config`
- ✅ **Read-only API** - [`ConfigReader`](https://docs.rs/qubit-config/latest/qubit_config/trait.ConfigReader.html) trait for typed reads without mutation; implemented by [`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html) and [`ConfigPrefixView`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html), with string helpers, multi-key reads, and field declarations that respect variable substitution
- ✅ **Configurable parsing** - [`ConfigReadOptions`](https://docs.rs/qubit-config/latest/qubit_config/options/struct.ConfigReadOptions.html) controls string trimming, blank handling, boolean literals, and scalar-string collection splitting globally or per field
- ✅ **Prefix views** - [`Config::prefix_view`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.prefix_view) returns a [`ConfigPrefixView`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html) scoped to a logical key prefix (relative keys map to `prefix.key`); nest with [`ConfigPrefixView::prefix_view`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html#method.prefix_view)
- ✅ **Zero-Cost Abstractions** - Uses enums instead of trait objects to avoid dynamic dispatch overhead

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
qubit-config = "0.11"
```

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
- Multi-value container
- Optional description
- Final flag (immutable after set)

### MultiValues

A type-safe container that can hold multiple values of the same data type.

### ConfigReader

[`ConfigReader`](https://docs.rs/qubit-config/latest/qubit_config/trait.ConfigReader.html) is the read-only configuration surface. Functions or types that only need settings can take `&impl ConfigReader` (or a generic `R: ConfigReader`) instead of `&Config`; the same API works for [`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html) and [`ConfigPrefixView`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html). `ConfigReader` has generic typed methods, so it is not object-safe and should not be used as `dyn ConfigReader`.

The main read APIs are:

| API | Behavior |
|-----|----------|
| `get<T>(name)` | Read a required value through `FromConfig`. |
| `get_optional<T>(name)` | Return `Ok(None)` when the key is missing or empty. |
| `get_or<T>(name, default)` | Use `default` only when the key is missing or empty. |
| `get_any<T>(&[names])` | Read the first present and non-empty key in order. |
| `get_optional_any<T>(&[names])` | Multi-key optional read. |
| `get_any_or<T>(&[names], default)` | Multi-key defaulted read. |
| `get_string`, `get_string_any`, `get_string_any_or` | String helpers with variable substitution. |
| `read(ConfigField<T>)` | Field declaration with name, aliases, default, and field-level read options. |
| `get_strict` / `get_list_strict` | Exact stored-type reads without cross-type conversion. |

Defaults do not hide bad configuration. If a key exists and its value fails parsing, type conversion, or variable substitution, the error is returned immediately instead of falling back to a default or later alias.

```rust
use qubit_config::{Config, ConfigError};

let mut config = Config::new();
config.set("worker.threads", "abc")?;

let missing = config.get_or("missing.threads", 4u16)?;
assert_eq!(missing, 4);

let invalid = config.get_or("worker.threads", 4u16);
assert!(matches!(invalid, Err(ConfigError::ConversionError { .. })));
```

### ConfigPrefixView

[`ConfigPrefixView`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html) is a zero-copy borrow of a `Config` with a logical key prefix. Use [`Config::prefix_view`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.prefix_view) to create it; keys passed to the view are resolved under that prefix. For example, prefix `db` and key `host` read the stored key `db.host`. Use [`ConfigPrefixView::prefix_view`](https://docs.rs/qubit-config/latest/qubit_config/struct.ConfigPrefixView.html#method.prefix_view) for nested views.

```rust
use qubit_config::{Config, ConfigReader};

let mut config = Config::new();
config.set("db.host", "localhost")?;
config.set("db.port", 5432i32)?;

let db = config.prefix_view("db");
let host: String = db.get_string("host")?;
let port: i32 = db.get("port")?;
```

### ConfigReadOptions

`ConfigReadOptions` controls how configured values are parsed. It can be set globally on a `Config`, or attached to a single `ConfigField<T>`.

| Option group | Controls |
|--------------|----------|
| `StringReadOptions` | Trimming and blank-string handling: preserve, treat as missing, or reject. |
| `BooleanReadOptions` | Accepted boolean literals and case sensitivity. |
| `CollectionReadOptions` | Splitting scalar strings into lists, delimiters, per-item trimming, and empty-item policy. |

`ConfigReadOptions::env_friendly()` is useful for environment-variable style values: it trims strings, treats blank scalar strings as missing, accepts `true/false`, `1/0`, `yes/no`, and `on/off`, and splits scalar strings on commas for `Vec<T>` reads while skipping empty items.

```rust
use qubit_config::{Config, options::ConfigReadOptions};

let mut config = Config::new().with_read_options(ConfigReadOptions::env_friendly());
config.set("HTTP_ENABLED", "yes")?;
config.set("HTTP_PORTS", "8080, 8081,,8082")?;

let enabled: bool = config.get("HTTP_ENABLED")?;
let ports: Vec<u16> = config.get("HTTP_PORTS")?;

assert!(enabled);
assert_eq!(ports, vec![8080, 8081, 8082]);
```

You can build stricter or domain-specific options with builder-style methods:

```rust
use qubit_config::{
    Config,
    options::{
        BooleanReadOptions, CollectionReadOptions, ConfigReadOptions, EmptyItemPolicy,
    },
};

let options = ConfigReadOptions::default()
    .with_boolean_options(
        BooleanReadOptions::strict()
            .with_true_literal("enabled")
            .with_false_literal("disabled"),
    )
    .with_collection_options(
        CollectionReadOptions::default()
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
use qubit_config::{Config, field::ConfigField, options::ConfigReadOptions};

let mut config = Config::new();
config.set("MIME_DETECTOR_ENABLE_PRECISE_DETECTION", "yes")?;

let enabled = config.read(
    ConfigField::<bool>::builder()
        .name("mime.enable_precise_detection")
        .alias("MIME_DETECTOR_ENABLE_PRECISE_DETECTION")
        .alias("ANOTHER_MIME_DETECTOR_ENABLE_PRECISE_DETECTION_PROPERTY")
        .default(false)
        .read_options(ConfigReadOptions::env_friendly())
        .build(),
)?;

assert!(enabled);
```

The builder makes the primary name explicit: `build()` is available only after `name(...)` has been supplied.

### Multi-Key Reads

Use `get_any`, `get_optional_any`, and `get_any_or` for lightweight alias reads when a full `ConfigField<T>` would be too verbose.

```rust
use qubit_config::{Config, options::ConfigReadOptions};

let mut config = Config::new().with_read_options(ConfigReadOptions::env_friendly());
config.set("SERVICE_URL", "http://localhost:8080")?;
config.set("SERVER_TIMEOUT", "30")?;

let url = config.get_string_any(&["service.url", "SERVICE_URL"])?;
let timeout = config.get_any_or(&["server.timeout", "SERVER_TIMEOUT"], 10u64)?;
let optional_port = config.get_optional_any::<u16>(&["server.port", "SERVER_PORT"])?;

assert_eq!(url, "http://localhost:8080");
assert_eq!(timeout, 30);
assert_eq!(optional_port, None);
```

Multi-key reads scan keys in order. Missing and empty values are skipped; the first configured non-empty value is parsed. If that value is invalid, the error is returned and later keys are not tried.

### Configuration sources

Implementations of [`ConfigSource`](https://docs.rs/qubit-config/latest/qubit_config/source/trait.ConfigSource.html) load external settings into a [`Config`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html). Call [`merge_from_source`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.merge_from_source) (or `load` on the source with a `&mut Config`) to apply them. When no pre-load customization is needed, use the convenience constructors such as [`Config::from_toml_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_toml_file), [`Config::from_yaml_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_yaml_file), [`Config::from_properties_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_properties_file), [`Config::from_env_file`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env_file), [`Config::from_env`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env), or [`Config::from_env_prefix`](https://docs.rs/qubit-config/latest/qubit_config/struct.Config.html#method.from_env_prefix).

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

// Variables are automatically substituted
let url = config.get_string("url")?;
// Result: "http://localhost:8080/api"

// Environment variables are also supported
std::env::set_var("APP_ENV", "production");
config.set("env", "${APP_ENV}")?;
let env = config.get_string("env")?;
// Result: "production"
```

### Structured Configuration

```rust
#[derive(Debug)]
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

let db_config = DatabaseConfig {
    host: config.get("db.host")?,
    port: config.get("db.port")?,
    username: config.get("db.username")?,
    password: config.get("db.password")?,
};
```

### Configurable Objects

```rust
use qubit_config::{Configurable, Configured};

// Use the Configured base class
let mut configured = Configured::new();
configured.config_mut().set("port", 3000)?;

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

## Supported Data Types

| Rust Type | Description | Example |
|-----------|-------------|---------|
| `bool` | Boolean value; string reads accept `true` / `false` and `1` / `0` by default; `ConfigReadOptions::env_friendly()` also accepts `yes` / `no` and `on` / `off` | `true`, `false`, `"0"`, `"yes"` |
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
        Port::new(value).map_err(|message| ConfigError::ConversionError {
            key: ctx.key().to_string(),
            message,
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

The configuration system uses `ConfigResult<T>` for error handling:

```rust
pub enum ConfigError {
    PropertyNotFound(String),           // Property does not exist
    PropertyHasNoValue(String),         // Property has no value
    TypeMismatch { key: String, expected: DataType, actual: DataType }, // Type mismatch
    ConversionError { key: String, message: String }, // Type conversion failed
    IndexOutOfBounds { index: usize, len: usize }, // Index out of bounds
    SubstitutionError(String),          // Variable substitution failed
    SubstitutionDepthExceeded(usize),   // Variable substitution depth exceeded
    MergeError(String),                 // Configuration merge failed
    PropertyIsFinal(String),            // Property is final and cannot be overwritten
    IoError(std::io::Error),            // IO error
    ParseError(String),                 // Parse error
    Other(String),                      // Other errors
}
```

## Performance Considerations

- **Zero-Cost Abstractions** - Uses enums instead of trait objects to avoid dynamic dispatch overhead
- **Variable Substitution Optimization** - Uses `OnceLock` to cache regex patterns, avoiding repeated compilation
- **Efficient Storage** - Properties stored in `HashMap` with O(1) lookup time complexity
- **Shallow Copy Optimization** - Cloning uses shallow copies when wrapped in `Arc`

## Testing

Run the test suite:

```bash
cargo test
```

Run with code coverage:

```bash
./coverage.sh
```

## Documentation

For detailed API documentation, visit [docs.rs/qubit-config](https://docs.rs/qubit-config).

For internal design documentation (Chinese), see [src/README.md](src/README.md).

## Dependencies

- `qubit-datatype` - Core utilities and data type definitions
- `qubit-value` - Value handling framework
- `serde` - Serialization framework
- `chrono` - Date and time handling
- `regex` - Regular expression support
- `toml` - TOML parsing for `TomlConfigSource`
- `serde_yaml` - YAML parsing for `YamlConfigSource`
- `dotenvy` - `.env` file parsing for `EnvFileConfigSource`

## Roadmap

- [ ] Additional configuration loaders (e.g. JSON, XML)
- [ ] Advanced merge / overlay policies beyond ordered `CompositeConfigSource`
- [ ] Configuration watching and hot reload
- [ ] Configuration validation framework
- [ ] Configuration encryption support
- [ ] Thread-safe wrapper type `SyncConfig`

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Copyright (c) 2025 - 2026. Haixing Hu, Qubit Co. Ltd. All rights reserved.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

See [LICENSE](LICENSE) for the full license text.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

---

For more information about the Qubit Rust libraries, visit our [GitHub organization](https://github.com/qubit-ltd).
