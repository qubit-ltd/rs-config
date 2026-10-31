# qubit-config User Guide

[简体中文](user_guide.zh_CN.md) | English

This guide describes `qubit-config` `0.15`. It is for Rust application developers who need to load configuration from more than one place, read it as typed values, and diagnose invalid input without coupling application code to a particular file format.

## Migrating from qubit-config 0.14

Update single-key arguments to values implementing `AsRef<str>` (usually `&str` or `String`). Replace the removed `ConfigName` type with the string key itself; keep `ConfigNames` for APIs that search multiple candidate keys. Replace `IntoConfigDefault` with `qubit_value::IntoValueDefault`. The old trait is no longer re-exported by this crate. If application code names the replacement trait explicitly, add a direct `qubit-value` dependency aligned with this crate's `0.13` requirement.

Check the dependency graph before passing config types across crate boundaries:

```sh
cargo tree -d
cargo tree -i qubit-config@0.14.0
cargo tree -i qubit-config@0.15.0
```

Replace the selectors with the exact versions shown by the lockfile. A selector for a version absent from the graph reports that no matching package was found; that does not itself indicate a dependency conflict. In the local workspace, `rs-ioc` and `rs-websocket` require `qubit-config` `^0.14`; `rs-http`, `rs-web`, `rs-mime`, and the `rs-magika` dev-dependency require `0.15`. The root crate uses `qubit-value` `0.13`. A `^0.14` constraint does not automatically admit `0.15`, and public `Config` or `ConfigReader` types from the two versions are distinct and cannot be passed across those version boundaries. Prefer downstream crates already aligned to `0.15`. If no usable version is available, retain a compatible dependency combination or handle downstream migration as a separately authorized task; do not use a `patch` that violates dependency constraints to force version unification.

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

The following uses an in-memory `.properties` layer so the example is deterministic, then adds the process environment layer. `EnvConfigSource::from_prefix` selects `APP_`, strips it, lowercases the remainder, and converts double underscores to dots while preserving single underscores.

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
    sources.add(EnvConfigSource::from_prefix("APP_"));

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
qubit-config = "0.15"
```

The default feature set is empty. Add optional capabilities explicitly:

```toml
# TOML and .env sources
qubit-config = { version = "0.15", features = ["toml", "env-file"] }
```

```toml
# Chrono and URL values
qubit-config = { version = "0.15", features = ["chrono", "url"] }
```

```toml
# All optional value types and format sources
qubit-config = { version = "0.15", features = ["full"] }
```

The atomic optional features are `bigdecimal`, `chrono`, `num-bigint`, `url`, `env-file`, `toml`, and `yaml`. `rich-types` groups the four rich-value features; `formats` groups the three format features; `full` enables both groups.

The structured deserialization and JSON persistence examples also require direct Serde dependencies:

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

For a small in-memory configuration, the minimum setup is:

<!-- example: config_guide_quickstart -->
```rust
use qubit_config::Config;

fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("server.port", 8080)?;
    let port: u16 = config.get("server.port")?;
    assert_eq!(port, 8080);
    Ok(())
}
```

## Core Workflow

### Write and read values

Keys are canonical non-empty dotted names. `server` and `server.port` are valid. `.server`, `server.`, and `server..port` are rejected without trimming or normalization.

```rust
use qubit_config::{Config, ConfigError};

fn main() -> Result<(), ConfigError> {
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
    Ok(())
}
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

fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("server.host", "localhost")?;
    config.set("server.port", 8080i32)?;
    assert_eq!(read_endpoint(&config)?, ("localhost".to_owned(), 8080));
    Ok(())
}
```

`ConfigSection` is strictly relative and can be nested:

Call `ensure_known_keys` when a component owns a closed set of names plus
explicitly open subsections. It checks names only; it does not convert values
or change the reader's default permissive behavior. Section declarations are
relative to that reader:

```rust
reader.ensure_known_keys(&["port", "max_connections"], &["proxy"])?;
```

The same API can validate the root reader. Unknown paths are reported as
root-relative dotted configuration paths.

<!-- example: config_sections -->
```rust
use qubit_config::Config;
use qubit_config::ConfigReader;

fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("server.tls.enabled", true)?;
    let server = config.section("server")?;
    let tls = server.section("tls")?;
    assert!(tls.get::<bool>("enabled")?);
    Ok(())
}
```

Use `contains_section("server.tls")` for dotted section membership. Use `contains_key_prefix("server")` only when raw character-prefix matching is intended; it can also match a sibling such as `server2`.

Path-sensitive methods such as `section`, `contains`, `get_property`, `is_unset`, `remove`, and `ConfigReader::resolve_key` return `ConfigResult` because invalid paths are observable errors.

### Load and merge sources

The default `ConfigSource::load` and every library-managed loading request create an
independent layer. Custom sources implement
`load_into(&mut SourceLoadContext)` and must use that context as their only output path.
The default `ConfigSource::load` method executes the source through the standard budget
and transactional layer executor. Library-managed entry points such as
`Config::from_source`, `merge_properties_from_source`, and composite loading always
use that executor and call `load_into`, even when a source overrides `load`. Direct
calls to an overridden `load` remain the implementation's responsibility. Move custom
behavior needed by managed entry points into `load_into`. Call `context.set(...)` for
assignments and report
external input/parser work with the corresponding accounting methods before doing it.
The framework cannot infer omitted external I/O or parser work from the finished layer.
`set` and `set_null` already charge one node and one property assignment; call
`consume_nodes` only for additional parser work. Each charge is atomic across
local and aggregate budgets, but charges accepted earlier in a failed load are
not rolled back. The failing layer is discarded and the error should be
propagated. The `ConfigSource` API documentation includes a complete in-memory
source example.

A custom source can mark a property it has already written as final with
`context.set_final("service.mode", true)`. This changes property metadata only,
so it does not charge the source budget again. Repeating the current final state
is allowed; a final property cannot be unlocked. Missing properties and invalid
keys return errors. Use this when a source owns a value that later precedence
layers must not replace:

```rust
use qubit_config::source::{ConfigSource, SourceLimits, SourceLoadContext};
use qubit_config::{ConfigError, ConfigResult};

struct FinalSource;
impl ConfigSource for FinalSource {
    fn source_id(&self) -> String { "managed-defaults".to_owned() }
    fn limits(&self) -> SourceLimits { SourceLimits::default() }
    fn load_into(&self, context: &mut SourceLoadContext<'_>) -> ConfigResult<()> {
        context.set("service.mode", "managed")?;
        context.set_final("service.mode", true)
    }
}

let source = qubit_config::source::CompositeConfigSource::builder()
    .add_source(FinalSource)
    .add_source(qubit_config::source::PropertiesConfigSource::from_content(
        "service.mode=override\n",
    ))
    .build();
assert!(matches!(source.load(), Err(ConfigError::PropertyIsFinal(_))));
```

Built-in sources include:

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

Physical properties lines may end with LF, CRLF, or CR. Formatting whitespace
is limited to ASCII space, tab, and form feed; other Unicode whitespace in keys
and values is preserved. A literal carriage return in a value can be written
with the `\r` escape.

`EnvFileConfigSource` preserves `$NAME` and `${NAME}` placeholders as literal
values, including values following comments; it never reads process-environment
values implicitly. Resolve placeholders later through an explicit
`*_interpolated` read policy. YAML anchors and aliases are rejected by a token
scanner before the YAML backend builds its AST. Quoted strings, comments, and
block scalar text remain ordinary scalar content; malformed YAML scanner input
also fails closed rather than bypassing the guard. Every built-in source loads
through `SourceLoadContext`; a
composite checks each charge against both the child's local policy and its
shared aggregate policy before committing either budget. For TOML and YAML,
node, property-count, and nesting-depth accounting begins only while flattening
the parser's already materialized AST. These dimensions constrain accepted
configuration output but do not bound parser allocation or recursion.
Comment-only and otherwise value-free single YAML documents load as empty
layers. Explicit root `null` and `~` remain invalid configuration roots.
TOML and YAML also reject empty or invalid dotted mapping member keys before
recursing; valid empty tables and mappings remain supported. When environment
normalization produces conflicts, diagnostics select the group with the
smallest normalized key and then the two smallest original names, provided the
input is valid and fits the source budget.
The following canonical example loads a baseline, applies explicit deployment overrides,
and checks an invalid override. It uses an isolated prefix in subprocesses for self-validation;
production applications can choose a prefix that fits their environment:

<!-- example: config_guide_sources -->
```rust
use std::process::Command;

use qubit_config::Config;
use qubit_config::ConfigErrorKind;
use qubit_config::ConfigReader;
use qubit_config::ConfigResult;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::EnvConfigSource;
use qubit_config::source::PropertiesConfigSource;

/// Loads a baseline and an explicit deployment environment layer.
fn load_server() -> ConfigResult<(String, u16, u64)> {
    let sources = CompositeConfigSource::builder()
        .add_source(PropertiesConfigSource::from_content(
            "server.host=localhost\nserver.port=8080\n",
        ))
        .add_source(EnvConfigSource::from_prefix("QUBIT_CONFIG_GUIDE_APP_"))
        .build();
    let config = Config::from_source(&sources)?;
    let server = config.section("server")?;
    Ok((server.get("host")?, server.get("port")?, server.get_or("timeout", 30)?))
}

/// Checks one deployment state inside its own environment.
fn check_child(mode: &str) -> Result<(), Box<dyn std::error::Error>> {
    match mode {
        "baseline" => assert_eq!(load_server()?, ("localhost".to_owned(), 8080, 30)),
        "override" => assert_eq!(load_server()?, ("deployed.test".to_owned(), 9090, 30)),
        "invalid" => {
            let error = load_server().expect_err("invalid port must fail");
            assert_eq!(error.kind(), ConfigErrorKind::Conversion);
            assert_eq!(error.path(), Some("server.port"));
        }
        _ => return Err("unexpected example child mode".into()),
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(mode) = std::env::args().nth(1) {
        return check_child(&mode);
    }
    for mode in ["baseline", "override", "invalid"] {
        let mut command = Command::new(std::env::current_exe()?);
        command.env_clear().arg(mode);
        if mode == "override" {
            command.env("QUBIT_CONFIG_GUIDE_APP_SERVER__HOST", "deployed.test");
            command.env("QUBIT_CONFIG_GUIDE_APP_SERVER__PORT", "9090");
        } else if mode == "invalid" {
            command.env("QUBIT_CONFIG_GUIDE_APP_SERVER__PORT", "not-a-number");
        }
        assert!(command.status()?.success(), "example child failed: {mode}");
    }
    Ok(())
}
```

Use a convenience constructor when no target customization is needed:

```rust,no_run
use qubit_config::Config;

fn main() -> Result<(), qubit_config::ConfigError> {
    let config = Config::from_properties_file("config.properties")?;
    assert!(config.contains("port")?);
    Ok(())
}
```

Use `merge_properties_from_source` when the target already has values or a read policy:

```rust
use qubit_config::source::{
    CompositeConfigSource, PropertiesConfigSource,
};
use qubit_config::Config;

fn main() -> Result<(), qubit_config::ConfigError> {
    let mut source = CompositeConfigSource::new();
    source.add(PropertiesConfigSource::from_content("port=8080\n"));
    source.add(PropertiesConfigSource::from_content("port=9090\n"));

    let mut config = Config::new();
    config.merge_properties_from_source(&source)?;
    assert_eq!(config.get::<i64>("port")?, 9090);
    Ok(())
}
```

Source loading and merging are transactional at the public boundary: a source is loaded into an independent layer, the incoming layer is validated, and a failed merge leaves the target unchanged. A final property rejects a later override with `PropertyIsFinal`.

### Deserialize structured values

`Config::deserialize` maps an exact property or dotted subtree to a Serde-owned type. The empty prefix selects the root map.

<!-- example: config_structured_guide -->
```rust
use qubit_config::Config;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    host: String,
    port: u16,
}

fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("server.host", "localhost")?;
    config.set("server.port", 8080)?;

    let server: Server = config.deserialize("server")?;
    assert_eq!(server.host, "localhost");
    assert_eq!(server.port, 8080);
    Ok(())
}
```

Import `ConfigSerdeExt` when calling `deserialize` or `deserialize_with` through
a generic `ConfigReader` or section. `deserialize(prefix)` is equivalent to
`deserialize_with(prefix, ConfigDeserializeOptions::default())`: interpolation
is off and fields for which the source deserializer receives an ignored-value
request are rejected
with sorted root-relative paths in `ConfigError::UnknownProperties`. This is
not complete schema validation: flatten structs, untagged enums, and custom
deserializers can buffer keys without issuing such requests, while
flatten maps may consume arbitrary keys. For closed ordinary structs, add
`#[serde(deny_unknown_fields)]` to the target and to every struct payload in an
untagged enum. Serde then returns a sanitized deserialization error, not
necessarily `UnknownProperties`. Since Serde forbids combining that attribute
with `flatten`, replace that shape with a closed nested struct or validate keys
against the application's schema before deserializing. To allow extra fields,
set `unknown_fields: UnknownFieldPolicy::Ignore`; to expand placeholders, set
`interpolate: true`. Struct fields, `serde(rename)`, `serde(alias)`,
`serde(default)`, nested structs, maps, and `serde(flatten)` declare the accepted
shape. A mismatch raised only by Serde becomes a sanitized `DeserializeError`.
Standard missing-field errors retain the JSON-escaped schema field name; invalid
type errors use a fixed summary. Input values, Serde `Expected` and `Unexpected`
descriptions, and custom deserializer messages are not retained. Fixed
library-owned summaries may report internal structure names, known complete
lengths, or extra-item information. These human-readable summaries may change;
use `ConfigError::kind()` and `ConfigError::path()` for programmatic handling
instead of matching `Display` text.

Interpolation scope follows the selected storage shape. When `prefix` selects
dotted child properties, lookup starts in that selected subtree. When it
selects an exact structured property such as a JSON object, lookup starts in
the calling reader's scope; object member names are values, not configuration
keys. Existing root and permitted environment fallback still applies.

Object member names enter Serde as strings. If the application wants numeric
keys, deserialize string keys first and parse them explicitly. The canonical example
rejects distinct strings such as `080` and `80` that normalize to the same key:

<!-- example: config_numeric_map_keys -->
```rust
use std::collections::BTreeMap;

use qubit_config::Config;

/// Converts string keys while rejecting normalized numeric duplicates.
fn numeric_ports(named: BTreeMap<String, u16>) -> Result<BTreeMap<u16, u16>, Box<dyn std::error::Error>> {
    let mut ports = BTreeMap::new();
    for (name, value) in named {
        let number = name.parse::<u16>()?;
        if ports.insert(number, value).is_some() {
            return Err("duplicate numeric port key".into());
        }
    }
    Ok(ports)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.set("ports.80", 8080_u16)?;
    config.set("ports.443", 8443_u16)?;
    let named: BTreeMap<String, u16> = config.deserialize("ports")?;
    let ports = numeric_ports(named)?;
    assert_eq!(ports.get(&80), Some(&8080));
    assert_eq!(ports.get(&443), Some(&8443));
    config.set("ports.080", 8081_u16)?;
    let duplicate: BTreeMap<String, u16> = config.deserialize("ports")?;
    assert!(numeric_ports(duplicate).is_err());
    Ok(())
}
```

Complete selected input is admitted even when the target ignores some fields.
Interpolation admits original and expanded input separately, then all leaf
conversions share one independent `ConversionSession`. These resource domains
use the reader's conversion limits; ignored fields cannot bypass input limits.
Only actual string leaves are interpolated, not text formatted from rich values.
Unchanged values remain borrowed internally, while the result is owned.

Typed numeric fields convert from their original runtime type. Targets using
`deserialize_any`, such as `serde_json::Value`, receive Natural JSON categories:
wide integers become decimal strings, and rich values use their natural form.
For JSON numbers, integer literals are preserved exactly through the signed or
unsigned 128-bit range. Larger integer literals and non-finite floating-point
values return conversion errors; finite floating-point values keep the existing
binary floating-point semantics. This conversion path is separate from the
generic `Config` wire deserializer, whose handling of a unified
`serde_json/arbitrary_precision` feature is described in the JSON persistence
section.
Not every rich type accepts that Serde shape: `std::time::Duration` expects
`secs` and `nanos` fields rather than a stored scalar Duration. Distinct object
leaves can merge with dotted properties; duplicate leaves and exact-property /
descendant conflicts return `KeyConflict`. Policy-missing subtree scalar strings
are omitted, but missing collection items remain errors.

### Persist and decode configuration

`Config` implements Serde serialization through the stable versioned V1 JSON wire format. It also continues to accept legacy unversioned payloads when decoding. For complete untrusted input, use the bounded decoder:

```rust
use qubit_config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.set("server.port", 8080)?;
    let bytes = config.encode_json_vec()?;
    let restored = Config::decode_json_slice(&bytes)?;
    assert_eq!(restored.get::<i64>("server.port")?, 8080);
    Ok(())
}
```

`Config::encode_json_vec_with_limits` and
`Config::decode_json_slice_with_limits` accept `ConfigWireLimits` when the
default profile is not suitable. The profile composes rs-budget's
directional `JsonDecodeLimits` and `JsonEncodeLimits` with configuration-only
property and property-key limits. Decode input is admitted through one
`JsonDecodeSession`; encode structure and output are charged through one
`JsonEncodeSession`. Configuration-specific limits remain local to this crate.
When starting from `ConfigWireLimits::builder_from(&limits)`, `max_input_bytes`
changes only the input-byte limit and preserves other decode dimensions. Use
`json_decode` to replace the complete decode profile.
Ordinary `Deserialize` constructs a budgeted decoded-value visitor with
`ConfigWireLimits::default()`, so decoded structure, payload, property count,
and property keys remain bounded. A general Serde deserializer does not expose
the original byte stream or lexical JSON tokens; ordinary Deserialize therefore
cannot enforce the raw-input limit. Ordinary Serde decoding reads native
Float32/Float64 wire fields through their schema roles, even when
`serde_json/arbitrary_precision` is unified. Arbitrary JSON-domain numbers
still follow generic Serde event behavior; use the dedicated JSON helpers when
their JSON number semantics must be preserved. Use `Config::decode_json_slice`
or its custom-limits variant for untrusted JSON. This wire budget is separate from
`SourceLimits`, which applies while ingesting text sources. V1 rejects an
explicit `read_options` field, including `null`; legacy unversioned input still
accepts and ignores it. Manually constructed V1 envelopes must omit it.

Malformed wire representations return
`ConfigWireDecodeError::Representation { path, reason }`. The path is a JSON
Pointer relative to the wire document, such as `/properties/server.port/value`;
the dot in `server.port` is part of one property name. JSON Pointer escapes
`~` as `~0` and `/` as `~1`. This differs from dotted configuration paths such
as `server.port` returned by typed reads. The bounded precheck identifies
missing fields, invalid envelope types, and unknown fields. Other nested decode
failures use the safe, general `InvalidValue` reason. The decoder does not
include invalid input values or invent source line and column coordinates.
Business representation failures that previously surfaced as
`Json(Data)` are now classified as `Representation`; callers matching the
former category should handle this variant. Syntax and budget failures retain
their existing categories.

`usize` is supported by typed scalar reads, including optional and defaulted
reads:

```rust
let maximum = reader.get_optional::<usize>("max_connections")?;
let fallback = reader.get_or("max_connections", 1024_usize)?;
```

Conversion delegates to the existing `u128` policy and then checks the current
target platform's range. Values from zero through `usize::MAX` are accepted;
a larger value returns a conversion error with the resolved key and, for a
collection's first item, its source index. `usize` is not a stored or wire
integer type, and this addition does not define `Vec<usize>` conversion.

Downstream code can replace duplicated unknown-key checks with
`ensure_known_keys` and duplicated platform checks with `get_optional::<usize>`.
For example:

```rust
reader.ensure_known_keys(&["port", "max_connections"], &["proxy"])?;
let maximum = reader.get_optional::<usize>("max_connections")?;
let fallback = reader.get_or("max_connections", 1024_usize)?;
```

`rs-websocket` currently depends on `qubit-config` 0.14 and must first upgrade
that dependency to a release containing these APIs. This guide describes a
migration pattern; it does not imply that downstream repositories are changed.

Neither V1 nor legacy persistence stores the runtime default `ReadPolicy`.
Both bounded JSON decoding and ordinary Serde deserialization restore
`ReadPolicy::default()`. `Config` equality compares persisted data only, so a
successful data roundtrip does not imply that a custom read policy was
preserved. Restore the policy explicitly or use `read_with` for a temporary
view:

<!-- example: config_policy_restore -->
```rust
use qubit_config::Config;
use qubit_config::ConfigReader;
use qubit_config::ReadPolicy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let policy = ReadPolicy::builder_from(&ReadPolicy::env_friendly())
        .max_interpolation_depth(7)
        .build();
    let mut original = Config::builder().default_read_policy(policy.clone()).build();
    original.set("enabled", "yes")?;
    assert!(original.get::<bool>("enabled")?);
    let mut restored = Config::decode_json_slice(&original.encode_json_vec()?)?;
    assert_eq!(restored, original);
    assert_eq!(restored.default_read_policy(), &ReadPolicy::default());
    assert!(restored.get::<bool>("enabled").is_err());
    assert!(restored.read_with(&policy).get::<bool>("enabled")?);
    restored.set_default_read_policy(policy.clone());
    assert_eq!(restored.default_read_policy(), &policy);
    assert!(restored.get::<bool>("enabled")?);
    Ok(())
}
```

### Choose a read policy

`Config` owns a default `ReadPolicy` for direct reads. `read_with` creates a borrowed view with a temporary policy without changing the configuration:

<!-- example: config_read_policy -->
```rust
use qubit_config::options::ReadPolicy;
use qubit_config::{Config, ConfigReader};

fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("HTTP_ENABLED", "yes")?;
    config.set("HTTP_PORTS", "8080, 8081,,8082")?;

    let policy = ReadPolicy::env_friendly();
    let reader = config.read_with(&policy);
    let enabled: bool = reader.get("HTTP_ENABLED")?;
    let ports: Vec<u16> = reader.get("HTTP_PORTS")?;
    assert!(enabled);
    assert_eq!(ports, [8080, 8081, 8082]);
    assert_eq!(config.default_read_policy(), &ReadPolicy::default());
    Ok(())
}
```

`ReadPolicy` groups string blank handling, boolean literals, collection splitting, numeric conversion, duration conversion, and interpolation limits. Use its builder methods when the application needs a stricter or different policy. Passing a complete `StringConversionPolicy` or `CollectionConversionPolicy` installs that nested policy unchanged; the leaf-level `blank_string_policy` and `empty_item_policy` setters change only their respective nested setting. The conversion option types are supplied by `qubit-datatype`; applications that configure those lower-level options directly should depend on that crate as well.

### Interpolate explicitly

Ordinary `get` and `deserialize` calls preserve placeholders such as `${host}`. Use the corresponding interpolated method when substitution is part of the configuration contract:

<!-- example: config_interpolation -->
```rust
use qubit_config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.set("host", "localhost")?;
    config.set("url", "http://${host}")?;

    assert_eq!(config.get::<String>("url")?, "http://${host}");
    assert_eq!(config.get_interpolated::<String>("url")?, "http://localhost",);
    Ok(())
}
```

The default interpolation source is `ConfigOnly`. To fall back to process environment variables, configure it explicitly:

```rust
use qubit_config::{Config, ConfigReader};
use qubit_config::options::{InterpolationSources, ReadPolicy};

fn main() {
    let policy = ReadPolicy::builder_from(&ReadPolicy::env_friendly())
        .interpolation_sources(InterpolationSources::ConfigThenEnv)
        .build();
    let config = Config::builder().default_read_policy(policy).build();
    let _ = config;
}
```

Treat configuration that can select environment-variable names as trusted input. The default interpolation limits are a reference-chain depth of 64, 4,096 placeholder expansions per read, and 1 MiB of UTF-8 output. `ReadPolicy::builder()` can change each dimension; failures are reported as structured `ConfigError` categories.
Saved overlays can also be admitted against cumulative aggregate budgets. A single parsed text input still has its own single-value limit; when cumulative admission is enabled, a budget error may take precedence over a later parse error.
Expansion count is shared across a logical read, including candidate-key
search and fields a structured visitor later ignores. Depth applies to the
current reference chain; the output-byte limit applies to each expanded value.
Reference values use the active `ReadPolicy` and its conversion limits. Each
reference conversion is an independent conversion operation; expansion limits
remain cumulative for the logical read.
Interpolated string collections are expanded and converted in source order
with one shared conversion session. The first substitution or conversion error
is returned with its original item index, and only one expanded item is held
while conversion proceeds. Collection elements remain individual values;
scalar text continues to use the configured list-splitting policy. Existing
calls and valid inputs require no migration.

### Custom Target Types

Implement `FromConfig` for an application type by delegating to a built-in
conversion. The reader provides the context, so delegation retains its
conversion policy and resolved key in errors. Interpolation remains explicit:
an ordinary read preserves `${default_port}` and fails conversion, while the
scoped interpolated read resolves it relative to `server`.

<!-- example: config_custom_target -->
```rust
use qubit_config::conversion::{ConfigParseContext, FromConfig};
use qubit_config::{Config, ConfigErrorKind, ConfigReader, ConfigResult, Property};

#[derive(Debug, PartialEq)]
struct ServicePort(u16);

impl FromConfig for ServicePort {
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        u16::from_config(property, ctx).map(Self)
    }
}

fn main() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("server.port", "8080")?;
    assert_eq!(config.get::<ServicePort>("server.port")?, ServicePort(8080));

    config.set("server.default_port", "9090")?;
    config.set("server.port", "${default_port}")?;
    let server = config.section("server")?;
    assert_eq!(server.get_interpolated::<ServicePort>("port")?, ServicePort(9090));
    assert!(server.get::<ServicePort>("port").is_err());

    config.set("server.port", "invalid")?;
    let error = config.get::<ServicePort>("server.port").unwrap_err();
    assert_eq!(error.kind(), ConfigErrorKind::Conversion);
    assert_eq!(error.path(), Some("server.port"));
    Ok(())
}
```

These four checks cover an ordinary typed read, explicit scoped interpolation,
placeholder preservation during ordinary reads, and a conversion error that
retains the root key. `ServicePort` is an application example; whether zero is
a valid port is a separate business validation decision.

### Normalize environment keys

`EnvConfigSource::from_prefix("APP_")` applies all of these transformations:

1. select names beginning with `APP_`;
2. remove the prefix;
3. lowercase the remaining name; and
4. convert `__` to `.` while preserving single `_`.

For example, `APP_DATABASE__MAX_CONNECTIONS` becomes `database.max_connections`. Use `EnvConfigOptions` when only some transformations are wanted. If two distinct environment names collapse to the same normalized key, loading returns `KeyConflict` rather than selecting one silently. The error reports the conflicting environment names in lexicographic order, so its diagnostics do not depend on the operating system's environment-variable iteration order.

TOML and YAML sources flatten mappings into dotted properties. They accept scalars,
empty sequences, and homogeneous scalar sequences of strings, booleans, or values
within the same existing numeric category. YAML sequences containing null
elements are unsupported, as are object arrays, nested arrays, and heterogeneous
scalar sequences; these errors include source, path, and index context. Values
are never silently stringified to hide a structural mismatch.

### Configure source limits

The default `SourceLimits` are:

| Limit | Default |
| --- | ---: |
| Input bytes | 8 MiB (`8 * 1024 * 1024`) |
| Emitted assignments | 65,536 |
| Parsed structural nodes | 262,144 |
| Composite child sources | 256 |
| Nesting depth | 64 |

All built-in sources use the same session API for file and in-memory entry
points. `CompositeConfigSource::builder().limits(...).build()` configures a shared aggregate
policy in addition to every child's local policy. Cumulative charges are
committed to all applicable scopes only after every scope accepts them.
For TOML and YAML, node, property, and depth checks occur after the parser has
built its AST. Depth checks cover both AST nesting and the final dotted property
path; AST roots have depth zero, and each mapping value or sequence element adds
one level. An empty sequence has no element depth. For example, `a=[1]` and
`a: [1]` are rejected at depth limit 1 and accepted at limit 2. These checks are
not parser-stage allocation or recursion limits.
`SourceLimits::unbounded()` disables every source dimension and should only be
used when the application controls the input boundary.

## Errors and Diagnostics

`ConfigResult<T>` is `Result<T, ConfigError>`. `ConfigError` is non-exhaustive, so downstream code should use stable categories and context accessors:

```rust
use qubit_config::{Config, ConfigErrorKind, ConfigReader};

let config = Config::new();
let error = config.get::<u16>("server.port").unwrap_err();

assert_eq!(error.kind(), ConfigErrorKind::PropertyNotFound);
assert_eq!(error.path(), Some("server.port"));
```

Common categories include `InvalidKey`, `InvalidPath`, `PropertyNotFound`, `PropertyHasNoValue`, `TypeMismatch`, `Conversion`, `Substitution`, `SubstitutionCycle`, `SourceLimitExceeded`, `KeyConflict`, `Merge`, `PropertyIsFinal`, `Io`, `Parse`, and `Deserialize`.

`source_id()` returns the path or stable label for source I/O, parse, limit,
and invalid-key errors raised by `SourceLoadContext::set` or `set_null`. Those
assignment errors use the `SourceInvalidKey` variant while retaining the
`InvalidKey` kind and rejected key path. Direct `Config::set` errors remain
`InvalidKey` without a source ID. Other errors return `None` unless their
variant carries source context. Code matching error variants should handle
`SourceInvalidKey`; matching `kind()`, `path()`, and `source_id()` is more
resilient to source-specific diagnostic variants.
For source budget failures, `source_budget_id()` identifies the local or
aggregate scope that rejected the charge, and `budget_error()` exposes the
structured rs-budget failure.

`candidate_paths()` is useful for `get_any` failures because a multi-key error can contain several ordered paths. `source_index()` identifies an element position when a collection conversion reports one.

Diagnostics should use `kind()`, `path()`, and candidate paths for program logic. `Debug` output for configuration values redacts stored values through `qubit-redact` while retaining property metadata.

## Troubleshooting

### A default was not used

Check whether the key exists and contains a value. Defaults apply only to a missing or effectively missing key. If the key exists but cannot be converted to the target type, `get_or` returns `ConversionError` immediately.

### `${...}` remained unchanged

Use `get_interpolated`, `get_interpolated_or`, `get_any_interpolated`, or
`deserialize_with` with `interpolate: true`. Ordinary reads intentionally
preserve the literal placeholder. If the placeholder should come from the
process environment, confirm that the active policy uses `ConfigThenEnv`.

### A section cannot read a key

Check the section scope and relative key. `config.section("server")?.get("port")` resolves `server.port`; a section does not include its exact root scalar. Use `contains_section` for section membership and inspect `keys()` to see visible relative keys.

### A file loader is unavailable

Check the feature in `Cargo.toml`: `toml` enables `TomlConfigSource`, `yaml` enables `YamlConfigSource`, and `env-file` enables `EnvFileConfigSource`. `PropertiesConfigSource` and `EnvConfigSource` do not require these format features.

### A source reports a key conflict

Inspect normalized environment names or flattened structured keys. Environment prefix stripping, lowercasing, and underscore conversion can collapse distinct names. TOML/YAML documents can also contain duplicate flattened keys. Rename the inputs or choose less aggressive normalization.
For environment-name conflicts, the diagnostic consistently selects the
smallest normalized-key group and its two smallest original names when the
input is valid and within budget.

### A source exceeds a limit

Read `budget_error()` and use `source_budget_id()` to identify the local or aggregate scope that rejected the charge. Splitting a source may help with a child-local cumulative limit only if every enclosing composite still admits the total input. It does not reset ancestor budgets or solve every resource dimension, such as nesting depth. For an aggregate limit, reduce the total input or adjust that composite's corresponding limit after evaluating the input boundary. Splitting also increases the number of child sources. For example, two sources of 4 UTF-8 bytes each still exceed a composite input-byte limit of 6 bytes: the aggregate charge is 8 bytes regardless of the split.

### A merge changed nothing

Check the source result independently with `source.load()`, then inspect its keys. A failed source load or failed transactional merge leaves the target unchanged. A final target property also rejects a later override.

## Limitations and Best Practices

- The default feature set is empty. Format and rich-value support must be enabled deliberately.
- Configuration keys and section paths are validated; callers should not rely on implicit trimming or normalization of ordinary keys.
- Ordinary reads do not interpolate. Keep interpolation at explicit call sites so the trust boundary is visible.
- Process-environment fallback is disabled unless `InterpolationSources::ConfigThenEnv` is selected.
- Built-in source loads are bounded locally and, for composites, in aggregate.
  TOML/YAML node, property, and depth accounting starts after third-party AST
  materialization and does not protect parser allocation or recursion. JSON
  wire operations have a separate `ConfigWireLimits` profile; use the bounded
  slice API when raw-input admission is required.
- Source layers are independent and merged transactionally, but `Config` itself is mutable; choose ownership and synchronization in the application that uses it.
- `ConfigReader` is sealed and not object-safe because it has generic methods. Use generic bounds such as `&impl ConfigReader` rather than `dyn ConfigReader`.
- `ConfigSection` is a borrowed view. Keep the originating `Config` alive while using it and use relative keys within the section.
- Treat `Debug` output as a diagnostic view: values are redacted, so it is not a serialization format.

## Further Reading

- [Project README](../README.md)
- [中文 README](../README.zh_CN.md)
- [中文用户手册](user_guide.zh_CN.md)
- [English design document](design.md)
- [中文设计说明](design.zh_CN.md)
- [API documentation on docs.rs](https://docs.rs/qubit-config)
- [Repository](https://github.com/qubit-ltd/rs-config)
