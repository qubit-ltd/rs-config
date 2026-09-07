#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Tests for the per-source coverage evidence checker."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check-coverage-files.py")


def load_checker():
    spec = importlib.util.spec_from_file_location("check_coverage_files", SCRIPT)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {SCRIPT}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def summary(functions: float, lines: float, regions: float) -> dict[str, dict[str, float | int]]:
    return {
        "functions": {"count": 100, "covered": int(functions), "percent": functions},
        "lines": {"count": 100, "covered": int(lines), "percent": lines},
        "regions": {"count": 100, "covered": int(regions), "percent": regions},
    }


class CoverageFileCheckerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.checker = load_checker()
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.root = Path(self.temporary_directory.name)
        (self.root / "src").mkdir()

    def write_inputs(
        self,
        files: list[tuple[str, dict[str, dict[str, float | int]]]],
        exemptions: list[str],
    ) -> tuple[Path, Path]:
        coverage_path = self.root / "coverage.json"
        config_path = self.root / ".rs-ci-coverage.json"
        coverage_path.write_text(
            json.dumps(
                {
                    "data": [
                        {
                            "files": [
                                {"filename": str(self.root / path), "summary": file_summary}
                                for path, file_summary in files
                            ]
                        }
                    ]
                }
            ),
            encoding="utf-8",
        )
        config_path.write_text(
            json.dumps({"threshold_exempt_files": {"qubit-config": exemptions}}),
            encoding="utf-8",
        )
        return coverage_path, config_path

    def test_excludes_configured_files_and_accepts_boundary_metrics(self) -> None:
        coverage_path, config_path = self.write_inputs(
            [
                ("src/checked.rs", summary(95.0, 91.0, 86.0)),
                ("src/exempt.rs", summary(0.0, 0.0, 0.0)),
            ],
            ["src/exempt.rs"],
        )

        result = self.checker.check_coverage_files(self.root, coverage_path, config_path)

        self.assertEqual(result.checked_files, 1)
        self.assertEqual(result.exempt_files, 1)
        self.assertEqual(result.errors, ())

    def test_reports_each_failed_metric_with_the_source_file(self) -> None:
        coverage_path, config_path = self.write_inputs(
            [("src/bad.rs", summary(94.0, 90.0, 85.0))],
            [],
        )

        result = self.checker.check_coverage_files(self.root, coverage_path, config_path)

        self.assertEqual(
            result.errors,
            (
                "src/bad.rs: functions=94.00% (94/100; requires >=95.00%), "
                "lines=90.00% (90/100; requires >90.00%), "
                "regions=85.00% (85/100; requires >85.00%)",
            ),
        )

    def test_rejects_an_exemption_missing_from_the_report(self) -> None:
        coverage_path, config_path = self.write_inputs(
            [("src/checked.rs", summary(100.0, 100.0, 100.0))],
            ["src/missing.rs"],
        )

        result = self.checker.check_coverage_files(self.root, coverage_path, config_path)

        self.assertEqual(result.errors, ("src/missing.rs: configured exemption is absent from the coverage report",))


if __name__ == "__main__":
    unittest.main()
