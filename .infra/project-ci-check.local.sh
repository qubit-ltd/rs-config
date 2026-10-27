#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$project_root"

python3 -B scripts/check_coverage_files_tests.py
python3 -B scripts/coverage_pipeline_tests.py
python3 -B scripts/check-doc-examples.py
