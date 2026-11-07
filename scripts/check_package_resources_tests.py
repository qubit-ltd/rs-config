#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Tests for the Cargo package resource checker."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import Mock


SCRIPT = Path(__file__).with_name("check-package-resources.py")


def load_checker():
    spec = importlib.util.spec_from_file_location("check_package_resources", SCRIPT)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {SCRIPT}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class PackageResourceCheckerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.checker = load_checker()
        self.paths = self.checker.REQUIRED_RESOURCES + (
            "doc/user_guide.md",
            "doc/user_guide.zh_CN.md",
            "doc/design.md",
            "doc/design.zh_CN.md",
            "doc/coverage_policy.md",
            "doc/coverage_policy.zh_CN.md",
            "doc/composite-resource-probe-20261106T065653653Z-81411945.md",
            "doc/implementation-results-20261106-f1e00ce7.md",
        )

    def runner_for(self, paths: tuple[str, ...]) -> Mock:
        return Mock(return_value=subprocess.CompletedProcess(
            args=self.checker.PACKAGE_LIST_COMMAND,
            returncode=0,
            stdout="\n".join(paths) + "\n",
            stderr="",
        ))

    def test_accepts_all_required_resources_and_uses_package_command_at_repository_root(self) -> None:
        runner = self.runner_for(self.paths)

        errors = self.checker.check_package_resources(runner)

        self.assertEqual(errors, [])
        runner.assert_called_once_with(
            ("cargo", "package", "--list", "--allow-dirty"),
            cwd=self.checker.ROOT,
            check=False,
            capture_output=True,
            text=True,
        )

    def test_reports_each_missing_resource(self) -> None:
        for missing_path in self.paths:
            with self.subTest(path=missing_path):
                included_paths = tuple(path for path in self.paths if path != missing_path)
                errors = self.checker.check_package_resources(self.runner_for(included_paths))
                self.assertEqual(errors, [f"required package resource is missing: {missing_path}"])

    def test_rejects_unapproved_document_paths(self) -> None:
        required_paths = self.paths
        for extra_path in (
            "doc/resource-probe-old.md",
            "doc/measurements.tar.gz",
            "doc/design-session.md",
        ):
            with self.subTest(path=extra_path):
                errors = self.checker.check_package_resources(
                    self.runner_for(required_paths + (extra_path,))
                )
                self.assertEqual(
                    errors,
                    [f"unexpected package document: {extra_path}"],
                )

    def test_composite_probe_is_packaged_with_its_documentation_references(self) -> None:
        record_name = "composite-resource-probe-20261106T065653653Z-81411945.md"
        record_path = f"doc/{record_name}"
        self.assertIn(record_path, self.checker.REQUIRED_DOCS)
        self.assertTrue((self.checker.ROOT / record_path).is_file())
        for document in (
            "doc/user_guide.md", "doc/user_guide.zh_CN.md",
            "doc/design.md", "doc/design.zh_CN.md",
        ):
            with self.subTest(document=document):
                self.assertIn(document, self.checker.REQUIRED_DOCS)
                text = (self.checker.ROOT / document).read_text(encoding="utf-8")
                self.assertIn(f"]({record_name})", text)
        self.assertTrue(
            (self.checker.ROOT / "examples/config_composite_resource_probe.rs").is_file()
        )

    def test_implementation_results_are_packaged_with_design_references(self) -> None:
        record_name = "implementation-results-20261106-f1e00ce7.md"
        record_path = f"doc/{record_name}"
        self.assertIn(record_path, self.checker.REQUIRED_DOCS)
        self.assertTrue((self.checker.ROOT / record_path).is_file())
        for document in ("doc/design.md", "doc/design.zh_CN.md"):
            with self.subTest(document=document):
                self.assertIn(document, self.checker.REQUIRED_DOCS)
                text = (self.checker.ROOT / document).read_text(encoding="utf-8")
                self.assertIn(f"]({record_name})", text)

    def test_cargo_failure_is_reported(self) -> None:
        runner = Mock(return_value=subprocess.CompletedProcess(
            args=self.checker.PACKAGE_LIST_COMMAND,
            returncode=101,
            stdout="",
            stderr="manifest error",
        ))

        self.assertEqual(
            self.checker.check_package_resources(runner),
            ["cargo package --list failed: manifest error"],
        )

    def test_unavailable_cargo_is_reported(self) -> None:
        runner = Mock(side_effect=FileNotFoundError("cargo not found"))

        self.assertEqual(
            self.checker.check_package_resources(runner),
            ["could not run cargo package --list: cargo not found"],
        )


if __name__ == "__main__":
    unittest.main()
