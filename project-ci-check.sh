#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cd "$PROJECT_ROOT"

python3 scripts/check-doc-examples.py
cargo check --examples
cargo check --all-features --examples
