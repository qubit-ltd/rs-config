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
        (self.root / ".infra/tools").mkdir(parents=True)
        for name in ("coverage.sh", "ci-check.sh"):
            shutil.copy(ROOT / name, self.root / name)
        self.log = self.root / "pipeline.log"
        self.write_script(".infra/tools/prepare-local-path-dependencies.sh", "#!/bin/bash\nexit 0\n")
        self.write_script(
            ".infra/tools/coverage-report.sh",
            """
            #!/bin/bash
            set -euo pipefail
            test -f collected.json
            echo report >> "$TEST_LOG"
            """,
        )

    def write_script(self, relative_path: str, body: str) -> None:
        path = self.root / relative_path
        path.write_text(textwrap.dedent(body).lstrip(), encoding="utf-8")
        path.chmod(0o755)

    def run_wrapper(self, name: str, *arguments: str) -> subprocess.CompletedProcess[str]:
        environment = os.environ.copy()
        environment["TEST_LOG"] = str(self.log)
        return subprocess.run(
            ["bash", str(self.root / name), *arguments],
            cwd=self.root, env=environment, capture_output=True, text=True, check=False,
        )

    def read_log(self) -> list[str]:
        return self.log.read_text().splitlines() if self.log.exists() else []

    def test_report_runs_after_successful_collection(self) -> None:
        self.write_script(".infra/tools/infra-tool.sh", """
            #!/bin/bash
            set -euo pipefail
            test "$1" = rs-infra-coverage
            test "$4" = collect
            echo collect >> "$TEST_LOG"
            touch collected.json
        """)
        result = self.run_wrapper("coverage.sh")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_log(), ["collect", "report"])

    def test_collection_failure_prevents_report_even_with_stale_output(self) -> None:
        (self.root / "collected.json").write_text("{}")
        self.write_script(".infra/tools/infra-tool.sh", "#!/bin/bash\nexit 23\n")
        result = self.run_wrapper("coverage.sh")
        self.assertEqual(result.returncode, 23, result.stderr)
        self.assertEqual(self.read_log(), [])

    def test_missing_report_input_fails(self) -> None:
        self.write_script(".infra/tools/infra-tool.sh", "#!/bin/bash\nexit 0\n")
        self.assertNotEqual(self.run_wrapper("coverage.sh").returncode, 0)

    def test_report_failure_propagates(self) -> None:
        self.write_script(".infra/tools/infra-tool.sh", "#!/bin/bash\nexit 0\n")
        self.write_script(".infra/tools/coverage-report.sh", "#!/bin/bash\nexit 29\n")
        self.assertEqual(self.run_wrapper("coverage.sh").returncode, 29)

    def test_ci_delegates_selected_tasks_and_preserves_artifacts(self) -> None:
        artifact = self.root / "target/llvm-cov/report.json"
        artifact.parent.mkdir(parents=True)
        artifact.write_text("{}")
        self.write_script(".infra/tools/infra-tool.sh", """
            #!/bin/bash
            set -euo pipefail
            test "$1" = rs-infra-ci
            test "$4" = --only=coverage
            test "$5" = check
            test -n "$RS_INFRA_STYLE_TOOLCHAIN"
            echo ci >> "$TEST_LOG"
        """)
        result = self.run_wrapper("ci-check.sh", "--only=coverage")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_log(), ["ci"])
        self.assertTrue(artifact.is_file())

    def test_ci_failure_propagates(self) -> None:
        self.write_script(".infra/tools/infra-tool.sh", "#!/bin/bash\nexit 31\n")
        self.assertEqual(self.run_wrapper("ci-check.sh").returncode, 31)

    def test_early_project_hook_does_not_own_the_coverage_gate(self) -> None:
        hook = (ROOT / "project-ci-check.sh").read_text(encoding="utf-8")
        self.assertNotIn("check-coverage-files.py", hook)


if __name__ == "__main__":
    unittest.main()
