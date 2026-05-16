#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
env RS_CI_PROJECT_ROOT="$PROJECT_ROOT" "$PROJECT_ROOT/.rs-ci/align-ci.sh" "$@"

if [ "$#" -eq 0 ]; then
    cd "$PROJECT_ROOT"
    echo "==> cargo check --no-default-features"
    cargo check --no-default-features
fi
