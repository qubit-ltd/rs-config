#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Validate the resolved qubit-config features in Cargo metadata JSON."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tomllib
from typing import Any


def check_features(metadata_json: str, manifest_path: Path, mode: str) -> list[str]:
    """Return validation errors for a metadata graph and package manifest."""
    errors: list[str] = []
    try:
        manifest = manifest_path.resolve()
        with manifest.open("rb") as stream:
            manifest_data = tomllib.load(stream)
    except (OSError, tomllib.TOMLDecodeError) as error:
        return [f"cannot read target manifest {manifest_path}: {error}"]

    package_data = manifest_data.get("package", {})
    package_name = package_data.get("name")
    package_version = package_data.get("version")
    if not isinstance(package_name, str) or not isinstance(package_version, str):
        return [f"target manifest has no valid package name/version: {manifest_path}"]
    if mode not in {"minimal", "full"}:
        return [f"unknown feature mode: {mode}"]

    try:
        metadata: Any = json.loads(metadata_json)
    except json.JSONDecodeError as error:
        return [f"invalid Cargo metadata JSON: {error}"]
    if not isinstance(metadata, dict):
        return ["Cargo metadata JSON root must be an object"]

    packages = metadata.get("packages")
    if not isinstance(packages, list):
        return ["Cargo metadata has no valid packages list"]
    matches = [
        package for package in packages
        if isinstance(package, dict)
        and package.get("name") == package_name
        and package.get("version") == package_version
        and _same_path(package.get("manifest_path"), manifest)
    ]
    if len(matches) != 1:
        return [
            f"expected one {package_name}@{package_version} package at {manifest}, found {len(matches)}"
        ]

    resolve = metadata.get("resolve")
    nodes = resolve.get("nodes") if isinstance(resolve, dict) else None
    if not isinstance(nodes, list):
        return ["Cargo metadata has no valid resolve.nodes"]
    target_id = matches[0].get("id")
    node_matches = [node for node in nodes if isinstance(node, dict) and node.get("id") == target_id]
    if len(node_matches) != 1:
        return [f"expected one resolved node for {package_name}@{package_version}, found {len(node_matches)}"]
    actual = node_matches[0].get("features")
    if not isinstance(actual, list) or not all(isinstance(item, str) for item in actual):
        return [f"resolved node for {package_name}@{package_version} has invalid features"]

    root_features = manifest_data.get("features", {})
    if not isinstance(root_features, dict):
        return [f"target manifest has invalid [features]: {manifest}"]
    optional_features = set(root_features) - {"default"}
    actual_features = set(actual)
    if mode == "minimal":
        unexpected = actual_features & optional_features
        if unexpected:
            errors.append(
                f"{mode} {package_name}@{package_version} enabled optional features: "
                f"{', '.join(sorted(unexpected))}; actual: {', '.join(sorted(actual_features))}"
            )
    else:
        missing = optional_features - actual_features
        if missing:
            errors.append(
                f"{mode} {package_name}@{package_version} is missing features: "
                f"{', '.join(sorted(missing))}; actual: {', '.join(sorted(actual_features))}"
            )
    return errors


def _same_path(value: object, expected: Path) -> bool:
    if not isinstance(value, str):
        return False
    try:
        return Path(value).resolve() == expected
    except OSError:
        return False


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest-path", required=True, type=Path)
    parser.add_argument("--mode", required=True, choices=("minimal", "full"))
    args = parser.parse_args()
    errors = check_features(sys.stdin.read(), args.manifest_path, args.mode)
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"validated {args.mode} feature graph for {args.manifest_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
