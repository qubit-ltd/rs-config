# RSC-101 parser resource probe report

- Measurement time: 2026-11-01 13:11 UTC (UTC clock; runs completed before report creation).
- Project: `rs-config`, worktree `/data/working/qubit/rust-common/.worktrees/rs-config/codex-rsc-parser-budget-20261101`.
- Source base: `452d3d77e9d4b37666946a9bb9eb31f322b2fd4f` (T5 documentation/fuzz changes uncommitted during measurement).
- Host: Intel Core i5-9600K (6 cores), Linux x86_64, kernel 7.0.0-38-generic; 32 GiB RAM.
- Rust: `rustc 1.94.0 (4a4ef493e 2026-03-02)`, host `x86_64-unknown-linux-gnu`.
- Build: `cargo build --locked --release --example config_source_resource_probe --features formats`; binary SHA-256 `e48d3efe006ea9b435e491a1a91f94a1deb066611d019d3eead504176454c8a6`.
- Process limits: each pressure run was a fresh child under `timeout --signal=TERM --kill-after=2s 15s prlimit --as=1073741824 --cpu=10 -- /usr/bin/time -v`; address space 1 GiB, CPU 10 seconds, wall timeout 15 seconds.
- Input: generated TOML bytes SHA-256 `ab0c674cad8e4f1380eb64c4a4d0aa27728a773fb07d63c3b7db210b4a8aa060` (1,050,007 bytes); YAML bytes SHA-256 `b6395f0c5d2a4eec4bea88bafd10e2572f77ae73495442750bb4f867fe3b9448` (1,050,008 bytes). Both inputs contain 525,000 scalar sequence elements.
- Matrix adjustment: the plan's `SIZE=4194300` exceeds the probe's 4 MiB default input cap because `SIZE` means element count and each element takes two input bytes. The probe would reject generation before parsing. Used 525,000 elements instead: above the default 1,048,576 parser-token budget for both formats while keeping generated input within the source byte cap. Semantic node/depth limits remain the plan's `262144/64` and `16/8` pairs.

## Pressure runs

Every invocation generated the input successfully, returned `SourceLimitExceeded` with `parser token count`, and exited 1 as expected. GNU time reported child exit 1 and no delivered signals. No timeout, signal termination, or OOM occurred. Peak RSS was below the 512 MiB investigation threshold in every run.

| Format | nodes/depth | Repeat | Outer exit | Child exit | Wall (GNU time) | User s | System s | Peak RSS kB | Signals | Error dimension |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| toml | 262144/64 | 1 | 1 (expected) | 1 | 0:00.02 | 0.00 | 0.01 | 34156 | 0 | parser token count |
| toml | 262144/64 | 2 | 1 (expected) | 1 | 0:00.02 | 0.01 | 0.00 | 34156 | 0 | parser token count |
| toml | 262144/64 | 3 | 1 (expected) | 1 | 0:00.02 | 0.00 | 0.01 | 34148 | 0 | parser token count |
| toml | 16/8 | 1 | 1 (expected) | 1 | 0:00.02 | 0.01 | 0.01 | 34120 | 0 | parser token count |
| toml | 16/8 | 2 | 1 (expected) | 1 | 0:00.02 | 0.01 | 0.00 | 34116 | 0 | parser token count |
| toml | 16/8 | 3 | 1 (expected) | 1 | 0:00.02 | 0.01 | 0.01 | 34212 | 0 | parser token count |
| yaml | 262144/64 | 1 | 1 (expected) | 1 | 0:00.07 | 0.06 | 0.00 | 12404 | 0 | parser token count |
| yaml | 262144/64 | 2 | 1 (expected) | 1 | 0:00.07 | 0.07 | 0.00 | 12380 | 0 | parser token count |
| yaml | 262144/64 | 3 | 1 (expected) | 1 | 0:00.07 | 0.06 | 0.00 | 12208 | 0 | parser token count |
| yaml | 16/8 | 1 | 1 (expected) | 1 | 0:00.07 | 0.07 | 0.00 | 12396 | 0 | parser token count |
| yaml | 16/8 | 2 | 1 (expected) | 1 | 0:00.07 | 0.06 | 0.00 | 12396 | 0 | parser token count |
| yaml | 16/8 | 3 | 1 (expected) | 1 | 0:00.07 | 0.06 | 0.00 | 12352 | 0 | parser token count |

## Controls

| Run | Child exit | Result | Peak RSS kB |
|---|---:|---|---:|
| `toml-small` | 0 | `result=ok` | 2840 |
| `yaml-small` | 0 | `result=ok` | 2968 |
| `toml-generate` | 0 | `result=generated-only` | 12220 |
| `yaml-generate` | 0 | `result=generated-only` | 12396 |

Small 1,024-element TOML and YAML inputs loaded successfully. Both 525,000-element generate-only controls completed, confirming generation is inside the allowed input size and the pressure failures occur during load admission.

## Captured application output

### toml-262144-64-r1

```text
format=toml shape=wide size=525000 input_bytes=1050007
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=16473706
```

GNU time: elapsed `0:00.02`, user `0.00`, system `0.01`, peak RSS `34156 kB`, signals `0`, child exit `1`.

### toml-262144-64-r2

```text
format=toml shape=wide size=525000 input_bytes=1050007
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=16498728
```

GNU time: elapsed `0:00.02`, user `0.01`, system `0.00`, peak RSS `34156 kB`, signals `0`, child exit `1`.

### toml-262144-64-r3

```text
format=toml shape=wide size=525000 input_bytes=1050007
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=16505651
```

GNU time: elapsed `0:00.02`, user `0.00`, system `0.01`, peak RSS `34148 kB`, signals `0`, child exit `1`.

### toml-16-8-r1

```text
format=toml shape=wide size=525000 input_bytes=1050007
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=16490893
```

GNU time: elapsed `0:00.02`, user `0.01`, system `0.01`, peak RSS `34120 kB`, signals `0`, child exit `1`.

### toml-16-8-r2

```text
format=toml shape=wide size=525000 input_bytes=1050007
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=16682975
```

GNU time: elapsed `0:00.02`, user `0.01`, system `0.00`, peak RSS `34116 kB`, signals `0`, child exit `1`.

### toml-16-8-r3

```text
format=toml shape=wide size=525000 input_bytes=1050007
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=16752301
```

GNU time: elapsed `0:00.02`, user `0.01`, system `0.01`, peak RSS `34212 kB`, signals `0`, child exit `1`.

### yaml-262144-64-r1

```text
format=yaml shape=wide size=525000 input_bytes=1050008
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=66483696
```

GNU time: elapsed `0:00.07`, user `0.06`, system `0.00`, peak RSS `12404 kB`, signals `0`, child exit `1`.

### yaml-262144-64-r2

```text
format=yaml shape=wide size=525000 input_bytes=1050008
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=66742928
```

GNU time: elapsed `0:00.07`, user `0.07`, system `0.00`, peak RSS `12380 kB`, signals `0`, child exit `1`.

### yaml-262144-64-r3

```text
format=yaml shape=wide size=525000 input_bytes=1050008
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=66926940
```

GNU time: elapsed `0:00.07`, user `0.06`, system `0.00`, peak RSS `12208 kB`, signals `0`, child exit `1`.

### yaml-16-8-r1

```text
format=yaml shape=wide size=525000 input_bytes=1050008
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=69169512
```

GNU time: elapsed `0:00.07`, user `0.07`, system `0.00`, peak RSS `12396 kB`, signals `0`, child exit `1`.

### yaml-16-8-r2

```text
format=yaml shape=wide size=525000 input_bytes=1050008
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=65236715
```

GNU time: elapsed `0:00.07`, user `0.06`, system `0.00`, peak RSS `12396 kB`, signals `0`, child exit `1`.

### yaml-16-8-r3

```text
format=yaml shape=wide size=525000 input_bytes=1050008
generated=complete
result=error error_kind=SourceLimitExceeded source_limit_kind=parser token count
load_elapsed_ns=66206584
```

GNU time: elapsed `0:00.07`, user `0.06`, system `0.00`, peak RSS `12352 kB`, signals `0`, child exit `1`.


The full GNU `time -v` stderr was captured during execution under `/tmp/rs-config-resource-probe-20261101/` and summarized above; those temporary capture files are removed after validation. The report preserves the application output and relevant per-run process metrics.

## Structured-read benchmark

`cargo bench --bench structured_read_bench -- deep_key_rejection` was run in optimized mode on the same host/toolchain, first at base `85d5e901c9787dd85ef9eb4aed9d71cfd38052e6` with only the benchmark source copied in, then at the implementation commit `452d3d77e9d4b37666946a9bb9eb31f322b2fd4f`. Both runs used Criterion defaults (100 samples, 3-second warmup), same allocator/build profile. The benchmark verifies the rejection result outside the timed loop; latency is descriptive and not a correctness threshold.

| Dotted segments | Baseline median | Updated median | Change |
|---:|---:|---:|---:|
| 64 | 7.1881 µs | 1.7945 µs | 4.0× lower |
| 256 | 34.292 µs | 9.5248 µs | 3.6× lower |
| 1,024 | 168.81 µs | 28.778 µs | 5.9× lower |
| 4,096 | 944.99 µs | 64.335 µs | 14.7× lower |

## Limits of this probe

The probe verifies public source loading stops on parser-token admission under isolated process limits. It does not measure hostile single-token scanner behavior, YAML's one-token lookahead, or the complete AST memory cost for accepted inputs. Those remain explicit resource boundaries.
