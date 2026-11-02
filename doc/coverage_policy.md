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

The sidecar stores per-file evidence. The exact measured source baseline is
`task worktree based on c8bd18f82f7189a8a6638cb441733e4d170f6407; RSC-201 source labels and source-id propagation were included in this report`.
The pinned TOML source SHA-256 is `fe9e1febe3bd7f5a4121802f39868f1df36e5bb35cef79a516443560e2e7c2c9`;
the YAML source SHA-256 is `4c2ef991bc399c38b4c9c28ee56254571595d42e0bf906591349eb4c460e9577`.
TOML and YAML were measured together at
`2026-11-02T00:37:50Z` with `cargo-llvm-cov 0.8.6`, Rust 1.94.0, and LLVM
21.1.8, using `./scripts/check-project-coverage.sh`. The measured task worktree
was based on HEAD `c8bd18f82f7189a8a6638cb441733e4d170f6407` and included
RSC-201 source labels and source-id propagation. The source SHA-256 values are
pinned separately in the sidecar. TOML's exact lower bounds reflect defensive
Config write-error paths and backend callbacks that must remain uncalled when
parser admission rejects input. YAML retains a function-only lower bound; its
lines and regions meet the normal thresholds.

| File | Functions | Lines | Regions | Active minimums |
| --- | ---: | ---: | ---: | --- |
| `src/source/toml_config_source.rs` | 73.81% (31/42) | 93.41% (255/273) | 85.90% (469/546) | functions >= 73.17073170731707; regions >= 85.84558823529412 |
| `src/source/yaml_config_source.rs` | 81.25% (39/48) | 95.14% (313/329) | 90.70% (585/645) | functions >= 80.85106382978722 |

The sidecar records full-precision measured percentages and covered/count
counters for all three metrics per overridden file. Only displayed percentages
are rounded to two decimal places; enforcement uses the JSON minimum and an
inclusive `>=` comparison. Metrics meeting the normal threshold have no
override. The six earlier exceptions remain removed:
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
