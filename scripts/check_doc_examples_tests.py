#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Unit tests for the documentation example checker."""

from __future__ import annotations

import importlib.machinery
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("check-doc-examples.py")
LOADER = importlib.machinery.SourceFileLoader("check_doc_examples", str(SCRIPT))
SPEC = importlib.util.spec_from_loader(LOADER.name, LOADER)
assert SPEC is not None
CHECKER = importlib.util.module_from_spec(SPEC)
sys.modules[LOADER.name] = CHECKER
LOADER.exec_module(CHECKER)


class HiddenRustLinesTests(unittest.TestCase):
    def test_hidden_import_and_tail_lines_are_reported(self) -> None:
        text = "Title\n```rust\n# use qubit_config::Config;\nfn main() {\n# let _ = Config::new();\n}\n```\n"
        self.assertEqual(CHECKER.hidden_rust_lines(text), [3, 5])

    def test_attributes_and_headings_outside_fence_are_allowed(self) -> None:
        text = "# Guide title\n```rust\n#[derive(Debug)]\nstruct Item;\n```\n"
        self.assertEqual(CHECKER.hidden_rust_lines(text), [])

    def test_rust_fence_with_trailing_whitespace_is_scanned(self) -> None:
        text = " ```rust  \n# use qubit_config::Config;\n  ``` \t\n# outside\n"
        self.assertEqual(CHECKER.hidden_rust_lines(text), [2])

    def test_documented_example_mismatch_is_reported(self) -> None:
        english = CHECKER.MarkedExample("guide.md", 2, "fn main() {}\n")
        chinese = CHECKER.MarkedExample("guide.zh_CN.md", 4, "fn main() { println!(\"x\"); }\n")
        errors = CHECKER.compare_documented_code("config_sections", english, chinese)
        self.assertEqual(len(errors), 1)
        self.assertIn("config_sections", errors[0])


class RequiredGuideExamplesTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.patch = patch.object(CHECKER, "ROOT", self.root)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        for document in CHECKER.DOCUMENTS:
            path = self.root / document
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("# Fixture\n", encoding="utf-8")
        (self.root / "examples").mkdir()
        for example_id, documents in CHECKER.EXAMPLE_DOCS.items():
            code = f"fn main() {{ /* {example_id} */ }}\n"
            (self.root / "examples" / f"{example_id}.rs").write_text(
                CHECKER.COPYRIGHT_HEADER + code, encoding="utf-8"
            )
            for document in documents:
                with (self.root / document).open("a", encoding="utf-8") as stream:
                    stream.write(f"<!-- example: {example_id} -->\n```rust\n{code}```\n")

    def test_new_guide_examples_are_required(self) -> None:
        for example_id in (
            "config_policy_restore", "config_numeric_map_keys", "config_guide_sources"
        ):
            self.assertEqual(
                CHECKER.EXAMPLE_DOCS[example_id],
                ("doc/user_guide.md", "doc/user_guide.zh_CN.md"),
            )
        self.assertEqual(CHECKER.check_examples(), [])

    def test_missing_chinese_marker_is_reported(self) -> None:
        path = self.root / "doc/user_guide.zh_CN.md"
        path.write_text(path.read_text(encoding="utf-8").replace(
            "<!-- example: config_policy_restore -->\n", ""
        ), encoding="utf-8")
        self.assertTrue(any(
            "missing example ID 'config_policy_restore'" in error
            for error in CHECKER.check_examples()
        ))

    def test_changed_chinese_code_is_reported(self) -> None:
        path = self.root / "doc/user_guide.zh_CN.md"
        path.write_text(path.read_text(encoding="utf-8").replace(
            "/* config_numeric_map_keys */", "/* changed */"
        ), encoding="utf-8")
        self.assertTrue(any(
            "config_numeric_map_keys" in error and "differs" in error
            for error in CHECKER.check_examples()
        ))

    def test_missing_canonical_file_is_reported(self) -> None:
        (self.root / "examples/config_guide_sources.rs").unlink()
        self.assertTrue(any(
            "config_guide_sources" in error and "has no file" in error
            for error in CHECKER.check_examples()
        ))


if __name__ == "__main__":
    unittest.main()
