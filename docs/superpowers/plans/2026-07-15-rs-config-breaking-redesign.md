# rs-config Breaking Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `executing-plans` to implement this plan task-by-task in the current session. Do not dispatch subagents because this workspace has not authorized delegation.

**Goal:** Add 128-bit structured integer support, replace ambiguous prefix views with strict sections, minimize optional dependencies through atomic features, and validate the local crate through `rs-http`.

**Architecture:** Keep `ConfigReader` as the shared read abstraction, but replace `ConfigPrefixView` with `ConfigSection`, whose property namespace is strictly relative and contains descendants only. Keep the core dependency set minimal, gate every format and rich type independently, and use aggregate features only as aliases. Preserve behavior during internal file splits and parser simplification through existing external tests.

**Tech Stack:** Rust 2024, Serde, qubit-datatype 0.6, local qubit-value 0.10, Cargo features, repository CI scripts.

## Global Constraints

- Build on the existing dirty worktrees; do not overwrite unrelated user changes.
- Use `apply_patch` for file creation, deletion, renaming, and source edits.
- Do not run `git add`, `git commit`, or `git push` without separate authorization.
- Breaking removal of `ConfigPrefixView`, `prefix_view`, and `source-*` features is intentional.
- Keep `ConfigReader` breadth and boxed iterator design unchanged.
- Put every newly split helper type in its own file with the repository copyright header and complete Rustdoc.
- Put tests under `tests/`; do not add inline source tests.
- For final verification run `./align-ci.sh`, then `./ci-check.sh`; run exactly `./coverage.sh json` only if CI reports coverage below threshold.

---

## File Structure

### rs-config files created

- `src/config_section.rs` — strict relative section implementation and `ConfigReader` forwarding.
- `src/config_value_deserializer/internal/mod.rs` — private helper module declarations and restricted re-exports.
- `src/config_value_deserializer/internal/config_enum_access.rs` — Serde enum access.
- `src/config_value_deserializer/internal/config_variant_access.rs` — Serde variant access.
- `src/config_value_deserializer/internal/config_seq_access.rs` — Serde sequence access.
- `src/config_value_deserializer/internal/config_map_access.rs` — Serde map access.
- `tests/config_section_tests.rs` — public section contract and regression tests.

### rs-config files removed

- `src/config_prefix_view.rs` — replaced by `config_section.rs`.
- `tests/config_prefix_view_tests.rs` — replaced by `config_section_tests.rs`.

### rs-config files modified

- `Cargo.toml`, `Cargo.lock`, `.rs-ci-cargo-matrix.json` — feature and dependency model.
- `src/lib.rs`, `src/config.rs`, `src/config_reader.rs` — section export and construction.
- `src/config_value_deserializer.rs` — 128-bit methods and helper-type extraction.
- `src/from/from_config.rs`, relevant tests — atomic rich-type gates.
- `src/source/mod.rs`, source implementations, relevant tests — atomic format gates.
- `src/source/toml_config_source.rs`, `src/source/yaml_config_source.rs` — smaller sequence helpers.
- `tests/config_value_deserializer_tests.rs`, `tests/config_reader_tests.rs`, `tests/config_name_tests.rs`, `tests/from/config_parse_context_tests.rs` — new behavior and renamed API.
- `README.md`, `README.zh_CN.md`, maintained guides and examples — new API and features.

### rs-http files modified

- `Cargo.toml`, `Cargo.lock` — local minimal `qubit-config` dependency.
- `src/**/*.rs`, `tests/**/*.rs` containing `prefix_view` or `ConfigPrefixView` — section migration and documentation.

---

### Task 1: Reproduce and fix 128-bit structured deserialization

**Files:**
- Modify: `tests/config_value_deserializer_tests.rs`
- Modify: `src/config_value_deserializer.rs`

**Interfaces:**
- Consumes: `Config::deserialize<T>(&self, prefix: &str) -> ConfigResult<T>`.
- Produces: explicit `Deserializer::deserialize_i128` and `deserialize_u128` implementations.

- [ ] **Step 1: Add the failing success-path regression test**

Add this external test type and test near the existing signed and unsigned scalar tests:

```rust
#[derive(Debug, Deserialize, PartialEq)]
struct WideIntegers {
    signed: i128,
    unsigned: u128,
}

#[test]
fn test_deserialize_wide_integers() -> ConfigResult<()> {
    let mut config = Config::new();
    config.set("wide.signed", i128::MIN)?;
    config.set("wide.unsigned", u128::MAX)?;

    let actual: WideIntegers = config.deserialize("wide")?;

    assert_eq!(actual.signed, i128::MIN);
    assert_eq!(actual.unsigned, u128::MAX);
    Ok(())
}
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cargo test --locked --no-default-features --test config_value_deserializer_tests test_deserialize_wide_integers -- --exact
```

Expected: FAIL with Serde reporting that `i128` or `u128` is unsupported.

- [ ] **Step 3: Implement the minimal deserializer methods**

Extend the existing numeric macro invocation list:

```rust
deserialize_number!(deserialize_i128, visit_i128, i128);
deserialize_number!(deserialize_u128, visit_u128, u128);
```

Place each method beside the matching signed or unsigned integer family.

- [ ] **Step 4: Verify GREEN**

Run the focused command from Step 2 again. Expected: PASS.

- [ ] **Step 5: Add invalid and overflow regression cases**

Add separate tests that store `"not-an-integer"` and a decimal value above
`u128::MAX`, deserialize `WideIntegers`, match `ConfigError::DeserializeError`,
and assert that the nested source contains `wide.signed` or `wide.unsigned`
without containing the original invalid value.

- [ ] **Step 6: Run the complete deserializer test target**

Run:

```bash
cargo test --locked --no-default-features --test config_value_deserializer_tests
```

Expected: all tests pass with no warnings.

- [ ] **Step 7: Inspect the task diff without committing**

Run `git --no-pager diff -- tests/config_value_deserializer_tests.rs src/config_value_deserializer.rs` and confirm only the 128-bit contract changed.

---

### Task 2: Replace prefix views with strict configuration sections

**Files:**
- Create: `tests/config_section_tests.rs`
- Create: `src/config_section.rs`
- Modify: `src/lib.rs`
- Modify: `src/config.rs`
- Modify: `src/config_reader.rs`
- Modify: `tests/config_reader_tests.rs`
- Modify: `tests/config_name_tests.rs`
- Modify: `tests/from/config_parse_context_tests.rs`
- Remove: `tests/config_prefix_view_tests.rs`
- Remove: `src/config_prefix_view.rs`

**Interfaces:**
- Produces: `ConfigSection<'a>`, `Config::section`, `ConfigSection::section`, `ConfigSection::path`, and `ConfigReader::section`.
- Removes: `ConfigPrefixView`, every `prefix_view` method, and `ConfigPrefixView::prefix`.

- [ ] **Step 1: Create the desired public contract test first**

Create `tests/config_section_tests.rs` with the repository header and tests equivalent to:

```rust
use qubit_config::{Config, ConfigReader};

#[test]
fn test_section_resolves_keys_strictly_relative() {
    let mut config = Config::new();
    config.set("http.host", "direct").expect("set direct host");
    config
        .set("http.http.host", "strict-relative")
        .expect("set nested host");

    let section = config.section("http");

    assert_eq!(section.path(), "http");
    assert_eq!(section.get_string("host").expect("read host"), "direct");
    assert_eq!(
        section.get_string("http.host").expect("read strict key"),
        "strict-relative",
    );
}

#[test]
fn test_section_excludes_exact_root_property() {
    let mut config = Config::new();
    config.set("http", "root").expect("set root scalar");
    config.set("http.host", "localhost").expect("set child");

    let section = config.section("http");
    let keys = section.keys();

    assert_eq!(section.len(), 1);
    assert_eq!(keys, vec!["host".to_string()]);
    assert!(!section.contains(""));
    assert_eq!(config.get_string("http").expect("read root"), "root");
}

#[test]
fn test_section_nests_and_reports_root_paths() {
    let config = Config::new();
    let proxy = config.section(".http.").section(".proxy.");

    assert_eq!(proxy.path(), "http.proxy");
    assert_eq!(proxy.resolve_key("host"), "http.proxy.host");
    assert_eq!(proxy.resolve_key(""), "http.proxy");
}
```

Sort collected keys before equality if the underlying `HashMap` order is observable.

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --locked --no-default-features --test config_section_tests
```

Expected: compile failure because `Config::section` and `ConfigSection` do not exist.

- [ ] **Step 3: Implement `ConfigSection` with strict joining**

Copy only the forwarding behavior from `ConfigPrefixView`, then change the core helpers to these semantics:

```rust
pub struct ConfigSection<'a> {
    config: &'a Config,
    path: String,
    child_prefix: Option<String>,
}

impl<'a> ConfigSection<'a> {
    pub(crate) fn new(config: &'a Config, path: &str) -> Self;
    pub fn path(&self) -> &str;
    pub fn section(&self, path: &str) -> ConfigSection<'a>;

    fn resolve_key_cow<'b>(&'b self, name: &'b str) -> Cow<'b, str> {
        if self.path.is_empty() {
            Cow::Borrowed(name)
        } else if name.is_empty() {
            Cow::Borrowed(self.path.as_str())
        } else {
            Cow::Owned(format!("{}.{}", self.path, name))
        }
    }
}
```

`visible_entries` returns every root entry for the empty section. For a non-empty section it returns only keys with `child_prefix`, stripped to relative names; it must not special-case `k == path`.

Property operations with an empty relative name on a non-root section return absent/false rather than exposing the exact root scalar. `resolve_key("")`, `subconfig("", ...)`, and `deserialize("")` still resolve to the section path.

- [ ] **Step 4: Export and wire the new API**

Change `src/lib.rs` to declare and re-export `config_section::ConfigSection`. Replace `Config::prefix_view` with:

```rust
#[inline]
pub fn section(&self, path: &str) -> ConfigSection<'_> {
    ConfigSection::new(self, path)
}
```

Replace the required `ConfigReader::prefix_view` method with `section`, then update the `Config` and `ConfigSection` implementations.

- [ ] **Step 5: Verify the focused section tests are GREEN**

Run the command from Step 2. Expected: PASS.

- [ ] **Step 6: Migrate rs-config tests to the breaking API**

Replace type references with `ConfigSection`, method calls with `section`, and assertions with strict semantics. Remove compatibility assertions that expect already-qualified names to bypass joining or exact section-root scalars to appear in iteration.

- [ ] **Step 7: Remove the obsolete files through an explicit patch**

Delete `src/config_prefix_view.rs` and `tests/config_prefix_view_tests.rs` only after their replacement files compile. Verify `rg -n 'ConfigPrefixView|prefix_view' src tests` has no result.

- [ ] **Step 8: Run affected rs-config tests**

Run:

```bash
cargo test --locked --no-default-features --test config_section_tests
cargo test --locked --no-default-features --test config_reader_tests
cargo test --locked --no-default-features --test config_name_tests
cargo test --locked --no-default-features --test config_parse_context_tests
```

Expected: all commands pass.

---

### Task 3: Redesign features and remove unused dependencies

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `.rs-ci-cargo-matrix.json`
- Modify: `src/config.rs`
- Modify: `src/from/from_config.rs`
- Modify: `src/source/mod.rs`
- Modify: `src/utils.rs`
- Modify: feature-gated tests under `tests/`

**Interfaces:**
- Produces atomic `bigdecimal`, `chrono`, `num-bigint`, `url`, `env-file`, `toml`, and `yaml` features plus `rich-types`, `formats`, and `full` aliases.
- Removes `source-env-file`, `source-toml`, and `source-yaml`.

- [ ] **Step 1: Replace the Cargo feature table**

Use the exact feature model from the approved design:

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

Remove `qubit-serde` and the duplicate `bigdecimal`, `chrono`, `num-bigint`, `url`, and `qubit-value` dev-dependency declarations. Keep `tempfile`.

- [ ] **Step 2: Convert source gates**

Map `source-env-file` to `env-file`, `source-toml` to `toml`, and `source-yaml` to `yaml` in source, tests, rustdoc, and examples.

- [ ] **Step 3: Convert rich-type gates to atomic features**

Gate each import and `FromConfig` implementation independently:

```rust
#[cfg(feature = "bigdecimal")]
use bigdecimal::BigDecimal;
#[cfg(feature = "chrono")]
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
#[cfg(feature = "num-bigint")]
use num_bigint::{BigInt, BigUint};
#[cfg(feature = "url")]
use url::Url;
```

Split the existing aggregate macro invocation so each type family compiles only under its atomic feature.

- [ ] **Step 4: Replace the CI feature matrix**

Keep the minimal check and add one entry for every atomic feature, `rich-types`, `formats`, and `full`. Every entry sets `defaultFeatures` to `false`. Minimal runs `check`, `test`, and `doc`; atomic and aggregate entries run `test` and `doc`.

- [ ] **Step 5: Refresh the lockfile without upgrading unrelated packages**

Run:

```bash
cargo check --no-default-features
```

This lets Cargo remove dependencies made unreachable by the manifest without
requesting package upgrades. Before accepting the result, inspect
`git --no-pager diff -- Cargo.lock`; if unrelated versions change, stop and
repair the manifest or lockfile narrowly rather than using a destructive Git
command. Finish with `cargo metadata --locked --format-version 1`.

- [ ] **Step 6: Verify feature isolation**

Run:

```bash
cargo check --locked --no-default-features
cargo test --locked --no-default-features --features bigdecimal
cargo test --locked --no-default-features --features chrono
cargo test --locked --no-default-features --features num-bigint
cargo test --locked --no-default-features --features url
cargo test --locked --no-default-features --features env-file
cargo test --locked --no-default-features --features toml
cargo test --locked --no-default-features --features yaml
cargo test --locked --no-default-features --features full
```

Expected: every command passes; minimal metadata excludes optional integrations.

- [ ] **Step 7: Verify obsolete names and dependency are gone**

Run:

```bash
rg -n 'source-(env-file|toml|yaml)|qubit_serde|qubit-serde' Cargo.toml src tests README.md README.zh_CN.md examples doc
```

Expected: no maintained source, test, example, or documentation match.

---

### Task 4: Split deserializer helper types without changing behavior

**Files:**
- Create: `src/config_value_deserializer/internal/mod.rs`
- Create: `src/config_value_deserializer/internal/config_enum_access.rs`
- Create: `src/config_value_deserializer/internal/config_variant_access.rs`
- Create: `src/config_value_deserializer/internal/config_seq_access.rs`
- Create: `src/config_value_deserializer/internal/config_map_access.rs`
- Modify: `src/config_value_deserializer.rs`

**Interfaces:**
- Consumes and preserves the green public deserialization contract from Task 1.
- Produces no public API change.

- [ ] **Step 1: Record a green baseline**

Run `cargo test --locked --no-default-features --test config_value_deserializer_tests`. Expected: PASS.

- [ ] **Step 2: Create one helper type per internal file**

Move each type, its constructor, and direct Serde trait implementation. Use `pub(super)` only where the parent module must name the type. Keep fields private and construct through documented restricted constructors.

- [ ] **Step 3: Declare restricted re-exports**

`internal/mod.rs` declares the four files and re-exports only the names needed by `config_value_deserializer.rs`:

```rust
mod config_enum_access;
mod config_map_access;
mod config_seq_access;
mod config_variant_access;

pub(super) use config_enum_access::ConfigEnumAccess;
pub(super) use config_map_access::ConfigMapAccess;
pub(super) use config_seq_access::ConfigSeqAccess;
pub(super) use config_variant_access::ConfigVariantAccess;
```

- [ ] **Step 4: Re-run the complete deserializer target**

Run the baseline command. Expected: PASS with identical public behavior.

- [ ] **Step 5: Run the style checker for the split scope**

Run `./style-check.sh`. Expected: exit 0; if it reports broader existing violations, fix only violations caused or directly exposed by this split.

---

### Task 5: Reduce parser complexity and reorder constructors

**Files:**
- Modify: `src/source/toml_config_source.rs`
- Modify: `src/source/yaml_config_source.rs`
- Modify: `src/config.rs`
- Modify: corresponding external tests

**Interfaces:**
- Preserves all TOML/YAML flattening behavior.
- Preserves constructor signatures while moving every factory before non-constructors.

- [ ] **Step 1: Record parser test baselines**

Run:

```bash
cargo test --locked --no-default-features --features toml --test toml_config_source_tests
cargo test --locked --no-default-features --features yaml --test yaml_config_source_tests
```

Expected: both pass before refactoring.

- [ ] **Step 2: Extract format-specific sequence setters**

Replace the long match bodies with private, documented helpers such as:

```rust
fn set_toml_integer_array(prefix: &str, values: &[TomlValue], config: &mut Config) -> ConfigResult<()>;
fn set_toml_float_array(prefix: &str, values: &[TomlValue], config: &mut Config) -> ConfigResult<()>;
fn set_toml_bool_array(prefix: &str, values: &[TomlValue], config: &mut Config) -> ConfigResult<()>;
fn set_toml_string_array(prefix: &str, values: &[TomlValue], config: &mut Config) -> ConfigResult<()>;
```

Create equivalent YAML helpers using `YamlValue`. Do not introduce a generic cross-format trait or a new classification enum.

- [ ] **Step 3: Re-run parser tests after each format refactor**

Run the TOML test immediately after the TOML change and the YAML test immediately after the YAML change. Expected: PASS after each isolated refactor.

- [ ] **Step 4: Reorder the complete `Config` constructor methods**

Move `from_source`, environment constructors, and feature-gated file factories directly after `new` and `with_description`, preserving each method's full Rustdoc and attributes. Keep `with_read_options(&self, ...)` with transformations rather than constructors because it clones existing state.

- [ ] **Step 5: Correct documentation and inline attributes for moved/touched items**

Use `# Arguments`, document every observable error, use `#[inline(always)]` for pure getters/setters/forwarders, `#[inline]` for short constructors, and no inline attribute for complex parser helpers.

- [ ] **Step 6: Run focused config and parser tests**

Run:

```bash
cargo test --locked --no-default-features --test config_tests
cargo test --locked --no-default-features --features toml --test toml_config_source_tests
cargo test --locked --no-default-features --features yaml --test yaml_config_source_tests
```

Expected: all pass.

---

### Task 6: Point rs-http at local rs-config and migrate callers

**Files:**
- Modify: `../rs-http/Cargo.toml`
- Modify: `../rs-http/Cargo.lock`
- Modify: every `../rs-http/src/**/*.rs` and `../rs-http/tests/**/*.rs` match for the removed API.

**Interfaces:**
- Consumes: local `qubit-config` 0.14 core with `default-features = false`.
- Produces: downstream calls using `ConfigReader::section` and `Config::section`.

- [ ] **Step 1: Change the dependency declaration**

Use exactly:

```toml
qubit-config = { path = "../rs-config", version = "0.14", default-features = false }
```

- [ ] **Step 2: Migrate source and test calls**

Replace `.prefix_view(` with `.section(`, `ConfigReader::prefix_view` with `ConfigReader::section`, and `ConfigPrefixView` documentation references with `ConfigSection`. Inspect every replacement in context; do not perform an unchecked repository-wide rewrite.

- [ ] **Step 3: Refresh only rs-http dependency resolution**

From `rs-http`, run `cargo metadata --format-version 1` and inspect `Cargo.lock` to confirm `qubit-config 0.14.0` has no registry `source` field and resolves to `../rs-config`.

- [ ] **Step 4: Run affected downstream test targets**

Run the option and factory test targets identified by `Cargo.toml` and the existing test discovery configuration. At minimum, exercise HTTP client options, timeout, proxy, logging, retry, from-config helpers, and HTTP client factory tests.

- [ ] **Step 5: Verify the removed API is absent downstream**

Run `rg -n 'ConfigPrefixView|prefix_view' src tests README.md README.zh_CN.md examples doc` from `rs-http`. Expected: no maintained match.

---

### Task 7: Update maintained documentation and examples

**Files:**
- Modify: `README.md`
- Modify: `README.zh_CN.md`
- Modify: maintained files under `doc/` and `examples/` that mention old features or prefix views.

**Interfaces:**
- Documents the approved feature names and strict section semantics.

- [ ] **Step 1: Replace feature documentation**

Document `default = []`, all atomic features, aggregate aliases, and examples using `--features toml`, `--features yaml`, and `--features full`.

- [ ] **Step 2: Replace prefix-view examples**

Use `ConfigSection`, `Config::section`, nested `section`, strict relative keys, and the rule excluding an exact section-root scalar.

- [ ] **Step 3: Run doc checks for minimal and full configurations**

Run:

```bash
cargo test --locked --doc --no-default-features
cargo test --locked --doc --no-default-features --features full
```

Expected: all doctests pass.

- [ ] **Step 4: Scan maintained text for stale names**

Run the obsolete-name scans from Tasks 2 and 3. Historical review/design documents may retain old names only when clearly describing prior behavior; maintained user documentation may not.

---

### Task 8: Repository-prescribed verification and final audit

**Files:**
- Inspect all modified files in `rs-config` and `rs-http`.

**Interfaces:**
- Verifies every approved acceptance criterion.

- [ ] **Step 1: Audit dirty worktrees before write-capable scripts**

Record `git status --short` and `git --no-pager diff --stat` separately in `rs-config` and `rs-http`. Distinguish pre-existing changes from this implementation.

- [ ] **Step 2: Verify rs-config in prescribed order**

From `rs-config`, run:

```bash
./align-ci.sh
./ci-check.sh
```

If and only if CI reports coverage below threshold, run exactly:

```bash
./coverage.sh json
```

Fix only in-scope failures, then rerun the failed command and all later required steps.

- [ ] **Step 3: Verify rs-http in prescribed order**

From `rs-http`, apply the same `align-ci.sh` then `ci-check.sh` ordering and the same conditional coverage rule.

- [ ] **Step 4: Perform final source and dependency scans**

Confirm:

```bash
rg -n 'ConfigPrefixView|prefix_view|source-(env-file|toml|yaml)|qubit_serde|qubit-serde' src tests README.md README.zh_CN.md examples doc Cargo.toml
```

has no stale maintained match in `rs-config`, and the removed section API has no maintained match in `rs-http`.

- [ ] **Step 5: Inspect final diffs without committing**

Use `git --no-pager diff` separately in both repositories. Confirm no unrelated user change was reverted, Cargo lock changes are scoped, new files carry the authoritative header, tests remain external, and public docs match actual feature/API behavior.

- [ ] **Step 6: Report exact evidence**

List every verification command actually run, exit status, test totals, coverage result when generated, unresolved issues, and unchecked scope. Do not claim commands that were not run.
