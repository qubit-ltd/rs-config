#!/usr/bin/env python3
"""Regression tests for the real downstream adapter CI wiring."""

from __future__ import annotations

import hashlib
from pathlib import Path
import re
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = REPO_ROOT / ".github/workflows/downstream-adapters.yml"
REVISIONS = REPO_ROOT / "scripts/downstream-adapters-revisions.tsv"
CHECKER = "scripts/check-downstream-adapters.sh"
FIXED_REVISION_MANIFEST_SHA256 = "dbe8316fd1324c3c0a5ceb1ffe0b9e6a39e1b1c0150017caa8feba99c8b8ca39"


def wiring_errors(workflow: str, revisions: str) -> list[str]:
    errors: list[str] = []
    if not re.search(r"(?m)^  pull_request:\s*$", workflow):
        errors.append("workflow must run for pull requests")
    if not re.search(r"(?m)^  push:\s*\n    branches: \[main\]", workflow):
        errors.append("workflow must run on main pushes")
    if "permissions:\n  contents: read" not in workflow:
        errors.append("workflow must use read-only contents permissions")
    if "runs-on: ubuntu-24.04" not in workflow or "timeout-minutes: 45" not in workflow:
        errors.append("adapter job runner and timeout are missing")
    if not re.search(r"(?m)^          toolchain: 1\.94\.0\s*$", workflow):
        errors.append("Rust 1.94.0 must be installed")
    if "uses: actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803" not in workflow:
        errors.append("checkout action must remain pinned")
    if "destination=\"$GITHUB_WORKSPACE/../$sibling\"" not in workflow:
        errors.append("pinned dependencies must be prepared beside the project")
    if "done < scripts/downstream-adapters-revisions.tsv" not in workflow:
        errors.append("dependency checkout must consume the revisions manifest")
    if 'fetch --quiet --depth=1 origin "$revision"' not in workflow:
        errors.append("dependency checkout must fetch the manifest revision")
    if '[[ "$actual" == "$revision" ]]' not in workflow:
        errors.append("dependency checkout must verify the fetched SHA")
    if not re.search(r"(?m)^        run: scripts/check-downstream-adapters\.sh\s*$", workflow):
        errors.append("workflow must invoke the real adapter checker")
    if "working-directory: ${{ github.workspace }}" not in workflow:
        errors.append("adapter checker must run from rs-config")
    if "set -euo pipefail" not in workflow:
        errors.append("dependency preparation must propagate failures")
    if re.search(r"(?m)^\s*continue-on-error:\s*true\s*$", workflow):
        errors.append("adapter failures must not be ignored")
    if re.search(r"(?m)^\s*if:.*(?:always\(\)|failure\(\))", workflow):
        errors.append("adapter job must not be conditionally skipped")

    entries = []
    canonical_rows = []
    for line_number, line in enumerate(revisions.splitlines(), start=1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 3:
            errors.append(f"invalid revision entry on line {line_number}")
            continue
        sibling, revision, repository = fields
        canonical_rows.append(line)
        if not re.fullmatch(r"[0-9a-f]{40}", revision):
            errors.append(f"invalid pinned SHA for {sibling}")
        if not re.fullmatch(r"rs-[a-z0-9-]+", sibling):
            errors.append(f"invalid sibling path for {sibling}")
        if repository != f"https://github.com/qubit-ltd/{sibling}.git":
            errors.append(f"unexpected repository URL for {sibling}")
        entries.append(sibling)
    if not entries:
        errors.append("revision manifest must contain dependencies")
    if len(entries) != 14:
        errors.append(f"revision manifest must contain 14 dependencies, found {len(entries)}")
    if len(entries) != len(set(entries)):
        errors.append("revision manifest contains duplicate sibling paths")
    canonical_manifest = "\n".join(sorted(canonical_rows)) + "\n"
    if hashlib.sha256(canonical_manifest.encode()).hexdigest() != FIXED_REVISION_MANIFEST_SHA256:
        errors.append("pinned dependency manifest contents differ from the expected fixed set")
    return errors


class DownstreamCIWiringTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.sample_dir = Path(self.temp.name)
        self.workflow_path = self.sample_dir / "workflow.yml"
        self.revisions_path = self.sample_dir / "revisions.tsv"
        self.workflow_path.write_text(WORKFLOW.read_text())
        self.revisions_path.write_text(REVISIONS.read_text())
        self.workflow = self.workflow_path.read_text()
        self.revisions = self.revisions_path.read_text()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def sample_errors(self, workflow: str, revisions: str) -> list[str]:
        self.workflow_path.write_text(workflow)
        self.revisions_path.write_text(revisions)
        return wiring_errors(self.workflow_path.read_text(), self.revisions_path.read_text())

    def assert_wired(self, workflow: str, revisions: str) -> None:
        self.assertEqual(self.sample_errors(workflow, revisions), [])

    def test_real_workflow_is_wired_to_pinned_downstream_contracts(self) -> None:
        self.assert_wired(self.workflow, self.revisions)

    def test_missing_checker_invocation_is_rejected(self) -> None:
        changed = self.workflow.replace(f"run: {CHECKER}", "run: echo skipped", 1)
        self.assertIn("workflow must invoke the real adapter checker", self.sample_errors(changed, self.revisions))

    def test_wrong_sha_or_unpinned_revision_is_rejected(self) -> None:
        changed = self.revisions.replace("f19ca48d114cb3223a079b043e4b49ef7c0b3c5d", "0" * 40, 1)
        self.assertIn("pinned dependency manifest contents differ from the expected fixed set", self.sample_errors(self.workflow, changed))
        malformed = self.revisions.replace("f19ca48d114cb3223a079b043e4b49ef7c0b3c5d", "main", 1)
        self.assertIn("invalid pinned SHA for rs-command", self.sample_errors(self.workflow, malformed))
        unpinned_fetch = self.workflow.replace('origin "$revision"', 'origin main', 1)
        self.assertIn("dependency checkout must fetch the manifest revision", self.sample_errors(unpinned_fetch, self.revisions))
        unchecked = self.workflow.replace('[[ "$actual" == "$revision" ]]', '[[ "$actual" == "$sibling" ]]', 1)
        self.assertIn("dependency checkout must verify the fetched SHA", self.sample_errors(unchecked, self.revisions))

    def test_missing_dependency_is_rejected(self) -> None:
        shortened_manifest = "\n".join(self.revisions.splitlines()[:-1]) + "\n"
        self.assertIn("revision manifest must contain 14 dependencies, found 13", self.sample_errors(self.workflow, shortened_manifest))

    def test_replaced_dependency_is_rejected(self) -> None:
        changed = self.revisions.replace("rs-command", "rs-other", 1).replace(
            "https://github.com/qubit-ltd/rs-command.git",
            "https://github.com/qubit-ltd/rs-other.git",
            1,
        )
        self.assertIn("pinned dependency manifest contents differ from the expected fixed set", self.sample_errors(self.workflow, changed))

    def test_wrong_working_directory_is_rejected(self) -> None:
        changed = self.workflow.replace("working-directory: ${{ github.workspace }}", "working-directory: .", 1)
        self.assertIn("adapter checker must run from rs-config", self.sample_errors(changed, self.revisions))

    def test_swallowed_failures_are_rejected(self) -> None:
        changed = self.workflow.replace("set -euo pipefail", "set +e", 1)
        self.assertIn("dependency preparation must propagate failures", self.sample_errors(changed, self.revisions))
        ignored = self.workflow.replace("timeout-minutes: 45", "timeout-minutes: 45\n    continue-on-error: true", 1)
        self.assertIn("adapter failures must not be ignored", self.sample_errors(ignored, self.revisions))


if __name__ == "__main__":
    unittest.main()
