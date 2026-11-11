#!/usr/bin/env python3
"""Run the real rs-http, rs-ioc, and rs-websocket adapters against rs-config."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


SCRIPT_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURE = SCRIPT_ROOT / "tests/fixtures/downstream_adapters"
SOURCE_REPOSITORIES = SCRIPT_ROOT.parent
TUNGSTENITE_URL = "https://github.com/qubit-ltd/tungstenite-rs"
TUNGSTENITE_REVISION = "b24a4442be0247a71fe9ee4e520eb483ac1b2873"


def run(command: list[str], *, cwd: Path) -> subprocess.CompletedProcess[str]:
    """Run a command without a shell and propagate any nonzero exit status."""
    print("+", " ".join(command), flush=True)
    return subprocess.run(command, cwd=cwd, check=True, text=True, capture_output=True)


def validate_metadata(metadata: dict[str, Any], workspace: Path, *, full: bool) -> None:
    """Assert the resolved graph uses exactly the prepared local source tree."""
    packages = metadata.get("packages")
    resolve = metadata.get("resolve")
    if not isinstance(packages, list) or not isinstance(resolve, dict):
        raise ValueError("cargo metadata must contain packages and resolve")

    by_name: dict[str, list[dict[str, Any]]] = {}
    for package in packages:
        by_name.setdefault(package.get("name", ""), []).append(package)
    for required in ("qubit-config", "qubit-http", "qubit-ioc", "qubit-websocket"):
        if len(by_name.get(required, [])) != 1:
            raise ValueError(f"expected exactly one {required} package node")

    expected_manifest = (workspace / "rs-config/Cargo.toml").resolve()
    config_package = by_name["qubit-config"][0]
    if Path(config_package["manifest_path"]).resolve() != expected_manifest:
        raise ValueError("qubit-config does not resolve to the prepared rs-config checkout")

    nodes = resolve.get("nodes")
    if not isinstance(nodes, list):
        raise ValueError("cargo metadata resolve.nodes is missing")
    package_by_id = {package["id"]: package for package in packages}
    node_by_id = {node["id"]: node for node in nodes}
    config_id = config_package["id"]
    for name in ("qubit-http", "qubit-ioc", "qubit-websocket"):
        package = by_name[name][0]
        node = node_by_id.get(package["id"])
        if node is None:
            raise ValueError(f"{name} has no resolved dependency node")
        config_edges = [dependency for dependency in node.get("deps", []) if dependency.get("name") == "qubit_config"]
        if not config_edges or any(edge["pkg"] != config_id for edge in config_edges):
            raise ValueError(f"{name} does not depend exclusively on the prepared qubit-config node")

    ioc_node = node_by_id[by_name["qubit-ioc"][0]["id"]]
    if "config" not in ioc_node.get("features", []):
        raise ValueError("qubit-ioc config feature is not enabled")

    websocket_package = by_name["qubit-websocket"][0]
    expected_websocket_manifest = (workspace / "rs-websocket/Cargo.toml").resolve()
    if Path(websocket_package["manifest_path"]).resolve() != expected_websocket_manifest:
        raise ValueError("qubit-websocket does not resolve to the prepared rs-websocket checkout")
    websocket_node = node_by_id[websocket_package["id"]]
    if "config" not in websocket_node.get("features", []):
        raise ValueError("qubit-websocket config feature is not enabled")

    tungstenite_packages = by_name.get("tungstenite", [])
    if not tungstenite_packages:
        raise ValueError("tungstenite patch package is missing")
    expected_source = f"git+{TUNGSTENITE_URL}?rev={TUNGSTENITE_REVISION}#{TUNGSTENITE_REVISION}"
    if any(package.get("source") != expected_source for package in tungstenite_packages):
        sources = sorted(str(package.get("source")) for package in tungstenite_packages)
        raise ValueError(f"tungstenite patch source/revision mismatch: {sources}")

    config_node = node_by_id.get(config_id)
    if config_node is None:
        raise ValueError("qubit-config has no resolved dependency node")
    config_features = set(config_node.get("features", []))
    if full and "full" not in config_features:
        raise ValueError("full fixture mode did not enable qubit-config/full")
    if not full and "full" in config_features:
        raise ValueError("minimal fixture mode unexpectedly enabled qubit-config/full")

    # Keep this lookup explicit so malformed graphs with an unresolved edge fail clearly.
    if any(edge["pkg"] not in package_by_id for node in nodes for edge in node.get("deps", [])):
        raise ValueError("cargo metadata contains a dependency edge to a missing package node")


def metadata_command(fixture: Path, *, locked: bool) -> list[str]:
    command = ["cargo", "metadata", "--format-version", "1", "--manifest-path", str(fixture / "Cargo.toml")]
    if locked:
        command.append("--locked")
    return command


def prepare_repositories(workspace: Path, revisions_path: Path) -> list[Path]:
    revisions = json.loads(revisions_path.read_text(encoding="utf-8"))
    repositories = revisions.get("repositories")
    if not isinstance(repositories, list) or not repositories:
        raise ValueError("revision manifest must define repositories")
    names: set[str] = set()
    destinations: list[Path] = []
    try:
        for repository in repositories:
            name = repository["name"]
            revision = repository["revision"]
            url = repository["url"]
            if name in names:
                raise ValueError(f"duplicate repository name: {name}")
            if len(revision) != 40 or any(char not in "0123456789abcdef" for char in revision):
                raise ValueError(f"{name} revision must be a full lowercase SHA-1")
            names.add(name)
            if name not in {"rs-http", "rs-ioc", "rs-redact", "rs-websocket"}:
                raise ValueError(f"unsupported repository name: {name}")
            destination = workspace / name
            source_repository = SOURCE_REPOSITORIES / name
            actual_url = run(["git", "-C", str(source_repository), "remote", "get-url", "origin"], cwd=workspace).stdout.strip()
            normalized_url = actual_url.replace("git@github.com:", "https://github.com/").removesuffix(".git")
            if normalized_url != url.removesuffix(".git"):
                raise ValueError(f"{name} canonical URL mismatch: expected {url}, found {actual_url}")
            run(["git", "-C", str(source_repository), "worktree", "add", "--detach", str(destination), revision], cwd=workspace)
            destinations.append(destination)
            actual = run(["git", "-C", str(destination), "rev-parse", "HEAD"], cwd=workspace).stdout.strip()
            if actual != revision:
                raise ValueError(f"{name} checkout SHA mismatch: expected {revision}, got {actual}")
        if names != {"rs-http", "rs-ioc", "rs-redact", "rs-websocket"}:
            raise ValueError("revision manifest must pin rs-http, rs-ioc, rs-redact, and rs-websocket")
        return destinations
    except Exception:
        for destination in reversed(destinations):
            source_repository = SOURCE_REPOSITORIES / destination.name
            run(["git", "-C", str(source_repository), "worktree", "remove", str(destination)], cwd=workspace)
        raise


def copy_current_source(source_root: Path, workspace: Path) -> Path:
    destination = workspace / "rs-config"
    ignored = shutil.ignore_patterns(".git", ".worktrees", "target", "__pycache__")
    shutil.copytree(source_root, destination, ignore=ignored, symlinks=True)
    return destination


def execute(fixture_path: Path, revisions_path: Path, *, update_lock: bool) -> None:
    source_root = SCRIPT_ROOT.resolve()
    fixture_rel = fixture_path.resolve().relative_to(source_root)
    with tempfile.TemporaryDirectory(prefix="rs-config-downstream-contracts-") as temporary:
        workspace = Path(temporary).resolve()
        prepared_repositories = prepare_repositories(workspace, revisions_path)
        try:
            copied_root = copy_current_source(source_root, workspace)
            fixture = copied_root / fixture_rel

            if update_lock:
                run(["cargo", "generate-lockfile", "--manifest-path", str(fixture / "Cargo.toml")], cwd=workspace)
                lock_destination = fixture_path / "Cargo.lock"
                shutil.copy2(fixture / "Cargo.lock", lock_destination)

            test_failures: list[subprocess.CalledProcessError] = []
            for full in (False, True):
                mode = "full" if full else "minimal"
                command = metadata_command(fixture, locked=True)
                if full:
                    command.extend(["--features", "full"])
                metadata = json.loads(run(command, cwd=workspace).stdout)
                validate_metadata(metadata, workspace, full=full)
                test_command = ["cargo", "test", "--locked", "--manifest-path", str(fixture / "Cargo.toml")]
                if full:
                    test_command.extend(["--features", "full"])
                print(f"Running real adapter contracts in {mode} mode", flush=True)
                try:
                    run(test_command, cwd=workspace)
                except subprocess.CalledProcessError as error:
                    test_failures.append(error)

            if test_failures:
                diagnostics: list[str] = []
                for error in test_failures:
                    diagnostics.append(f"{error.cmd} exited with status {error.returncode}")
                    diagnostics.append(error.stdout or "")
                    diagnostics.append(error.stderr or "")
                first = test_failures[0]
                raise subprocess.CalledProcessError(
                    first.returncode,
                    first.cmd,
                    output="\n".join(diagnostics),
                )
        finally:
            for destination in reversed(prepared_repositories):
                source_repository = SOURCE_REPOSITORIES / destination.name
                run(["git", "-C", str(source_repository), "worktree", "remove", str(destination)], cwd=workspace)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--revisions", type=Path, default=DEFAULT_FIXTURE / "downstream-revisions.json")
    parser.add_argument(
        "--update-lock",
        action="store_true",
        help="maintenance only: regenerate the fixture lockfile after preparing fixed checkouts",
    )
    arguments = parser.parse_args()
    try:
        execute(arguments.fixture, arguments.revisions, update_lock=arguments.update_lock)
    except subprocess.CalledProcessError as error:
        sys.stderr.write(error.stdout or "")
        sys.stderr.write(error.stderr or "")
        print(f"downstream command failed with status {error.returncode}: {error.cmd}", file=sys.stderr)
        return error.returncode or 1
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"downstream contract setup failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
