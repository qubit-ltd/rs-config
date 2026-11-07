# Implementation results: 2026-11-07

Design: `doc/design-20261107T153856264Z-e7fdb431.md`
Plan: `doc/implementation-plan-20261107T153856264Z-e7fdb431.md`

## RSC-401: Persistent documentation consumer dependency baseline

- Added an isolated Cargo consumer fixture with a committed lockfile and a local patch to this checkout.
- Updated the checker to require the fixture, validate its direct dependencies against the English and Chinese documentation manifests, copy identical lock bytes into both temporary consumers, run each with `cargo run --locked`, and reject lockfile drift. Lockfile maintenance remains an explicit `cargo +1.94.0 generate-lockfile --manifest-path tests/fixtures/doc_consumer/Cargo.toml` operation followed by the documented regression checks.
- Updated the Python regression suite to cover pinned-lock behavior and failure cases.
- Commits: `f2ccd47` (`fix(rs-config): pin documentation consumer dependencies`) and `7b39b18` (`fix(rs-config): pin doc fixture to locked adapter versions`).
- The first lock baseline reflected the pre-existing dirty sibling `rs-redact` 0.10.1 checkout. The clean TSV-pinned validation environment uses `rs-redact` 0.10.0, so normal locked resolution rejected that baseline. Regenerated the fixture lock in the pinned environment; it now fixes local `qubit-redact` at 0.10.0 and the registry copy required by the `qubit-value` dependency at 0.10.1. No sibling checkout was modified.
- Validation: `python3 -B scripts/check_doc_consumer_tests.py` — exit 0, 13 tests; `python3 -B scripts/check-doc-consumer.py` — exit 0, English and Simplified Chinese consumers compiled and ran in the clean pinned environment. Both temporary lockfiles and the fixture remained byte-identical during each run.
- The standalone `cargo package --list --locked --manifest-path Cargo.toml` attempt in the original dirty-sibling layout could not reconcile the adjacent path dependency. The complete CI `cargo package --list` and package verification passed in the pinned validation environment.

## RSC-402: ConfigReader rustdoc

- Removed repeated type-parameter sections from `get`, `get_strict`, `get_list`, `get_list_strict`, `get_optional`, `get_any`, `get_optional_any`, and `get_any_or`.
- Kept conversion language for ordinary `FromConfig` reads; strict reads state that they require exact stored types without cross-type conversion. List docs identify `T` as the element type and state the `Vec<T>` result.
- Repository style and strict rustdoc/doctest suites passed. The initial style run rejected eight missing or incorrectly cased sections; the affected methods retain one correctly titled `# Type Parameters` section each with accurate semantics.

## RSC-403: Windows CI coverage

- The project reusable workflow at `.github/workflows/ci.yml` delegates to `qubit-ltd/rs-infra-ci/.github/workflows/github-ci.yml@main`. The resolved shared workflow commit was `2c965cad450e9f0e4d01cf1e5985e6d5e11e75b4`; at that revision `verify-os` defaults to `ubuntu-latest`.
- Recent runs `42357451473` and `42344115733` each show only `rust-ci / verify (ubuntu-latest)` and no Windows jobs. No project input enabled Windows execution, confirming missing platform coverage.
- Added a separate `windows-latest` job with read-only contents permission, a 30-minute timeout, Rust 1.94.0, and `rs-value` pinned to `0ae2aecdc02df7940be1d7101e112f9edb004376`. It verifies that the checked-out path dependency version matches the root lockfile, then runs `cargo +1.94.0 test --locked --lib source::env_config_source` and `cargo +1.94.0 test --locked --test env_config_source_tests`.
- `actionlint` v1.7.12 was initially absent; downloaded the official Linux archive to `/tmp`, verified its published checksum, and `/tmp/rs-config-actionlint/actionlint .github/workflows/ci.yml` exited 0.
- Actual Windows runner evidence is pending the authorized push and resulting GitHub Actions run. This item remains incomplete until the job reports successful nonzero test execution.

## Integration verification

The first `.infra/bin/ci-check.sh` run exited 1 after passing style, build, tests/doctests, strict docs, README versions, feature checks, package/resource checks and fuzz smoke. The failure was the locked arbitrary-precision fixture resolving the adjacent pre-existing dirty `rs-redact` 0.10.1 instead of the TSV-pinned 0.10.0. Root `Cargo.lock` and `fuzz/Cargo.lock` changes from local-path preparation were restored. The fixture lock correction described under RSC-401 was then made and committed.

Using a clean isolated validation layout with sibling revisions from `scripts/downstream-adapters-revisions.tsv`, the following validations passed:

- `./.infra/bin/ci-check.sh` — exit 0. Style, build, full tests, 66 doctests, strict rustdoc, README/version checks, feature matrix, package contents and verification, 8-target fuzz smoke, coverage thresholds, security audit, and local project checks completed. Coverage: functions 99.46% (914/919), lines 97.33% (6264/6436), regions 96.16% (9462/9840).
- `scripts/check-downstream-adapters.sh` — exit 0; 8 adapter contract tests passed.
- `./scripts/check-project-coverage.sh` — exit 0; 99.35% functions (913/919), 97.31% lines (6263/6436), 96.16% regions (9462/9840); metric-specific thresholds satisfied.
- `actionlint .github/workflows/ci.yml` — exit 0.
- `git diff --check` — exit 0 before commits.

The first root `dev-starfish` worktree began at `9ec872c` with extensive pre-existing changes and both plan documents untracked; they were left untouched. The existing `style-source` worktree was detached at `9c98306` and was not touched. Original `rs-value` and `rs-redact` sibling worktrees were dirty at `50b9733` and `d3116d1`, with 2 and 308 existing modified/untracked paths. Related repositories also had pre-existing modifications in `rs-command` (4), `rs-http` (2), `rs-ioc` (4), and `rs-web` (2). None of these original worktrees were changed. All validations used a separate Git worktree with the exact revisions in `scripts/downstream-adapters-revisions.tsv`.

## Git integration and remaining evidence

- Implementation commits on `task/rs-config-rsc-401-402-403-20261107`: `f2ccd47`, `17a822c`, `55e009e`, `82f7d24`, and `7b39b18`.
- Remaining: review this result document and final diff; commit the result document; fast-forward integrate to the starting branch; synchronize and push existing `dev`, `main`, and `dev-starfish` branches in the requested order; inspect the resulting native Windows job and confirm both target test counts are nonzero; clean only task-generated temporary files/worktrees; confirm both planning documents remain on disk.
