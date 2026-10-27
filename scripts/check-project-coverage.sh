#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$project_root"

report_path="$project_root/coverage.json"
stale_report_dir=$(mktemp -d "${TMPDIR:-/tmp}/qubit-config-coverage.XXXXXX")
stale_report_saved=false

restore_stale_report() {
    if [[ "$stale_report_saved" == true && ! -e "$report_path" && -f "$stale_report_dir/coverage.json" ]]; then
        mv -- "$stale_report_dir/coverage.json" "$report_path"
    fi
    rm -rf -- "$stale_report_dir"
}
trap restore_stale_report EXIT

if [[ -e "$report_path" ]]; then
    [[ -f "$report_path" ]] || { echo "error: coverage report path is not a regular file: $report_path" >&2; exit 1; }
    mv -- "$report_path" "$stale_report_dir/coverage.json"
    stale_report_saved=true
fi

"$project_root/.infra/bin/coverage.sh" "$@"
[[ -f "$report_path" ]] || { echo "error: coverage command did not create $report_path" >&2; exit 1; }
python3 -B "$project_root/scripts/check-coverage-files.py" "$report_path" \
    --config "$project_root/.infra/coverage/coverage.json"
