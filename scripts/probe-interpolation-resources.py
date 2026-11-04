#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Run isolated interpolation resource probes and preserve per-run evidence."""

from __future__ import annotations

import argparse
import csv
import json
import os
import platform
import signal
import subprocess
import sys
import time
from pathlib import Path

try:
    import resource
except ImportError:  # pragma: no cover - exercised on platforms without POSIX resource limits.
    resource = None


ROOT = Path(__file__).resolve().parent.parent
TIMEOUT_SECONDS = 10
STACKS = (2 * 1024 * 1024, 8 * 1024 * 1024)


def cases() -> list[dict[str, int | str]]:
    matrix: list[dict[str, int | str]] = []
    for references in (0, 1, 63, 64, 65):
        for stack in STACKS:
            matrix.append({"category": "default-boundary", "mode": "chain", "references": references, "depth": 64, "expansions": "default", "stack": stack})
    for references in (256, 1024, 4096, 8192):
        for depth in (references, references - 1):
            for stack in STACKS:
                matrix.append({"category": "raised-depth", "mode": "chain", "references": references, "depth": depth, "expansions": 16384, "stack": stack})
    for references in (256, 1024):
        for stack in STACKS:
            matrix.append({"category": "long-cycle", "mode": "cycle", "references": references, "depth": references + 1, "expansions": 16384, "stack": stack})
    return matrix


def set_child_limit(name: str, requested: int) -> dict[str, object]:
    if resource is None or os.name != "posix":
        return {"status": "unsupported", "reason": "Python POSIX resource limits are unavailable"}
    limit_kind = getattr(resource, name, None)
    if limit_kind is None:
        return {"status": "unsupported", "reason": f"{name} is not provided by this platform"}
    try:
        resource.setrlimit(limit_kind, (requested, requested))
        soft, hard = resource.getrlimit(limit_kind)
    except (OSError, ValueError) as error:
        return {"status": "failed", "reason": f"{type(error).__name__}: {error}"}
    applied = soft == requested and hard == requested
    return {
        "status": "applied" if applied else "not-effective",
        "requested": requested,
        "soft": soft,
        "hard": hard,
    }


def child_limits(status_path: str) -> None:
    statuses = {
        "RLIMIT_CORE": set_child_limit("RLIMIT_CORE", 0),
        "RLIMIT_AS": set_child_limit("RLIMIT_AS", 512 * 1024 * 1024),
    }
    try:
        Path(status_path).write_text(json.dumps(statuses, sort_keys=True) + "\n", encoding="utf-8")
    except OSError:
        pass


def unsupported_limit_status() -> dict[str, dict[str, str]]:
    reason = "preexec_fn or Python POSIX resource limits are unavailable on this platform"
    return {
        "RLIMIT_CORE": {"status": "unsupported", "reason": reason},
        "RLIMIT_AS": {"status": "unsupported", "reason": reason},
    }


def classify(returncode: int | None, timed_out: bool, stdout: str, stderr: str) -> tuple[str, str]:
    phase = "unknown"
    for line in stdout.splitlines():
        if line.startswith("phase="):
            phase = line.removeprefix("phase=")
    if timed_out:
        return "timeout", phase
    if returncode == 0 and "result=complete" in stdout:
        return "expected", phase
    if returncode is not None and returncode < 0:
        signal_name = signal.Signals(-returncode).name
        if phase == "read" and "stack overflow" in stderr.lower():
            return "stack-overflow", phase
        return f"signal-{signal_name.lower()}", phase
    if "cannot allocate memory" in stderr.lower() or "memory allocation" in stderr.lower():
        return "allocation-failure", phase
    if phase == "construct":
        return "construct-failure", phase
    if phase == "read":
        return "read-failure", phase
    return "process-failure", phase


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=("debug", "release"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    binary = ROOT / "target" / args.profile / "examples" / "config_interpolation_resource_probe"
    if not binary.is_file():
        print(f"missing built probe binary: {binary}", file=sys.stderr)
        return 2
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    can_set_limits = os.name == "posix" and resource is not None
    (output / "toolchain.txt").write_text(
        f"python={sys.version.split()[0]}\nrustc={subprocess.run(['rustc', '--version', '--verbose'], cwd=ROOT, capture_output=True, text=True, check=True).stdout}"
        f"cargo={subprocess.run(['cargo', '--version'], cwd=ROOT, capture_output=True, text=True, check=True).stdout}"
        f"os={platform.platform()}\narchitecture={platform.machine()}\nprofile={args.profile}\ntimeout_seconds={TIMEOUT_SECONDS}\n"
        f"stack_bytes=2097152,8388608\nresource_limits={'attempt per child' if can_set_limits else 'unsupported on this platform'}\n"
        "per-run RLIMIT_AS/RLIMIT_CORE outcome is in each resource-limits.json\n",
        encoding="utf-8",
    )
    rows: list[dict[str, object]] = []
    for index, case in enumerate(cases(), start=1):
        for repetition in range(1, 4):
            run_id = f"{index:02d}-{repetition}"
            run_dir = output / run_id
            run_dir.mkdir()
            command = [str(binary), str(case["mode"]), str(case["references"]), str(case["depth"]), str(case["stack"])]
            started = time.monotonic()
            timed_out = False
            returncode: int | None = None
            stdout = ""
            stderr = ""
            limit_error = ""
            resource_limit_status = unsupported_limit_status()
            limits_path = run_dir / "resource-limits.json"
            try:
                popen_options: dict[str, object] = {}
                if can_set_limits:
                    popen_options["preexec_fn"] = lambda: child_limits(str(limits_path))
                process = subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, **popen_options)
                try:
                    out_bytes, err_bytes = process.communicate(timeout=TIMEOUT_SECONDS)
                    returncode = process.returncode
                except subprocess.TimeoutExpired:
                    timed_out = True
                    process.kill()
                    out_bytes, err_bytes = process.communicate()
                stdout = out_bytes.decode("utf-8", errors="replace")
                stderr = err_bytes.decode("utf-8", errors="replace")
                if limits_path.is_file():
                    resource_limit_status = json.loads(limits_path.read_text(encoding="utf-8"))
            except (OSError, subprocess.SubprocessError, ValueError) as error:
                limit_error = f"{type(error).__name__}: {error}"
            elapsed = round(time.monotonic() - started, 6)
            status, phase = classify(returncode, timed_out, stdout, stderr)
            (run_dir / "stdout.txt").write_text(stdout, encoding="utf-8")
            (run_dir / "stderr.txt").write_text(stderr, encoding="utf-8")
            row = {
                "run_id": run_id,
                "category": case["category"],
                "mode": case["mode"],
                "references": case["references"],
                "depth_limit": case["depth"],
                "expansion_limit": case["expansions"],
                "output_limit": "default",
                "stack_bytes": case["stack"],
                "repetition": repetition,
                "profile": args.profile,
                "timeout_seconds": TIMEOUT_SECONDS,
                "elapsed_seconds": elapsed,
                "stage": phase,
                "status": status,
                "exit_code": returncode if returncode is None or returncode >= 0 else "",
                "signal": signal.Signals(-returncode).name if returncode is not None and returncode < 0 else "",
                "timed_out": timed_out,
                "rlimit_as_status": resource_limit_status["RLIMIT_AS"]["status"],
                "core_limit_status": resource_limit_status["RLIMIT_CORE"]["status"],
                "resource_limits_file": f"{run_id}/resource-limits.json" if limits_path.is_file() else "",
                "driver_error": limit_error,
                "stdout_file": f"{run_id}/stdout.txt",
                "stderr_file": f"{run_id}/stderr.txt",
            }
            rows.append(row)
            print(f"{args.profile} {run_id} {case['category']} R={case['references']} L={case['depth']} stack={case['stack']} status={status} phase={phase} elapsed={elapsed}s")

    with (output / "summary.csv").open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    (output / "summary.json").write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8")
    counts: dict[str, int] = {}
    for row in rows:
        counts[str(row["status"])] = counts.get(str(row["status"]), 0) + 1
    (output / "counts.json").write_text(json.dumps(counts, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"summary={output / 'summary.csv'} counts={json.dumps(counts, sort_keys=True)}")
    return 1 if any(row["status"] not in ("expected", "stack-overflow") for row in rows) else 0


if __name__ == "__main__":
    raise SystemExit(main())
