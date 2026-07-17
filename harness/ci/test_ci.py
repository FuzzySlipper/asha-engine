#!/usr/bin/env python3
"""Selection and fail-closed tests for the small ASHA CI planner."""

from __future__ import annotations

import json
import pathlib
import tempfile
import unittest

import ci


class CiSelectionTests(unittest.TestCase):
    def selected(self, path: str) -> set[str]:
        plan = ci.plan_document("fast", [path])
        return {gate["id"] for gate in plan["gates"]}

    def test_representative_change_classes_select_responsible_gates(self) -> None:
        cases = {
            "engine-rs/crates/state/core-scene/src/lib.rs": {"rust", "depgraph"},
            "ts/packages/ui-dom/src/index.ts": {"typescript", "depgraph"},
            "engine-rs/crates/protocol/protocol-scene/src/lib.rs": {"contracts", "rust", "typescript", "bridge"},
            "harness/identity/execution.py": {"identities"},
            "harness/public-surface/check-public-rust-distribution.py": {"identities", "bridge"},
            "engine-rs/crates/bridge/native-bridge/src/lib.rs": {"rust", "bridge", "native"},
            "engine-rs/crates/sim/sim-replay/src/lib.rs": {"rust", "replays"},
            "engine-rs/crates/render/render-bridge/src/lib.rs": {"rust", "render-goldens"},
        }
        for path, required in cases.items():
            with self.subTest(path=path):
                self.assertTrue(required.issubset(self.selected(path)))

    def test_unknown_changes_expand_to_full(self) -> None:
        for path in ("unclassified/new-root.file", "harness/fixtures/new-fixture.json"):
            plan = ci.plan_document("fast", [path])
            with self.subTest(path=path):
                self.assertTrue(plan["expandedToFull"])
                self.assertEqual(
                    [gate["id"] for gate in plan["gates"]],
                    ci.FULL_ORDER,
                )
                self.assertNotIn(
                    "ASHA_HARNESS_SELF_TESTS=0",
                    next(
                        gate["normalizedCommand"]
                        for gate in plan["gates"]
                        if gate["id"] == "typescript"
                    ),
                )

    def test_ci_policy_changes_validate_policy_and_expand_to_full(self) -> None:
        for path in (
            "harness/ci/guardrail-policy.json",
            "harness/ci/check-all.sh",
            "harness/ci/check-native.sh",
            ".github/workflows/offline-ci.yml",
        ):
            plan = ci.plan_document("fast", [path])
            with self.subTest(path=path):
                self.assertTrue(plan["expandedToFull"])
                self.assertEqual(
                    [gate["id"] for gate in plan["gates"]],
                    ["guardrail-policy", *ci.FULL_ORDER],
                )

    def test_docs_only_change_keeps_baseline_rails_and_advisory_vocabulary(self) -> None:
        self.assertEqual(
            self.selected("docs/runtime-session-facade.md"),
            set(ci.FAST_ALWAYS),
        )

    def test_each_selection_class_propagates_its_responsible_gate_failure(self) -> None:
        cases = {
            "engine-rs/crates/state/core-scene/src/lib.rs": "rust",
            "ts/packages/ui-dom/src/index.ts": "typescript",
            "engine-rs/crates/protocol/protocol-scene/src/lib.rs": "contracts",
            "harness/identity/execution.py": "identities",
            "engine-rs/crates/bridge/native-bridge/src/lib.rs": "native",
            "engine-rs/crates/sim/sim-replay/src/lib.rs": "replays",
            "unclassified/new-root.file": "depgraph",
        }
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            for index, (path, responsible_gate) in enumerate(cases.items()):
                selected = ci.plan_document("fast", [path])
                matching_gates = [
                    gate for gate in selected["gates"] if gate["id"] == responsible_gate
                ]
                self.assertEqual(len(matching_gates), 1)
                isolated_plan = {**selected, "gates": matching_gates}
                output = root / f"report-{index}.json"
                exit_code = ci.run_plan(isolated_plan, output, responsible_gate)
                with self.subTest(path=path, gate=responsible_gate):
                    self.assertEqual(exit_code, 86)
                    report = json.loads(output.read_text(encoding="utf-8"))
                    self.assertFalse(report["valid"])
                    self.assertEqual(report["results"][0]["exitCode"], 86)

    def test_advisory_failure_warns_without_invalidating_the_run(self) -> None:
        plan = ci.plan_document("fast", ["docs/runtime-session-facade.md"])
        vocabulary_gate = [gate for gate in plan["gates"] if gate["id"] == "vocabulary"]
        self.assertEqual(len(vocabulary_gate), 1)
        isolated_plan = {**plan, "gates": vocabulary_gate}
        with tempfile.TemporaryDirectory() as temporary:
            output = pathlib.Path(temporary) / "advisory.json"
            exit_code = ci.run_plan(isolated_plan, output, "vocabulary")
            report = json.loads(output.read_text(encoding="utf-8"))
        self.assertEqual(exit_code, 0)
        self.assertTrue(report["valid"])
        self.assertEqual(report["summary"]["advisoryWarningCount"], 1)
        self.assertEqual(report["summary"]["blockingFailureCount"], 0)
        self.assertEqual(report["results"][0]["outcome"], "warning")
        self.assertEqual(report["results"][0]["owner"], "Architecture stewardship")


if __name__ == "__main__":
    unittest.main()
