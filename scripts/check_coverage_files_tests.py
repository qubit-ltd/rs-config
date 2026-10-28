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
    def counter(percent: float) -> dict[str, float | int]:
        return {"count": 100, "covered": int(percent), "percent": percent}

    return {name: counter(value) for name, value in {
        "functions": functions,
        "lines": lines,
        "regions": regions,
    }.items()}


class CoverageFileCheckerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.checker = load_checker()
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.root = Path(self.temporary_directory.name)
        (self.root / "src").mkdir()
        (self.root / ".infra/coverage").mkdir(parents=True)
        (self.root / "src/checked.rs").write_text("fn checked() {}\n", encoding="utf-8")
        (self.root / "src/low.rs").write_text("fn low() {}\n", encoding="utf-8")
        (self.root / "src/lib.rs").write_text("mod checked;\nmod low;\n", encoding="utf-8")

    def write_inputs(
        self,
        files: list[tuple[str, dict[str, dict[str, float | int]]]],
        exemptions: list[str] | None = None,
    ) -> tuple[Path, Path]:
        coverage_path = self.root / "coverage.json"
        config_path = self.root / ".infra/coverage/coverage.json"
        non_executable_path = self.root / ".infra/coverage/non-executable-files.json"
        coverage_path.write_text(
            json.dumps({"data": [{"files": [
                {"filename": str(self.root / path), "summary": file_summary}
                for path, file_summary in files
            ]}]}),
            encoding="utf-8",
        )
        config_path.write_text(json.dumps({
            "scope": "package",
            "source_dirs": {"qubit-config": ["src"]},
            "thresholds": {"functions": 95, "lines": 90, "regions": 85},
            "threshold_exempt_files": {"qubit-config": exemptions or []},
        }), encoding="utf-8")
        non_executable_path.write_text(json.dumps({"qubit-config": [{
                "path": "src/lib.rs",
                "reason": "module declarations only",
            }]}), encoding="utf-8")
        return coverage_path, config_path

    def test_accepts_thresholds_and_explicit_aggregation_classification(self) -> None:
        coverage_path, config_path = self.write_inputs([
            ("src/checked.rs", summary(95.0, 91.0, 86.0)),
            ("src/low.rs", summary(100.0, 100.0, 100.0)),
        ])
        result = self.checker.check_coverage_files(self.root, coverage_path, config_path)
        self.assertEqual(result, self.checker.CheckResult(2, 0, ()))

    def test_low_file_fails_even_when_another_file_is_fully_covered(self) -> None:
        coverage_path, config_path = self.write_inputs([
            ("src/checked.rs", summary(100.0, 100.0, 100.0)),
            ("src/low.rs", summary(20.0, 20.0, 20.0)),
        ])
        result = self.checker.check_coverage_files(self.root, coverage_path, config_path)
        self.assertEqual(len(result.errors), 1)
        self.assertTrue(result.errors[0].startswith("src/low.rs:"))
        self.assertIn("functions=20.00%", result.errors[0])

    def test_missing_production_file_fails_even_when_report_has_a_high_summary(self) -> None:
        coverage_path, config_path = self.write_inputs([
            ("src/checked.rs", summary(100.0, 100.0, 100.0)),
        ])
        result = self.checker.check_coverage_files(self.root, coverage_path, config_path)
        self.assertEqual(
            result.errors,
            ("src/low.rs: production source file is missing from the coverage report",),
        )

    def test_rejects_an_exemption_missing_from_the_report(self) -> None:
        coverage_path, config_path = self.write_inputs(
            [("src/checked.rs", summary(100.0, 100.0, 100.0))],
            ["src/low.rs"],
        )
        result = self.checker.check_coverage_files(self.root, coverage_path, config_path)
        self.assertEqual(
            result.errors,
            ("src/low.rs: configured exemption is absent from the coverage report",),
        )

    def test_rejects_malformed_metric_counters(self) -> None:
        malformed = summary(100.0, 100.0, 100.0)
        malformed["lines"]["covered"] = 101
        coverage_path, config_path = self.write_inputs([
            ("src/checked.rs", malformed),
            ("src/low.rs", summary(100.0, 100.0, 100.0)),
        ])
        with self.assertRaisesRegex(self.checker.CheckerInputError, "invalid lines coverage counters"):
            self.checker.check_coverage_files(self.root, coverage_path, config_path)

    def test_configured_test_only_module_is_not_a_production_coverage_file(self) -> None:
        (self.root / "src/internal_tests.rs").write_text("#[test] fn example() {}\n", encoding="utf-8")
        coverage_path, config_path = self.write_inputs([
            ("src/checked.rs", summary(100.0, 100.0, 100.0)),
            ("src/low.rs", summary(100.0, 100.0, 100.0)),
        ])
        classification_path = self.root / ".infra/coverage/non-executable-files.json"
        classification = json.loads(classification_path.read_text(encoding="utf-8"))
        classification["qubit-config"].append({
            "path": "src/internal_tests.rs", "reason": "test-only module has no production code",
        })
        classification_path.write_text(json.dumps(classification), encoding="utf-8")
        result = self.checker.check_coverage_files(self.root, coverage_path, config_path)
        self.assertEqual(result, self.checker.CheckResult(2, 0, ()))


if __name__ == "__main__":
    unittest.main()
