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
        overrides: dict[str, dict] | None = None,
    ) -> tuple[Path, Path, Path]:
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
        override_path = self.root / "scripts/overrides.json"
        override_path.parent.mkdir(parents=True, exist_ok=True)
        override_path.write_text(json.dumps({"version": 1, "files": overrides or {}}), encoding="utf-8")
        return coverage_path, config_path, override_path

    def check(self, paths: tuple[Path, Path, Path]):
        coverage_path, config_path, override_path = paths
        return self.checker.check_coverage_files(
            self.root, coverage_path, config_path, metric_overrides_path=override_path
        )

    def override(self, minimums: dict[str, float]) -> dict:
        return {
            "minimums": minimums,
            "reason": "LLVM attribution is incomplete for the covered defensive branches.",
            "evidence": {
                "generated_at": "2026-10-30T00:00:00Z",
                "cargo_llvm_cov": "cargo-llvm-cov test fixture",
                "rust_llvm": "rustc fixture / LLVM fixture",
                "command": "fixture only",
                "source_baseline": "fixture source",
                "metrics": {
                    name: {"covered": 78, "count": 100, "percent": 78.0}
                    for name in ("functions", "lines", "regions")
                },
            },
        }

    def test_accepts_thresholds_and_explicit_aggregation_classification(self) -> None:
        paths = self.write_inputs([
            ("src/checked.rs", summary(95.0, 91.0, 86.0)),
            ("src/low.rs", summary(100.0, 100.0, 100.0)),
        ])
        result = self.check(paths)
        self.assertEqual(result, self.checker.CheckResult(2, 0, ()))

    def test_low_file_fails_even_when_another_file_is_fully_covered(self) -> None:
        paths = self.write_inputs([
            ("src/checked.rs", summary(100.0, 100.0, 100.0)),
            ("src/low.rs", summary(20.0, 20.0, 20.0)),
        ])
        result = self.check(paths)
        self.assertEqual(len(result.errors), 1)
        self.assertTrue(result.errors[0].startswith("src/low.rs:"))
        self.assertIn("functions=20.00%", result.errors[0])

    def test_missing_production_file_fails_even_when_report_has_a_high_summary(self) -> None:
        paths = self.write_inputs([
            ("src/checked.rs", summary(100.0, 100.0, 100.0)),
        ])
        result = self.check(paths)
        self.assertEqual(
            result.errors,
            ("src/low.rs: production source file is missing from the coverage report",),
        )

    def test_rejects_an_exemption_missing_from_the_report(self) -> None:
        paths = self.write_inputs(
            [("src/checked.rs", summary(100.0, 100.0, 100.0))],
            ["src/low.rs"],
            {"src/low.rs": self.override({})},
        )
        result = self.check(paths)
        self.assertEqual(
            result.errors,
            ("src/low.rs: configured exemption is absent from the coverage report",),
        )

    def test_rejects_malformed_metric_counters(self) -> None:
        malformed = summary(100.0, 100.0, 100.0)
        malformed["lines"]["covered"] = 101
        paths = self.write_inputs([
            ("src/checked.rs", malformed),
            ("src/low.rs", summary(100.0, 100.0, 100.0)),
        ])
        with self.assertRaisesRegex(self.checker.CheckerInputError, "invalid lines coverage counters"):
            self.check(paths)

    def test_configured_test_only_module_is_not_a_production_coverage_file(self) -> None:
        (self.root / "src/internal_tests.rs").write_text("#[test] fn example() {}\n", encoding="utf-8")
        paths = self.write_inputs([
            ("src/checked.rs", summary(100.0, 100.0, 100.0)),
            ("src/low.rs", summary(100.0, 100.0, 100.0)),
        ])
        classification_path = self.root / ".infra/coverage/non-executable-files.json"
        classification = json.loads(classification_path.read_text(encoding="utf-8"))
        classification["qubit-config"].append({
            "path": "src/internal_tests.rs", "reason": "test-only module has no production code",
        })
        classification_path.write_text(json.dumps(classification), encoding="utf-8")
        result = self.check(paths)
        self.assertEqual(result, self.checker.CheckResult(2, 0, ()))

    def test_override_is_per_metric_and_uses_inclusive_minimum(self) -> None:
        entry = self.override({"functions": 78.0})
        paths = self.write_inputs([
            ("src/checked.rs", summary(100, 100, 100)),
            ("src/low.rs", summary(78, 91, 86)),
        ], ["src/low.rs"], {"src/low.rs": entry})
        self.assertEqual(self.check(paths), self.checker.CheckResult(2, 1, ()))
        paths = self.write_inputs([
            ("src/checked.rs", summary(100, 100, 100)),
            ("src/low.rs", summary(78, 20, 86)),
        ], ["src/low.rs"], {"src/low.rs": entry})
        result = self.check(paths)
        self.assertEqual(len(result.errors), 1)
        self.assertIn("lines=20.00%", result.errors[0])

    def test_line_override_applies_at_normal_threshold(self) -> None:
        paths = self.write_inputs([
            ("src/checked.rs", summary(100, 100, 100)),
            ("src/low.rs", summary(100, 90, 100)),
        ], ["src/low.rs"], {"src/low.rs": self.override({"lines": 89.11})})
        result = self.check(paths)
        self.assertEqual(result.errors, ())
        self.assertEqual(result.exempt_files, 1)

    def test_override_boundaries_match_normal_thresholds(self) -> None:
        cases = [
            ("lines", 89.11, [
                (89.10, False, 1), (89.11, True, 1), (89.99, True, 1),
                (90.0, True, 1), (90.01, True, 0),
            ]),
            ("regions", 83.37, [
                (83.36, False, 1), (83.37, True, 1), (84.99, True, 1),
                (85.0, True, 1), (85.01, True, 0),
            ]),
            ("functions", 77.50, [
                (77.49, False, 1), (77.50, True, 1), (94.99, True, 1),
                (95.0, True, 0), (95.01, True, 0),
            ]),
        ]
        metric_indexes = {"functions": 0, "lines": 1, "regions": 2}
        for name, minimum, observations in cases:
            for observed, should_pass, expected_exempt_files in observations:
                with self.subTest(metric=name, observed=observed):
                    values = [100.0, 100.0, 100.0]
                    values[metric_indexes[name]] = observed
                    paths = self.write_inputs([
                        ("src/checked.rs", summary(100, 100, 100)),
                        ("src/low.rs", summary(*values)),
                    ], ["src/low.rs"], {
                        "src/low.rs": self.override({name: minimum}),
                    })
                    result = self.check(paths)
                    self.assertEqual(result.exempt_files, expected_exempt_files)
                    if should_pass:
                        self.assertEqual(result.errors, ())
                    else:
                        self.assertEqual(len(result.errors), 1)
                        self.assertIn(f"{name}={observed:.2f}%", result.errors[0])

    def test_strict_threshold_equalities_still_fail_without_matching_override(self) -> None:
        for values, overrides, failed_metric in [
            ((100, 90, 100), {}, "lines"),
            ((100, 100, 85), {}, "regions"),
            ((100, 90, 100), {"functions": 77.5}, "lines"),
        ]:
            with self.subTest(values=values, overrides=overrides):
                paths = self.write_inputs([
                    ("src/checked.rs", summary(100, 100, 100)),
                    ("src/low.rs", summary(*values)),
                ], ["src/low.rs"], {
                    "src/low.rs": self.override(overrides),
                })
                result = self.check(paths)
                self.assertEqual(result.exempt_files, 0)
                self.assertEqual(len(result.errors), 1)
                self.assertIn(f"{failed_metric}=", result.errors[0])

    def test_override_below_minimum_and_zero_coverage_fail(self) -> None:
        entry = self.override({"functions": 78.0})
        paths = self.write_inputs([
            ("src/checked.rs", summary(100, 100, 100)),
            ("src/low.rs", summary(77.99, 91, 86)),
        ], ["src/low.rs"], {"src/low.rs": entry})
        self.assertIn("functions=77.99%", self.check(paths).errors[0])
        entry = self.override({"functions": 78.0, "lines": 70.0, "regions": 60.0})
        paths = self.write_inputs([
            ("src/checked.rs", summary(100, 100, 100)),
            ("src/low.rs", summary(0, 0, 0)),
        ], ["src/low.rs"], {"src/low.rs": entry})
        self.assertEqual(len(self.check(paths).errors), 1)

    def test_exception_summary_is_always_validated(self) -> None:
        invalid_summaries = [None, {"functions": summary(100, 100, 100)["functions"]}]
        invalid_summaries.append(summary(100, 100, 100))
        invalid_summaries[-1]["lines"]["covered"] = 101
        invalid_summaries.append(summary(100, 100, 100))
        invalid_summaries[-1]["regions"]["percent"] = float("nan")
        for invalid in invalid_summaries:
            with self.subTest(invalid=invalid):
                paths = self.write_inputs([
                    ("src/checked.rs", summary(100, 100, 100)),
                    ("src/low.rs", invalid),
                ], ["src/low.rs"], {"src/low.rs": self.override({})})
                with self.assertRaises(self.checker.CheckerInputError):
                    self.check(paths)

    def test_override_policy_fails_closed(self) -> None:
        for sidecar in [
            None,
            {"version": 2, "files": {}},
            {"version": 1, "files": {"src/low.rs": {**self.override({}), "minimums": {"branches": 1}}}},
            {"version": 1, "files": {"src/low.rs": {**self.override({}), "reason": " "}}},
            {"version": 1, "files": {"src/low.rs": {"minimums": {}, "reason": "missing evidence"}}},
            {"version": 1, "files": {"src/low.rs": self.override({"functions": float("inf")})}},
            {"version": 1, "files": {"src/low.rs": self.override({"functions": 101})}},
            {"version": 1, "files": {"../outside.rs": self.override({})}},
        ]:
            paths = self.write_inputs([
                ("src/checked.rs", summary(100, 100, 100)),
                ("src/low.rs", summary(100, 100, 100)),
            ], ["src/low.rs"])
            sidecar_path = paths[2]
            if sidecar is None:
                sidecar_path.unlink()
            else:
                sidecar_path.write_text(json.dumps(sidecar), encoding="utf-8")
            with self.assertRaises(self.checker.CheckerInputError):
                self.check(paths)

    def test_exception_must_be_reported(self) -> None:
        paths = self.write_inputs([("src/checked.rs", summary(100, 100, 100))], ["src/low.rs"], {
            "src/low.rs": self.override({}),
        })
        self.assertIn("configured exemption is absent", self.check(paths).errors[0])


if __name__ == "__main__":
    unittest.main()
