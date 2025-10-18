# prism3-config

[![CircleCI](https://circleci.com/gh/3-prism/prism3-rust-config.svg?style=shield)](https://circleci.com/gh/3-prism/prism3-rust-config)
[![Coverage Status](https://coveralls.io/repos/github/3-prism/prism3-rust-config/badge.svg?branch=main)](https://coveralls.io/github/3-prism/prism3-rust-config?branch=main)
[![Crates.io](https://img.shields.io/crates/v/prism3-config.svg?color=blue)](https://crates.io/crates/prism3-config)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

A powerful, type-safe configuration management system for Rust, providing flexible configuration management with support for multiple data types, variable substitution, and multi-value properties.

## Features

- ✅ **Pure Generic API** - Use `get<T>()` and `set<T>()` generic methods with full type inference support
- ✅ **Rich Data Types** - Support for all primitive types, temporal types, strings, byte arrays, and more
- ✅ **Multi-Value Properties** - Each configuration property can contain multiple values with list operations
- ✅ **Variable Substitution** - Support for `${var_name}` style variable substitution from config or environment
- ✅ **Type Safety** - Compile-time type checking to prevent runtime type errors
- ✅ **Serialization Support** - Full serde support for serialization and deserialization
- ✅ **Extensible** - Trait-based design for easy custom type support
- ✅ **Zero-Cost Abstractions** - Uses enums instead of trait objects to avoid dynamic dispatch overhead

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
prism3-config = "0.1"
```

## Quick Start

```rust
use prism3_config::Config;

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
    let timeout: u64 = config.get_or("timeout", 30);

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

## Usage Examples

### Basic Configuration

```rust
use prism3_config::Config;

let mut config = Config::new();

// Set various types
config.set("port", 8080)?;
config.set("host", "localhost")?;
config.set("debug", true)?;
config.set("timeout", 30.5)?;

// Get values with type inference
let port: i32 = config.get("port")?;
let host: String = config.get("host")?;
let debug: bool = config.get("debug")?;
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
use prism3_config::{Configurable, Configured};

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
| `bool` | Boolean value | `true`, `false` |
| `char` | Character | `'a'`, `'中'` |
| `i8`, `i16`, `i32`, `i64`, `i128` | Signed integers | `42`, `-100` |
| `u8`, `u16`, `u32`, `u64`, `u128` | Unsigned integers | `255`, `1000` |
| `f32`, `f64` | Floating point | `3.14`, `2.718` |
| `String` | String | `"hello"`, `"世界"` |
| `Vec<u8>` | Byte array | `[1, 2, 3, 4]` |
| `chrono::NaiveDate` | Date | `2025-01-01` |
| `chrono::NaiveTime` | Time | `12:30:45` |
| `chrono::NaiveDateTime` | Date and time | `2025-01-01 12:30:45` |
| `chrono::DateTime<Utc>` | Timestamped datetime | `2025-01-01T12:30:45Z` |

## Extending with Custom Types

To support custom types in the configuration system, you need to implement the necessary traits from `prism3_value`. The configuration system uses the `MultiValues` infrastructure for type-safe storage and retrieval.

Here's an example of how to work with custom types:

```rust
use prism3_config::Config;

// Define your custom type
#[derive(Debug, Clone, PartialEq)]
struct Port(u16);

// You can use the configuration system with types that can be converted to/from primitive types
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

// Usage with the configuration system
let mut config = Config::new();

// Store the port as a primitive type
config.set("port", 8080u16)?;

// Retrieve and wrap in custom type
let port_value: u16 = config.get("port")?;
let port = Port::new(port_value).map_err(|e| ConfigError::ConversionError(e))?;

// Or use get_or with validation
let port = Port::new(config.get_or("port", 8080u16))
    .map_err(|e| ConfigError::ConversionError(e))?;
```

For more advanced type conversions, you can implement the traits from `prism3_value` (`MultiValuesFirstGetter`, `MultiValuesSetter`, etc.). See the `prism3_value` documentation for details on implementing these traits for custom types.

## API Design Philosophy

### Why Pure Generic API?

We use a pure generic approach (only providing `get<T>()`, `set<T>()`, `get_or<T>()` core methods) instead of type-specific methods (like `get_i32()`, `get_string()`, etc.) because:

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
    TypeMismatch { expected: DataType, actual: DataType }, // Type mismatch
    ConversionError(String),            // Type conversion failed
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

For detailed API documentation, visit [docs.rs/prism3-config](https://docs.rs/prism3-config).

For internal design documentation (Chinese), see [src/README.md](src/README.md).

## Dependencies

- `prism3-core` - Core utilities and data type definitions
- `prism3-value` - Value handling framework
- `serde` - Serialization framework
- `chrono` - Date and time handling
- `regex` - Regular expression support

## Roadmap

- [ ] Configuration file loading (XML, TOML, YAML, JSON)
- [ ] Configuration merge strategies
- [ ] Configuration watching and hot reload
- [ ] Configuration validation framework
- [ ] Configuration encryption support
- [ ] Thread-safe wrapper type `SyncConfig`

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Copyright (c) 2025 3-Prism Co. Ltd. All rights reserved.

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

**Hu Haixing** - *3-Prism Co. Ltd.*

---

For more information about the Prism3 ecosystem, visit our [GitHub homepage](https://github.com/3-prism).
