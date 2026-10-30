#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Unit tests for the documentation example checker."""

from __future__ import annotations

import importlib.machinery
import importlib.util
from pathlib import Path
import sys
import unittest


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


if __name__ == "__main__":
    unittest.main()
