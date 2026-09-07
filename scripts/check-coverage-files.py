#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Enforce per-source coverage thresholds from cargo-llvm-cov JSON."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_COVERAGE = ROOT / "target/llvm-cov/coverage.json"
DEFAULT_CONFIG = ROOT / ".rs-ci-coverage.json"
THRESHOLDS = {
    "functions": (95.0, ">="),
    "lines": (90.0, ">"),
    "regions": (85.0, ">"),
}


@dataclass(frozen=True)
class CheckResult:
    checked_files: int
    exempt_files: int
    errors: tuple[str, ...]


class CheckerInputError(ValueError):
    """Raised when policy or coverage evidence cannot be checked safely."""


def load_json(path: Path, description: str) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise CheckerInputError(f"cannot read {description} '{path}': {error}") from error
    except json.JSONDecodeError as error:
        raise CheckerInputError(f"invalid JSON in {description} '{path}': {error}") from error


def configured_exemptions(config: Any) -> set[str]:
    try:
        by_package = config["threshold_exempt_files"]
    except (KeyError, TypeError) as error:
        raise CheckerInputError("coverage policy must contain an object named 'threshold_exempt_files'") from error
    if not isinstance(by_package, dict):
        raise CheckerInputError("coverage policy field 'threshold_exempt_files' must be an object")

    exemptions: set[str] = set()
    for package, paths in by_package.items():
        if not isinstance(package, str) or not isinstance(paths, list) or not all(isinstance(path, str) for path in paths):
            raise CheckerInputError("each threshold exemption package must map to a list of paths")
        exemptions.update(paths)
    return exemptions


def project_source_path(root: Path, filename: Any) -> str | None:
    if not isinstance(filename, str):
        raise CheckerInputError("coverage file entry has a non-string filename")
    path = Path(filename)
    if not path.is_absolute():
        path = root / path
    try:
        relative = path.resolve().relative_to(root.resolve())
    except ValueError:
        return None
    if len(relative.parts) < 2 or relative.parts[0] != "src" or relative.suffix != ".rs":
        return None
    return relative.as_posix()


def reported_source_summaries(root: Path, coverage: Any) -> dict[str, Any]:
    try:
        data = coverage["data"]
    except (KeyError, TypeError) as error:
        raise CheckerInputError("coverage report must contain a list named 'data'") from error
    if not isinstance(data, list):
        raise CheckerInputError("coverage report field 'data' must be a list")

    summaries: dict[str, Any] = {}
    for export in data:
        if not isinstance(export, dict) or not isinstance(export.get("files"), list):
            raise CheckerInputError("each coverage data entry must contain a list named 'files'")
        for file_entry in export["files"]:
            if not isinstance(file_entry, dict):
                raise CheckerInputError("coverage files must be objects")
            relative = project_source_path(root, file_entry.get("filename"))
            if relative is None:
                continue
            if relative in summaries:
                raise CheckerInputError(f"coverage report contains duplicate source file '{relative}'")
            summaries[relative] = file_entry.get("summary")
    if not summaries:
        raise CheckerInputError("coverage report contains no Rust source files beneath 'src/'")
    return summaries


def metric(summary: Any, name: str, source: str) -> tuple[float, int, int]:
    try:
        values = summary[name]
        percent = float(values["percent"])
        covered = int(values["covered"])
        count = int(values["count"])
    except (KeyError, TypeError, ValueError) as error:
        raise CheckerInputError(f"{source}: missing or invalid {name} coverage summary") from error
    return percent, covered, count


def failing_metric(summary: Any, name: str, source: str) -> str | None:
    percent, covered, count = metric(summary, name, source)
    threshold, operator = THRESHOLDS[name]
    passed = percent >= threshold if operator == ">=" else percent > threshold
    if passed:
        return None
    return f"{name}={percent:.2f}% ({covered}/{count}; requires {operator}{threshold:.2f}%)"


def check_coverage_files(root: Path, coverage_path: Path, config_path: Path) -> CheckResult:
    """Check coverage evidence for production source files."""
    coverage = load_json(coverage_path, "coverage report")
    exemptions = configured_exemptions(load_json(config_path, "coverage policy"))
    summaries = reported_source_summaries(root, coverage)
    errors = [
        f"{source}: configured exemption is absent from the coverage report"
        for source in sorted(exemptions - summaries.keys())
    ]

    checked_files = 0
    for source, summary in sorted(summaries.items()):
        if source in exemptions:
            continue
        checked_files += 1
        failures = [
            failure
            for name in THRESHOLDS
            if (failure := failing_metric(summary, name, source)) is not None
        ]
        if failures:
            errors.append(f"{source}: {', '.join(failures)}")

    return CheckResult(
        checked_files=checked_files,
        exempt_files=len(exemptions & summaries.keys()),
        errors=tuple(errors),
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "coverage_json",
        nargs="?",
        type=Path,
        default=DEFAULT_COVERAGE,
        help="cargo-llvm-cov JSON report (default: target/llvm-cov/coverage.json)",
    )
    parser.add_argument(
        "--config",
        type=Path,
        default=DEFAULT_CONFIG,
        help="coverage exception policy (default: .rs-ci-coverage.json)",
    )
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    try:
        result = check_coverage_files(ROOT, arguments.coverage_json, arguments.config)
    except CheckerInputError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    if result.errors:
        for error in result.errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(
        f"checked {result.checked_files} production coverage files; "
        f"skipped {result.exempt_files} configured instrumentation exceptions"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
