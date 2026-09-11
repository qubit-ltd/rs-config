# qubit-config Design

[Simplified Chinese](design.zh_CN.md) | English

This document describes the implemented architecture of `qubit-config` `0.14`.
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

`PropertiesConfigSource`, `EnvConfigSource`, and `CompositeConfigSource` are
always available. TOML, YAML, and `.env` adapters are feature-gated. A composite
loads children in insertion order and merges each completed child layer into its
own layer. Later values override earlier values unless an existing property is
final. Loading and merging are transactional at the public boundary: an error
does not expose a partial source layer or partially mutate the caller's target.

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
and `${NAME}` literals rather than reading the process environment implicitly.

## Structured Serde

`ConfigSerdeExt` selects an exact property when one exists; otherwise it builds a
JSON-like object from descendants below the requested prefix. An exact property
and descendants at the same prefix are a `KeyConflict`. The empty prefix selects
all properties visible to the reader.

Structured reads reject unconsumed properties by default and report sorted,
root-relative paths through `ConfigError::UnknownProperties`.
`deserialize(prefix)` defaults to no interpolation and rejected unknown fields;
`deserialize_with(prefix, ConfigDeserializeOptions)` allows `interpolate` and
`unknown_fields: UnknownFieldPolicy::{Reject, Ignore}`. These are the only two
structured-read entry points. Lookup,
interpolation, and conversion errors preserve configuration context, while
shape mismatches raised only by Serde become sanitized `DeserializeError` values.
The projection follows Serde's JSON-like data model and does not promise every
native rich-value conversion shape supported by `ConfigReader::get`.

The prepared tree stores borrowed values, collection slices, JSON nodes, and
synthetic object indices. It never clones a complete intermediate JSON payload.
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

`Config::encode_json_vec` and `Config::decode_json_slice` apply the default
bounded wire profile. Their `*_with_limits` variants accept a custom
`ConfigWireLimits`. Ordinary Serde `Deserialize` bounds the decoded value but
cannot admit the original raw byte stream or lexical JSON tokens; callers must
use `decode_json_slice` for complete untrusted JSON input.

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

See the [user guide](user_guide.md), [API documentation](https://docs.rs/qubit-config),
and [project README](../README.md) for the supported usage surface.
