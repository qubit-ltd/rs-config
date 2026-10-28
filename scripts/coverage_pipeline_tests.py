#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Integration tests for the project coverage gate wrapper."""

from __future__ import annotations

import json
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
        (self.root / ".infra/bin").mkdir(parents=True)
        (self.root / ".infra/coverage").mkdir(parents=True)
        (self.root / "scripts").mkdir()
        (self.root / "src").mkdir()
        (self.root / "src/app.rs").write_text("fn app() {}\n", encoding="utf-8")
        (self.root / "src/lib.rs").write_text("mod app;\n", encoding="utf-8")
        shutil.copy(ROOT / "scripts/check-project-coverage.sh", self.root / "scripts/check-project-coverage.sh")
        shutil.copy(ROOT / "scripts/check-coverage-files.py", self.root / "scripts/check-coverage-files.py")
        (self.root / "scripts/check-project-coverage.sh").chmod(0o755)
        self.log = self.root / "pipeline.log"
        self.write_config()
        self.write_script(".infra/bin/coverage.sh", """
            #!/usr/bin/env bash
            set -euo pipefail
            printf '%s\\n' "$TEST_COVERAGE_MODE" >> "$TEST_LOG"
            case "$TEST_COVERAGE_MODE" in
                failure) exit 23 ;;
                missing) exit 0 ;;
                malformed) printf '{malformed' > "$TEST_REPORT" ;;
                low) python3 "$TEST_REPORT_WRITER" low "$TEST_REPORT" "$TEST_SOURCE" ;;
                success) python3 "$TEST_REPORT_WRITER" success "$TEST_REPORT" "$TEST_SOURCE" ;;
                *) exit 24 ;;
            esac
        """)
        self.write_script("write_report.py", """
            import json
            import sys
            mode, report_path, source_path = sys.argv[1:]
            value = 100 if mode == "success" else 20
            def metric(percent):
                return {"count": 100, "covered": percent, "percent": percent}
            summary = {name: metric(value) for name in ("functions", "lines", "regions")}
            with open(report_path, "w", encoding="utf-8") as report:
                json.dump({"data": [{"files": [
                    {"filename": source_path, "summary": summary},
                ]}]}, report)
        """)

    def write_config(self) -> None:
        (self.root / ".infra/coverage/coverage.json").write_text(json.dumps({
            "scope": "package",
            "source_dirs": {"qubit-config": ["src"]},
            "thresholds": {"functions": 95, "lines": 90, "regions": 85},
            "threshold_exempt_files": {"qubit-config": []},
        }), encoding="utf-8")
        (self.root / ".infra/coverage/non-executable-files.json").write_text(json.dumps({"qubit-config": [{
                "path": "src/lib.rs", "reason": "module declarations only",
            }]}), encoding="utf-8")

    def write_script(self, relative_path: str, body: str) -> None:
        path = self.root / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(textwrap.dedent(body).lstrip(), encoding="utf-8")
        path.chmod(0o755)

    def run_wrapper(self, mode: str) -> subprocess.CompletedProcess[str]:
        environment = os.environ.copy()
        environment.update({
            "TEST_COVERAGE_MODE": mode,
            "TEST_LOG": str(self.log),
            "TEST_REPORT": str(self.root / "coverage.json"),
            "TEST_REPORT_WRITER": str(self.root / "write_report.py"),
            "TEST_SOURCE": str(self.root / "src/app.rs"),
        })
        return subprocess.run(
            ["bash", str(self.root / "scripts/check-project-coverage.sh")],
            cwd=self.root,
            env=environment,
            capture_output=True,
            text=True,
            check=False,
        )

    def test_success_checks_the_fresh_report_after_collection(self) -> None:
        (self.root / "coverage.json").write_text("stale report", encoding="utf-8")
        result = self.run_wrapper("success")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads((self.root / "coverage.json").read_text(encoding="utf-8"))
        self.assertEqual(report["data"][0]["files"][0]["summary"]["functions"]["percent"], 100)
        self.assertEqual(self.log.read_text(encoding="utf-8").splitlines(), ["success"])
        self.assertIn("checked 1 production coverage files", result.stdout)

    def test_collection_uses_ci_incremental_setting(self) -> None:
        self.write_script(".infra/bin/coverage.sh", """
            #!/usr/bin/env bash
            set -euo pipefail
            [[ "${CARGO_INCREMENTAL:-}" == 1 ]]
            python3 "$TEST_REPORT_WRITER" success "$TEST_REPORT" "$TEST_SOURCE"
        """)
        result = self.run_wrapper("success")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_collection_failure_does_not_pass_from_stale_report(self) -> None:
        stale = json.dumps({"stale": True})
        (self.root / "coverage.json").write_text(stale, encoding="utf-8")
        result = self.run_wrapper("failure")
        self.assertEqual(result.returncode, 23, result.stderr)
        self.assertEqual((self.root / "coverage.json").read_text(encoding="utf-8"), stale)
        self.assertEqual(self.log.read_text(encoding="utf-8").splitlines(), ["failure"])

    def test_missing_report_fails_even_when_old_report_is_restored(self) -> None:
        (self.root / "coverage.json").write_text("stale report", encoding="utf-8")
        result = self.run_wrapper("missing")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("did not create", result.stderr)
        self.assertEqual((self.root / "coverage.json").read_text(encoding="utf-8"), "stale report")

    def test_malformed_report_fails(self) -> None:
        result = self.run_wrapper("malformed")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("invalid JSON", result.stderr)

    def test_low_file_coverage_fails(self) -> None:
        result = self.run_wrapper("low")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("src/app.rs", result.stderr)
        self.assertIn("functions=20.00%", result.stderr)

    def test_missing_configuration_fails(self) -> None:
        (self.root / ".infra/coverage/coverage.json").unlink()
        result = self.run_wrapper("success")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cannot read coverage policy", result.stderr)

    def test_project_ci_hook_does_not_consume_a_coverage_report(self) -> None:
        hook = (ROOT / ".infra/project-ci-check.local.sh").read_text(encoding="utf-8")
        self.assertNotIn("check-coverage-files.py", hook)
        self.assertIn("check_coverage_files_tests.py", hook)
        self.assertIn("coverage_pipeline_tests.py", hook)
        self.assertIn("check-doc-examples.py", hook)


if __name__ == "__main__":
    unittest.main()
