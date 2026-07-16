#!/usr/bin/env python3
"""Execute the declared conformance suites through the shared proof scheduler."""

from __future__ import annotations

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "harness/identity"))

from execution import ExecutionError, make_plan, run_plan  # noqa: E402

DECLARED_EXTERNAL_EXECUTION_ROOTS = {"asha-demo", "asha-studio"}


def available_execution_ids() -> list[str]:
    definitions_path = ROOT / "harness/identity/executions.json"
    definitions = json.loads(definitions_path.read_text(encoding="utf-8"))["executions"]
    available: list[str] = []
    for definition in definitions:
        external_roots = {
            parts[1]
            for source in definition.get("inputs", [])
            if len(parts := pathlib.PurePosixPath(source).parts) >= 2
            and parts[0] == ".."
            and parts[1] in DECLARED_EXTERNAL_EXECUTION_ROOTS
        }
        if any(not (ROOT.parent / root).is_dir() for root in external_roots):
            continue
        available.append(definition["id"])
    return available


def main() -> int:
    try:
        plan = make_plan(available_execution_ids())
        exit_code, report = run_plan(plan)
    except ExecutionError as error:
        print(f"conformance execution: {error}", file=sys.stderr)
        return 1
    if exit_code != 0:
        return exit_code
    report_path = ROOT / "harness/smoke-out/proof-execution/conformance-report.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    suite_count = sum(len(item["attributions"]) for item in report["executions"])
    shared_count = sum(len(item["attributions"]) > 1 for item in report["executions"])
    print(
        f"Real conformance probes passed ({suite_count} suite attributions, "
        f"{len(report['executions'])} executions, {shared_count} shared; "
        f"report {report_path.relative_to(ROOT)})."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
