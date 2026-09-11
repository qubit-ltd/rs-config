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
`target/llvm-cov/coverage.json`. After the shared coverage generator succeeds,
the project `coverage.sh` wrapper immediately runs
`scripts/check-coverage-files.py`. The checker reads that fresh report and
`.rs-ci-coverage.json`, excludes only the configured instrumentation exceptions,
and checks every remaining reported production file under `src/`:

- functions must be at least 95%;
- lines must be greater than 90%; and
- regions must be greater than 85%.

The aggregate summary printed by `coverage.sh` cannot replace this per-file
gate. A high crate-wide percentage can otherwise hide a low-coverage file.
Missing or invalid JSON fails the wrapper; it is never treated as a skipped
check.

The project-specific hook runs before the shared CI runner's coverage step and
therefore does not own this gate. The root `ci-check.sh` wrapper preserves the
fresh JSON until the shared runner finishes, invokes the per-file checker, and
only then applies the requested artifact cleanup policy. This ordering makes a
clean checkout enforce the same gate as the authoritative coverage command.

The evidence below was captured on 2026-09-10 with `cargo-llvm-cov 0.8.6` from
`target/llvm-cov/coverage.json`, after running the command above. The JSON
contains no usable source branch counters (`branches.count` is zero), so region
counters are the branch-sensitive evidence used by the gate. The annotated
counter view can be reproduced without rerunning tests with:

```text
cargo llvm-cov report --text --show-missing-lines
```

## Current Instrumentation Exceptions

The table is an exact, ordered copy of the eight paths currently configured in
`.rs-ci-coverage.json`. Percentages and counts come from the evidence report.

| File | Functions | Lines | Regions | Counter shape and behavioral evidence |
| --- | ---: | ---: | ---: | --- |
| `src/config/access.rs` | 94.12% (16/17) | 95.83% (69/72) | 92.97% (119/128) | Forwarding accessors retain an inline-attribution gap; section tests cover presence, absence and relative paths. |
| `src/error/config_error.rs` | 100.00% (16/16) | 87.91% (160/182) | 86.07% (173/201) | Error classification and source adapters include generic and defensive branches; error tests cover types, paths, missing facts and source chains. |
| `src/key/config_key.rs` | 85.71% (6/7) | 88.89% (24/27) | 89.19% (33/37) | Key accessors retain an inline-attribution gap; key tests cover AsRef, Unicode, whitespace, invalid keys and Serde validation. |
| `src/key/config_path.rs` | 85.71% (12/14) | 89.66% (52/58) | 88.89% (72/81) | Path accessors retain inline-attribution gaps; path tests cover root and relative paths, validation and Serde. |
| `src/property/property.rs` | 75.00% (15/20) | 85.85% (91/106) | 82.20% (97/118) | Five always-inlined accessors/mutators (`value_mut`, `description`, `set_description`, `data_type`, and `len`) retain zero-count bodies even though direct tests execute them. `property_tests` covers scalar and collection mutation, metadata, final flags, unset state, type and length, cloning, and wire round trips; `config_property_mut_tests` covers mutation through the configuration facade. |
| `src/reader/config_section.rs` | 90.91% (30/33) | 92.67% (139/150) | 91.34% (232/254) | Thin forwarding methods and generic AsRef calls retain attribution gaps; section tests cover nested paths, visibility, iteration, policies and empty sections. |
| `src/source/toml_config_source.rs` | 78.95% (30/38) | 92.48% (209/226) | 83.53% (350/419) | Feature-gated builds duplicate generic builder/conversion functions and iterator/error closures across test binaries. The scalar-string fallback arms for integer, float, boolean, and nested values are defensive: homogeneous arrays are dispatched before that converter and nested arrays/tables are rejected earlier. `toml_config_source_tests` exercises every accepted scalar/array kind, mixed and nested rejection, parse/I/O errors, collisions, limits, final values, and transactionality. |
| `src/source/yaml_config_source.rs` | 81.25% (39/48) | 87.33% (317/363) | 83.31% (549/659) | Feature-gated generic sequence converters, scanner closures, and error adapters have duplicated zero-attributed instances. The `unreachable!` nested-sequence arm and nested values in the scalar-to-string converter are defensive because the pre-scan rejects mapping, sequence, and tagged items first; a parser error without a public location is backend-dependent and cannot be constructed through the public load path. `yaml_config_source_tests` covers accepted scalar/sequence/tagged forms, alias rejection and quoted/block-scalar exceptions, non-string keys, mixed and nested rejection, collisions, limits, final values, and transactionality. |


The refactor removes the old scalar sequence implementation and its exemption. `property_mut.rs` is retained as an instrumentation exception because its inline guard and deref methods remain under-counted after direct behavioral tests; the current report is 90.91% functions, 94.34% lines, and 94.44% regions. `config_wire_limits.rs` remains covered without an exemption. No structured_read module is exempt.

## Maintaining the List

Do not add a business branch to the exception list. A proposed exception must
include a fresh JSON report, the exact uncounted function or defensive branch,
and a focused test that proves the corresponding observable behavior. Changes
to the English and Simplified Chinese documents must remain synchronized.

When a Rust/LLVM or `cargo-llvm-cov` update fixes counter attribution, remove
the affected exception before weakening or changing a threshold. Run the
authoritative coverage wrapper and the package listing after every policy
change:

```text
COVERAGE_ENFORCE_THRESHOLDS=1 ./coverage.sh json
cargo package --list --allow-dirty
```
