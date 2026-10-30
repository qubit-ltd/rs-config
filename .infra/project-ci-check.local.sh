#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$project_root"

python3 -B scripts/check_coverage_files_tests.py
python3 -B scripts/coverage_pipeline_tests.py
python3 -B scripts/check-doc-examples.py
python3 -B scripts/check_doc_examples_tests.py
cargo run --example config_policy_restore
cargo run --example config_numeric_map_keys
cargo run --example config_guide_sources
python3 -B scripts/check_package_resources_tests.py
python3 -B scripts/check-package-resources.py
cargo test --manifest-path tests/fixtures/arbitrary_precision/Cargo.toml --target-dir target/arbitrary-precision
cargo test --manifest-path tests/fixtures/standard_json/Cargo.toml --target-dir target/standard-json
