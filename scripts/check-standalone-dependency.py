#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Reject local or Git overrides of the production qubit-value dependency."""
import sys
import tomllib
from pathlib import Path

root = Path(__file__).resolve().parent.parent
with (root / "Cargo.toml").open("rb") as stream:
    manifest = tomllib.load(stream)
dependency = manifest["dependencies"]["qubit-value"]
if not isinstance(dependency, dict) or not dependency.get("version"):
    sys.exit("qubit-value must declare its registry version requirement")
if any(key in dependency for key in ("path", "git")):
    sys.exit("qubit-value must resolve without a sibling repository")
print("standalone qubit-value dependency declaration: OK")
