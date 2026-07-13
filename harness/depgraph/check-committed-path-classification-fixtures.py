#!/usr/bin/env python3
"""Negative fixture for tracked build/cache/output classification."""

from __future__ import annotations

import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
CHECKER = ROOT / "harness/depgraph/check-committed-path-classification.py"


def run(paths: list[str]) -> subprocess.CompletedProcess[str]:
    with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", suffix=".txt") as fixture:
        fixture.write("\n".join(paths) + "\n")
        fixture.flush()
        return subprocess.run(
            [sys.executable, str(CHECKER), "--paths-file", fixture.name],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )


valid = run([
    "engine-rs/crates/foundation/core-ids/src/lib.rs",
    "ts/packages/contracts/src/generated/input.ts",
    "docs/code-map/generated-inventory.md",
])
if valid.returncode != 0:
    raise SystemExit(f"classification valid fixture failed:\n{valid.stdout}{valid.stderr}")

invalid = run(["ts/packages/app/dist/index.js"])
output = invalid.stdout + invalid.stderr
if invalid.returncode == 0 or "tracked build/cache/output path" not in output:
    raise SystemExit(f"classification negative fixture failed:\n{output}")

print("Committed path classification fixtures: OK (tracked build path rejected)")
