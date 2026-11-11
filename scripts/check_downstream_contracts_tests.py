#!/usr/bin/env python3
"""Unit tests for downstream fixture selection and failure propagation."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("check-downstream-contracts.py")
SPEC = importlib.util.spec_from_file_location("check_downstream_contracts", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
checker = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = checker
SPEC.loader.exec_module(checker)


class MetadataValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.workspace = Path("/tmp/contract-root")
        self.metadata = {
            "packages": [
                {"id": "config 0.15.0 (path+file:///tmp/contract-root/rs-config)", "name": "qubit-config", "manifest_path": "/tmp/contract-root/rs-config/Cargo.toml"},
                {"id": "http 0.16.0 (path+file:///tmp/contract-root/rs-http)", "name": "qubit-http", "manifest_path": "/tmp/contract-root/rs-http/Cargo.toml"},
                {"id": "ioc 0.4.0 (path+file:///tmp/contract-root/rs-ioc)", "name": "qubit-ioc", "manifest_path": "/tmp/contract-root/rs-ioc/Cargo.toml"},
                {"id": "websocket 0.3.0 (path+file:///tmp/contract-root/rs-websocket)", "name": "qubit-websocket", "manifest_path": "/tmp/contract-root/rs-websocket/Cargo.toml"},
                {"id": "tungstenite 0.26.2 (git+https://github.com/qubit-ltd/tungstenite-rs?rev=b24a4442be0247a71fe9ee4e520eb483ac1b2873#b24a4442be0247a71fe9ee4e520eb483ac1b2873)", "name": "tungstenite", "manifest_path": "/tmp/tungstenite/Cargo.toml", "source": "git+https://github.com/qubit-ltd/tungstenite-rs?rev=b24a4442be0247a71fe9ee4e520eb483ac1b2873#b24a4442be0247a71fe9ee4e520eb483ac1b2873"},
            ],
            "resolve": {
                "nodes": [
                    {"id": "config 0.15.0 (path+file:///tmp/contract-root/rs-config)", "features": [], "deps": []},
                    {"id": "http 0.16.0 (path+file:///tmp/contract-root/rs-http)", "features": [], "deps": [{"name": "qubit_config", "pkg": "config 0.15.0 (path+file:///tmp/contract-root/rs-config)"}]},
                    {"id": "ioc 0.4.0 (path+file:///tmp/contract-root/rs-ioc)", "features": ["config"], "deps": [{"name": "qubit_config", "pkg": "config 0.15.0 (path+file:///tmp/contract-root/rs-config)"}]},
                    {"id": "websocket 0.3.0 (path+file:///tmp/contract-root/rs-websocket)", "features": ["config"], "deps": [{"name": "qubit_config", "pkg": "config 0.15.0 (path+file:///tmp/contract-root/rs-config)"}]},
                ]
            },
        }

    def test_accepts_both_local_adapter_edges_in_minimal_mode(self) -> None:
        checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_accepts_full_feature_only_when_enabled(self) -> None:
        metadata = self.metadata.copy()
        metadata["packages"] = [dict(package) for package in self.metadata["packages"]]
        metadata["resolve"] = {"nodes": [dict(node, deps=list(node["deps"])) for node in self.metadata["resolve"]["nodes"]]}
        metadata["resolve"]["nodes"][0]["features"] = ["full"]
        checker.validate_metadata(metadata, self.workspace, full=True)

    def test_rejects_full_mode_without_forwarded_feature(self) -> None:
        with self.assertRaisesRegex(ValueError, "did not enable qubit-config/full"):
            checker.validate_metadata(self.metadata, self.workspace, full=True)

    def test_rejects_full_leakage_into_minimal_mode(self) -> None:
        self.metadata["resolve"]["nodes"][0]["features"] = ["full"]
        with self.assertRaisesRegex(ValueError, "unexpectedly enabled"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_rejects_registry_config_substitute(self) -> None:
        self.metadata["packages"][0]["manifest_path"] = "/home/runner/.cargo/registry/qubit-config/Cargo.toml"
        with self.assertRaisesRegex(ValueError, "prepared rs-config"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_rejects_duplicate_config_versions(self) -> None:
        self.metadata["packages"].append({"id": "config 0.14.0 (registry)", "name": "qubit-config", "manifest_path": "/registry/Cargo.toml"})
        with self.assertRaisesRegex(ValueError, "exactly one qubit-config"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_rejects_ioc_without_config_feature(self) -> None:
        self.metadata["resolve"]["nodes"][2]["features"] = []
        with self.assertRaisesRegex(ValueError, "config feature"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_rejects_missing_adapter_node(self) -> None:
        self.metadata["packages"].pop(2)
        with self.assertRaisesRegex(ValueError, "exactly one qubit-ioc"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_rejects_missing_config_dependency_edge(self) -> None:
        self.metadata["resolve"]["nodes"][1]["deps"] = []
        with self.assertRaisesRegex(ValueError, "qubit-http does not depend"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_accepts_websocket_source_and_config_edge(self) -> None:
        checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_rejects_missing_websocket_node(self) -> None:
        self.metadata["packages"] = [package for package in self.metadata["packages"] if package["name"] != "qubit-websocket"]
        with self.assertRaisesRegex(ValueError, "exactly one qubit-websocket"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_rejects_duplicate_websocket_node(self) -> None:
        self.metadata["packages"].append(dict(self.metadata["packages"][3]))
        with self.assertRaisesRegex(ValueError, "exactly one qubit-websocket"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_rejects_websocket_manifest_outside_prepared_checkout(self) -> None:
        self.metadata["packages"][3]["manifest_path"] = "/registry/qubit-websocket/Cargo.toml"
        with self.assertRaisesRegex(ValueError, "prepared rs-websocket"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_rejects_websocket_without_config_feature(self) -> None:
        self.metadata["resolve"]["nodes"][3]["features"] = []
        with self.assertRaisesRegex(ValueError, "qubit-websocket config feature"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_rejects_websocket_edge_to_other_config_node(self) -> None:
        self.metadata["resolve"]["nodes"][3]["deps"][0]["pkg"] = "config 0.14 (registry)"
        with self.assertRaisesRegex(ValueError, "qubit-websocket does not depend exclusively"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)

    def test_rejects_tungstenite_patch_source_or_revision_mismatch(self) -> None:
        self.metadata["packages"][4]["source"] = "registry+https://github.com/rust-lang/crates.io-index"
        with self.assertRaisesRegex(ValueError, "tungstenite patch source/revision mismatch"):
            checker.validate_metadata(self.metadata, self.workspace, full=False)


class FailurePropagationTests(unittest.TestCase):
    def test_failed_child_command_is_nonzero_from_main(self) -> None:
        failure = subprocess.CalledProcessError(23, ["cargo", "test"], stderr="fixture failed")
        with patch.object(sys, "argv", [str(SCRIPT)]), patch.object(checker, "execute", side_effect=failure):
            self.assertEqual(checker.main(), 23)

    def test_command_runner_uses_check_and_argument_array(self) -> None:
        failure = subprocess.CalledProcessError(9, ["cargo", "metadata"])
        with patch.object(checker.subprocess, "run", side_effect=failure) as mocked_run:
            with self.assertRaises(subprocess.CalledProcessError):
                checker.run(["cargo", "metadata", "--format-version", "1"], cwd=Path("/tmp"))
        self.assertEqual(mocked_run.call_args.args[0], ["cargo", "metadata", "--format-version", "1"])
        self.assertTrue(mocked_run.call_args.kwargs["check"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
