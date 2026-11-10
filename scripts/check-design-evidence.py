#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Check that formal design documents pin their resource evidence links."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
COMMIT = "827e2b8ffbae88238be1aba4fd42c0922f5f86e5"
REPORTS = (
    "resource-probe-20261101T042738659Z-7be245fc.md",
    "resource-probe-20261101T124404Z-rsc101.md",
)
DOCUMENTS = ("doc/design.md", "doc/design.zh_CN.md")
LINK_RE = re.compile(r"\[[^\]]*\]\(([^)]+)\)")


def validate_documents(documents: dict[str, str]) -> list[str]:
    """Return errors unless each design document links once to each pinned report."""
    errors: list[str] = []
    for document in DOCUMENTS:
        content = documents.get(document)
        if content is None:
            errors.append(f"{document}: required design document is missing")
            continue

        links = LINK_RE.findall(content)
        for report in REPORTS:
            matching = [url for url in links if url.split("?", 1)[0].endswith(report)]
            if not matching:
                errors.append(f"{document}: missing reference to {report}")
                continue
            if len(matching) != 1:
                errors.append(
                    f"{document}: expected exactly one reference to {report}, found {len(matching)}"
                )
            expected = (
                f"https://github.com/qubit-ltd/rs-config/blob/{COMMIT}/doc/{report}"
            )
            for url in matching:
                if url != expected:
                    errors.append(
                        f"{document}: {report} must link to {expected}; found {url}"
                    )
    return errors


def main() -> int:
    documents: dict[str, str] = {}
    for document in DOCUMENTS:
        path = ROOT / document
        try:
            documents[document] = path.read_text(encoding="utf-8")
        except OSError as error:
            print(f"{document}: could not read required design document: {error}", file=sys.stderr)
            return 1
        except UnicodeError as error:
            print(f"{document}: could not decode required design document as UTF-8: {error}", file=sys.stderr)
            return 1

    errors = validate_documents(documents)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
