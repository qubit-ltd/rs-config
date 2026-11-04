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

The exact driver iterated the following values serially, running each successful point three times and each expected default-budget rejection once; total driver deadline was five minutes:

- `default`: N = 0, 1, 32, 64, 128, 255, 256, 257.
- `raised`, `unbounded`: N = 512, 1024, 2048.
- Both `chain` and `wide` at each value. `raised` sets `max_sources(10000)` on composites; `unbounded` uses `SourceLimits::unbounded()`; default uses `SourceLimits::default()`.

All 76 child invocations completed in 2.863 seconds (40 default, 18 raised, 18 unbounded). The full captured command/phase log was `/tmp/config-composite-resource-probe.log` during execution; the sample-level outcomes and timings are transcribed below. Matrix driver exit status: 0.

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
