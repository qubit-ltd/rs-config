#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Compile marked bilingual examples using the checked-in consumer lock baseline.

To intentionally refresh that baseline, run
`cargo +1.94.0 generate-lockfile --manifest-path tests/fixtures/doc_consumer/Cargo.toml`,
then run `python3 -B scripts/check_doc_consumer_tests.py` and this checker.
Normal checks never resolve or update dependencies.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCUMENTS = ("doc/user_guide.md", "doc/user_guide.zh_CN.md")
DEPENDENCY_MARKER = "<!-- consumer-dependencies: diagnostics -->"
EXAMPLE_MARKER = "<!-- example: config_error_diagnostics -->"
EXPECTED_DEPENDENCIES = {
    "qubit-config": "0.15",
    "qubit-datatype": {
        "version": "0.15",
        "default-features": False,
        "features": ["converter"],
    },
}
FIXTURE_MANIFEST = Path("tests/fixtures/doc_consumer/Cargo.toml")
FIXTURE_LOCK = Path("tests/fixtures/doc_consumer/Cargo.lock")


def _marked_block(text: str, marker: str, language: str) -> tuple[str | None, str | None]:
    lines = text.splitlines()
    indexes = [index for index, line in enumerate(lines) if line == marker]
    if len(indexes) != 1:
        return None, f"expected exactly one marker {marker!r}, found {len(indexes)}"
    index = indexes[0] + 1
    if index >= len(lines) or lines[index] != f"```{language}":
        return None, f"marker {marker!r} is not followed by a {language} fence"
    end = index + 1
    while end < len(lines) and lines[end] != "```":
        end += 1
    if end == len(lines):
        return None, f"marker {marker!r} has an unclosed {language} fence"
    return "\n".join(lines[index + 1 : end]) + "\n", None


def validate_documents(documents: dict[str, str]) -> list[str]:
    errors: list[str] = []
    parsed: dict[str, dict] = {}
    for path, text in documents.items():
        block, error = _marked_block(text, DEPENDENCY_MARKER, "toml")
        if error:
            errors.append(f"{path}: {error}")
            continue
        assert block is not None
        try:
            data = tomllib.loads(block)
        except tomllib.TOMLDecodeError as exc:
            errors.append(f"{path}: invalid dependency TOML: {exc}")
            continue
        dependencies = data.get("dependencies")
        if not isinstance(dependencies, dict):
            errors.append(f"{path}: dependency block must define [dependencies]")
            continue
        if set(data) != {"dependencies"}:
            extra_tables = ", ".join(sorted(set(data) - {"dependencies"}))
            errors.append(f"{path}: unexpected TOML tables outside [dependencies]: {extra_tables}")
        actual_names = set(dependencies)
        expected_names = set(EXPECTED_DEPENDENCIES)
        missing = expected_names - actual_names
        extra = actual_names - expected_names
        if missing:
            errors.append(f"{path}: missing direct dependencies: {', '.join(sorted(missing))}")
        if extra:
            errors.append(f"{path}: unexpected direct dependencies: {', '.join(sorted(extra))}")
        for name in sorted(expected_names & actual_names):
            expected = EXPECTED_DEPENDENCIES[name]
            actual = dependencies[name]
            if actual != expected:
                errors.append(f"{path}: {name} declaration expected {expected!r}, found {actual!r}")
        parsed[path] = dependencies
    if len(parsed) == len(documents) and len(set(map(repr, parsed.values()))) != 1:
        errors.append("bilingual diagnostic dependency declarations differ")
    return errors


def _toml_value(value: object) -> str:
    if isinstance(value, str):
        return json.dumps(value)
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, list):
        return "[" + ", ".join(_toml_value(item) for item in value) + "]"
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{key} = {_toml_value(item)}" for key, item in value.items()) + " }"
    if isinstance(value, (int, float)):
        return str(value)
    raise TypeError(f"unsupported TOML value: {value!r}")


def _manifest(dependencies: dict, root: Path) -> str:
    lines = [
        "[package]",
        'name = "doc-consumer"',
        'version = "0.0.0"',
        'edition = "2024"',
        "",
        "[workspace]",
        "",
        "[dependencies]",
    ]
    lines.extend(f"{name} = {_toml_value(value)}" for name, value in dependencies.items())
    lines.extend(("", "[patch.crates-io]", f"qubit-config = {{ path = {_toml_value(str(root.resolve()))} }}", ""))
    return "\n".join(lines)


def _load_fixture(root: Path) -> tuple[dict | None, bytes | None, list[str]]:
    manifest_path = root / FIXTURE_MANIFEST
    lock_path = root / FIXTURE_LOCK
    errors: list[str] = []
    try:
        manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        return None, None, [f"consumer fixture manifest is missing or invalid: {exc}"]
    dependencies = manifest.get("dependencies")
    if not isinstance(dependencies, dict):
        errors.append("consumer fixture must define [dependencies]")
        dependencies = {}
    for name in sorted(set(dependencies) | set(EXPECTED_DEPENDENCIES)):
        if dependencies.get(name) != EXPECTED_DEPENDENCIES.get(name):
            errors.append(
                f"consumer fixture {name} declaration expected "
                f"{EXPECTED_DEPENDENCIES.get(name)!r}, found {dependencies.get(name)!r}"
            )
    try:
        lock_bytes = lock_path.read_bytes()
    except OSError as exc:
        errors.append(f"consumer fixture lockfile is missing or unreadable: {exc}")
        lock_bytes = None
    return dependencies, lock_bytes, errors


def compile_document_consumers(documents: dict[str, str]) -> list[str]:
    errors: list[str] = []
    fixture_dependencies, fixture_lock, fixture_errors = _load_fixture(ROOT)
    errors.extend(fixture_errors)
    if errors:
        return errors
    assert fixture_dependencies is not None and fixture_lock is not None
    for path in DOCUMENTS:
        text = documents.get(path, "")
        block, dep_error = _marked_block(text, DEPENDENCY_MARKER, "toml")
        code, code_error = _marked_block(text, EXAMPLE_MARKER, "rust")
        if dep_error or code_error:
            errors.append(f"{path}: {dep_error or code_error}")
            continue
        try:
            dependencies = tomllib.loads(block or "").get("dependencies", {})
        except tomllib.TOMLDecodeError as exc:
            errors.append(f"{path}: invalid dependency TOML: {exc}")
            continue
        if dependencies != fixture_dependencies:
            errors.append(f"{path}: dependencies do not match the checked-in consumer fixture")
            continue
        source = ROOT / "examples" / "config_error_diagnostics.rs"
        if not source.is_file():
            errors.append(f"{path}: consumer source is missing: {source}")
            continue
        with tempfile.TemporaryDirectory(prefix="qubit-config-doc-consumer-") as temporary:
            project = Path(temporary)
            (project / "src").mkdir()
            (project / "Cargo.toml").write_text(_manifest(dependencies, ROOT), encoding="utf-8")
            (project / "src/main.rs").write_text(code or "", encoding="utf-8")
            lock_path = project / "Cargo.lock"
            lock_path.write_bytes(fixture_lock)
            environment = os.environ.copy()
            environment["CARGO_TARGET_DIR"] = str(ROOT / "target" / "doc-consumer")
            command = ["cargo", "run", "--locked"]
            print(f"{path}: source={source}")
            print(f"{path}: command={' '.join(command)}")
            result = subprocess.run(
                command,
                cwd=project,
                env=environment,
                text=True,
                capture_output=True,
            )
            if result.returncode:
                detail = (result.stdout + result.stderr).strip()
                errors.append(f"{path}: {' '.join(command)} failed ({result.returncode}): {detail}")
            if not lock_path.is_file() or lock_path.read_bytes() != fixture_lock:
                errors.append(f"{path}: consumer lockfile changed while checking the example")
    try:
        if (ROOT / FIXTURE_LOCK).read_bytes() != fixture_lock:
            errors.append("consumer fixture lockfile changed while checking examples")
    except OSError as exc:
        errors.append(f"consumer fixture lockfile became unavailable while checking: {exc}")
    return errors


def main() -> int:
    documents = {path: (ROOT / path).read_text(encoding="utf-8") for path in DOCUMENTS}
    errors = validate_documents(documents)
    if not errors:
        errors.extend(compile_document_consumers(documents))
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"checked isolated consumers for {DOCUMENTS[0]} and {DOCUMENTS[1]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
