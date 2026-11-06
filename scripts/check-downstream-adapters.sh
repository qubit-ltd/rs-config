#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
fixture_manifest="$repo_root/tests/fixtures/downstream_adapters/Cargo.toml"
revision_file="$repo_root/scripts/downstream-adapters-revisions.tsv"
[[ -f "$fixture_manifest" ]] || { echo "error: missing downstream adapter fixture: $fixture_manifest" >&2; exit 1; }
[[ -f "$revision_file" ]] || { echo "error: missing pinned revision list: $revision_file" >&2; exit 1; }

while IFS=$'\t' read -r sibling revision repository; do
    [[ -z "$sibling" || "$sibling" == \#* ]] && continue
    [[ "$sibling" =~ ^rs-[a-z0-9-]+$ && "$revision" =~ ^[0-9a-f]{40}$ && "$repository" == https://github.com/qubit-ltd/*.git ]] || {
        echo "error: malformed pinned dependency entry: $sibling $revision $repository" >&2; exit 1;
    }
    dependency="$repo_root/../$sibling"
    [[ -d "$dependency/.git" || -f "$dependency/.git" ]] || {
        echo "error: required sibling repository is missing: $dependency (expected $revision from $repository)" >&2; exit 1;
    }
    actual=$(git -C "$dependency" rev-parse HEAD 2>/dev/null || true)
    [[ "$actual" == "$revision" ]] || {
        echo "error: $sibling revision mismatch: expected $revision, found ${actual:-unavailable}" >&2; exit 1;
    }
    dependency_status=$(git -C "$dependency" status --porcelain=v1 --untracked-files=all --ignore-submodules=none) || {
        echo "error: cannot inspect worktree for $sibling" >&2
        exit 1
    }
    [[ -z "$dependency_status" ]] || {
        echo "error: $sibling worktree differs from pinned revision; preserve local changes and prepare clean pinned dependencies before retrying" >&2
        exit 1
    }
done < "$revision_file"

metadata_file=$(mktemp)
trap 'rm -f "$metadata_file"' EXIT

run_mode() {
    local mode=$1
    shift
    local -a features=("$@")
    cargo metadata --locked --format-version 1 --manifest-path "$fixture_manifest" --no-default-features "${features[@]}" > "$metadata_file"
    python3 "$repo_root/scripts/check-config-features.py" \
        --manifest-path "$repo_root/Cargo.toml" --mode "$mode" < "$metadata_file"
    cargo tree --locked --manifest-path "$fixture_manifest" --no-default-features "${features[@]}" -d
    cargo tree --locked --manifest-path "$fixture_manifest" --no-default-features "${features[@]}" -i qubit-config
    cargo tree --locked --manifest-path "$fixture_manifest" --no-default-features "${features[@]}" -e features
    cargo test --locked --manifest-path "$fixture_manifest" --no-default-features "${features[@]}" --target-dir "$repo_root/target/downstream-adapters"
}

run_mode minimal
run_mode full --features full
