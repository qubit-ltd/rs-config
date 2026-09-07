#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Integration tests for the project coverage gate wrappers."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent


class CoveragePipelineTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.root = Path(self.temporary_directory.name)
        (self.root / ".rs-ci").mkdir()
        (self.root / "scripts").mkdir()
        shutil.copy(ROOT / "coverage.sh", self.root / "coverage.sh")
        shutil.copy(ROOT / "ci-check.sh", self.root / "ci-check.sh")
        self.log = self.root / "pipeline.log"
        self.write_script(
            "scripts/check-coverage-files.py",
            """
            import os
            from pathlib import Path

            root = Path(__file__).resolve().parent.parent
            report = root / "target/llvm-cov/coverage.json"
            if not report.is_file():
                raise SystemExit("coverage checker ran without a report")
            with Path(os.environ["TEST_LOG"]).open("a", encoding="utf-8") as log:
                log.write("checker\\n")
            """,
        )

    def write_script(self, relative_path: str, body: str) -> None:
        path = self.root / relative_path
        path.write_text(textwrap.dedent(body).lstrip(), encoding="utf-8")
        path.chmod(0o755)

    def run_wrapper(self, name: str, *arguments: str, **environment: str) -> subprocess.CompletedProcess[str]:
        process_environment = os.environ.copy()
        process_environment.update(environment)
        process_environment["TEST_LOG"] = str(self.log)
        return subprocess.run(
            ["bash", str(self.root / name), *arguments],
            cwd=self.root,
            env=process_environment,
            capture_output=True,
            text=True,
            check=False,
        )

    def read_log(self) -> list[str]:
        if not self.log.exists():
            return []
        return self.log.read_text(encoding="utf-8").splitlines()

    def test_coverage_wrapper_checks_a_fresh_report_after_generation(self) -> None:
        self.write_script(
            ".rs-ci/coverage.sh",
            """
            #!/bin/bash
            set -euo pipefail
            echo coverage-start >> "$TEST_LOG"
            mkdir -p target/llvm-cov
            printf '{"data": []}\n' > target/llvm-cov/coverage.json
            echo coverage-finish >> "$TEST_LOG"
            """,
        )

        result = self.run_wrapper("coverage.sh", "json")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_log(), ["coverage-start", "coverage-finish", "checker"])

    def test_coverage_wrapper_does_not_check_after_generation_failure(self) -> None:
        self.write_script(
            ".rs-ci/coverage.sh",
            """
            #!/bin/bash
            echo coverage-failed >> "$TEST_LOG"
            exit 23
            """,
        )

        result = self.run_wrapper("coverage.sh", "json")

        self.assertEqual(result.returncode, 23, result.stderr)
        self.assertEqual(self.read_log(), ["coverage-failed"])

    def test_coverage_help_does_not_require_a_report(self) -> None:
        self.write_script(
            ".rs-ci/coverage.sh",
            """
            #!/bin/bash
            echo help >> "$TEST_LOG"
            """,
        )

        result = self.run_wrapper("coverage.sh", "--help")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_log(), ["help"])

    def test_success_without_a_report_fails_instead_of_skipping_gate(self) -> None:
        self.write_script(
            ".rs-ci/coverage.sh",
            """
            #!/bin/bash
            echo coverage-finish >> "$TEST_LOG"
            """,
        )

        result = self.run_wrapper("coverage.sh", "json")

        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.read_log(), ["coverage-finish"])

    def test_ci_wrapper_preserves_report_until_checker_then_cleans(self) -> None:
        self.write_script(
            ".rs-ci/ci-check.sh",
            """
            #!/bin/bash
            set -euo pipefail
            echo "ci:$RS_CI_ARTIFACT_CLEANUP_MODE" >> "$TEST_LOG"
            mkdir -p target/rs-ci fuzz/target target/llvm-cov
            printf '{"data": []}\n' > target/llvm-cov/coverage.json
            """,
        )

        result = self.run_wrapper("ci-check.sh")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_log(), ["ci:never", "checker"])
        self.assertFalse((self.root / "target/rs-ci").exists())
        self.assertFalse((self.root / "fuzz/target").exists())
        self.assertFalse((self.root / "target/llvm-cov").exists())

    def test_ci_wrapper_preserves_artifacts_when_requested(self) -> None:
        self.write_script(
            ".rs-ci/ci-check.sh",
            """
            #!/bin/bash
            set -euo pipefail
            echo "ci:$RS_CI_ARTIFACT_CLEANUP_MODE" >> "$TEST_LOG"
            mkdir -p target/llvm-cov
            printf '{"data": []}\n' > target/llvm-cov/coverage.json
            """,
        )

        result = self.run_wrapper("ci-check.sh", RS_CI_ARTIFACT_CLEANUP_MODE="never")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_log(), ["ci:never", "checker"])
        self.assertTrue((self.root / "target/llvm-cov/coverage.json").is_file())

    def test_early_project_hook_does_not_own_the_coverage_gate(self) -> None:
        hook = (ROOT / "project-ci-check.sh").read_text(encoding="utf-8")

        self.assertNotIn("check-coverage-files.py", hook)


if __name__ == "__main__":
    unittest.main()
