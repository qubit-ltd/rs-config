# qubit-config Design

[Simplified Chinese](design.zh_CN.md) | English

This document describes the implemented architecture of `qubit-config` `0.15`.
Public API documentation and tests remain authoritative for individual calls and
edge cases.

## Stable Core

The compatibility center is `Config`, `ConfigReader`, `ConfigSection`,
`ReadPolicy`, and `ConfigSerdeExt`:

- `Config` owns canonical dotted-key properties and the default runtime read
  policy.
- `ConfigReader` supplies typed, optional, defaulted, multi-key, list, strict,
  and interpolated reads for `Config` and `ConfigSection`.
- `ConfigSection` is a borrowed view whose keys are strictly relative to its
  dotted path; the scalar stored at the section path is not one of its children.
- `ReadPolicy` groups conversion semantics, conversion limits, interpolation
  sources, and interpolation limits.
- `ConfigSerdeExt` projects an exact property or subtree into a Serde-owned type.

`ConfigReader` is sealed because its default methods depend on crate-owned
invariants. Its generic methods also make it non-object-safe. Downstream code
should use bounds such as `R: ConfigReader + ?Sized` or `&impl ConfigReader`, not
third-party implementations or `dyn ConfigReader`.

Single-key arguments use `AsRef<str>`, including borrowed/owned strings and
`ConfigKey`. Key validation stays in the configuration boundary; the argument
trait does not certify validity. Multi-candidate `ConfigNames` remains separate.
Default arguments use `qubit_value::IntoValueDefault` directly, converting the
default only when fallback is needed. Neither `ConfigName` nor
`IntoConfigDefault` remains a public compatibility alias.

## Source Pipeline

`ConfigSource::load` is the public execution boundary. It creates a fresh
`SourceLoadContext`, calls `load_into`, and returns the context-owned independent
`Config` layer only after loading succeeds. Custom sources write assignments
through `SourceLoadContext::set` or `set_null` and report input bytes, parser
nodes, and child sources before consuming those resources. Work not reported by
a custom source cannot be inferred from its final layer.
Invalid assignment keys returned by these context methods use
`ConfigError::SourceInvalidKey`; `kind()` remains `InvalidKey`, `path()` keeps
the rejected key, and `source_id()` identifies the assigning source (including
the leaf source in nested composites). Direct `Config::set` continues to return
`InvalidKey` without source context. Budget checks and charges occur before
assignment validation.

`PropertiesConfigSource`, `EnvConfigSource`, and `CompositeConfigSource` are
always available. TOML, YAML, and `.env` adapters are feature-gated. A composite
loads children in insertion order and merges each completed child layer into its
own layer. Later values override earlier values unless an existing property is
final. Loading and merging are transactional at the public boundary: an error
does not expose a partial source layer or partially mutate the caller's target.
TOML and YAML depth limits cover both AST nesting and the final dotted property
path after flattening; parser allocation and recursion remain outside this limit.
AST roots have depth zero; each mapping value and sequence element adds one
level, while an empty sequence adds no element depth. Custom sources may mark an
existing property final through `SourceLoadContext::set_final`; this updates
metadata without charging assignment budgets again and preserves the existing
one-way final-property rule during later merges.
TOML and YAML reject empty or otherwise invalid mapping member keys before
recursing into their values; valid empty tables and mappings remain accepted.
Properties physical lines use LF, CRLF, or CR terminators. Only ASCII space,
tab, and form feed are formatting whitespace; Unicode whitespace in keys and
values is preserved. Comment-only and otherwise value-free single YAML
documents load as empty layers, while explicit root null values remain invalid.
Built-in text-source file reads retry `Interrupted` without charging bytes;
other I/O failures retain their source path and error kind.
Environment normalization conflicts are checked in sorted order, selecting the
group with the smallest normalized key and then its two smallest original names.
That diagnostic is deterministic for valid inputs that fit the source budget.

## Read Policy and Sections

The default `ReadPolicy` belongs to a `Config` at runtime. `read_with` creates a
borrowed view with a temporary policy, and nested sections inherit that override.
Runtime policy is not part of persisted equality or the V1 wire representation.

Defaults apply only when a property is absent or effectively missing under the
active string policy. They do not hide conversion failures. An explicit empty
collection remains present. Candidate-key reads inspect names in caller-supplied
order, while section reads resolve only relative keys beneath the section path.

`Config::get` remains a policy-controlled converting read. Missing errors from
the value layer preserve `ValueMissing`, the original conversion source, and
the collection index. An invalid or policy-missing collection item stops both
fallback and candidate-key search. A concrete empty collection has no defaultable
first item. `PropertyHasNoValue` remains only for configuration reader policy
prechecks, not as a lossy replacement for value errors.

Each ordinary typed read has its own conversion operation. One structured Serde
materialization instead shares one `ConversionSession` across all fields, maps,
sequences, enum variants, and nested values, so its operation limits accumulate
across the complete result.

## Interpolation

Ordinary `get` and `deserialize` calls preserve `${name}` literally.
Interpolation occurs only through ordinary `*_interpolated` APIs or the
`interpolate` option of `deserialize_with`. A scoped read resolves
the current reader first, then the root configuration. The process environment
is consulted only when the policy explicitly selects
`InterpolationSources::ConfigThenEnv`; `ReadPolicy::env_friendly()` changes
conversion behavior but does not enable that fallback.

The default limits are a reference-chain depth of 64, 4,096 placeholder
expansions per read, and 1 MiB of UTF-8 output. Cycles and each exhausted resource
have structured `ConfigError` categories. Configuration allowed to choose
environment-variable names is a trust boundary. `.env` loading preserves `$NAME`
and `${NAME}` literals rather than reading the process environment implicitly,
including when comments contain quote characters. YAML anchors and aliases are
rejected from the scanner token stream before AST construction; scanner errors
fail closed, while quoted, commented, and block-scalar text is not treated as
anchor or alias tokens.

Expansion counts accumulate across one logical read, including candidate-key
search and fields later ignored by a structured visitor. Depth is measured along
the current reference chain, and the output-byte limit applies to each expanded
value. These limits are not reset for each candidate or field.
Each referenced value is converted to text with the active `ReadPolicy` and its
conversion limits. Each reference conversion is an independent conversion
operation; interpolation expansion counters remain shared by the logical read.
Interpolated string collections are expanded and converted in source order with
one shared conversion session. This returns the first substitution or
conversion error with its original source index while keeping at most one
expanded item in flight. Collection values are not split; scalar list text
continues to follow the configured splitting policy. Existing valid inputs and
public calls do not require migration.
Saved overlays are admitted against cumulative aggregate budgets, while one
parsed text input still has its own single-value limit. With cumulative
admission, a budget error may take precedence over a later parse error.

## Structured Serde

`ConfigSerdeExt` selects an exact property when one exists; otherwise it builds a
JSON-like object from descendants below the requested prefix. An exact property
and descendants at the same prefix are a `KeyConflict`. The empty prefix selects
all properties visible to the reader.

`UnknownFieldPolicy::Reject` reports sorted, root-relative paths for fields exposed through Serde's ignored-field callback as `ConfigError::UnknownProperties`. This is not complete schema validation: flatten structs, untagged enums, and custom deserializers can buffer fields that `serde_ignored` cannot observe, while flatten maps can consume arbitrary keys. Closed ordinary structs can use `#[serde(deny_unknown_fields)]` (including each struct payload of an untagged enum); Serde returns a sanitized deserialization error, not necessarily `UnknownProperties`. Since the attribute cannot be combined with `flatten`, applications needing a closed schema should use a closed nested struct or validate keys against their schema before deserializing. `deserialize(prefix)` defaults to no interpolation and rejected observable unknown fields;
`deserialize_with(prefix, ConfigDeserializeOptions)` allows `interpolate` and
`unknown_fields: UnknownFieldPolicy::{Reject, Ignore}`. These are the only two
structured-read entry points. Lookup,
interpolation, and conversion errors preserve configuration context, while
shape mismatches raised only by Serde become sanitized `DeserializeError` values.
The projection follows Serde's JSON-like data model and does not promise every
native rich-value conversion shape supported by `ConfigReader::get`.
Object member names are exposed to Serde's key deserializer as strings; numeric
map keys require explicit parsing by the application.

The prepared tree stores borrowed values, collection slices, JSON nodes, and
synthetic object indices. Node IDs index the arena and its per-node origin table;
property indexes address a separate table of borrowed source properties. A source
location identifies one original property plus typed nested key segments, so
literal dots in JSON keys remain distinct from dotted configuration paths. The
requested prefix is retained as `root_path` for diagnostics. The tree never
clones a complete intermediate JSON payload.
Object/object contributions with distinct leaves merge; duplicate leaves and
scalar/object conflicts fail. Interpolation overlays use typed source locations,
so literal dots in JSON keys cannot collide with nested paths. Only actual string
leaves are expanded, once per leaf; formatted URLs and other rich values do not
become interpolation input. Unchanged text stays borrowed.

Before visitors run, the complete selected input is admitted, including fields
the target ignores. Interpolated reads admit original and expanded input
independently. These passes use the reader's conversion limits but independent
counters from the shared leaf `ConversionSession`. Known numeric targets convert
from the original runtime type; `deserialize_any` uses Natural JSON categories,
including decimal text for wide integers. Scalar list splitting uses already
admitted items, and strings inside actual collections are not split again.
JSON integer literals remain exact through the signed and unsigned 128-bit
ranges. Out-of-range integer literals and non-finite floating-point values
return conversion errors; finite floats retain the existing binary
floating-point semantics. This structured conversion contract is distinct from
the generic wire deserializer when `serde_json/arbitrary_precision` is unified.
Policy-missing exact scalars can deserialize as `None`; missing subtree scalar
strings are omitted, while missing collection items remain errors. The public
result still requires `DeserializeOwned`.

## Wire Persistence

Serde serialization emits a deterministic V1 JSON envelope with an explicit
`version` field and lexically ordered property keys. Decoding accepts V1 and the
legacy unversioned top-level representation, validates canonical map keys and
embedded property names, and rejects unsupported versions. Runtime read policy
is deliberately absent from the V1 persisted contract; legacy policy data is
accepted but ignored.
V1 rejects any explicit `read_options` field, including `null`; legacy
unversioned input continues to accept and ignore it. Producers that construct a
V1 envelope manually must omit that field.

`Config::encode_json_vec` and `Config::decode_json_slice` apply the default
bounded wire profile. Their `*_with_limits` variants accept a custom
`ConfigWireLimits`. Ordinary Serde `Deserialize` bounds the decoded value but
cannot admit the original raw byte stream or lexical JSON tokens; callers must
use `decode_json_slice` for complete untrusted JSON input.

Both wire paths reject duplicate object keys before applying configuration
property-count and UTF-8 key-byte limits, and apply those domain limits before
constructing typed `Property` values. The bounded JSON entry point first
materializes a unique-key JSON tree under its JSON profile, so these domain
checks do not prevent that upstream tree from being allocated. When a property
also has an invalid value, an exceeded domain limit is reported first; with
limits satisfied, the existing Serde property error is preserved.

This persistence serialization is distinct from reading a business struct with
`ConfigSerdeExt`. No inverse business-struct flattening serializer is introduced,
and the existing Wire V1 representation is unchanged.

## Resource Budgets

The crate has separate resource domains rather than one shared counter:

| Domain | Default limits | Boundary |
| --- | --- | --- |
| Typed conversion | `qubit-datatype` `ConversionLimits::default()` | One ordinary read, or one complete structured materialization |
| Structured input admission | Reader conversion operation and structured-value limits | Complete selected input; independent original/expanded passes for interpolation |
| Interpolation | depth 64; 4,096 expansions; 1 MiB output | One interpolated read |
| Source loading | 8 MiB input; 65,536 assignments; 262,144 nodes; 256 composite children; depth 64 | One local source and every enclosing composite scope |
| JSON wire | 1 MiB input/output; depth 64; 100,000 nodes; 4,096 sequence items/map entries/properties; 256 KiB strings/object keys; 4 KiB numbers; 1 MiB payload; 256-byte property keys | One encode or decode operation |

Source byte, assignment, node, and child-source charges are cumulative and are
committed to local and aggregate scopes only when every applicable scope accepts
the charge. Depth is a point limit. TOML and YAML build third-party ASTs before
flattening; node, assignment, and depth accounting therefore bounds accepted
configuration output, not parser allocation or parser recursion. Wire budgets
are independent of `SourceLimits` and read conversion limits.
`ConfigWireLimitsBuilder::max_input_bytes` changes only the input-byte dimension
and preserves the other decode limits; `json_decode` replaces the whole profile.

## Feature Boundaries

The default feature set is empty. Core storage, readers, properties,
`.properties`, process-environment, and composite sources remain available
without optional features.

| Feature | Boundary |
| --- | --- |
| `bigdecimal`, `chrono`, `num-bigint`, `url` | Enable the corresponding rich value and conversion support |
| `env-file`, `toml`, `yaml` | Enable the corresponding format source and convenience constructor |
| `rich-types` | Enables all four rich-value features |
| `formats` | Enables all three optional format sources |
| `full` | Enables both `rich-types` and `formats` |

Applications should enable only the surfaces they use and depend directly on
`serde`, `serde_json`, or `qubit-datatype` when application code imports those
crates' APIs.

## Downstream Compatibility Commitments

Downstream code may rely on the public typed-read combinations, strict relative
section semantics, caller-ordered candidate keys, source ordering and final-value
protection, opt-in interpolation, stable error categories and accessors, declared
feature gates, and V1 wire decoding together with legacy unversioned decoding.

Compatibility does not extend to private module layout, exact error display text,
third-party `ConfigReader` implementations, `dyn ConfigReader`, implicit
environment interpolation, asynchronous loading, reload orchestration, or a
schema DSL. Error enums are non-exhaustive; downstream logic should branch on
`ConfigErrorKind` and context accessors such as `path()`, `candidate_paths()`,
`source_id()`, `source_budget_id()`, and `budget_error()`.

### Migration Notes

See the user guide's [qubit-config 0.14 migration steps](user_guide.md#migrating-from-qubit-config-014)
before upgrading downstream crates. The current workspace has downstream
version constraints on both sides of this release; cross-version public
`Config` and `ConfigReader` types cannot be passed between them.

Check TOML/YAML mapping member names for empty or invalid dotted keys during
upgrade. Such input is now rejected as a source parse error before recursion;
valid empty objects remain supported. When normalized environment names
conflict, the error remains `KeyConflict`, but now consistently reports the
group with the smallest normalized key and its two smallest original names.
These notes describe the library contract and do not claim validation across
all downstream applications.

See the [user guide](user_guide.md), [API documentation](https://docs.rs/qubit-config),
and [project README](../README.md) for the supported usage surface.
