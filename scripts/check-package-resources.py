#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Ensure required fuzz corpus resources are included in the Cargo package."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from typing import Callable


ROOT = Path(__file__).resolve().parent.parent
REQUIRED_RESOURCES = (
    "tests/data/fuzz_corpus/structured_deserialize/collections-nested.json",
    "tests/data/fuzz_corpus/structured_deserialize/typed-record.json",
    "tests/data/fuzz_corpus/structured_deserialize/typed-labeled-interpolation.json",
    "tests/data/fuzz_corpus/structured_deserialize_typed/collections-nested.json",
    "tests/data/fuzz_corpus/structured_deserialize_typed/typed-record.json",
    "tests/data/fuzz_corpus/structured_deserialize_typed/typed-labeled-interpolation.json",
)
PACKAGE_LIST_COMMAND = ("cargo", "package", "--list", "--allow-dirty")
Runner = Callable[..., subprocess.CompletedProcess[str]]


def check_package_resources(runner: Runner = subprocess.run) -> list[str]:
    try:
        result = runner(
            PACKAGE_LIST_COMMAND,
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError as error:
        return [f"could not run cargo package --list: {error}"]

    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        message = "cargo package --list failed"
        if detail:
            message = f"{message}: {detail}"
        return [message]

    packaged_paths = set(result.stdout.splitlines())
    return [
        f"required package resource is missing: {path}"
        for path in REQUIRED_RESOURCES
        if path not in packaged_paths
    ]


def main() -> int:
    errors = check_package_resources()
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    print(f"checked {len(REQUIRED_RESOURCES)} required package resources")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
