#!/usr/bin/env python3
"""Regression tests for Cargo feature metadata validation."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import unittest


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts" / "check-config-features.py"
SPEC = importlib.util.spec_from_file_location("check_config_features", CHECKER)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load checker: {CHECKER}")
CHECKER_MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER_MODULE)


class ConfigFeaturesTests(unittest.TestCase):
    def setUp(self) -> None:
        self.package = {
            "id": "path+file:///workspace/rs-config#qubit-config@0.15.1",
            "name": "qubit-config",
            "version": "0.15.1",
            "manifest_path": str((ROOT / "Cargo.toml").resolve()),
        }
        self.metadata = {
            "packages": [self.package],
            "resolve": {
                "nodes": [{"id": self.package["id"], "features": []}],
            },
        }
        self.manifest = ROOT / "Cargo.toml"

    def check(self, features: list[str], mode: str = "minimal") -> list[str]:
        if "resolve" in self.metadata:
            self.metadata["resolve"]["nodes"][0]["features"] = features
        return CHECKER_MODULE.check_features(json.dumps(self.metadata), self.manifest, mode)

    def test_minimal_with_empty_optional_features_passes(self) -> None:
        self.assertEqual(self.check([]), [])

    def test_minimal_rejects_yaml_feature(self) -> None:
        self.assertTrue(self.check(["yaml"]))

    def test_full_rejects_missing_feature(self) -> None:
        self.assertTrue(self.check(["full"], "full"))

    def test_wrong_package_path_fails(self) -> None:
        self.package["manifest_path"] = "/elsewhere/Cargo.toml"
        self.assertTrue(self.check([]))

    def test_duplicate_target_packages_fail(self) -> None:
        self.metadata["packages"].append(dict(self.package))
        self.assertTrue(self.check([]))

    def test_missing_resolve_fails(self) -> None:
        del self.metadata["resolve"]
        self.assertTrue(self.check([]))

    def test_invalid_json_fails(self) -> None:
        self.assertTrue(CHECKER_MODULE.check_features("{", self.manifest, "minimal"))

    def test_cli_maps_validation_errors_to_exit_one(self) -> None:
        result = subprocess.run(
            [sys.executable, str(CHECKER), "--manifest-path", str(self.manifest), "--mode", "minimal"],
            input="{", text=True, capture_output=True, check=False,
        )
        self.assertEqual(result.returncode, 1)


if __name__ == "__main__":
    unittest.main()
