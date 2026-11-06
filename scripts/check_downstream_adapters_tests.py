#!/usr/bin/env python3
"""Regression tests for the pinned downstream adapter preflight."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[1]
CHECKER = REPO_ROOT / "scripts" / "check-downstream-adapters.sh"
REVISION = "0123456789abcdef0123456789abcdef01234567"


class DownstreamAdapterCheckerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / "root"
        self.project = self.root / "rs-config"
        self.sibling = self.root / "rs-fixture"
        self.bin = Path(self.temp.name) / "bin"
        self.bin.mkdir()
        self.cargo_log = Path(self.temp.name) / "cargo.log"
        (self.project / "scripts").mkdir(parents=True)
        (self.project / "tests/fixtures/downstream_adapters").mkdir(parents=True)
        shutil.copy2(CHECKER, self.project / "scripts/check-downstream-adapters.sh")
        shutil.copy2(
            CHECKER.with_name("check-config-features.py"),
            self.project / "scripts/check-config-features.py",
        )
        shutil.copy2(REPO_ROOT / "Cargo.toml", self.project / "Cargo.toml")
        (self.project / "tests/fixtures/downstream_adapters/Cargo.toml").write_text(
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
        )
        (self.project / "scripts/downstream-adapters-revisions.tsv").write_text(
            f"rs-fixture\t{REVISION}\thttps://github.com/qubit-ltd/rs-fixture.git\n"
        )
        cargo = self.bin / "cargo"
        cargo.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            "args = sys.argv[1:]\n"
            "with open(os.environ['CARGO_LOG'], 'a') as log: log.write(' '.join(args) + '\\n')\n"
            "if args and args[0] == 'metadata':\n"
            "    if os.environ.get('FAIL_METADATA'): sys.exit(9)\n"
            "    with open(os.environ['METADATA_FILE']) as source: data = json.load(source)\n"
            "    full = '--features' in args and 'full' in args\n"
            "    data['resolve']['nodes'][0]['features'] = list(data['all_features'] if full else [])\n"
            "    if os.environ.get('POLLUTE_MINIMAL') and not full: data['resolve']['nodes'][0]['features'] = ['yaml']\n"
            "    data.pop('all_features')\n"
            "    print(json.dumps(data))\n"
        )
        cargo.chmod(0o755)
        self._git("init", "-q", str(self.sibling), cwd=self.root)
        self._git("config", "user.name", "Checker Fixture", cwd=self.sibling)
        self._git("config", "user.email", "checker-fixture@example.invalid", cwd=self.sibling)
        (self.sibling / "tracked.txt").write_text("fixture\n")
        self._git("add", "tracked.txt", cwd=self.sibling)
        self._git("commit", "-qm", "fixture", cwd=self.sibling)
        self.commit = self._git("rev-parse", "HEAD", cwd=self.sibling).stdout.strip()
        (self.project / "scripts/downstream-adapters-revisions.tsv").write_text(
            f"rs-fixture\t{self.commit}\thttps://github.com/qubit-ltd/rs-fixture.git\n"
        )
        self.metadata_file = Path(self.temp.name) / "metadata.json"
        import tomllib

        with (self.project / "Cargo.toml").open("rb") as source:
            root_manifest = tomllib.load(source)
        all_features = sorted(set(root_manifest["features"]) - {"default"})
        package_id = f"path+file://{self.project}#qubit-config@0.15.0"
        self.metadata_file.write_text(json.dumps({
            "packages": [{"id": package_id, "name": "qubit-config", "version": "0.15.0",
                          "manifest_path": str(self.project / "Cargo.toml")}],
            "resolve": {"nodes": [{"id": package_id, "features": []}]},
            "all_features": all_features,
        }))

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _git(self, *args: str, cwd: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", *args], cwd=cwd, check=True, text=True, capture_output=True,
            env={**os.environ, "GIT_ALLOW_PROTOCOL": "file:https"},
        )

    def _run(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["bash", str(self.project / "scripts/check-downstream-adapters.sh")],
            cwd=self.root, text=True, capture_output=True,
            env={**os.environ, "PATH": f"{self.bin}{os.pathsep}{os.environ['PATH']}",
                 "CARGO_LOG": str(self.cargo_log), "METADATA_FILE": str(self.metadata_file)},
        )

    def _assert_clean_success(self) -> None:
        result = self._run()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.cargo_log.read_text().splitlines()
        self.assertEqual(len(calls), 8, calls)
        self.assertEqual(sum(call.startswith("metadata ") for call in calls), 2, calls)
        self.assertEqual(sum(call.startswith("tree ") for call in calls), 4, calls)
        self.assertEqual(sum(call.startswith("test ") for call in calls), 2, calls)
        selected = [call.split() for call in calls if "--features" in call]
        self.assertEqual(len(selected), 4, calls)
        self.assertTrue(all(call[call.index("--features") + 1] == "full" for call in selected), calls)
        self.assertEqual(sum("--features" not in call for call in calls), 4, calls)
        self.assertTrue(all("--no-default-features" in call for call in calls), calls)
        expected_manifest = str(self.project / "tests/fixtures/downstream_adapters/Cargo.toml")
        self.assertTrue(all(expected_manifest in call for call in calls), calls)

    def _assert_dirty_rejected(self) -> None:
        result = self._run()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("rs-fixture worktree differs from pinned revision", result.stderr)
        self.assertFalse(self.cargo_log.exists(), self.cargo_log.read_text() if self.cargo_log.exists() else "")

    def test_clean_worktree_runs_expected_cargo_commands(self) -> None:
        self._assert_clean_success()

    def test_ignored_only_worktree_runs_expected_cargo_commands(self) -> None:
        (self.sibling / ".gitignore").write_text("ignored.txt\n")
        self._git("add", ".gitignore", cwd=self.sibling)
        self._git("commit", "-qm", "ignore fixture", cwd=self.sibling)
        (self.sibling / "ignored.txt").write_text("ignored\n")
        # Update the pin to this fixture commit; the ignored file remains invisible.
        self._set_pin(self._git("rev-parse", "HEAD", cwd=self.sibling).stdout.strip())
        self._assert_clean_success()

    def _set_pin(self, revision: str) -> None:
        (self.project / "scripts/downstream-adapters-revisions.tsv").write_text(
            f"rs-fixture\t{revision}\thttps://github.com/qubit-ltd/rs-fixture.git\n"
        )

    def test_metadata_failure_blocks_adapter_tests(self) -> None:
        result = self._run_with_env(FAIL_METADATA="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(any(call.startswith("test ") for call in self.cargo_log.read_text().splitlines()))

    def test_polluted_minimal_feature_graph_blocks_adapter_tests(self) -> None:
        result = self._run_with_env(POLLUTE_MINIMAL="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("enabled optional features", result.stderr)
        self.assertFalse(any(call.startswith("test ") for call in self.cargo_log.read_text().splitlines()))

    def _run_with_env(self, **extra: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["bash", str(self.project / "scripts/check-downstream-adapters.sh")],
            cwd=self.root, text=True, capture_output=True,
            env={**os.environ, "PATH": f"{self.bin}{os.pathsep}{os.environ['PATH']}",
                 "CARGO_LOG": str(self.cargo_log), "METADATA_FILE": str(self.metadata_file), **extra},
        )

    def test_unstaged_modification_is_rejected_before_cargo(self) -> None:
        (self.sibling / "tracked.txt").write_text("modified\n")
        self._assert_dirty_rejected()

    def test_staged_modification_is_rejected_before_cargo(self) -> None:
        (self.sibling / "tracked.txt").write_text("staged\n")
        self._git("add", "tracked.txt", cwd=self.sibling)
        self._assert_dirty_rejected()

    def test_deletion_is_rejected_before_cargo(self) -> None:
        (self.sibling / "tracked.txt").unlink()
        self._assert_dirty_rejected()

    def test_untracked_file_is_rejected_before_cargo(self) -> None:
        (self.sibling / "untracked.txt").write_text("untracked\n")
        self._assert_dirty_rejected()

    def test_head_mismatch_remains_rejected(self) -> None:
        self._set_pin("f" * 40)
        result = self._run()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("rs-fixture revision mismatch", result.stderr)
        self.assertFalse(self.cargo_log.exists())

    def test_missing_repository_remains_rejected(self) -> None:
        shutil.rmtree(self.sibling)
        result = self._run()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("required sibling repository is missing", result.stderr)
        self.assertFalse(self.cargo_log.exists())

    def test_status_failure_fails_closed(self) -> None:
        real_git = shutil.which("git")
        self.assertIsNotNone(real_git)
        proxy = self.bin / "git"
        proxy.write_text(
            f"#!/bin/sh\nfor arg in \"$@\"; do [ \"$arg\" = status ] && exit 77; done\nexec {real_git} \"$@\"\n"
        )
        proxy.chmod(0o755)
        result = self._run()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cannot inspect worktree for rs-fixture", result.stderr)
        self.assertFalse(self.cargo_log.exists())

    def test_dirty_submodule_is_rejected_when_local_submodules_are_supported(self) -> None:
        submodule = Path(self.temp.name) / "submodule"
        submodule.mkdir()
        self._git("init", "-q", str(submodule), cwd=self.root)
        self._git("config", "user.name", "Checker Fixture", cwd=submodule)
        self._git("config", "user.email", "checker-fixture@example.invalid", cwd=submodule)
        (submodule / "inside.txt").write_text("clean\n")
        self._git("add", "inside.txt", cwd=submodule)
        self._git("commit", "-qm", "submodule fixture", cwd=submodule)
        try:
            self._git("-c", "protocol.file.allow=always", "submodule", "add", "-q", str(submodule), "nested", cwd=self.sibling)
            self._git("add", ".gitmodules", "nested", cwd=self.sibling)
            self._git("commit", "-qm", "add submodule", cwd=self.sibling)
        except subprocess.CalledProcessError as error:
            self.skipTest(f"local Git submodule setup unavailable: {error.stderr}")
        self._set_pin(self._git("rev-parse", "HEAD", cwd=self.sibling).stdout.strip())
        (self.sibling / "nested/inside.txt").write_text("dirty\n")
        self._assert_dirty_rejected()


if __name__ == "__main__":
    unittest.main()
