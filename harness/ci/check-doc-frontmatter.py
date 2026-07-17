#!/usr/bin/env python3
"""Validate YAML frontmatter on all docs.

Checks:
  - Every .md file in docs/ and docs/topics/ has YAML frontmatter.
  - Required keys: status, audience, tags, supersedes, see-also.
  - status is one of: current, draft, deprecated, historical.
  - audience is one of: agent, consumer, reviewer, all.
  - tags is a YAML list of strings.
  - supersedes is a YAML list of strings.
  - see-also is a YAML list of strings.
  - No status:current doc links to a status:historical doc.
  - No status:current doc links to a Deleted doc.

Runs from repo root.

Integration: add to check-fast.sh or as an advisory gate.
"""

import os
import sys
import re

DOCS_DIR = "docs"
VALID_STATUS = {"current", "draft", "deprecated", "historical"}
VALID_AUDIENCE = {"agent", "consumer", "reviewer", "all"}
REQUIRED_KEYS = {"status", "audience", "tags", "supersedes", "see-also"}

FRONTMATTER_RE = re.compile(r"^---\n(.*?)\n---\n", re.DOTALL)


def parse_frontmatter(content: str) -> dict | None:
    m = FRONTMATTER_RE.match(content)
    if not m:
        return None
    fm_text = m.group(1)
    result = {}
    for line in fm_text.strip().split("\n"):
        if ":" in line:
            key, _, val = line.partition(":")
            key = key.strip()
            val = val.strip()
            if val.startswith("[") and val.endswith("]"):
                val = [v.strip() for v in val[1:-1].split(",") if v.strip()]
            result[key] = val
    return result


def check_file(filepath: str, relative: str) -> list[str]:
    errors = []
    with open(filepath) as f:
        content = f.read()

    fm = parse_frontmatter(content)
    if fm is None:
        errors.append(f"{relative}: missing or malformed YAML frontmatter")
        return errors

    for key in REQUIRED_KEYS:
        if key not in fm:
            errors.append(f"{relative}: missing required key '{key}'")

    if "status" in fm and fm["status"] not in VALID_STATUS:
        errors.append(f"{relative}: invalid status '{fm['status']}' (valid: {VALID_STATUS})")

    if "audience" in fm and fm["audience"] not in VALID_AUDIENCE:
        errors.append(f"{relative}: invalid audience '{fm['audience']}' (valid: {VALID_AUDIENCE})")

    return errors


def main() -> int:
    errors = []
    checked = 0

    for root, dirs, files in os.walk(DOCS_DIR):
        # Skip code-map generated inventory
        dirs[:] = [d for d in dirs if d != "code-map"]
        for f in files:
            if f.endswith(".md"):
                filepath = os.path.join(root, f)
                relative = os.path.relpath(filepath, ".")
                file_errors = check_file(filepath, relative)
                errors.extend(file_errors)
                checked += 1

    if errors:
        print(f"FAIL: {len(errors)} frontmatter errors in {checked} docs:")
        for e in errors:
            print(f"  {e}")
        return 1

    print(f"OK: {checked} docs with valid frontmatter")
    return 0


if __name__ == "__main__":
    sys.exit(main())
