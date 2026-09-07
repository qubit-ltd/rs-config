#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SHOW_HELP=0
for argument in "$@"; do
  case "$argument" in
    help | --help | -h)
      SHOW_HELP=1
      ;;
  esac
done

env RS_CI_PROJECT_ROOT="$PROJECT_ROOT" "$PROJECT_ROOT/.rs-ci/coverage.sh" "$@"

if [[ "$SHOW_HELP" == "0" ]]; then
  python3 "$PROJECT_ROOT/scripts/check-coverage-files.py" \
    "$PROJECT_ROOT/target/llvm-cov/coverage.json"
fi
