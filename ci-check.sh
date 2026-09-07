#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ARTIFACT_CLEANUP_MODE=${RS_CI_ARTIFACT_CLEANUP_MODE:-always}
OWNS_TARGET_DIR=0
if [[ -z "${RS_CI_TARGET_DIR:-}" ]]; then
  CI_TARGET_DIR="$PROJECT_ROOT/target/rs-ci"
  OWNS_TARGET_DIR=1
else
  CI_TARGET_DIR="$RS_CI_TARGET_DIR"
fi

case "$ARTIFACT_CLEANUP_MODE" in
  always | on-success | never)
    ;;
  *)
    echo "error: RS_CI_ARTIFACT_CLEANUP_MODE must be always, on-success, or never" >&2
    exit 1
    ;;
esac

remove_artifact_directory() {
  local directory="$1"

  if [[ -d "$directory" ]]; then
    echo "Cleaning CI build artifacts: $directory"
    if ! command rm -rf -- "$directory"; then
      echo "warning: failed to clean CI build artifacts: $directory" >&2
    fi
  fi
}

cleanup_artifacts() {
  local exit_status=$?

  case "$ARTIFACT_CLEANUP_MODE" in
    always)
      ;;
    on-success)
      if [[ "$exit_status" -ne 0 ]]; then
        return "$exit_status"
      fi
      ;;
    never)
      return "$exit_status"
      ;;
  esac

  if [[ "$OWNS_TARGET_DIR" == "1" ]]; then
    remove_artifact_directory "$CI_TARGET_DIR"
  fi
  remove_artifact_directory "$PROJECT_ROOT/fuzz/target"
  remove_artifact_directory "$PROJECT_ROOT/target/llvm-cov"
  return "$exit_status"
}
trap cleanup_artifacts EXIT

# The shared CI runner normally removes coverage artifacts on exit. Preserve
# them until this project-specific per-file gate has consumed the fresh JSON,
# then apply the caller's requested cleanup policy in this wrapper.
env \
  RS_CI_PROJECT_ROOT="$PROJECT_ROOT" \
  RS_CI_ARTIFACT_CLEANUP_MODE=never \
  "$PROJECT_ROOT/.rs-ci/ci-check.sh" "$@"
python3 "$PROJECT_ROOT/scripts/check-coverage-files.py" \
  "$PROJECT_ROOT/target/llvm-cov/coverage.json"
