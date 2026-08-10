# qubit-config User Guide

[简体中文](user_guide.zh_CN.md) | English

This guide describes `qubit-config` `0.16.0`. It is for Rust application developers who need to load configuration from more than one place, read it as typed values, and diagnose invalid input without coupling application code to a particular file format.

## Purpose and Audience

Use this guide when an application has a configuration lifecycle such as:

1. load a checked-in or generated baseline;
2. apply deployment-specific values from a file or process environment;
3. read the resulting values inside application components; and
4. report missing, malformed, conflicting, or oversized input with useful paths.

The guide covers the public `qubit_config` API. It does not describe private modules or promise behavior that is not stated by the public API and tests.

## Conceptual Model

`qubit-config` separates storage, reading, conversion, and loading:

| Concept | Role |
| --- | --- |
| `Config` | Owns configuration properties, mutation methods, source merging, serialization, and the root reader scope. |
| `Property` | Stores one canonical key, its scalar-or-collection value container, optional description, and final flag. |
| `ValueContainer` | Preserves whether a source supplied a scalar or an explicit collection; collection elements are converted individually. |
| `ConfigReader` | Sealed read-only interface implemented by `Config` and `ConfigSection`; provides typed, optional, defaulted, multi-key, list, and strict reads. |
| `ConfigSection` | Borrowed reader view with strictly relative keys below a dotted path. |
| `ReadPolicy` | Controls string, boolean, collection, numeric, duration, and interpolation behavior for reads. |
| `ConfigSource` | Produces an independent `Config` layer that can be inspected or merged into a target configuration. |
| `CompositeConfigSource` | Loads several sources in insertion order; later layers override earlier layers for the same key unless a property is final. |

There are two different kinds of conversion to keep separate:

- `ConfigReader` methods such as `get::<T>` convert one stored property to a target type through `FromConfig`.
- `ConfigSerdeExt` and `Config::deserialize` project one exact property or subtree into a Serde-owned type.

The root `Config` and a section share the same read methods, but their key scopes differ. A section created at `server` resolves `host` to `server.host`; it does not expose the exact scalar property `server` as a child.

### Public API layers

The intended stable core is `Config`, `ConfigReader`, `ConfigSection`,
`ReadPolicy`, and `ConfigSerdeExt`. These types cover configuration ownership,
typed reads, scoped views, conversion policy, and structured deserialization.

Source adapters such as `PropertiesConfigSource`, `EnvConfigSource`,
`TomlConfigSource`, and `YamlConfigSource` form a separate loading layer and
can be enabled or selected according to the application's input formats.
Persistence and wire decoding, together with low-level `Property` operations,
are peripheral APIs for applications that need those specific capabilities.
The crate does not imply a reload framework, asynchronous source API,
object-safe reader, or schema DSL as part of the stable core.

## Scenario: Baseline Plus Deployment Overrides

Suppose a service has a local baseline:

```properties
server.host=localhost
server.port=8080
server.timeout=30
```

The deployment may set `APP_SERVER__HOST` and `APP_SERVER__PORT`. The success criteria are:

- local values remain available when no override exists;
- environment values override only matching keys;
- the application reads `server` through a relative section;
- a malformed value returns an error instead of silently selecting a default.

The following uses an in-memory `.properties` layer so the example is deterministic, then adds the process environment layer. `EnvConfigSource::with_prefix` selects `APP_`, strips it, lowercases the remainder, and converts double underscores to dots while preserving single underscores.

```rust
use qubit_config::{Config, ConfigReader};
use qubit_config::source::{
    CompositeConfigSource, EnvConfigSource, PropertiesConfigSource,
};

fn load_server() -> Result<(String, u16, u64), Box<dyn std::error::Error>> {
    let mut sources = CompositeConfigSource::new();
    sources.add(PropertiesConfigSource::from_content(
        "server.host=localhost\nserver.port=8080\nserver.timeout=30\n",
    ));
    sources.add(EnvConfigSource::with_prefix("APP_"));

    let mut config = Config::new();
    config.merge_properties_from_source(&sources)?;
    let server = config.section("server")?;

    Ok((
        server.get("host")?,
        server.get("port")?,
        server.get_or("timeout", 30)?,
    ))
}
```

Run the application with `APP_SERVER__PORT=9090` to observe the environment override. Keys that exist only in the properties layer remain visible. If `APP_SERVER__PORT=not-a-number`, `server.get::<u16>("port")` returns a conversion error; `get_or` does not hide an invalid present value.

## Installation and Minimal Configuration

The crate requires Rust `1.94` or newer and uses edition `2024`.

```toml
[dependencies]
qubit-config = "0.16"
```

The default feature set is empty. Add optional capabilities explicitly:

```toml
# TOML and .env sources
qubit-config = { version = "0.16", features = ["toml", "env-file"] }
```

```toml
# Chrono and URL values
qubit-config = { version = "0.16", features = ["chrono", "url"] }
```

```toml
# All optional value types and format sources
qubit-config = { version = "0.16", features = ["full"] }
```

The atomic optional features are `bigdecimal`, `chrono`, `num-bigint`, `url`, `env-file`, `toml`, and `yaml`. `rich-types` groups the four rich-value features; `formats` groups the three format features; `full` enables both groups.

The structured deserialization and JSON persistence examples also require direct Serde dependencies:

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

For a small in-memory configuration, the minimum setup is:

```rust
use qubit_config::Config;

let mut config = Config::new();
config.set("server.port", 8080)?;
let port: u16 = config.get("server.port")?;
# Ok::<(), qubit_config::ConfigError>(())
```

## Core Workflow

### Write and read values

Keys are canonical non-empty dotted names. `server` and `server.port` are valid. `.server`, `server.`, and `server..port` are rejected without trimming or normalization.

```rust
use qubit_config::{Config, ConfigError};

let mut config = Config::new();
config.set("worker.threads", 4u16)?;
config.set("worker.labels", ["api", "critical"])?;

let threads: u16 = config.get("worker.threads")?;
let labels: Vec<String> = config.get_list("worker.labels")?;
let missing: u16 = config.get_or("worker.timeout", 30)?;

assert_eq!(threads, 4);
assert_eq!(labels, ["api", "critical"]);
assert_eq!(missing, 30);

config.set("worker.invalid", "abc")?;
let error = config.get_or::<u16>("worker.invalid", 30).unwrap_err();
assert_eq!(error.kind(), qubit_config::ConfigErrorKind::Conversion);
assert!(matches!(error, ConfigError::ConversionError { .. }));
```

The important missing-value rules are:

- a missing key is handled by `get_optional`, `get_or`, and their multi-key variants;
- an unset property or a scalar blank string can be effectively missing under the active `ReadPolicy`;
- an explicit empty collection is present, so `get_optional_list` returns `Some(Vec::new())`;
- a present value that cannot be converted returns an error immediately.

For candidate names, use `get_any`, `get_optional_any`, or `get_any_or`. They inspect names in the supplied order and stop at the first value that is not effectively missing.

### Use read-only readers and sections

Code that only consumes settings can accept `&impl ConfigReader`:

```rust
use qubit_config::{Config, ConfigReader, ConfigResult};

fn read_endpoint(reader: &impl ConfigReader) -> ConfigResult<(String, u16)> {
    let server = reader.section("server")?;
    Ok((server.get("host")?, server.get("port")?))
}

let mut config = Config::new();
config.set("server.host", "localhost")?;
config.set("server.port", 8080i32)?;
assert_eq!(read_endpoint(&config)?, ("localhost".to_owned(), 8080));
# Ok::<(), qubit_config::ConfigError>(())
```

`ConfigSection` is strictly relative and can be nested:

```rust
let server = config.section("server")?;
let tls = server.section("tls")?;
let enabled: bool = tls.get("enabled")?;
# let _ = enabled;
```

Use `contains_section("server.tls")` for dotted section membership. Use `contains_key_prefix("server")` only when raw character-prefix matching is intended; it can also match a sibling such as `server2`.

Path-sensitive methods such as `section`, `contains`, `get_property`, `is_unset`, `remove`, and `ConfigReader::resolve_key` return `ConfigResult` because invalid paths are observable errors.

### Load and merge sources

Every `ConfigSource::load` call creates an independent layer. Built-in sources include:

- `PropertiesConfigSource`, always available, from `.properties` files or in-memory content;
- `EnvConfigSource`, always available, from process environment variables;
- `TomlConfigSource`, behind `toml`;
- `YamlConfigSource`, behind `yaml`;
- `EnvFileConfigSource`, behind `env-file`;
- `CompositeConfigSource`, which merges other sources in order.

The properties parser follows the Java properties escape dialect. It decodes
`\t`, `\n`, `\r`, `\f`, escaped separators and spaces, valid `\uXXXX` UTF-16
code units, and valid surrogate pairs. Malformed or incomplete Unicode escapes
such as `\u12G4` are preserved verbatim; unknown non-Unicode escapes drop the
leading backslash, matching Java properties behavior.

`EnvFileConfigSource` preserves `$NAME` and `${NAME}` placeholders as literal
values. Resolve them later through an explicit `*_interpolated` read policy;
loading a `.env` file never reads process-environment values implicitly. YAML
anchors and aliases are rejected by a pre-scan that skips quoted, commented,
and block-scalar content, so alias expansion cannot multiply the materialized
configuration. The input-byte limit is checked before parsing. For TOML and
YAML, property-count and nesting-depth limits are enforced while flattening
the parser's materialized AST; they constrain the resulting configuration but
do not bound intermediate parser allocation or recursion.

Use a convenience constructor when no target customization is needed:

```rust
let config = Config::from_properties_file("config.properties")?;
# let _ = config;
```

Use `merge_properties_from_source` when the target already has values or a read policy:

```rust
use qubit_config::source::{
    CompositeConfigSource, PropertiesConfigSource,
};

let mut source = CompositeConfigSource::new();
source.add(PropertiesConfigSource::from_content("port=8080\n"));
source.add(PropertiesConfigSource::from_content("port=9090\n"));

let mut config = Config::new();
config.merge_properties_from_source(&source)?;
assert_eq!(config.get::<i64>("port")?, 9090);
# Ok::<(), qubit_config::ConfigError>(())
```

Source loading and merging are transactional at the public boundary: a source is loaded into an independent layer, the incoming layer is validated, and a failed merge leaves the target unchanged. A final property rejects a later override with `PropertyIsFinal`.

### Deserialize structured values

`Config::deserialize` maps an exact property or dotted subtree to a Serde-owned type. The empty prefix selects the root map.

```rust
use qubit_config::Config;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

let mut config = Config::new();
config.set("server.host", "localhost")?;
config.set("server.port", 8080)?;

let server: Server = config.deserialize("server")?;
assert_eq!(server.port, 8080);
# Ok::<(), qubit_config::ConfigError>(())
```

Import `ConfigSerdeExt` when calling `deserialize`, `deserialize_interpolated`, `deserialize_lenient`, or `deserialize_interpolated_lenient` through a generic `ConfigReader`. Structured reads are strict by default: fields not consumed by the target `Deserialize` type return `UnknownProperties`. Use the lenient variants only when extra fields are intentionally allowed. Struct fields, `serde(rename)`, `serde(alias)`, `serde(default)`, nested structs, maps, and `serde(flatten)` declare the accepted configuration shape. Structured reads preserve configuration lookup/conversion context; a mismatch raised only by Serde becomes a sanitized `DeserializeError`.

### Persist and decode configuration

`Config` implements Serde serialization through the stable versioned V1 JSON wire format. It also continues to accept legacy unversioned payloads when decoding. For complete untrusted input, use the bounded decoder:

```rust
use qubit_config::Config;

let mut config = Config::new();
config.set("server.port", 8080)?;
let bytes = config.encode_json_vec()?;
let restored = Config::decode_json_slice(&bytes)?;
assert_eq!(restored.get::<i64>("server.port")?, 8080);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Config::encode_json_vec_with_limits` and
`Config::decode_json_slice_with_limits` accept `ConfigWireLimits` when the
default profile is not suitable. The profile composes rs-budget's
`JsonLimits` with configuration-only property and property-key limits. The
shared JSON adapter charges input, output, depth, node, collection, key,
string, and number resources during one traversal; configuration-specific
limits remain local to this crate. Ordinary Serde serialization remains
available and unchanged. This JSON budget is separate from `SourceLimits`,
which applies while ingesting text sources.

## Advanced Usage

### Choose a read policy

`Config` owns a default `ReadPolicy` for direct reads. `read_with` creates a borrowed view with a temporary policy without changing the configuration:

```rust
use qubit_config::{Config, ConfigReader};
use qubit_config::options::ReadPolicy;

let mut config = Config::new();
config.set("HTTP_ENABLED", "yes")?;
config.set("HTTP_PORTS", "8080, 8081,,8082")?;

let reader = config.read_with(&ReadPolicy::env_friendly());
let enabled: bool = reader.get("HTTP_ENABLED")?;
let ports: Vec<u16> = reader.get("HTTP_PORTS")?;
assert!(enabled);
assert_eq!(ports, [8080, 8081, 8082]);
# Ok::<(), qubit_config::ConfigError>(())
```

`ReadPolicy` groups string blank handling, boolean literals, collection splitting, numeric conversion, duration conversion, and interpolation limits. Use its builder methods when the application needs a stricter or different policy. The conversion option types are supplied by `qubit-datatype`; applications that configure those lower-level options directly should depend on that crate as well.

### Interpolate explicitly

Ordinary `get` and `deserialize` calls preserve placeholders such as `${host}`. Use the corresponding interpolated method when substitution is part of the configuration contract:

```rust
use qubit_config::{Config, ConfigReader};

let mut config = Config::new();
config.set("host", "localhost")?;
config.set("url", "http://${host}")?;

assert_eq!(config.get::<String>("url")?, "http://${host}");
assert_eq!(config.get_interpolated::<String>("url")?, "http://localhost");
# Ok::<(), qubit_config::ConfigError>(())
```

The default interpolation source is `ConfigOnly`. To fall back to process environment variables, configure it explicitly:

```rust
use qubit_config::{Config, ConfigReader};
use qubit_config::options::{InterpolationSources, ReadPolicy};

let policy = ReadPolicy::env_friendly()
    .with_interpolation_sources(InterpolationSources::ConfigThenEnv);
let config = Config::new().with_default_read_policy(policy);
```

Treat configuration that can select environment-variable names as trusted input. Interpolation also has configurable recursion-depth, expansion-count, and output-size limits; failures are reported as structured `ConfigError` categories.

### Normalize environment keys

`EnvConfigSource::with_prefix("APP_")` applies all of these transformations:

1. select names beginning with `APP_`;
2. remove the prefix;
3. lowercase the remaining name; and
4. convert `__` to `.` while preserving single `_`.

For example, `APP_DATABASE__MAX_CONNECTIONS` becomes `database.max_connections`. Use `EnvConfigOptions` when only some transformations are wanted. If two distinct environment names collapse to the same normalized key, loading returns `KeyConflict` rather than selecting one silently. The error reports the conflicting environment names in lexicographic order, so its diagnostics do not depend on the operating system's environment-variable iteration order.

TOML and YAML sources flatten mappings into dotted properties. They accept scalars,
empty sequences, and homogeneous scalar sequences. Object arrays, nested arrays,
and heterogeneous YAML scalar sequences are rejected with source, path, and index
context; values are never silently stringified to hide a structural mismatch.

### Configure source limits

The default `SourceLimits` are:

| Limit | Default |
| --- | ---: |
| Input bytes | 8 MiB (`8 * 1024 * 1024`) |
| Emitted assignments | 65,536 |
| Nesting depth | 64 |

All built-in text sources apply the same budget to file and in-memory entry
points. `max_input_bytes` is checked before parsing. For TOML and YAML,
`max_properties` and `max_nesting_depth` apply after the parser has built its
AST, while the source is being flattened; they are not parser-stage memory or
recursion limits. `SourceLimits::unbounded()` disables these three source
limits; use it only when the input boundary is controlled by the application.

## Errors and Diagnostics

`ConfigResult<T>` is `Result<T, ConfigError>`. `ConfigError` is non-exhaustive, so downstream code should use stable categories and context accessors:

```rust
use qubit_config::{Config, ConfigErrorKind, ConfigReader};

let config = Config::new();
let error = config.get::<u16>("server.port").unwrap_err();

assert_eq!(error.kind(), ConfigErrorKind::PropertyNotFound);
assert_eq!(error.path(), Some("server.port"));
# let _ = ConfigErrorKind::PropertyNotFound;
```

Common categories include `InvalidKey`, `InvalidPath`, `PropertyNotFound`, `PropertyHasNoValue`, `TypeMismatch`, `Conversion`, `Substitution`, `SubstitutionCycle`, `SourceLimitExceeded`, `KeyConflict`, `Merge`, `PropertyIsFinal`, `Io`, `Parse`, and `Deserialize`.

`source_id()` returns the path or stable label for source I/O, parse, and limit
errors. It returns `None` for errors that are not tied to a source loader.

`candidate_paths()` is useful for `get_any` failures because a multi-key error can contain several ordered paths. `source_index()` identifies an element position when a collection conversion reports one.

Diagnostics should use `kind()`, `path()`, and candidate paths for program logic. `Debug` output for configuration values redacts stored values through `qubit-redact` while retaining property metadata.

## Troubleshooting

### A default was not used

Check whether the key exists and contains a value. Defaults apply only to a missing or effectively missing key. If the key exists but cannot be converted to the target type, `get_or` returns `ConversionError` immediately.

### `${...}` remained unchanged

Use `get_interpolated`, `get_interpolated_or`, `get_any_interpolated`, or `deserialize_interpolated`. Ordinary reads intentionally preserve the literal placeholder. If the placeholder should come from the process environment, confirm that the active policy uses `ConfigThenEnv`.

### A section cannot read a key

Check the section scope and relative key. `config.section("server")?.get("port")` resolves `server.port`; a section does not include its exact root scalar. Use `contains_section` for section membership and inspect `keys()` to see visible relative keys.

### A file loader is unavailable

Check the feature in `Cargo.toml`: `toml` enables `TomlConfigSource`, `yaml` enables `YamlConfigSource`, and `env-file` enables `EnvFileConfigSource`. `PropertiesConfigSource` and `EnvConfigSource` do not require these format features.

### A source reports a key conflict

Inspect normalized environment names or flattened structured keys. Environment prefix stripping, lowercasing, and underscore conversion can collapse distinct names. TOML/YAML documents can also contain duplicate flattened keys. Rename the inputs or choose less aggressive normalization.

### A source exceeds a limit

Read the `SourceLimitKind` in the error and compare it with `SourceLimits::max_input_bytes`, `max_properties`, and `max_nesting_depth`. Increase one limit explicitly only when the input boundary is understood, or split the input into smaller source layers.

### A merge changed nothing

Check the source result independently with `source.load()`, then inspect its keys. A failed source load or failed transactional merge leaves the target unchanged. A final target property also rejects a later override.

## Limitations and Best Practices

- The default feature set is empty. Format and rich-value support must be enabled deliberately.
- Configuration keys and section paths are validated; callers should not rely on implicit trimming or normalization of ordinary keys.
- Ordinary reads do not interpolate. Keep interpolation at explicit call sites so the trust boundary is visible.
- Process-environment fallback is disabled unless `InterpolationSources::ConfigThenEnv` is selected.
- Built-in source loads are bounded by default. The input-byte limit protects
  the parser boundary; TOML/YAML property and depth limits protect AST
  flattening. JSON wire decoding has a separate `ConfigWireLimits` profile
  backed by rs-budget's generic `JsonBudget`.
- Source layers are independent and merged transactionally, but `Config` itself is mutable; choose ownership and synchronization in the application that uses it.
- `ConfigReader` is sealed and not object-safe because it has generic methods. Use generic bounds such as `&impl ConfigReader` rather than `dyn ConfigReader`.
- `ConfigSection` is a borrowed view. Keep the originating `Config` alive while using it and use relative keys within the section.
- Treat `Debug` output as a diagnostic view: values are redacted, so it is not a serialization format.

## Further Reading

- [Project README](../README.md)
- [中文 README](../README.zh_CN.md)
- [中文用户手册](user_guide.zh_CN.md)
- [API documentation on docs.rs](https://docs.rs/qubit-config)
- [Repository](https://github.com/qubit-ltd/rs-config)
