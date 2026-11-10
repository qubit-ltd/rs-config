#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Regression tests for pinned design evidence links."""

from __future__ import annotations

import importlib.machinery
import importlib.util
from pathlib import Path
import sys
import unittest


SCRIPT = Path(__file__).with_name("check-design-evidence.py")
LOADER = importlib.machinery.SourceFileLoader("check_design_evidence", str(SCRIPT))
SPEC = importlib.util.spec_from_loader(LOADER.name, LOADER)
assert SPEC is not None
CHECKER = importlib.util.module_from_spec(SPEC)
sys.modules[LOADER.name] = CHECKER
LOADER.exec_module(CHECKER)


COMMIT = "827e2b8ffbae88238be1aba4fd42c0922f5f86e5"
REPORTS = (
    "resource-probe-20261101T042738659Z-7be245fc.md",
    "resource-probe-20261101T124404Z-rsc101.md",
)
DOCS = ("doc/design.md", "doc/design.zh_CN.md")


def valid_documents() -> dict[str, str]:
    return {
        path: "\n".join(
            f"[Evidence {i}](https://github.com/qubit-ltd/rs-config/blob/{COMMIT}/doc/{report})"
            for i, report in enumerate(REPORTS, start=1)
        )
        for path in DOCS
    }


class DesignEvidenceTests(unittest.TestCase):
    def test_accepts_correct_bilingual_text(self) -> None:
        self.assertEqual(CHECKER.validate_documents(valid_documents()), [])

    def test_rejects_main_floating_link(self) -> None:
        documents = valid_documents()
        documents[DOCS[0]] = documents[DOCS[0]].replace(COMMIT, "main")
        self.assertTrue(CHECKER.validate_documents(documents))

    def test_rejects_missing_reference(self) -> None:
        documents = valid_documents()
        documents[DOCS[0]] = documents[DOCS[0]].replace(
            f"[Evidence 2](https://github.com/qubit-ltd/rs-config/blob/{COMMIT}/doc/{REPORTS[1]})", ""
        )
        self.assertTrue(CHECKER.validate_documents(documents))

    def test_rejects_wrong_commit(self) -> None:
        documents = valid_documents()
        documents[DOCS[0]] = documents[DOCS[0]].replace(COMMIT, "827e2b8")
        self.assertTrue(CHECKER.validate_documents(documents))

    def test_rejects_wrong_report_path(self) -> None:
        documents = valid_documents()
        documents[DOCS[0]] = documents[DOCS[0]].replace(REPORTS[0], "other-report.md")
        self.assertTrue(CHECKER.validate_documents(documents))

    def test_rejects_duplicate_reference(self) -> None:
        documents = valid_documents()
        documents[DOCS[0]] += "\n" + documents[DOCS[0]].splitlines()[0]
        self.assertTrue(CHECKER.validate_documents(documents))

    def test_accepts_different_bilingual_link_titles(self) -> None:
        documents = valid_documents()
        documents[DOCS[1]] = documents[DOCS[1]].replace("Evidence", "证据材料")
        self.assertEqual(CHECKER.validate_documents(documents), [])

    def test_rejects_missing_document(self) -> None:
        documents = valid_documents()
        del documents[DOCS[1]]
        self.assertTrue(CHECKER.validate_documents(documents))


if __name__ == "__main__":
    unittest.main()
