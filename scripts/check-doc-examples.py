#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Validate stable IDs for the canonical Rust examples in bilingual docs."""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
EXAMPLE_DOCS = {
    "config_quickstart": ("README.md", "README.zh_CN.md"),
    "config_sources": ("README.md", "README.zh_CN.md"),
    "config_interpolation": ("doc/user_guide.md", "doc/user_guide.zh_CN.md"),
    "config_structured": ("README.md", "README.zh_CN.md"),
    "config_guide_quickstart": ("doc/user_guide.md", "doc/user_guide.zh_CN.md"),
    "config_sections": ("doc/user_guide.md", "doc/user_guide.zh_CN.md"),
    "config_policy_restore": ("doc/user_guide.md", "doc/user_guide.zh_CN.md"),
    "config_numeric_map_keys": ("doc/user_guide.md", "doc/user_guide.zh_CN.md"),
    "config_guide_sources": ("doc/user_guide.md", "doc/user_guide.zh_CN.md"),
}
DOCUMENTS = tuple(dict.fromkeys(path for paths in EXAMPLE_DOCS.values() for path in paths))
MARKER_PATTERN = re.compile(r"^<!-- example: (.*?) -->$")
EXAMPLE_ID_PATTERN = re.compile(r"^[a-z][a-z0-9_]*$")
RUST_FENCE_PATTERN = re.compile(r"^ {0,3}```rust(?:,[a-z0-9_-]+)*[ \t]*$")
RUST_FENCE_END_PATTERN = re.compile(r"^ {0,3}```[ \t]*$")
COPYRIGHT_HEADER = """// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

"""


@dataclass(frozen=True)
class MarkedExample:
    document: str
    line: int
    code: str


def parse_document(path: str) -> tuple[dict[str, MarkedExample], list[str]]:
    lines = (ROOT / path).read_text(encoding="utf-8").splitlines()
    examples: dict[str, MarkedExample] = {}
    errors: list[str] = []

    for index, line in enumerate(lines):
        marker = MARKER_PATTERN.fullmatch(line)
        if marker is None:
            continue

        example_id = marker.group(1)
        if EXAMPLE_ID_PATTERN.fullmatch(example_id) is None:
            errors.append(f"{path}:{index + 1}: invalid example ID '{example_id}'")
            continue
        if example_id in examples:
            errors.append(f"{path}:{index + 1}: duplicate example ID '{example_id}'")
            continue

        fence_index = index + 1
        if fence_index >= len(lines) or RUST_FENCE_PATTERN.fullmatch(lines[fence_index]) is None:
            errors.append(
                f"{path}:{index + 1}: example ID '{example_id}' is not followed by a Rust fence"
            )
            continue

        closing_index = fence_index + 1
        while closing_index < len(lines) and RUST_FENCE_END_PATTERN.fullmatch(lines[closing_index]) is None:
            closing_index += 1
        if closing_index == len(lines):
            errors.append(f"{path}:{index + 1}: example ID '{example_id}' has an unclosed Rust fence")
            continue

        code = "\n".join(lines[fence_index + 1 : closing_index]) + "\n"
        examples[example_id] = MarkedExample(path, index + 1, code)

    return examples, errors


def hidden_rust_lines(text: str) -> list[int]:
    """Return line numbers of rustdoc-hidden lines inside Rust fences."""
    hidden: list[int] = []
    in_rust_fence = False
    for line_number, line in enumerate(text.splitlines(), start=1):
        if not in_rust_fence and RUST_FENCE_PATTERN.fullmatch(line):
            in_rust_fence = True
        elif in_rust_fence and RUST_FENCE_END_PATTERN.fullmatch(line):
            in_rust_fence = False
        elif in_rust_fence and re.match(r"^\s*#(?:[ \t]|$)", line):
            hidden.append(line_number)
    return hidden


def compare_documented_code(
    example_id: str, english: MarkedExample, chinese: MarkedExample
) -> list[str]:
    if english.code == chinese.code:
        return []
    return [
        f"{chinese.document}:{chinese.line}: example ID '{example_id}' differs from "
        f"{english.document}:{english.line}"
    ]


def check_examples() -> list[str]:
    errors: list[str] = []
    by_document: dict[str, dict[str, MarkedExample]] = {}
    occurrences: dict[str, list[MarkedExample]] = {}

    for document in DOCUMENTS:
        examples, parse_errors = parse_document(document)
        by_document[document] = examples
        errors.extend(parse_errors)
        if document in ("doc/user_guide.md", "doc/user_guide.zh_CN.md"):
            text = (ROOT / document).read_text(encoding="utf-8")
            errors.extend(
                f"{document}:{line}: rustdoc-hidden lines are not allowed in user-guide Rust fences"
                for line in hidden_rust_lines(text)
            )
        for example_id, example in examples.items():
            occurrences.setdefault(example_id, []).append(example)

    expected_ids = set(EXAMPLE_DOCS)
    for example_id, examples in sorted(occurrences.items()):
        if example_id not in expected_ids:
            for example in examples:
                errors.append(
                    f"{example.document}:{example.line}: unknown example ID '{example_id}'"
                )

    for example_id, (english_path, chinese_path) in EXAMPLE_DOCS.items():
        example_path = ROOT / "examples" / f"{example_id}.rs"
        canonical_code: str | None = None
        if not example_path.is_file():
            errors.append(f"{english_path}: example ID '{example_id}' has no file at {example_path.relative_to(ROOT)}")
        else:
            example_code = example_path.read_text(encoding="utf-8")
            if not example_code.startswith(COPYRIGHT_HEADER):
                errors.append(
                    f"{example_path.relative_to(ROOT)}: example ID '{example_id}' "
                    "does not start with the canonical copyright header"
                )
            else:
                canonical_code = example_code.removeprefix(COPYRIGHT_HEADER)

        english = by_document[english_path].get(example_id)
        chinese = by_document[chinese_path].get(example_id)
        if english is None:
            errors.append(f"{english_path}: missing example ID '{example_id}'")
        if chinese is None:
            errors.append(f"{chinese_path}: missing example ID '{example_id}'")
        if english is not None and chinese is not None and english.code != chinese.code:
            errors.extend(compare_documented_code(example_id, english, chinese))
        for documented in (english, chinese):
            if documented is not None and canonical_code is not None and documented.code != canonical_code:
                errors.append(
                    f"{documented.document}:{documented.line}: example ID '{example_id}' "
                    f"differs from canonical {example_path.relative_to(ROOT)}"
                )

        actual_paths = {example.document for example in occurrences.get(example_id, [])}
        expected_paths = {english_path, chinese_path}
        for unexpected_path in sorted(actual_paths - expected_paths):
            example = by_document[unexpected_path][example_id]
            errors.append(
                f"{unexpected_path}:{example.line}: example ID '{example_id}' belongs in "
                f"{english_path} and {chinese_path}"
            )

    example_dir = ROOT / "examples"
    for example_path in sorted(example_dir.glob("*.rs")):
        if example_path.stem not in expected_ids:
            errors.append(f"{example_path.relative_to(ROOT)}: no document marker for example ID '{example_path.stem}'")

    return errors


def main() -> int:
    errors = check_examples()
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    print(f"checked {len(EXAMPLE_DOCS)} canonical example IDs in {len(DOCUMENTS)} documents")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
