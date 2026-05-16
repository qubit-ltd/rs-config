#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
env RS_CI_PROJECT_ROOT="$PROJECT_ROOT" "$PROJECT_ROOT/.rs-ci/ci-check.sh" "$@"

if [ "$#" -eq 0 ]; then
    cd "$PROJECT_ROOT"

    echo ""
    echo "==> Running minimal feature tests (cargo test --no-default-features)"
    cargo test --no-default-features --verbose

    echo ""
    echo "==> Building minimal feature documentation with warnings denied"
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --no-default-features --verbose
fi
