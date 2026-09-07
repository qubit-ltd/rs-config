# qubit-config Coverage Policy

[Simplified Chinese](coverage_policy.zh_CN.md) | English

This document records why the files in `.rs-ci-coverage.json` are temporarily
excluded from per-source thresholds. An exception is allowed only for an LLVM
instrumentation or attribution limitation, or for a defensive branch that
cannot be reached after an earlier invariant check. It is not permission to
leave production behavior untested.

## Enforcement

The authoritative coverage run is:

```text
COVERAGE_ENFORCE_THRESHOLDS=1 ./coverage.sh json
```

The run uses all Cargo features and writes the machine-readable report to
`target/llvm-cov/coverage.json`. `scripts/check-coverage-files.py` reads that
report and `.rs-ci-coverage.json`, excludes only the configured instrumentation
exceptions, and checks every remaining reported production file under `src/`:

- functions must be at least 95%;
- lines must be greater than 90%; and
- regions must be greater than 85%.

The aggregate summary printed by `coverage.sh` cannot replace this per-file
gate. A high crate-wide percentage can otherwise hide a low-coverage file.

The evidence below was captured on 2026-09-07 with `cargo-llvm-cov 0.8.6` from
`target/llvm-cov/coverage.json`, after running the command above. The JSON
contains no usable source branch counters (`branches.count` is zero), so region
counters are the branch-sensitive evidence used by the gate. The annotated
counter view can be reproduced without rerunning tests with:

```text
cargo llvm-cov report --text --show-missing-lines
```

## Current Instrumentation Exceptions

The table is an exact, ordered copy of the eleven paths currently configured in
`.rs-ci-coverage.json`. Percentages and counts come from the evidence report.

| File | Functions | Lines | Regions | Counter shape and behavioral evidence |
| --- | ---: | ---: | ---: | --- |
| `src/config/access.rs` | 90.91% (20/22) | 92.68% (76/82) | 89.47% (119/133) | The two missed function records are the always-inlined `Config::section` and `Config::section_if_present` forwarding accessors. `inherent_section_accessors_preserve_presence_semantics` invokes both through black-boxed function pointers and asserts present and missing section behavior, but their out-of-line definitions remain at zero. |
| `src/conversion/internal/config_scalar_seq_access.rs` | 80.00% (4/5) | 84.38% (27/32) | 83.93% (47/56) | Generic `SeqAccess::next_element_seed` monomorphizations and its error closures produce zero-count copies in separate integration-test binaries. `test_scalar_list_error_preserves_original_index`, `test_scalar_list_visitor_can_stop_before_tail_limit`, `test_scalar_string_sequence_charges_source_and_items_once`, and `test_scalar_string_sequence_preserves_admitted_item_errors` exercise successful, early-stop, indexed-error, and budgeted sequence reads. |
| `src/config_wire_limits.rs` | 90.48% (19/21) | 94.50% (103/109) | 92.31% (96/104) | The two missed records are the always-inlined scalar limit getters `max_properties` and `max_property_key_bytes`. `config_wire_scalar_limit_getters_are_callable_as_functions` calls both through black-boxed function pointers and checks non-default values; bounded wire tests separately prove both limits reject oversized inputs. |
| `src/error/config_error.rs` | 86.67% (13/15) | 83.62% (148/177) | 80.00% (156/195) | Small `#[inline]`/`#[inline(always)]` context accessors and match arms are attributed inconsistently across test binaries. `test_config_error_kind_covers_every_public_variant`, `test_source_errors_expose_source_id`, `config_error_optional_context_accessors_are_callable_as_functions`, the candidate-path tests, and source-limit tests call the public classifications and context accessors directly, including through black-boxed function pointers. |
| `src/key/config_key.rs` | 71.43% (5/7) | 77.78% (21/27) | 75.68% (28/37) | The missed function records are the always-inlined text and formatting accessors, not key validation. `key_and_path_wrapper_traits_preserve_the_validated_text` calls `ConfigKey::as_str`, `AsRef<str>`, Serde deserialization, and `Display` directly; the key boundary tests exercise empty, separator, Unicode, and whitespace cases. |
| `src/key/config_path.rs` | 92.86% (13/14) | 94.83% (55/58) | 95.06% (77/81) | The remaining zero-count record maps to the always-inlined `ConfigPath::as_str` body. `key_and_path_wrapper_traits_preserve_the_validated_text` invokes it through a black-boxed function pointer and also verifies `AsRef`, `Display`, and Serde; path tests cover root and every validation violation. |
| `src/property/property.rs` | 75.00% (15/20) | 85.85% (91/106) | 82.20% (97/118) | Five always-inlined accessors/mutators (`value_mut`, `description`, `set_description`, `data_type`, and `len`) retain zero-count bodies even though direct tests execute them. `property_tests` covers scalar and collection mutation, metadata, final flags, unset state, type and length, cloning, and wire round trips; `config_property_mut_tests` covers mutation through the configuration facade. |
| `src/property/property_mut.rs` | 90.91% (10/11) | 94.34% (50/53) | 94.44% (68/72) | The remaining missed record is the always-inlined `ConfigPropertyMut::as_property` accessor. `test_property_mut_guard_allows_mutation_before_final` calls it directly and asserts the guarded property name; the focused guard tests also cover successful mutation, final-state rejection, and error-path retention. |
| `src/reader/config_section.rs` | 89.47% (34/38) | 91.88% (147/160) | 91.12% (236/259) | Thin inherent-to-trait forwarding methods, always-inlined accessors, iterator closures, and generic `ConfigName` call sites create duplicate or zero-attributed instances. `config_section_tests` exercises root and nested paths, strict relative resolution, visibility, iteration boundaries, inherited policies, empty sections, and both `path` and `contains_section` through black-boxed function pointers. |
| `src/source/toml_config_source.rs` | 78.95% (30/38) | 92.48% (209/226) | 83.53% (350/419) | Feature-gated builds duplicate generic builder/conversion functions and iterator/error closures across test binaries. The scalar-string fallback arms for integer, float, boolean, and nested values are defensive: homogeneous arrays are dispatched before that converter and nested arrays/tables are rejected earlier. `toml_config_source_tests` exercises every accepted scalar/array kind, mixed and nested rejection, parse/I/O errors, collisions, limits, final values, and transactionality. |
| `src/source/yaml_config_source.rs` | 81.25% (39/48) | 87.33% (317/363) | 83.31% (549/659) | Feature-gated generic sequence converters, scanner closures, and error adapters have duplicated zero-attributed instances. The `unreachable!` nested-sequence arm and nested values in the scalar-to-string converter are defensive because the pre-scan rejects mapping, sequence, and tagged items first; a parser error without a public location is backend-dependent and cannot be constructed through the public load path. `yaml_config_source_tests` covers accepted scalar/sequence/tagged forms, alias rejection and quoted/block-scalar exceptions, non-string keys, mixed and nested rejection, collisions, limits, final values, and transactionality. |

## Maintaining the List

Do not add a business branch to the exception list. A proposed exception must
include a fresh JSON report, the exact uncounted function or defensive branch,
and a focused test that proves the corresponding observable behavior. Changes
to the English and Simplified Chinese documents must remain synchronized.

When a Rust/LLVM or `cargo-llvm-cov` update fixes counter attribution, remove
the affected exception before weakening or changing a threshold. Run both the
per-file checker and the package listing after every policy change:

```text
python3 scripts/check-coverage-files.py
cargo package --list --allow-dirty
```
