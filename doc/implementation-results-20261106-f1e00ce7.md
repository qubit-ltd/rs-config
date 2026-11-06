# Implementation Results: RSC-401, RSC-402, and RSC-403

## Scope

This record summarizes the implementation of all three accepted targets in
`implementation-plan-20261106T145254049Z-f1e00ce7.md`. Validation used Rust
1.94.0 on x86_64 Linux. The original design and implementation-plan files are
kept outside the task commits.

## Git baseline and preservation

The target repository started on `dev-starfish` at
`07322822bd01fb92caa61e0f796b34c2adf20179`. Before implementation it already
had modified `Cargo.lock`, `Cargo.toml`, `fuzz/Cargo.lock`,
`tests/fixtures/arbitrary_precision/Cargo.lock`, and
`tests/fixtures/standard_json/Cargo.lock`; the two input documents and a
self-referential `rs-config` symlink were untracked. They were preserved in the
original working tree and excluded from task commits. The task ran in a separate
Git worktree branched from that baseline.

The downstream baseline used for adapter work was `rs-command`
`f19ca48d114cb3223a079b043e4b49ef7c0b3c5d`, `rs-fs`
`6fc76f0b3dd0d7795d000c11cac749a3791402c2`, `rs-fs-local`
`8998cb0c4d9a314e62e025a3e5adc36bf0bca301`, `rs-fs-registry`
`6f4e8fcb9407152dffa9ff65b32fdf02f46bfc05`, `rs-fs-testkit`
`b9425fec4cd142826f95cede3715db57b82d8b08`, `rs-http`
`a144cfcfd134f04e70596159a2ffd2ddb88ec490`, `rs-ioc`
`c4e2557fb8db37635b56f8cf94c482cac4aa8ba6`, `rs-json`
`b07f7f3a66f1de38c62839988c0afbeee08e4ebd`, `rs-local-files`
`f55f4706d157585e1ef751e8dcb384259244b5cb`, `rs-mime`
`73fe046cc52dc6c67e3a03257314456f2f14e187`, `rs-redact`
`8d437767dcc0dd3085a8aaac6a27a34347c94807`, `rs-value`
`6f5ae6e2acd592dcb07d30b232b84735e1cc592d`, and `rs-websocket`
`1caed5cd57efd7237b14ee65d843f3ce05b1a282`. Pre-existing downstream changes
noted at start were untracked planning documents in `rs-http`, `rs-websocket`,
and `rs-value`; modified lockfiles and untracked planning documents in
`rs-ioc`; and existing infrastructure/test changes in `rs-json`. None of those
repositories' source files were changed by this implementation. `rs-command`
and `rs-http` later advanced in their live working paths during concurrent
workspace activity; the checker was rerun against temporary detached worktrees
at the pinned baseline revisions, leaving those live paths untouched.

## RSC-401: Composite source depth guard

The built-in composite executor now rejects a 257th active child edge. Root
depth is zero, and the fixed guard also applies when caller source-count limits
are raised or unbounded. Existing source-count charging runs first, preserving
its prior error precedence. Tests cover the 255/256/257 boundaries, wide trees,
raised and unbounded limits, and structured error attribution. A regression
assertion also covers the existing source-count precedence.

The supervised probe was built before applying `prlimit`, so the 512 MiB
address-space limit applies to the probe processes and not to Cargo's compiler:

```text
CARGO_BUILD_JOBS=1 cargo build --locked --release --example config_composite_resource_probe
prlimit --core=0 --as=536870912 target/release/examples/config_composite_resource_probe --shape chain --depth 254 --limits default --stack-bytes 2097152
prlimit --core=0 --as=536870912 target/release/examples/config_composite_resource_probe --shape chain --depth 255 --limits default --stack-bytes 2097152
prlimit --core=0 --as=536870912 target/release/examples/config_composite_resource_probe --shape chain --depth 256 --limits raised --stack-bytes 2097152
prlimit --core=0 --as=536870912 target/release/examples/config_composite_resource_probe --shape chain --depth 1024 --limits raised --stack-bytes 2097152
prlimit --core=0 --as=536870912 target/release/examples/config_composite_resource_probe --shape chain --depth 1024 --limits unbounded --stack-bytes 2097152
```

The probe reports `edges=depth+1`. All five commands exited 0. The 255- and
256-edge chains loaded successfully; the raised 257-edge chain returned
`SourceNestingDepth` with observed depth 257; raised and unbounded 1,025-edge
chains returned the same structured error. Every case reported
`constructed`, `loaded`, and `destroyed`. With default limits, 257 edges
returned the existing `SourceCount` error first, then reached `destroyed`.
The repository's isolated matrix additionally exercised debug/release,
2/8 MiB stacks, a 1,025-child wide source, and all three limit profiles; no
cross-platform stack guarantee is inferred from these local runs.

An initial attempt to run `cargo run --release` under `prlimit` failed while
Cargo/rustc tried to create compiler threads within the 512 MiB limit
(exit 134, `WouldBlock`/allocation failures). Building with
`CARGO_BUILD_JOBS=1` outside `prlimit`, then limiting only the probe, resolved
that environment issue.

## RSC-402: Real downstream adapter contracts

Added a locked integration fixture that exercises the public HTTP, MIME, IOC,
and WebSocket adapter entry points. It checks error conversion and source
metadata, MIME candidate precedence and mapping, interpolation/list/fallback
behavior, and IOC/WebSocket defaults and invalid inputs. The checker pins and
verifies 13 sibling repositories by revision; the GitHub Actions workflow runs
the same locked fixture against those revisions. The dependency tree check
confirmed one shared `qubit-config` instance for the adapter fixture.

Validation:

```text
bash -n scripts/check-downstream-adapters.sh                         exit 0
scripts/check-downstream-adapters.sh                                  exit 0; fixture tests 5/5
cargo tree -d                                                        exit 0
cargo tree -i qubit-config                                            exit 0; one instance
```

The workflow definition was validated locally through its checker and fixture;
it was not triggered on GitHub during this implementation.

## RSC-403: Direct properties loading and measurements

The parser now emits each validated assignment to a receiver. The public parse
API still collects and returns its `Vec`, while `PropertiesConfigSource::load`
sets values directly into a temporary `Config` and publishes its layer only
after successful parsing. Added tests cover direct-load allocation behavior,
limits, transactional publication, and successful last-assignment-wins
semantics.

Command:

```text
cargo bench --locked --bench source_parsing_bench -- properties
```

The benchmark used Rust 1.94.0 optimized builds and 100 Criterion samples.
At 1,024 assignments, allocation counts were unchanged because property
storage dominates: repeated keys 4,115 vs. 4,115; distinct keys 6,318 vs.
6,318. Peak live bytes decreased from 121,897 to 55,599 for repeated keys and
from 405,371 to 356,361 for distinct keys. The comparison's legacy path
reconstructs the previous public-parse-plus-collect-plus-`Config::set` flow.

| Assignments | Input | Direct load | Reconstructed legacy flow |
| ---: | --- | ---: | ---: |
| 32 | distinct | 26.049 µs [25.825, 26.338] | 23.187 µs [22.995, 23.424] |
| 32 | repeated | 12.648 µs [12.499, 12.849] | 12.530 µs [12.355, 12.838] |
| 1,024 | distinct | 1.1688 ms [1.1537, 1.1901] | 1.1550 ms [1.1356, 1.1786] |
| 1,024 | repeated | 371.39 µs [365.04, 378.45] | 474.82 µs [451.61, 502.01] |
| 16,384 | distinct | 22.914 ms [22.512, 23.350] | 30.438 ms [28.649, 32.382] |
| 16,384 | repeated | 5.8937 ms [5.8187, 5.9922] | 6.5899 ms [6.5429, 6.6532] |

The small-input measurements are similar, with direct load somewhat slower in
some cases. Larger workloads improved in these local measurements, especially
repeated keys; no fixed percentage claim is made.

## Validation log

| Command | Result |
| --- | --- |
| `./.infra/bin/align-ci.sh` | Passed after adding the required Rust documentation sections, copyright headers, and test naming. Earlier attempts failed on these style requirements and were corrected. |
| `cargo test --locked --test source_load_session_tests --test composite_config_source_tests --test source_limits_tests --test properties_config_source_tests --test properties_load_allocation_tests` | Passed after adding the last-assignment-wins assertion; 107 tests total across the five targets. |
| `python3 -B scripts/check-doc-examples.py` | Passed: 12 canonical example IDs in 4 documents. |
| `python3 -B scripts/check-package-resources.py` | Passed: 6 required package resources and 7 required package documents. |
| `cargo fmt --all -- --check` | Failed on existing formatting differences across unrelated files; no unrelated files were reformatted. The repository's required `align-ci.sh` style check passed. |
| First `./.infra/bin/ci-check.sh` | Failed at the locked `arbitrary_precision` fixture because its lockfile was stale for the local path dependencies. Both JSON fixture locks were refreshed offline and their tests passed (13 and 5 tests). |
| Final `./.infra/bin/ci-check.sh` (after the last-assignment-wins test and results record) | Passed (exit 0), including feature matrix, strict Rustdoc, Clippy, fuzz smoke targets, both JSON fixtures (13 and 5 tests), package checks, security scan, and coverage. Coverage thresholds: functions >=95%, lines >90%, regions >85%; measured 99.46%, 97.33%, and 96.15%. |
| GitHub Actions `downstream-adapters` workflow | Not run locally; it is intended to run after branch synchronization. The exact pinned revisions and fixture were checked locally. |
| 32-bit default contracts (`CARGO_TARGET_DIR=target/i686-contracts`, `CARGO_TARGET_I686_UNKNOWN_LINUX_GNU_LINKER=gcc`, `CARGO_TARGET_I686_UNKNOWN_LINUX_GNU_RUSTFLAGS='-C link-arg=-m32' cargo +1.94.0 test --locked --target i686-unknown-linux-gnu --test from_config_tests --test downstream_contract_tests`) | Passed on the installed i686 target: 30 tests. |
| 32-bit all-feature contracts (same environment, plus `--all-features`) | Passed: 33 tests. |
| `scripts/check-project-coverage.sh` | Passed: 99.46% functions, 97.36% lines, 96.19% regions; all configured thresholds satisfied. |

The adapter checker briefly failed when live sibling paths no longer matched
the pinned `rs-command` and `rs-http` commits. Both expected commits were
already present locally. The checker then passed with temporary detached
worktrees at those exact commits; those worktrees were removed and the task's
original sibling aliases restored. The original sibling working trees were not
modified.

The CI configuration reported `coverage-cfg-clippy: not configured; skipping`;
the repository has no configured job for that optional mode. Ordinary Clippy
completed successfully as part of `ci-check.sh`.

The CI failure required task-local updates to `Cargo.lock`, `fuzz/Cargo.lock`,
and the two JSON fixture lockfiles. The changes preserve `--locked` checks and
do not weaken validation thresholds.
