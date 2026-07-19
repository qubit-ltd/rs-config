# rs-config Breaking Redesign

## Context

`rs-config` has completed its migration to the local `qubit-value` 0.10
contracts, but its public section-view API still mixes relative and absolute
keys, its default feature set enables every optional integration, and its
structured serde deserializer does not implement `i128` or `u128`.

This change intentionally permits breaking API and feature changes. It also
updates `rs-http`, the most demanding downstream consumer, to compile against
the local `rs-config` implementation.

## Goals

- Support `i128` and `u128` fields in `Config::deserialize` and section
  deserialization.
- Make the core crate compile with no optional format or rich-type dependency.
- Give every optional third-party integration an explicit feature.
- Remove dependencies that production code and tests do not use.
- Replace the ambiguous prefix view with a strictly relative section API.
- Apply the agreed file-organization, method-order, and parser-complexity
  corrections in the files touched by this redesign.
- Validate the current local `rs-config` through `rs-http`.

## Non-goals

- Splitting the broad `ConfigReader` trait or replacing its boxed iterators.
- Preserving the names or semantics of `ConfigPrefixView`, `prefix_view`, or
  the existing `source-*` features.
- Introducing a validated `ConfigPath` or `ConfigKey` newtype.
- Refactoring unrelated source types or changing stored value semantics.

## Feature Model

The crate has an empty default feature set:

```toml
[features]
default = []

bigdecimal = ["dep:bigdecimal", "qubit-value/big-number"]
chrono = ["dep:chrono", "qubit-value/chrono"]
num-bigint = ["dep:num-bigint", "qubit-value/big-number"]
url = ["dep:url", "qubit-value/url"]

env-file = ["dep:dotenvy"]
toml = ["dep:toml"]
yaml = ["dep:serde_norway"]

rich-types = ["bigdecimal", "chrono", "num-bigint", "url"]
formats = ["env-file", "toml", "yaml"]
full = ["rich-types", "formats"]
```

The atomic features are the canonical gates used by Rust `cfg` attributes.
The three aggregate features are convenience aliases only.

`qubit-serde` is removed because no source or test uses it. Duplicate
rich-type and `qubit-value` dev-dependencies are also removed: tests use the
normal optional dependency selected by the feature under test. `tempfile`
remains a dev-dependency.

The CI feature matrix covers the minimal build, every atomic feature, both
aggregate groups, and `full`. This prevents an `all` dev-dependency from
masking missing feature wiring.

## Structured Integer Deserialization

`ConfigValueDeserializer` implements `deserialize_i128` and
`deserialize_u128` through the same configured numeric conversion path used by
the smaller integer types. The implementation must preserve:

- native `Int128` and `UInt128` values;
- accepted string-to-number conversion under the current read options;
- overflow and signedness errors with the original configuration path;
- value redaction in public error messages.

External regression tests deserialize structs containing minimum, maximum,
and ordinary `i128`/`u128` fields. They also cover invalid and overflowing
input. The pre-fix test must fail because Serde's default methods report that
128-bit integers are unsupported.

## Section API

`ConfigSection<'a>` replaces `ConfigPrefixView<'a>`.

```rust
let http = config.section("http");
let host: String = http.get("host")?;
let proxy = http.section("proxy");
let proxy_host: String = proxy.get("host")?;
```

The public surface is:

```rust
impl Config {
    pub fn section(&self, path: &str) -> ConfigSection<'_>;
}

impl<'a> ConfigSection<'a> {
    pub fn path(&self) -> &str;
    pub fn section(&self, path: &str) -> ConfigSection<'a>;
}

pub trait ConfigReader {
    fn section(&self, path: &str) -> ConfigSection<'_>;
}
```

Section construction trims leading and trailing `.` separators. An empty path
represents the root section. Nested sections join normalized paths with one
`.` separator.

Every property name passed to a non-root section is strictly relative. The
implementation always joins the section path and supplied name; it never
detects or accepts an already-qualified name. Therefore:

- `config.section("http").get("host")` resolves `http.host`;
- `config.section("http").get("http.host")` resolves `http.http.host`;
- `resolve_key("host")` returns `http.host` for diagnostics;
- nested `section("proxy")` resolves under `http.proxy`.

A section contains only descendants beginning with `{path}.`. A scalar stored
at the exact section path is not a section entry and does not appear in
`contains`, `keys`, `len`, `iter`, or `iter_prefix`. It remains accessible from
the root `Config`. Empty relative property names are not exposed as section
entries. Subtree operations retain their established meaning:
`section.deserialize("")` and `section.subconfig("", ...)` operate on the
section's root path.

The old `ConfigPrefixView` export, source file, tests, `prefix_view` methods,
and `prefix` accessor are removed. README files, user guides, examples, and
all local downstream call sites migrate to `ConfigSection::section`.

## Internal Organization and Readability

`config_value_deserializer.rs` keeps only `ConfigValueDeserializer` and its
direct free helpers. Its private Serde access types move under:

```text
src/config_value_deserializer/internal/
  mod.rs
  config_enum_access.rs
  config_variant_access.rs
  config_seq_access.rs
  config_map_access.rs
```

Each file contains one helper type, its fields, constructor when needed, and
direct trait implementation. Visibility is restricted to the parent module.
Public API paths do not change.

The TOML and YAML sequence flatteners are split into small format-specific
helpers for integer, float, boolean, and string sequences. No generic
cross-format abstraction is introduced because the two parser ASTs have
different number and tagged-value semantics. Function-local classification
enums are eliminated, complex functions lose their long type-switch bodies,
and existing format behavior remains covered by external tests.

All `Config` constructors, including `from_source`, `from_env`, and file
factories, move before non-constructor methods. Moved and newly created items
receive complete Rustdoc, correct argument/return/error sections, and inline
attributes based on body complexity. Unrelated methods are not behaviorally
refactored.

`config_prefix_view.rs` and `tests/config_prefix_view_tests.rs` are replaced by
`config_section.rs` and `tests/config_section_tests.rs`. These file removals,
creations, module declaration changes, and public re-exports are intentional
breaking changes approved by this design.

## rs-http Integration

`rs-http/Cargo.toml` changes its dependency to:

```toml
qubit-config = { path = "../rs-config", version = "0.14", default-features = false }
```

Its source and tests replace `prefix_view` with `section` and update type names
in documentation. `rs-http` does not enable format or rich-type features
because it only consumes the core reader API. Its lockfile must resolve
`qubit-config` from the local path.

## Error Handling

Numeric conversion continues to use `ConfigError` with root-relative key
context. The redesign does not expose raw configuration values in errors.
Section lookup errors always report the canonical root key produced by strict
relative joining.

Feature-disabled APIs are absent at compile time. The README and rustdoc state
the feature required for every optional source and rich type.

## Test and Verification Contract

Implementation follows red-green-refactor cycles:

1. Add `i128`/`u128` structured-deserialization tests and confirm they fail for
   the unsupported Serde methods.
2. Replace prefix-view tests with the desired section API and confirm they fail
   to compile before the new API exists.
3. Implement the smallest integer and section changes needed to pass those
   tests.
4. Update feature gates and the feature matrix, then verify minimal, atomic,
   aggregate, and full configurations.
5. Point `rs-http` at local `rs-config`, migrate its section calls, and run its
   affected tests.
6. Perform internal file splits and parser refactoring while the behavioral
   tests remain green.

For each affected crate, repository verification runs in the prescribed order:
`./align-ci.sh`, then `./ci-check.sh`. `./coverage.sh json` runs only if CI
reports coverage below the configured threshold. The final report distinguishes
fresh results from historical coverage artifacts.

## Acceptance Criteria

- Struct fields of type `i128` and `u128` deserialize successfully and reject
  invalid or overflowing values with the correct root path.
- `cargo check --no-default-features` does not enable format or rich-type
  dependencies.
- Every atomic and aggregate feature compiles and is represented in the CI
  feature matrix.
- `qubit-serde` and duplicate rich-type/value dev-dependencies are absent.
- No `ConfigPrefixView`, `prefix_view`, or `source-*` feature reference remains
  in production, tests, examples, or maintained documentation.
- Sections resolve all supplied property names strictly relative and exclude
  the exact section-root scalar from their visible entries.
- `rs-http` resolves and tests against local `qubit-config` 0.14 without
  enabling optional rs-config features.
- The required repository verification sequence completes, or every blocker
  is reported with its exact command and output.
