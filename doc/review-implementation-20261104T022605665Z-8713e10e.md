# T3 composite resource probe report

## Baseline and scope

- Isolated worktree: `/data/working/qubit/rust-common/.worktrees/rs-config/t3`.
- Baseline HEAD: `c7c1ff41e25d5b7e45846b42bf44c27e3cf68853` (`docs(config): clarify empty table section semantics`); initial worktree clean; required report path absent.
- Toolchain: rustc 1.94.0, cargo 1.94.0. Host: Linux x86_64, 64-bit; process stack limit 8192 KiB.
- Probe profile: release. RSS was not collected (not exposed by the example/driver); timing is load-only nanoseconds. Each shape/profile/scale is a separate child process launched by the example's parent, with 30-second child timeout. The matrix driver enforced a 300-second total deadline.
- Chain depth N means N nested composites above a root composite with one properties leaf; charged edges = N+1. Wide depth N is one root with N+1 properties leaves. Thus equal-N chain/wide cases charge the same child-source count. N=0/1 calibrate to 1/2 edges.

## Reproducible commands

Build:

```sh
cargo build --locked --release --example config_composite_resource_probe
```

Each parent command launches the same executable with `--child` and bounds the child to 30 seconds. Matrix invocation template:

```sh
target/release/examples/config_composite_resource_probe --shape chain|wide --depth N --limits default|raised|unbounded
```

After building the release example, this shell driver reproduces the complete matrix from the repository root. Each invocation is the parent mode; the example launches and bounds its isolated `--child`. The outer `timeout` enforces the five-minute matrix deadline.

```sh
timeout 300 bash <<'SH'
set -eu
bin=target/release/examples/config_composite_resource_probe
run_point() {
  local shape profile depth samples i
  shape=$1 profile=$2 depth=$3 samples=$4
  i=1
  while [ "$i" -le "$samples" ]; do
    "$bin" --shape "$shape" --depth "$depth" --limits "$profile"
    i=$((i + 1))
  done
}
for shape in chain wide; do
  for depth in 0 1 32 64 128 255; do run_point "$shape" default "$depth" 3; done
  for depth in 256 257; do run_point "$shape" default "$depth" 1; done
  for profile in raised unbounded; do
    for depth in 512 1024 2048; do run_point "$shape" "$profile" "$depth" 3; done
  done
done
SH
```

The exact driver iterated the following values serially, running each successful point three times and each expected default-budget rejection once; total driver deadline was five minutes:

- `default`: N = 0, 1, 32, 64, 128, 255, 256, 257.
- `raised`, `unbounded`: N = 512, 1024, 2048.
- Both `chain` and `wide` at each value. `raised` sets `max_sources(10000)` on composites; `unbounded` uses `SourceLimits::unbounded()`; default uses `SourceLimits::default()`.

All 76 child invocations completed in 2.863 seconds (40 default, 18 raised, 18 unbounded). The original run's raw phase log was `/tmp/config-composite-resource-probe.log`; its sample-level outcomes and timings are transcribed below. The raw log was a temporary execution artifact and is removed after implementation cleanup. Matrix driver exit status: 0.

## Results

Every child printed the five required stages in sequence: `constructed`, `loading`, `loaded`, `destroying`, `destroyed`. Every child and parent exited status 0. For successful samples, `loaded` reported `load_result=ok properties=1`. At default N=256 and 257 (257 and 258 charged edges), both shapes completed all five stages and reported `SourceLimitExceeded`, `SourceCount requested 1, but only 0 of 256 remains`; these are expected budget refusals, not probe failures. No child timed out, crashed, or aborted. No scale-up was stopped for an unexpected failure.

`elapsed_ns` values below are the three serial samples, except expected default budget refusals (one sample each). All timings are load-only.

| Profile | Shape | Depth (charged edges) | Samples | Outcome per sample | elapsed_ns per sample | Child exit per sample |
| --- | --- | ---: | ---: | --- | --- | --- |
| default | chain | 0 (1 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 24173, 18982, 16128 | 0, 0, 0 |
| default | chain | 1 (2 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 33823, 24320, 23992 | 0, 0, 0 |
| default | chain | 32 (33 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 231845, 220104, 179400 | 0, 0, 0 |
| default | chain | 64 (65 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 598860, 608176, 608041 | 0, 0, 0 |
| default | chain | 128 (129 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 1780483, 1826069, 1794875 | 0, 0, 0 |
| default | chain | 255 (256 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 5653736, 5731864, 5604347 | 0, 0, 0 |
| default | chain | 256 (257 edges) | 1 | EXPECTED_SOURCECOUNT_REJECTION | 4407342 | 0 |
| default | chain | 257 (258 edges) | 1 | EXPECTED_SOURCECOUNT_REJECTION | 5566652 | 0 |
| default | wide | 0 (1 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 16727, 29164, 17480 | 0, 0, 0 |
| default | wide | 1 (2 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 22432, 24505, 24465 | 0, 0, 0 |
| default | wide | 32 (33 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 112525, 100011, 104740 | 0, 0, 0 |
| default | wide | 64 (65 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 169276, 171495, 180655 | 0, 0, 0 |
| default | wide | 128 (129 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 314560, 308796, 340916 | 0, 0, 0 |
| default | wide | 255 (256 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 621023, 638138, 594687 | 0, 0, 0 |
| default | wide | 256 (257 edges) | 1 | EXPECTED_SOURCECOUNT_REJECTION | 753726 | 0 |
| default | wide | 257 (258 edges) | 1 | EXPECTED_SOURCECOUNT_REJECTION | 634577 | 0 |
| raised | chain | 512 (513 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 19638877, 19312123, 18843341 | 0, 0, 0 |
| raised | chain | 1024 (1025 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 58002885, 58230393, 57392315 | 0, 0, 0 |
| raised | chain | 2048 (2049 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 233450243, 223322120, 234871775 | 0, 0, 0 |
| raised | wide | 512 (513 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 1199783, 863688, 1120255 | 0, 0, 0 |
| raised | wide | 1024 (1025 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 1719428, 2264976, 2272222 | 0, 0, 0 |
| raised | wide | 2048 (2049 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 4207776, 4294601, 4285019 | 0, 0, 0 |
| unbounded | chain | 512 (513 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 17384303, 17396259, 17190365 | 0, 0, 0 |
| unbounded | chain | 1024 (1025 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 58510400, 59767969, 56465741 | 0, 0, 0 |
| unbounded | chain | 2048 (2049 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 224646425, 225364507, 222411816 | 0, 0, 0 |
| unbounded | wide | 512 (513 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 1269371, 1009807, 1193229 | 0, 0, 0 |
| unbounded | wide | 1024 (1025 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 2326821, 2357662, 2261657 | 0, 0, 0 |
| unbounded | wide | 2048 (2049 edges) | 3 | SUCCESS, SUCCESS, SUCCESS | 4372327, 4384348, 4483684 | 0, 0, 0 |

## Decision

RSC-20261104-003 is closed without a production change. At the supported default budget boundary, the N=255 chain (256 charged edges) completed construction, loading, and destruction in all three runs; N=256 (257 charged edges) and N=257 (258 charged edges) were refused by the existing aggregate `SourceCount` budget before an abnormal failure. Raised and unbounded chains through 2048 edges completed all phases in all three runs. The largest chain load samples were approximately 223–235 ms; equal-edge wide controls were approximately 4.2–4.3 ms. This confirms depth-dependent cost, but no crash or unsafe default-range behavior, and the design explicitly does not treat cost growth alone as an optimization trigger. No depth limit, ownership rewrite, or other production change is justified by this evidence. The sample size is three and RSS was unavailable; results are a bounded local probe, not a general performance guarantee.

## Files and validation

- Added `examples/config_composite_resource_probe.rs`.
- Added this report.
- `cargo build --locked --release --example config_composite_resource_probe` — exit 0 (initial release build and final post-rustfmt rebuild). One intermediate rerun failed to resolve a shared sibling `rs-redact` symlink during concurrent worktree cleanup; it passed after the symlink was restored.
- `rustfmt --check examples/config_composite_resource_probe.rs` — exit 0.
- `git diff --check` — exit 0.
- Prescribed matrix — all points completed; per-child statuses and expected budget refusals shown above; group elapsed 2.863 seconds.


## T0 initial repository baseline

The following snapshots were captured before implementation. All repositories were on `dev-starfish`. The accepted change set targets only `rs-config`; `rs-ioc` and `rs-websocket` were inspected as downstream context and were not modified by this task. The `rs-websocket` branch advanced later due to concurrent work in the shared workspace; this task did not adopt those changes.

### rs-config

- Start branch: `dev-starfish`
- Start HEAD: `c7c1ff41e25d5b7e45846b42bf44c27e3cf68853`
- Pre-existing status entries: 119

```text
 M benches/config_lookup_bench.rs
 M benches/interpolation_bench.rs
 M benches/source_parsing_bench.rs
 M benches/structured_read_bench.rs
 M benches/typed_collection_bench.rs
 M examples/config_custom_target.rs
 M examples/config_guide_sources.rs
 M examples/config_interpolation.rs
 M examples/config_numeric_map_keys.rs
 M examples/config_policy_restore.rs
 M examples/config_source_resource_probe.rs
 M examples/config_structured.rs
 M src/config.rs
 M src/config/access.rs
 M src/config/internal/config_wire.rs
 M src/config/internal/config_wire_seed.rs
 M src/config/internal/config_wire_seed/accounting_unique_json_value_seed.rs
 M src/config/internal/config_wire_seed/typed_float_visitor.rs
 M src/config/internal/wire_diagnostics.rs
 M src/config/mutation.rs
 M src/config/read.rs
 M src/config/traversal.rs
 M src/config_wire_decode_error.rs
 M src/config_wire_encode_error.rs
 M src/config_wire_limits.rs
 M src/config_wire_limits_builder.rs
 M src/conversion/config_parse_context.rs
 M src/conversion/from_config.rs
 M src/conversion/helpers.rs
 M src/conversion/internal/prepared_scalar.rs
 M src/conversion/structured_read.rs
 M src/conversion/structured_read/input_budget.rs
 M src/conversion/structured_read/internal/index_capacity.rs
 M src/conversion/structured_read/internal/merge_entries.rs
 M src/conversion/structured_read/interpolation_overlay.rs
 M src/conversion/structured_read/path.rs
 M src/conversion/structured_read/preparation_tests.rs
 M src/conversion/structured_read/prepared_config_read.rs
 M src/conversion/structured_read/read_node.rs
 M src/conversion/structured_read/scalar_seq_access.rs
 M src/conversion/structured_read/source_deserializer.rs
 M src/conversion/structured_read/source_enum_access.rs
 M src/conversion/structured_read/source_map_access.rs
 M src/conversion/structured_read/source_seq_access.rs
 M src/conversion/structured_read/traversal.rs
 M src/error/config_deserialize_error.rs
 M src/error/config_error.rs
 M src/property/property.rs
 M src/property/property_mut.rs
 M src/reader/config_reader.rs
 M src/reader/config_section.rs
 M src/reader/read_policy.rs
 M src/reader/read_policy_builder.rs
 M src/source/env_config_source.rs
 M src/source/env_config_source/env_key_tests.rs
 M src/source/env_config_source/permutation_tests.rs
 M src/source/env_file_config_source.rs
 M src/source/internal/cumulative_dimension.rs
 M src/source/internal/dotenv_line_state.rs
 M src/source/internal/source_load_budget.rs
 M src/source/internal/toml_parser_depth.rs
 M src/source/internal/toml_parser_guard.rs
 M src/source/internal/yaml_alias_guard.rs
 M src/source/properties_config_source.rs
 M src/source/source_input.rs
 M src/source/source_load_session.rs
 M src/source/toml_config_source.rs
 M src/source/yaml_config_source.rs
 M src/utils.rs
 M src/utils/internal/interpolation_session.rs
 M src/utils/interpolation.rs
 M tests/config/config_collection_tests.rs
 M tests/config/config_navigation_tests.rs
 M tests/config/config_serialization_tests.rs
 M tests/config/config_tests.rs
 M tests/config/config_wire_tests.rs
 M tests/config_wire_encode_error_tests.rs
 M tests/config_wire_limits_tests.rs
 M tests/constants_tests.rs
 M tests/conversion/config_serde_ext_tests.rs
 M tests/conversion/config_value_deserializer_tests.rs
 M tests/conversion/from/config_parse_context_tests.rs
 M tests/conversion/from/from_config_tests.rs
 M tests/conversion/from/helpers_tests.rs
 M tests/conversion/from/into_config_default_tests.rs
 M tests/conversion/from/raw_read_allocation_tests.rs
 M tests/downstream_contract_tests.rs
 M tests/error/config_deserialize_error_tests.rs
 M tests/error/config_error_kind_tests.rs
 M tests/error/config_error_tests.rs
 M tests/error/error_tests.rs
 M tests/fuzz_corpus_tests.rs
 M tests/key/config_name_tests.rs
 M tests/key/config_names_tests.rs
 M tests/key/config_path_tests.rs
 M tests/property/config_property_mut_tests.rs
 M tests/property/property_tests.rs
 M tests/reader/config_reader_tests.rs
 M tests/reader/config_section_tests.rs
 M tests/reader/explicit_interpolation_tests.rs
 M tests/reader/read_policy_tests.rs
 M tests/source/composite_config_source_tests.rs
 M tests/source/config_source_tests.rs
 M tests/source/env_config_source_tests.rs
 M tests/source/env_file_config_source_tests.rs
 M tests/source/properties_config_source_tests.rs
 M tests/source/source_input_tests.rs
 M tests/source/source_limits_tests.rs
 M tests/source/source_load_session_tests.rs
 M tests/source/toml_config_source_tests.rs
 M tests/source/yaml_config_source_tests.rs
 M tests/utils/deserialize_tests.rs
 M tests/utils/interpolation_tests.rs
 M tests/utils/structured_serde_tests.rs
?? doc/design-20261102T184725405Z-cd786f06.md
?? doc/design-20261104T022605665Z-8713e10e.md
?? doc/implementation-plan-20261102T184725405Z-cd786f06.md
?? doc/implementation-plan-20261104T022605665Z-8713e10e.md
?? rs-config
```

### rs-ioc

- Start branch: `dev-starfish`
- Start HEAD: `c4e2557fb8db37635b56f8cf94c482cac4aa8ba6`
- Pre-existing status entries: 4

```text
 M tests/fixtures/ioc_bench/Cargo.lock
 M tests/fixtures/public_definition_consumer/Cargo.lock
?? doc/design-20261103T014723849Z-b3d6b659.md
?? doc/implementation-plan-20261103T014723849Z-b3d6b659.md
```

### rs-websocket

- Start branch: `dev-starfish`
- Start HEAD: `c0e47fb`
- Pre-existing status entries: 4

```text
?? doc/design-20261103T090256915Z-003bf730.md
?? doc/design-20261104T022555308Z-2d595796.md
?? doc/implementation-plan-20261103T090256915Z-003bf730.md
?? doc/implementation-plan-20261104T022555308Z-2d595796.md
```

The initial `rs-config` diff summary was 114 tracked files changed (4,106 insertions, 1,209 deletions), plus five untracked paths including both timestamped design/plan pairs and the untracked `rs-config` path. These existing changes remained outside task commits. Relevant sibling path dependencies are `../rs-redact` and `../rs-value`; isolated worktrees used temporary symlinks to the existing repositories and those links were removed after validation. Tool versions: Python 3.12.3, Cargo 1.94.0, rustc 1.94.0.

## T1 and T2 implementation and checks

- T1 commit: `a005a37 fix(config): constrain packaged documentation`. The crate package explicitly includes six current docs; checker tests retain six fuzz corpus requirements, require the six docs, reject other `doc/` paths and verify real `cargo package --list --allow-dirty` output. Historical references in the two current design docs now link to repository files. Python checker tests (5) and package checker passed; package listing contained exactly the allowed docs and six corpus files and excluded the timestamped task documents.
- T2 commit: `eaf7c40 test(config): cover downstream reader contracts`. Added IoC-style structured read/default/error-source assertions, observable unknown-field and closed-struct behavior, WebSocket-style section/key/optional-size ordering and `usize` boundary coverage. `cargo test --locked --test downstream_contract_tests` and the all-features equivalent each passed 11 tests on x86_64.

## T4 integrated verification

- `./.infra/bin/align-ci.sh` first exited 1 because the new probe lacked the required Apache license line in the seven-line header. The required line was added; rerun exited 0.
- `python3 -B scripts/check_doc_examples_tests.py` was run RED after updating the regression: 1 expected failure identified the missing non-documented probe exemption. The checker allowlist was extended for `config_composite_resource_probe.rs`; rerun passed 9 tests and `python3 -B scripts/check-doc-examples.py` passed.
- `./.infra/project-ci-check.local.sh` exited 0: coverage helper tests 18, coverage pipeline tests 11, doc example tests 9, JSON fixture suites and package checker all passed.
- First `./.infra/bin/ci-check.sh` exited 1 at the project hook with `examples/config_composite_resource_probe.rs: no document marker for example ID 'config_composite_resource_probe'`. All preceding suites, including fuzz, had passed. After the allowlist fix, the full captured rerun exited 0. It passed build, test, rustdoc, README, all 11 feature-matrix configurations, all 8 fuzz targets, package verification for 1 workspace package, coverage thresholds, security audit, and the project hook. `coverage-cfg-clippy` was not configured and the runner skipped it. Coverage: functions 99.45% (901/906), lines 97.30% (6061/6229), regions 95.92% (9098/9485); configured thresholds were satisfied. The separate `scripts/check-project-coverage.sh` was not rerun because this same CI run executed the coverage collector and threshold check.
- 32-bit check: `rustup target add i686-unknown-linux-gnu` exited 0. `cargo test --locked --target i686-unknown-linux-gnu --test downstream_contract_tests` could not build the existing `alloca` C dependency because the host lacks 32-bit libc development headers (`bits/libc-header-start.h`); no 32-bit tests ran. Per the plan, this is recorded as unavailable link environment; no system package/admin change was made.

## Task commits

- `a005a37 fix(config): constrain packaged documentation` — T1.
- `eaf7c40 test(config): cover downstream reader contracts` — T2.
- `bdb9e23 perf(config): add composite resource probe` — T3 on the integration branch.
- `f3c3f11 fix(config): exempt resource probes from guide registry` — project-hook repair and required style alignment.

Captured raw CI and probe logs were temporary execution artifacts; their complete outcomes are summarized above and the logs are removed during final cleanup.
