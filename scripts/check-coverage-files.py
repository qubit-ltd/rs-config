#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Enforce per-source coverage thresholds from cargo-llvm-cov JSON."""

from __future__ import annotations

import argparse
import json
import math
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_COVERAGE = ROOT / "coverage.json"
DEFAULT_CONFIG = ROOT / ".infra/coverage/coverage.json"
DEFAULT_NON_EXECUTABLE = ROOT / ".infra/coverage/non-executable-files.json"
DEFAULT_METRIC_OVERRIDES = ROOT / "scripts/coverage-metric-overrides.json"
METRICS = ("functions", "lines", "regions")


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


def package_paths(config: Any, field: str) -> dict[str, list[str]]:
    try:
        mapping = config[field]
    except (KeyError, TypeError) as error:
        raise CheckerInputError(f"coverage policy must contain '{field}'") from error
    if not isinstance(mapping, dict):
        raise CheckerInputError(f"coverage policy field '{field}' must be an object")
    result: dict[str, list[str]] = {}
    for package, paths in mapping.items():
        if not isinstance(package, str) or not isinstance(paths, list) or not all(
            isinstance(path, str) for path in paths
        ):
            raise CheckerInputError(f"each '{field}' package must map to a list of paths")
        result[package] = paths
    return result


def policy(
    config: Any, non_executable_config: Any
) -> tuple[dict[str, list[str]], dict[str, set[str]], dict[str, set[str]], dict[str, float]]:
    if not isinstance(config, dict) or config.get("scope") != "package":
        raise CheckerInputError("coverage policy scope must be 'package'")
    source_dirs = package_paths(config, "source_dirs")
    exemptions = package_paths(config, "threshold_exempt_files")
    non_executable = non_executable_config
    if not isinstance(non_executable, dict):
        raise CheckerInputError("coverage policy field 'non_executable_files' must be an object")
    non_executable_paths: dict[str, set[str]] = {}
    for package, entries in non_executable.items():
        if not isinstance(package, str) or not isinstance(entries, list):
            raise CheckerInputError("each 'non_executable_files' package must map to a list")
        paths: set[str] = set()
        for entry in entries:
            if (
                not isinstance(entry, dict)
                or not isinstance(entry.get("path"), str)
                or not isinstance(entry.get("reason"), str)
                or not entry["reason"].strip()
            ):
                raise CheckerInputError("each non-executable file needs a path and evidence reason")
            paths.add(entry["path"])
        non_executable_paths[package] = paths
    thresholds = config.get("thresholds")
    if not isinstance(thresholds, dict):
        raise CheckerInputError("coverage policy field 'thresholds' must be an object")
    parsed_thresholds: dict[str, float] = {}
    for name in METRICS:
        try:
            value = float(thresholds[name])
        except (KeyError, TypeError, ValueError) as error:
            raise CheckerInputError(f"coverage policy has an invalid '{name}' threshold") from error
        if not 0 <= value <= 100:
            raise CheckerInputError(f"coverage policy '{name}' threshold must be between 0 and 100")
        parsed_thresholds[name] = value
    if not source_dirs or not any(source_dirs.values()):
        raise CheckerInputError("coverage policy must select at least one package source directory")
    return (
        source_dirs,
        {package: set(paths) for package, paths in exemptions.items()},
        non_executable_paths,
        parsed_thresholds,
    )


def selected_sources(root: Path, source_dirs: dict[str, list[str]]) -> set[str]:
    sources: set[str] = set()
    resolved_root = root.resolve()
    for package, directories in source_dirs.items():
        if package != "qubit-config":
            raise CheckerInputError(f"unsupported coverage package '{package}'")
        for directory in directories:
            relative_directory = Path(directory)
            if relative_directory.is_absolute() or ".." in relative_directory.parts:
                raise CheckerInputError(f"invalid source directory '{directory}'")
            source_root = resolved_root / relative_directory
            if not source_root.is_dir():
                raise CheckerInputError(f"configured source directory is missing: '{directory}'")
            for path in source_root.rglob("*.rs"):
                try:
                    relative = path.resolve().relative_to(resolved_root).as_posix()
                except ValueError as error:
                    raise CheckerInputError(f"source symlink escapes project root: '{path}'") from error
                sources.add(relative)
    return sources


def project_source_path(root: Path, filename: Any, source_roots: list[tuple[Path, Path]]) -> str | None:
    if not isinstance(filename, str):
        raise CheckerInputError("coverage file entry has a non-string filename")
    path = Path(filename)
    if not path.is_absolute():
        path = root / path
    try:
        resolved = path.resolve()
    except OSError:
        return None
    for source_root, relative_root in source_roots:
        try:
            relative = resolved.relative_to(source_root)
        except ValueError:
            continue
        if relative.suffix == ".rs":
            return (relative_root / relative).as_posix()
    return None


def reported_source_summaries(root: Path, coverage: Any, source_dirs: dict[str, list[str]]) -> dict[str, Any]:
    try:
        data = coverage["data"]
    except (KeyError, TypeError) as error:
        raise CheckerInputError("coverage report must contain a list named 'data'") from error
    if not isinstance(data, list):
        raise CheckerInputError("coverage report field 'data' must be a list")

    source_roots = [
        ((root / directory).resolve(), Path(directory))
        for directories in source_dirs.values()
        for directory in directories
    ]
    summaries: dict[str, Any] = {}
    for export in data:
        if not isinstance(export, dict) or not isinstance(export.get("files"), list):
            raise CheckerInputError("each coverage data entry must contain a list named 'files'")
        for file_entry in export["files"]:
            if not isinstance(file_entry, dict):
                raise CheckerInputError("coverage files must be objects")
            relative = project_source_path(root, file_entry.get("filename"), source_roots)
            if relative is None:
                continue
            if relative in summaries:
                raise CheckerInputError(f"coverage report contains duplicate source file '{relative}'")
            summaries[relative] = file_entry.get("summary")
    if not summaries:
        raise CheckerInputError("coverage report contains no selected package Rust source files")
    return summaries


def metric(summary: Any, name: str, source: str) -> tuple[float, int, int]:
    try:
        values = summary[name]
        raw_percent = values["percent"]
        covered = values["covered"]
        count = values["count"]
    except (KeyError, TypeError) as error:
        raise CheckerInputError(f"{source}: missing or invalid {name} coverage summary") from error
    if (
        not isinstance(values, dict)
        or not isinstance(raw_percent, (int, float))
        or isinstance(raw_percent, bool)
        or not isinstance(covered, int)
        or isinstance(covered, bool)
        or not isinstance(count, int)
        or isinstance(count, bool)
    ):
        raise CheckerInputError(f"{source}: missing or invalid {name} coverage summary")
    percent = float(raw_percent)
    if count < 0 or covered < 0 or covered > count or not 0 <= percent <= 100:
        raise CheckerInputError(f"{source}: invalid {name} coverage counters")
    return percent, covered, count


def failing_metric(summary: Any, name: str, threshold: float, source: str) -> str | None:
    percent, covered, count = metric(summary, name, source)
    passed = percent >= threshold if name == "functions" else percent > threshold
    if passed:
        return None
    operator = ">=" if name == "functions" else ">"
    return f"{name}={percent:.2f}% ({covered}/{count}; requires {operator}{threshold:.2f}%)"


def metric_overrides(value: Any) -> dict[str, dict[str, float]]:
    if not isinstance(value, dict) or value.get("version") != 1 or not isinstance(value.get("files"), dict):
        raise CheckerInputError("metric override policy must contain version 1 and a files object")
    parsed: dict[str, dict[str, float]] = {}
    for source, entry in value["files"].items():
        if not isinstance(source, str) or not isinstance(entry, dict):
            raise CheckerInputError("each metric override must be a file object")
        minimums = entry.get("minimums")
        reason = entry.get("reason")
        evidence = entry.get("evidence")
        if not isinstance(minimums, dict) or not isinstance(reason, str) or not reason.strip():
            raise CheckerInputError(f"{source}: override needs minimums and a non-empty reason")
        if not isinstance(evidence, dict) or not all(
            isinstance(evidence.get(key), str) and evidence[key].strip()
            for key in ("generated_at", "cargo_llvm_cov", "rust_llvm", "command", "source_baseline")
        ) or not isinstance(evidence.get("metrics"), dict):
            raise CheckerInputError(f"{source}: override needs complete coverage evidence")
        if set(evidence["metrics"]) != set(METRICS):
            raise CheckerInputError(f"{source}: evidence must include all coverage metrics")
        for name, measurement in evidence["metrics"].items():
            if name not in METRICS or not isinstance(measurement, dict):
                raise CheckerInputError(f"{source}: evidence contains an unknown metric")
            try:
                covered = measurement["covered"]
                count = measurement["count"]
                percent = float(measurement["percent"])
            except (KeyError, TypeError, ValueError) as error:
                raise CheckerInputError(f"{source}: invalid {name} evidence") from error
            if (
                not isinstance(covered, int)
                or isinstance(covered, bool)
                or not isinstance(count, int)
                or isinstance(count, bool)
                or count < 0
                or covered < 0
                or covered > count
                or not math.isfinite(percent)
                or not 0 <= percent <= 100
            ):
                raise CheckerInputError(f"{source}: invalid {name} evidence")
        parsed_minimums: dict[str, float] = {}
        for name, raw_minimum in minimums.items():
            if name not in METRICS:
                raise CheckerInputError(f"{source}: unknown metric '{name}' in override")
            if not isinstance(raw_minimum, (int, float)) or isinstance(raw_minimum, bool):
                raise CheckerInputError(f"{source}: invalid {name} minimum")
            minimum = float(raw_minimum)
            if not math.isfinite(minimum) or not 0 <= minimum <= 100:
                raise CheckerInputError(f"{source}: invalid {name} minimum")
            parsed_minimums[name] = minimum
        parsed[source] = parsed_minimums
    return parsed


def check_coverage_files(
    root: Path, coverage_path: Path, config_path: Path, non_executable_path: Path | None = None,
    metric_overrides_path: Path | None = None,
) -> CheckResult:
    """Check complete coverage evidence for configured production source files."""
    coverage = load_json(coverage_path, "coverage report")
    non_executable_path = non_executable_path or root / ".infra/coverage/non-executable-files.json"
    source_dirs, exemptions, non_executable, thresholds = policy(
        load_json(config_path, "coverage policy"), load_json(non_executable_path, "non-executable file classification")
    )
    override_path = metric_overrides_path or root / "scripts/coverage-metric-overrides.json"
    overrides = metric_overrides(load_json(override_path, "metric override policy"))
    selected = selected_sources(root, source_dirs)
    exempt = set().union(*exemptions.values()) if exemptions else set()
    classified = set().union(*non_executable.values()) if non_executable else set()
    configured_paths = exempt | classified
    unknown = configured_paths - selected
    if unknown:
        raise CheckerInputError(
            "coverage policy classifies files outside selected sources: " + ", ".join(sorted(unknown))
        )

    summaries = reported_source_summaries(root, coverage, source_dirs)
    required = selected - classified
    invalid_overrides = set(overrides) - exempt
    if invalid_overrides:
        raise CheckerInputError(
            "metric overrides must refer to configured exception files: "
            + ", ".join(sorted(invalid_overrides))
        )
    missing_overrides = exempt - overrides.keys()
    if missing_overrides:
        raise CheckerInputError(
            "configured exception files need a metric override entry: "
            + ", ".join(sorted(missing_overrides))
        )
    missing = sorted((required - exempt) - summaries.keys())
    errors = [f"{source}: production source file is missing from the coverage report" for source in missing]
    errors.extend(f"{source}: configured exemption is absent from the coverage report" for source in sorted(exempt - summaries.keys()))

    checked_files = 0
    used_overrides: set[str] = set()
    for source in sorted(required & summaries.keys()):
        checked_files += 1
        summary = summaries[source]
        failures = []
        for name in METRICS:
            percent, covered, count = metric(summary, name, source)
            minimum = overrides.get(source, {}).get(name)
            if minimum is not None and percent < thresholds[name]:
                used_overrides.add(source)
                if percent < minimum:
                    failures.append(f"{name}={percent:.2f}% ({covered}/{count}; requires >={minimum:.2f}%)")
            else:
                failure = failing_metric(summary, name, thresholds[name], source)
                if failure is not None:
                    failures.append(failure)
        if failures:
            errors.append(f"{source}: {', '.join(failures)}")

    return CheckResult(checked_files, len(used_overrides), tuple(errors))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "coverage_json",
        nargs="?",
        type=Path,
        default=DEFAULT_COVERAGE,
        help="cargo-llvm-cov JSON report (default: coverage.json in the project root)",
    )
    parser.add_argument(
        "--metric-overrides",
        type=Path,
        default=DEFAULT_METRIC_OVERRIDES,
        help="project per-metric coverage evidence (default: scripts/coverage-metric-overrides.json)",
    )
    parser.add_argument(
        "--config",
        type=Path,
        default=DEFAULT_CONFIG,
        help="coverage policy (default: .infra/coverage/coverage.json)",
    )
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    try:
        result = check_coverage_files(
            ROOT,
            arguments.coverage_json,
            arguments.config,
            metric_overrides_path=arguments.metric_overrides,
        )
    except CheckerInputError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    if result.errors:
        for error in result.errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(
        f"checked {result.checked_files} production coverage files; "
        f"used metric-specific lower bounds for {result.exempt_files} production coverage files"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
