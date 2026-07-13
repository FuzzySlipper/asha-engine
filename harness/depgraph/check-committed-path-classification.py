#!/usr/bin/env python3
"""Classify tracked paths and reject committed build/cache/output artifacts."""

from __future__ import annotations

import argparse
import pathlib
import sys

from committed_paths import report, tracked_paths

ROOT = pathlib.Path(__file__).resolve().parents[2]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--paths-file", type=pathlib.Path)
    args = parser.parse_args()
    paths = (
        [
            line.strip()
            for line in args.paths_file.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
        if args.paths_file is not None
        else tracked_paths(ROOT)
    )
    result = report(paths)
    outputs = result["buildCacheOutputPaths"]
    if outputs:
        for path in outputs:
            print(f"FAIL: tracked build/cache/output path: {path}", file=sys.stderr)
        return 1
    counts = result["counts"]
    print(
        "Committed path classification: OK "
        f"({counts['committedSource']} source, {counts['generatedSource']} generated, "
        f"{counts['otherCommitted']} other, 0 build/cache/output)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
