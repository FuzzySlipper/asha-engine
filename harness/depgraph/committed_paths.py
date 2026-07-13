"""Canonical committed-path classification shared by gates and reports."""

from __future__ import annotations

import pathlib
import subprocess
from collections import Counter

OUTPUT_PARTS = {
    ".cache",
    ".mypy_cache",
    ".nyc_output",
    ".parcel-cache",
    ".pnpm-store",
    ".pytest_cache",
    ".ruff_cache",
    ".turbo",
    ".vite",
    "__pycache__",
    "bower_components",
    "coverage",
    "dist",
    "node_modules",
    "perf-out",
    "shell-out",
    "smoke-out",
    "target",
}
OUTPUT_PREFIXES = ("ts/artifacts/",)
OUTPUT_SUFFIXES = {
    ".a",
    ".dll",
    ".dylib",
    ".node",
    ".o",
    ".obj",
    ".pyc",
    ".pyo",
    ".rlib",
    ".rmeta",
    ".so",
}
GENERATED_PREFIXES = (
    "docs/code-map/generated-",
    "ts/packages/contracts/src/generated/",
)
GENERATED_FILES = {"ts/eslint-boundaries.generated.mjs"}
SOURCE_SUFFIXES = {".cjs", ".js", ".mjs", ".py", ".rs", ".sh", ".ts", ".tsx"}


def classify(path: str) -> str:
    pure = pathlib.PurePosixPath(path)
    if (
        any(part in OUTPUT_PARTS for part in pure.parts)
        or path.startswith(OUTPUT_PREFIXES)
        or pure.suffix in OUTPUT_SUFFIXES
    ):
        return "buildCacheOutput"
    if path.startswith(GENERATED_PREFIXES) or path in GENERATED_FILES:
        return "generatedSource"
    if pure.suffix in SOURCE_SUFFIXES:
        return "committedSource"
    return "otherCommitted"


def tracked_paths(root: pathlib.Path) -> list[str]:
    completed = subprocess.run(
        ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True
    )
    return sorted(item.decode("utf-8") for item in completed.stdout.split(b"\0") if item)


def report(paths: list[str]) -> dict[str, object]:
    classified = [(path, classify(path)) for path in paths]
    counts = Counter(kind for _path, kind in classified)
    outputs = [path for path, kind in classified if kind == "buildCacheOutput"]
    return {
        "counts": {
            kind: counts.get(kind, 0)
            for kind in (
                "committedSource",
                "generatedSource",
                "buildCacheOutput",
                "otherCommitted",
            )
        },
        "buildCacheOutputPaths": outputs,
    }
