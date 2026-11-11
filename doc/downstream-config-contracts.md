# Downstream configuration contract coverage

Checked on 2026-11-11 from rs-config worktree `cccfc5eaf34f8778a5621d66317b9f4fcd7a0bc4`.

## Existing CI evidence

| Source | Revision / trigger | Coverage and dependency source | Result against the equivalence criteria |
| --- | --- | --- | --- |
| `.github/workflows/ci.yml` | rs-config workflow at the checked-out revision; push to `main`, pull request, or manual dispatch | Calls `qubit-ltd/rs-infra-ci/.github/workflows/github-ci.yml@main`; also checks out rs-redact at `a9b9d767d33b80541a743e19ce1e39afb3c0c29f` for 32-bit tests. The local workflow's `config-32bit` job runs rs-config's `from_config_tests` and `downstream_contract_tests`. | Does not compile or call real rs-http or rs-ioc adapters. Its 32-bit local path setup does not add real downstream API tests. |
| `.infra/project-ci-check.local.sh` | Invoked by the shared project's `project-hook` task | Runs project-local feature/doc/example/package fixtures and checks. It does not mention rs-http or rs-ioc. | No real downstream adapters. |
| `.infra/ci/ci.toml` | Tasks include `project-hook`, `verify`, `feature-matrix`, `coverage`, and other project gates | Selects `.infra/bin/project-ci-check.sh` as the project hook. | No real downstream adapters. |
| `tests/downstream_contract_tests.rs` | Built by existing Rust CI and the 32-bit job | Exercises `qubit_config::Config`/`ConfigSection` reader behavior and local stand-in IOC structs. `IocServerConfig` and `IocConfigError` are defined in this file; the tests do not link either real downstream crate. | Useful reader coverage, but not an adapter contract. |
| `rs-ioc/.github/workflows/downstream-contracts.yml` | Locally accessible checkout `rs-ioc` at `49dccd37a58e322c1a430be37b470afe88b6ffaa`; push to main, pull request, or manual dispatch | Its baseline and current consumer jobs check out `qubit-ltd/rs-config` at fixed SHAs (`98efea94b2d5b83c9f26f230d75e6cde202e1931` and `2958c1d1f1f3e1ac257a26bbad10be740b373fd4`) and run rs-ioc/execution-services consumer fixtures. The current job uses real qubit-ioc, but does not use the current rs-config checkout or test `deserialize_properties` behavior as a focused contract; it does not check out/test rs-http or rs-websocket. | Partial adjacent coverage only; does not meet the full RSC-003 equivalence criteria. |
| `rs-http/.github/workflows/ci.yml` | Locally accessible checkout `rs-http` at `1a8a2de33c8adbcc4bb7209823d870e596c08c7e`; push to main, pull request, or manual dispatch | Calls the shared generic CI workflow and adds only `tests/check_guide_examples.py`. No rs-ioc or rs-websocket checkout or explicit real config adapter contract against the current rs-config source. | No equivalent RSC-003 task. |
| `rs-infra-ci/.github/workflows/github-ci.yml` | Shared CI revision fetched from the configured `@main`: `2c965cad450e9f0e4d01cf1e5985e6d5e11e75b4`; called from rs-config workflow | Generic workflow-call jobs for verify, feature matrix, project hook and additional configured gates. No rs-http/rs-ioc checkout or config adapter contract job. | No equivalent task in the inspected shared workflow. |

The rs-http, rs-ioc, and rs-websocket local repositories are independently inspected for their manifests and public adapter entry points. The referenced adapter implementations are `HttpClientOptions::from_config` in `rs-http/src/options/http_client_options.rs`, `qubit_ioc::config::deserialize_properties` in `rs-ioc/src/config.rs`, and `WsClientBuilder::from_config` in `rs-websocket/src/client/ws_client_builder/config.rs`. Their manifests use sibling path dependencies to `../rs-config`; merely patching a crates.io package would not replace those dependencies.

The rs-ioc repository workflow and shared CI repository were accessible from this environment. This check does not claim visibility into private or otherwise inaccessible organization-wide jobs; those are unknown. The inspected public/local jobs above do not provide equivalent coverage, so this project adds an independent fixture and workflow that runs all three real adapters against the current rs-config source and fails on setup, metadata, or test errors.

## Fixed inputs for the independent fixture

| Repository | Commit | Validation |
| --- | --- | --- |
| rs-http | `1a8a2de33c8adbcc4bb7209823d870e596c08c7e` | Fetched from `https://github.com/qubit-ltd/rs-http.git`; manifest is `qubit-http` 0.16.0 and exposes `HttpClientOptions::from_config`. |
| rs-ioc | `49dccd37a58e322c1a430be37b470afe88b6ffaa` | Fetched from `https://github.com/qubit-ltd/rs-ioc.git`; manifest is `qubit-ioc` 0.4.0 with `config` feature and exposes `config::deserialize_properties`. |
| rs-redact | `d3116d19fc40d6e23db0fdca4db3748d0dede7d8` | Fetched from `https://github.com/qubit-ltd/rs-redact.git`; manifest is `qubit-redact` 0.10.1 and satisfies rs-http's sibling path dependency. |
| rs-websocket | `e62a8437eb60fab664f781ef6d344041d713a601` | Canonical URL `https://github.com/qubit-ltd/rs-websocket.git`; `qubit-websocket` 0.3.0 exposes the config feature and public section adapter. |

The original HTTP/IOC/redact revisions above were fetched directly from their canonical HTTPS repository URLs and matched the requested full SHAs. The fixture now prepares all four fixed repositories as detached worktrees from canonical local source repositories beside a copy of the current rs-config working tree, so real manifests' relative paths resolve to the tested source. See `tests/fixtures/downstream_adapters/README.md` for commands and update procedure.

## Implementation verification status

The pre-RSC-002 HTTP/IOC record reports that `python3 -B scripts/check_downstream_contracts_tests.py` passed 11 script unit tests, including path/registry/duplicate-node rejection, rs-ioc `config` feature enforcement, minimal/full feature checks, missing-node/edge rejection, and nonzero subprocess propagation. The generated fixture lockfile had exactly one `qubit-config` package entry, and metadata resolved both real adapter package edges to the prepared current rs-config source.

The initial clean-HEAD attempt exposed pre-existing missing and mismatched module sources. After authorization, the compatible staged and untracked source, test, and example changes present in the original checkout were reviewed and incorporated into the task worktree; the design and implementation-plan documents remained excluded. A required `structured_read/internal.rs` mapping was retained with the rest of that baseline.

The preceding records describe the HTTP/IOC fixture before RSC-002. On 2026-11-11, the fixture was extended with the fixed rs-websocket revision and tungstenite patch. `python3 -B scripts/check_downstream-contracts.py --update-lock` exited 0, refreshed the lockfile, and passed both metadata checks and both real-adapter test runs. The normal locked command `python3 -B scripts/check-downstream-contracts.py` also exited 0 in minimal and `full` modes. The added WebSocket tests cover all seven cases documented in the design; Python metadata unit tests passed all 18 cases. Both script runs removed their four temporary worktrees.

Historical pre-RSC-002 HTTP/IOC-only verification ran these two real adapter test commands. Each ran the same two HTTP/IOC contract tests, covering success values, conversion error categories and paths, unknown-key handling, Serde defaults, and rejection of an unknown IOC field:

```text
cargo test --locked --manifest-path tests/fixtures/downstream_adapters/Cargo.toml
cargo test --locked --manifest-path tests/fixtures/downstream_adapters/Cargo.toml --features full
```
