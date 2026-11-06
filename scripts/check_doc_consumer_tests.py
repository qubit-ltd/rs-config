#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Regression tests for the documentation consumer checker."""

from __future__ import annotations

import importlib.machinery
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name("check-doc-consumer.py")
LOADER = importlib.machinery.SourceFileLoader("check_doc_consumer", str(SCRIPT))
SPEC = importlib.util.spec_from_loader(LOADER.name, LOADER)
assert SPEC is not None
CHECKER = importlib.util.module_from_spec(SPEC)
sys.modules[LOADER.name] = CHECKER
LOADER.exec_module(CHECKER)

VALID_TOML = '''<!-- consumer-dependencies: diagnostics -->
```toml
[dependencies]
qubit-config = "0.15"
qubit-datatype = { version = "0.15", default-features = false, features = ["converter"] }
```
'''
VALID_RUST = '''use qubit_config::{Config, ConfigError, ConfigErrorKind};
use qubit_datatype::DataType;
fn main() {}
'''
VALID_DOC = VALID_TOML + '''
<!-- example: config_error_diagnostics -->
```rust
'''+ VALID_RUST + '''```
'''


class DependencyBlockTests(unittest.TestCase):
    def test_valid_bilingual_blocks_are_accepted(self) -> None:
        docs = {"doc/user_guide.md": VALID_DOC, "doc/user_guide.zh_CN.md": VALID_DOC}
        self.assertEqual(CHECKER.validate_documents(docs), [])

    def test_missing_and_duplicate_markers_fail(self) -> None:
        self.assertTrue(
            CHECKER.validate_documents(
                {"doc/user_guide.md": "", "doc/user_guide.zh_CN.md": VALID_TOML}
            )
        )
        duplicate = VALID_TOML + "\n<!-- consumer-dependencies: diagnostics -->\n```toml\n[dependencies]\nqubit-config = \"0.15\"\n```\n"
        self.assertTrue(CHECKER.validate_documents({"doc/user_guide.md": duplicate, "doc/user_guide.zh_CN.md": VALID_TOML}))

    def test_invalid_toml_and_missing_datatype_fail(self) -> None:
        malformed = "<!-- consumer-dependencies: diagnostics -->\n```toml\n[dependencies\n```\n"
        self.assertTrue(
            CHECKER.validate_documents(
                {"doc/user_guide.md": malformed, "doc/user_guide.zh_CN.md": VALID_TOML}
            )
        )
        missing = VALID_TOML.replace('qubit-datatype = { version = "0.15", default-features = false, features = ["converter"] }\n', '')
        self.assertTrue(
            CHECKER.validate_documents(
                {"doc/user_guide.md": missing, "doc/user_guide.zh_CN.md": missing}
            )
        )

    def test_extra_direct_dependency_fails_even_when_bilingual_blocks_match(self) -> None:
        unexpected = VALID_TOML.replace(
            'qubit-config = "0.15"\n',
            'qubit-config = "0.15"\nserde = "1"\n',
        )
        errors = CHECKER.validate_documents(
            {"doc/user_guide.md": unexpected, "doc/user_guide.zh_CN.md": unexpected}
        )
        self.assertTrue(any("unexpected direct dependencies" in error for error in errors))

    def test_documented_dependency_versions_must_match_expected_values(self) -> None:
        for name, changed in (
            ("qubit-config", VALID_TOML.replace('qubit-config = "0.15"', 'qubit-config = "0.14"')),
            (
                "qubit-datatype",
                VALID_TOML.replace('version = "0.15", default-features', 'version = "0.14", default-features'),
            ),
        ):
            with self.subTest(dependency=name):
                errors = CHECKER.validate_documents(
                    {"doc/user_guide.md": changed, "doc/user_guide.zh_CN.md": changed}
                )
                self.assertTrue(any(f"{name}" in error and "expected" in error for error in errors))

    def test_bilingual_dependency_mismatch_fails(self) -> None:
        changed = VALID_TOML.replace('"0.15"', '"0.14"', 1)
        self.assertTrue(CHECKER.validate_documents({"doc/user_guide.md": VALID_TOML, "doc/user_guide.zh_CN.md": changed}))


class TemporaryConsumerTests(unittest.TestCase):
    def test_consumer_is_isolated_and_cargo_errors_fail(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "examples").mkdir()
            (root / "examples/config_error_diagnostics.rs").write_text(VALID_RUST, encoding="utf-8")
            docs = {"doc/user_guide.md": VALID_DOC, "doc/user_guide.zh_CN.md": VALID_DOC}
            manifests = []
            def success(command, *, cwd, **kwargs):
                manifests.append((cwd / "Cargo.toml").read_text(encoding="utf-8"))
                return type("Result", (), {"returncode": 0, "stdout": "", "stderr": ""})()
            with patch.object(CHECKER, "ROOT", root), patch.object(CHECKER.subprocess, "run", side_effect=success) as run:
                self.assertEqual(CHECKER.compile_document_consumers(docs), [])
            self.assertEqual(run.call_count, 4)
            commands = [call.args[0] for call in run.call_args_list]
            self.assertEqual(commands[0][:2], ["cargo", "generate-lockfile"])
            self.assertEqual(commands[1], ["cargo", "run", "--locked"])
            self.assertIn("[workspace]", manifests[0])
            self.assertNotIn("[dev-dependencies]", manifests[0])
            self.assertIn("qubit-datatype", manifests[0])
            self.assertNotIn("serde", manifests[0])
            with patch.object(CHECKER, "ROOT", root), patch.object(
                CHECKER.subprocess,
                "run",
                return_value=type(
                    "Result",
                    (),
                    {"returncode": 101, "stdout": "", "stderr": "cargo failed"},
                )(),
            ):
                errors = CHECKER.compile_document_consumers(docs)
            self.assertTrue(any("cargo failed" in error for error in errors))

    def test_missing_rust_example_marker_fails_before_cargo(self) -> None:
        malformed = VALID_TOML + "\n```rust\n" + VALID_RUST + "```\n"
        documents = {
            "doc/user_guide.md": malformed,
            "doc/user_guide.zh_CN.md": malformed,
        }
        for document in documents.values():
            block, error = CHECKER._marked_block(
                document, CHECKER.EXAMPLE_MARKER, "rust"
            )
            self.assertIsNone(block)
            self.assertIn("found 0", error or "")
        with patch.object(CHECKER.subprocess, "run") as run:
            errors = CHECKER.compile_document_consumers(documents)
        self.assertTrue(any("found 0" in error for error in errors))
        run.assert_not_called()

    def test_duplicate_rust_example_marker_fails_before_cargo(self) -> None:
        duplicate = "\n<!-- example: config_error_diagnostics -->\n```rust\nfn main() {}\n```\n"
        malformed = VALID_DOC + duplicate
        documents = {
            "doc/user_guide.md": malformed,
            "doc/user_guide.zh_CN.md": malformed,
        }
        for document in documents.values():
            block, error = CHECKER._marked_block(
                document, CHECKER.EXAMPLE_MARKER, "rust"
            )
            self.assertIsNone(block)
            self.assertIn("found 2", error or "")
        with patch.object(CHECKER.subprocess, "run") as run:
            errors = CHECKER.compile_document_consumers(documents)
        self.assertTrue(any("found 2" in error for error in errors))
        run.assert_not_called()

    def test_missing_source_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(CHECKER, "ROOT", root):
                errors = CHECKER.compile_document_consumers({"doc/user_guide.md": VALID_DOC, "doc/user_guide.zh_CN.md": VALID_DOC})
            self.assertTrue(any("source" in error.lower() for error in errors))


if __name__ == "__main__":
    unittest.main()
