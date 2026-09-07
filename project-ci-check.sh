#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cd "$PROJECT_ROOT"

python3 scripts/check-doc-examples.py

cargo check --examples
cargo check --all-features --examples

if [[ "${CHECK_DOWNSTREAM_COMPATIBILITY:-0}" == "1" ]]; then
  DOWNSTREAM_ROOT=${DOWNSTREAM_ROOT:?DOWNSTREAM_ROOT must point to the checkout root}
  downstream_repositories=(
    rs-config
    rs-http
    rs-mime
    rs-magika
    rs-retry
    rs-redact
    rs-fs
    rs-command
    rs-local-files
    rs-spi
    rs-fs-local
    rs-fs-registry
    rs-fs-testkit
    rs-value
  )

  for repository in "${downstream_repositories[@]}"; do
    repository_path="${DOWNSTREAM_ROOT}/${repository}"
    if [[ ! -d "$repository_path" ]]; then
      echo "Missing downstream checkout: ${repository_path}" >&2
      exit 1
    fi
    if [[ ! -f "${repository_path}/Cargo.toml" ]]; then
      echo "Missing downstream manifest: ${repository_path}/Cargo.toml" >&2
      exit 1
    fi
  done

  downstream_consumers=(rs-http rs-mime rs-magika)
  for consumer in "${downstream_consumers[@]}"; do
    consumer_path="${DOWNSTREAM_ROOT}/${consumer}"
    printf '+ (cd %q && cargo metadata --locked --format-version 1)\n' "$consumer_path"
    printf '+ (cd %q && cargo test --locked --all-features --quiet)\n' "$consumer_path"
    if [[ "${DOWNSTREAM_DRY_RUN:-0}" != "1" ]]; then
      (
        cd "$consumer_path"
        cargo metadata --locked --format-version 1 > /dev/null
        cargo test --locked --all-features --quiet
      )
    fi
  done
fi
