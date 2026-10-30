# qubit-config Coverage Policy

[Simplified Chinese](coverage_policy.zh_CN.md) | English

This policy describes the package-scoped, per-file coverage gate for `qubit-config`.
The project uses the shared coverage generator for instrumentation and report
production, then applies its own file-level policy to `src/`.

## Enforcement

Run the complete local coverage flow with:

```text
./scripts/check-project-coverage.sh
```

The script runs `.infra/bin/coverage.sh` with incremental compilation enabled,
matching CI, to generate `coverage.json` at the repository root, then invokes:

```text
python3 -B scripts/check-coverage-files.py coverage.json --config .infra/coverage/coverage.json
```

The checker also reads `scripts/coverage-metric-overrides.json` by default. Pass
`--metric-overrides PATH` to select another project sidecar. Unit and pipeline
fixtures provide their own sidecar explicitly through the checker default path.

The CI `coverage-files` job downloads the `coverage-reports` artifact produced
by the `rust-ci` job and checks `artifacts/coverage/coverage.json` against the
same `.infra/coverage/coverage.json` policy. The shared coverage summary and
badge remain useful aggregate views; they do not replace the per-file gate.

The policy scope is package `qubit-config`, with source directory `src/` and
these thresholds for each non-exempt source file:

- functions: at least 95%;
- lines: greater than 90%;
- regions: greater than 85%.

Every selected production file must be present in the report with valid
functions, lines, and regions counters, including files with metric overrides.
Without an override, functions must be at least 95%, lines greater than 90%,
and regions greater than 85%. A sidecar minimum applies only to its named
metric and is inclusive (`>=`); every other metric keeps its normal threshold.
An override is used only while that metric is below its normal threshold, so a
metric that meets the normal rule does not count as an exception. The checker
fails closed when a sidecar, version, required file entry, reason, evidence, or
valid summary is missing. `checked_files` includes all executable production
files; `exempt_files` reports files where at least one lower bound is active.
At the exact normal threshold, lines and regions still need an override because
their normal rules are strict (`>`); functions at exactly 95% meet their normal
rule (`>=`) and do not count as an active exception.

The current sidecar is based on a fresh report generated on 2026-10-30 with
`cargo-llvm-cov 0.8.6`, Rust 1.94.0, and LLVM 21.1.8, using
`CARGO_INCREMENTAL=1 ./.infra/bin/coverage.sh`. Its source baseline is HEAD
`5d772eb560666b7f6872f8f53649b8972d90a64b`; SHA-256 values for the three
measured files are recorded in the sidecar. All three files are present in the
report, and their measured counters are:

| File | Functions | Lines | Regions | Active minimums |
| --- | ---: | ---: | ---: | --- |
| `src/error/config_error.rs` | 100.00% (17/17) | 89.12% (172/193) | 87.32% (186/213) | lines >= 89.11% |
| `src/source/toml_config_source.rs` | 77.50% (31/40) | 92.44% (220/238) | 83.37% (376/451) | functions >= 77.50%; regions >= 83.37% |
| `src/source/yaml_config_source.rs` | 83.72% (36/43) | 93.18% (246/264) | 88.24% (450/510) | functions >= 83.72% |

The sidecar records full-precision measured percentages and covered/count
counters for all three metrics per file. Active minimums are the measured
percentage rounded down to two decimal places; metrics meeting the normal
threshold have no override. The historical six exceptions remain removed:
`src/config/access.rs`, `src/key/config_key.rs`, `src/key/config_path.rs`,
`src/property/property.rs`, `src/property/property_mut.rs`, and
`src/reader/config_section.rs`.
The shared `.infra/coverage/coverage.json` follows the strict schema consumed by
`.infra/bin/coverage.sh`. The project checker explicitly classifies files with
only declarations and test-only modules in `.infra/coverage/non-executable-files.json`;
it does not silently ignore a file because it is absent from the report.

## Maintaining the Policy

Do not add a business branch as an exception. A proposed minimum needs a fresh
coverage report, the exact attribution gap or unreachable defensive branch, and
focused tests for observable behavior. Keep `minimums` limited to the metrics
that remain below normal thresholds; never use a file-wide bypass. Update the
machine-readable sidecar and both language versions of this document together.
Remove a minimum when a Rust/LLVM or `cargo-llvm-cov` update fixes the
attribution issue.

The checker can be run independently against an existing report:

```text
python3 -B scripts/check-coverage-files.py coverage.json --config .infra/coverage/coverage.json
```

To select a non-default evidence file, add `--metric-overrides PATH`.
