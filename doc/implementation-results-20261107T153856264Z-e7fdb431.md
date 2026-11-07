# Implementation results: 2026-11-07

Design: `doc/design-20261107T153856264Z-e7fdb431.md`
Plan: `doc/implementation-plan-20261107T153856264Z-e7fdb431.md`

## RSC-401: Persistent documentation consumer dependency baseline

- Added an isolated Cargo consumer fixture with a committed lockfile and a local patch to this checkout.
- Updated the checker to require the fixture, validate its direct dependencies against the English and Chinese documentation manifests, copy identical lock bytes into both temporary consumers, run each with `cargo run --locked`, and reject lockfile drift. Lockfile maintenance remains an explicit `cargo +1.94.0 generate-lockfile --manifest-path tests/fixtures/doc_consumer/Cargo.toml` operation followed by the documented regression checks.
- Updated the Python regression suite to cover pinned-lock behavior and failure cases.
- Commits: `f2ccd47` (`fix(rs-config): pin documentation consumer dependencies`) and `7b39b18` (`fix(rs-config): pin doc fixture to locked adapter versions`).
- The first lock baseline reflected the pre-existing dirty sibling `rs-redact` 0.10.1 checkout. The clean TSV-pinned validation environment uses `rs-redact` 0.10.0, so normal locked resolution rejected that baseline. Regenerated the fixture lock in the pinned environment; it now fixes local `qubit-redact` at 0.10.0 and the registry copy required by `qubit-value` at 0.10.1. No sibling checkout was modified.
- Validation: `python3 -B scripts/check_doc_consumer_tests.py` — exit 0, 13 tests; `python3 -B scripts/check-doc-consumer.py` — exit 0, English and Simplified Chinese consumers compiled and ran in the clean pinned environment. The fixture lock SHA-256 is `53a50389a1550a791f9d42014543f7d324d9ad7812f4cc8c89251647eea01d3c`.
- The standalone `cargo package --list --locked --manifest-path Cargo.toml` attempt in the original dirty-sibling layout could not reconcile the adjacent path dependency. The complete CI package listing and package verification passed in the pinned validation environment.

## RSC-402: ConfigReader rustdoc

- Removed repeated type-parameter sections from `get`, `get_strict`, `get_list`, `get_list_strict`, `get_optional`, `get_any`, `get_optional_any`, and `get_any_or`.
- Kept conversion language for ordinary `FromConfig` reads; strict reads state that they require exact stored types without cross-type conversion. List docs identify `T` as the element type and state the `Vec<T>` result.
- Repository style and strict rustdoc/doctest suites passed. The initial style run rejected eight missing or incorrectly cased sections; the affected methods retain one correctly titled `# Type Parameters` section each with accurate semantics.

## RSC-403: Windows CI coverage

- The project reusable workflow delegates to `qubit-ltd/rs-infra-ci/.github/workflows/github-ci.yml@main`. The resolved shared workflow commit was `2c965cad450e9f0e4d01cf1e5985e6d5e11e75b4`; at that revision `verify-os` defaults to `ubuntu-latest`.
- Recent runs `42357451473` and `42344115733` showed only `rust-ci / verify (ubuntu-latest)` and no Windows jobs. No project input enabled Windows execution, confirming missing platform coverage.
- Added a separate `windows-latest` job with read-only contents permission, a 30-minute timeout, and Rust 1.94.0. It checks out `rs-value` at `6f5ae6e2acd592dcb07d30b232b84735e1cc592d` and `rs-redact` at `8d437767dcc0dd3085a8aaac6a27a34347c94807`, then verifies both path package versions against `Cargo.lock`. It runs `cargo +1.94.0 test --locked --lib source::env_config_source` and `cargo +1.94.0 test --locked --test env_config_source_tests`.
- `actionlint` v1.7.12 was initially absent; downloaded the official Linux archive to `/tmp`, verified its published checksum, and `/tmp/rs-config-actionlint/actionlint .github/workflows/ci.yml` exited 0.
- Native Windows evidence: [Actions run 42393525980](https://github.com/qubit-ltd/rs-config/actions/runs/42393525980), job “Windows environment source tests” succeeded. The unit command ran 7 tests (7 passed, 0 failed); the public API integration command ran 19 tests (19 passed, 0 failed). This satisfies the RSC-403 Windows execution criterion.
- The first Windows attempt failed because `rs-redact` was not checked out; the next attempt passed the unit tests but exposed a missing `OsStrExt` import in the Windows integration test. The following attempt exposed a wrong Windows-only assertion comparing the returned environment value to the variable key. Added the sibling checkout and pinned versions, imported the required trait, and corrected the assertion. The final native run passed both targets.

## Integration verification

The first `.infra/bin/ci-check.sh` run exited 1 after passing style, build, tests/doctests, strict docs, README versions, feature checks, package/resource checks and fuzz smoke. The failure was the locked arbitrary-precision fixture resolving adjacent pre-existing dirty `rs-redact` 0.10.1 instead of the TSV-pinned 0.10.0. Root `Cargo.lock` and `fuzz/Cargo.lock` changes from local-path preparation were restored. The fixture lock correction described under RSC-401 was then made and committed.

Using a clean isolated validation layout with sibling revisions from `scripts/downstream-adapters-revisions.tsv`, these validations passed:

- `./.infra/bin/ci-check.sh` — exit 0. Style, build, full tests, 66 doctests, strict rustdoc, README/version checks, feature matrix, package contents and verification, 8-target fuzz smoke, coverage thresholds, security audit, and local project checks completed. Coverage: functions 99.46% (914/919), lines 97.33% (6264/6436), regions 96.16% (9462/9840).
- `scripts/check-downstream-adapters.sh` — exit 0; 8 adapter contract tests passed.
- `./scripts/check-project-coverage.sh` — exit 0; functions 99.35% (913/919), lines 97.31% (6263/6436), regions 96.16% (9462/9840); thresholds satisfied.
- `actionlint .github/workflows/ci.yml` — exit 0 after the final workflow edits.
- `git diff --check` — exit 0 before task commits.
- `cargo +1.94.0 test --locked --lib source::env_config_source && cargo +1.94.0 test --locked --test env_config_source_tests` — exit 0 on Linux (8 unit-filtered and 24 integration tests); the native Windows run above covers Windows-only cases.
- `cargo +1.94.0 test --locked --target i686-unknown-linux-gnu --test from_config_tests --test downstream_contract_tests` — exit 0 (19 + 11 tests). The corresponding `--all-features` command exited 0 (22 + 11 tests). Updating the sibling pin to the exact lock-compatible revision made these jobs pass remotely too.
- Supplemental `cargo +1.94.0 check --locked --target x86_64-pc-windows-gnu --test env_config_source_tests` — exit 101 because this environment lacks `x86_64-w64-mingw32-gcc`. Native Windows execution was available and passed, so no cross-linker installation was needed.

The first root `dev-starfish` worktree began at `9ec872c` with extensive pre-existing changes and both plan documents untracked; they were left untouched. The existing `style-source` worktree was detached at `9c98306` and was not touched. Original `rs-value` and `rs-redact` sibling worktrees were dirty at `50b9733` and `d3116d1`, with 2 and 308 existing modified/untracked paths. Related repositories also had pre-existing modifications in `rs-command` (4), `rs-http` (2), `rs-ioc` (4), and `rs-web` (2). None of these original worktrees were changed. All local validations used a separate Git worktree with the exact revisions in the downstream adapter TSV.

## Shared GitHub CI baseline issue

The project Windows and 32-bit jobs pass on the latest code commit, but the reusable shared workflow still fails Linux setup jobs. In run 42393525980, `dependency-baseline` and feature-matrix checks report that `../rs-redact/Cargo.toml` is missing after `Prepare local path dependencies`; the verify job reports the same path while checking `Cargo.lock`. The same dependency-baseline failure was present in earlier runs 42357451473 and 42344115733. Local CI and downstream validation pass when run in the task-created layout containing the exact pinned siblings.

Fixing the reusable-job path setup requires changing project path-dependency preparation and its `.infra/bootstrap-source.json` integrity digest. That manifest already has pre-existing user modifications in the original `dev-starfish` worktree, including a different dependency-tool snapshot, package digest, and files. Those changes have been preserved; no overlapping edit has been staged or committed. A safe resolution needs the user to decide whether to combine those bootstrap changes or leave this pre-existing shared-workflow problem outside this task.

## Git integration and remaining work

- Implementation commits on `task/rs-config-rsc-401-402-403-20261107`: `f2ccd47`, `17a822c`, `55e009e`, `82f7d24`, `7b39b18`, `34773fb`, `82f38db`, `4e66722`, and `2e0f265`.
- The implementation branch was fast-forward integrated into the starting `dev-starfish` branch. The existing `dev`, `main`, and `dev-starfish` branches are synchronized and pushed. The Windows and 32-bit project jobs passed in run 42393525980 with nonzero test counts.
- Run 42393525980 completed with shared reusable-workflow setup failures on the missing `../rs-redact` path. A safe project-level bootstrap fix is blocked by the pre-existing uncommitted `.infra/bootstrap-source.json` changes; user direction is needed before touching that integrity manifest.
