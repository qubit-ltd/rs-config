# Real downstream adapter contracts

This independent Cargo workspace compiles the public adapter APIs in fixed rs-http, rs-ioc, and rs-websocket source revisions against the current rs-config checkout. It covers `HttpClientOptions::from_config`, `qubit_ioc::config::deserialize_properties`, and WebSocket section mapping and build validation through `WsClientBuilder::from_config`; tests only read configuration and never start a service or make a network request.

From the rs-config repository root, run both the minimal/default configuration and the `full` feature configuration with:

```sh
python3 -B scripts/check-downstream-contracts.py
```

The script prepares detached worktrees from the canonical local source repositories under an automatically managed system temporary directory, copies the current rs-config working tree there, checks out every repository at the full SHA in `downstream-revisions.json`, checks the Cargo metadata graph and tungstenite patch source, and runs the fixture tests twice. Temporary worktrees are removed on success and failure. Failures during setup, metadata validation, or tests produce a nonzero exit status.

To intentionally refresh the fixture lockfile after changing fixed revisions, use:

```sh
python3 -B scripts/check-downstream-contracts.py --update-lock
```

Review the new full revisions and fixture `Cargo.lock` together, then run the normal command above. Do not use floating branches, omit `--locked` from routine Cargo invocations, or accept a lockfile containing a registry copy of `qubit-config`. Cargo metadata must show exactly one `qubit-config`, with all three real downstream packages resolving to the prepared rs-config path; rs-ioc and rs-websocket must have `config` enabled, and tungstenite must resolve from the pinned git revision. The minimal run verifies the default no-optional-feature contract, while the second run explicitly forwards and checks `qubit-config/full`.

These tests cover focused configuration adapters. They do not replace rs-http/rs-ioc repository CI, rs-config's complete feature matrix, platform-specific tests, or the main crate's CI gates.
