#!/usr/bin/env python3
"""Join public ASHA catalogs into one deterministic reachability report."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import sys
import tempfile
import tomllib
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = ROOT / "harness/reachability/manifest.json"
DEFAULT_REPORT = ROOT / "harness/reachability/validation-report.json"

# This independent inventory prevents a reachability entry from proving its own
# completeness or silently changing which downstream contract it represents.
EXPECTED_CAPABILITIES: dict[str, str | None] = {
    "animation.runtime-projection": "asha-demo.animation-controller-projection",
    "audio.runtime-projection": "asha-demo.audio-projection",
    "billboard.runtime-projection": "asha-demo.billboard-projection",
    "bridge.camera-controller": None,
    "bridge.project-bundle.load": "asha-demo.runtime-load",
    "bridge.session-input-replay": "asha-demo.input-replay",
    "bridge.session-time-control": "asha-demo.time-control-command",
    "feedback.integrated-public-projection": "asha-demo.integrated-feedback-projection",
    "gameplay.event-bound-read": None,
    "gameplay.generic-event": "pulse.subscribe",
    "gameplay.module-binding": "pulse.configuration",
    "gameplay.module-conformance": "pulse.conformance",
    "gameplay.module-named-read": "pulse.read",
    "gameplay.runtime-host": "pulse.runtime-host",
    "gameplay.runtime-session-composition": "pulse.runtime-session-composition",
    "gameplay.runtime-host.decision": "pulse.runtime-host",
    "gameplay.trigger-lifecycle": "pulse.trigger-lifecycle",
    "particle.runtime-projection": "asha-demo.particle-projection",
    "prefab.public-authoring": "asha-demo.game-workspace-prefab-authoring",
    "prefab.runtime-placement": "asha-demo.prefab-runtime-placement",
    "telemetry.live-snapshot-overlay": "asha-demo.telemetry-live-overlay",
}

COMPATIBLE_NEED_KINDS: dict[str, set[str]] = {
    "bridgeOperation": {"runtimeOperation"},
    "gameplayBinding": {"gameplayBindingSchema"},
    "gameplayConformance": {"conformanceEntrypoint", "rustCrate"},
    "gameplayEvent": {"gameplayEventPublish", "gameplayEventSubscribe"},
    "gameplayRead": {"gameplayRead", "serviceQuery"},
    "gameplayRuntimeHost": {"rustCrate"},
    "prefabAuthoring": {"typescriptPackage"},
    "projectionChannel": {"projectionChannel"},
    "runtimeOperation": {"runtimeOperation", "runtimeReadout", "rustCrate"},
}


def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def relative(path: pathlib.Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def digest(path: pathlib.Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def gap(gaps: list[dict[str, str]], capability: str, code: str, path: str, message: str) -> None:
    gaps.append({
        "capability": capability,
        "code": code,
        "path": path,
        "message": message,
    })


def evidence_token(
    gaps: list[dict[str, str]],
    capability: str,
    evidence: Any,
    code: str,
    path: str,
) -> None:
    if not isinstance(evidence, dict):
        gap(gaps, capability, code, path, "evidence must be an object with path and token")
        return
    source = evidence.get("path")
    token = evidence.get("token")
    if not isinstance(source, str) or not isinstance(token, str) or not source or not token:
        gap(gaps, capability, code, path, "evidence path and token are required")
        return
    source_path = pathlib.PurePosixPath(source)
    if (
        source_path.suffix.lower() == ".md"
        or source_path.name.lower().startswith("readme")
        or "docs" in source_path.parts
    ):
        gap(
            gaps,
            capability,
            "invalid_evidence_source",
            f"{path}.path",
            "reachability evidence must be executable code, a generated contract, a manifest, or a test; prose is not proof",
        )
        return
    file_path = ROOT / source
    if not file_path.is_file():
        gap(gaps, capability, "missing_evidence_path", f"{path}.path", f"missing evidence file {source!r}")
        return
    if token not in file_path.read_text(encoding="utf-8"):
        gap(gaps, capability, code, f"{path}.token", f"token {token!r} is absent from {source}")


def exported_symbols(path: pathlib.Path) -> set[str]:
    symbols: set[str] = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        for prefix in ("export interface ", "export type ", "export const ", "export function "):
            if stripped.startswith(prefix):
                symbols.add(stripped[len(prefix):].split("<", 1)[0].split(" ", 1)[0].split("(", 1)[0])
    return symbols


def public_catalogs() -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]]]:
    ts = load_json(ROOT / "harness/public-surface/ts-packages.json")
    rust = load_json(ROOT / "harness/public-surface/rust-crates.json")
    return (
        {item["package"]: item for item in ts["packages"]},
        {item["crate"]: item for item in rust["crates"]},
    )


def consumer_needs() -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for path in sorted((ROOT / "harness/consumer-needs/manifests").glob("*.json")):
        for item in load_json(path)["requirements"]:
            result[item["id"]] = item
    return result


def bridge_operations() -> dict[str, dict[str, Any]]:
    path = ROOT / "engine-rs/crates/bridge/runtime-bridge-api/bridge-manifest.toml"
    document = tomllib.loads(path.read_text(encoding="utf-8"))
    return {item["name"]: item for item in document["operation"]}


def validate(manifest_path: pathlib.Path) -> dict[str, Any]:
    document = load_json(manifest_path)
    gaps: list[dict[str, str]] = []
    assertions = document.get("catalogAssertions")
    capabilities = document.get("capabilities")
    exemptions = document.get("internalExemptions")
    if document.get("schemaVersion") != 1:
        gap(gaps, "manifest", "unsupported_schema", "schemaVersion", "reachability schemaVersion must be 1")
    if not isinstance(assertions, list):
        assertions = []
        gap(gaps, "manifest", "missing_catalog_assertions", "catalogAssertions", "catalogAssertions must be an array")
    if not isinstance(capabilities, list):
        capabilities = []
        gap(gaps, "manifest", "missing_capabilities", "capabilities", "capabilities must be an array")
    if not isinstance(exemptions, list):
        exemptions = []
        gap(gaps, "manifest", "missing_exemptions", "internalExemptions", "internalExemptions must be an array")

    ids = [item.get("id") for item in capabilities if isinstance(item, dict)]
    if ids != sorted(ids) or len(ids) != len(set(ids)):
        gap(gaps, "manifest", "noncanonical_capabilities", "capabilities", "capability ids must be sorted and unique")
    missing_capabilities = sorted(set(EXPECTED_CAPABILITIES) - set(ids))
    unexpected_capabilities = sorted(set(ids) - set(EXPECTED_CAPABILITIES))
    for capability in missing_capabilities:
        gap(
            gaps, capability, "missing_required_capability", "capabilities",
            "the independently governed public capability inventory requires this entry",
        )
    for capability in unexpected_capabilities:
        gap(
            gaps, capability, "unexpected_capability", "capabilities",
            "new public capabilities require an explicit governed inventory update",
        )
    exemption_ids = [item.get("id") for item in exemptions if isinstance(item, dict)]
    if exemption_ids != sorted(exemption_ids) or len(exemption_ids) != len(set(exemption_ids)):
        gap(gaps, "manifest", "noncanonical_exemptions", "internalExemptions", "exemption ids must be sorted and unique")

    assertion_ids = [item.get("id") for item in assertions if isinstance(item, dict)]
    if assertion_ids != sorted(assertion_ids) or len(assertion_ids) != len(set(assertion_ids)):
        gap(gaps, "manifest", "noncanonical_catalog_assertions", "catalogAssertions", "catalog assertion ids must be sorted and unique")

    ts_public, rust_public = public_catalogs()
    needs = consumer_needs()
    semantic_need_checks = {
        item["id"].removeprefix("consumerNeed."): item
        for item in load_json(ROOT / "harness/consumer-needs/validation-report.json")
        .get("semanticValidation", {})
        .get("checks", [])
        if isinstance(item, dict) and isinstance(item.get("id"), str)
    }
    bridge = bridge_operations()
    generated_export_count = sum(
        1
        for source in (ROOT / "ts/packages/contracts/src/generated").glob("*.ts")
        if source.name != "index.ts"
        for line in source.read_text(encoding="utf-8").splitlines()
        if re.match(r"export (interface|type|const|function) ", line.strip())
    )
    actual_catalog_counts = {
        "bridgeOperations": (len(bridge), sum(item.get("surface") == "stable" for item in bridge.values())),
        "consumerNeeds": (len(needs), None),
        "generatedContracts": (generated_export_count, None),
        "rustPublicSurfaces": (len(rust_public), None),
        "typescriptPublicSurfaces": (len(ts_public), None),
    }
    assertion_results: list[dict[str, Any]] = []
    for index, item in enumerate(assertions):
        if not isinstance(item, dict):
            gap(gaps, "manifest", "invalid_catalog_assertion", f"catalogAssertions[{index}]", "catalog assertion must be an object")
            continue
        assertion_id = item.get("id", f"catalogAssertions[{index}]")
        kind = item.get("kind")
        actual = actual_catalog_counts.get(kind)
        before = len(gaps)
        if actual is None:
            gap(gaps, assertion_id, "unknown_catalog_kind", "kind", f"unknown catalog kind {kind!r}")
        else:
            actual_count, actual_stable = actual
            if item.get("expectedCount") != actual_count:
                gap(gaps, assertion_id, "catalog_count_changed", "expectedCount", f"expected {item.get('expectedCount')}, found {actual_count}; review the new or removed public entries")
            if actual_stable is not None and item.get("expectedStableCount") != actual_stable:
                gap(gaps, assertion_id, "stable_catalog_count_changed", "expectedStableCount", f"expected {item.get('expectedStableCount')}, found {actual_stable}; review the surface classification")
        evidence_token(gaps, assertion_id, item.get("proof"), "catalog_proof_unreachable", "proof")
        assertion_results.append({
            "id": assertion_id,
            "kind": kind,
            "valid": len(gaps) == before,
            "actualCount": actual[0] if actual else None,
        })
    results: list[dict[str, Any]] = []
    for index, item in enumerate(capabilities):
        if not isinstance(item, dict):
            gap(gaps, "manifest", "invalid_capability", f"capabilities[{index}]", "capability must be an object")
            continue
        capability = item.get("id") if isinstance(item.get("id"), str) else f"capabilities[{index}]"
        before = len(gaps)
        protocol = item.get("protocol")
        if not isinstance(protocol, dict) or not isinstance(protocol.get("path"), str) or not isinstance(protocol.get("symbol"), str):
            gap(gaps, capability, "missing_protocol", "protocol", "protocol path and symbol are required")
        else:
            protocol_path = ROOT / protocol["path"]
            if not protocol_path.is_file():
                gap(gaps, capability, "missing_protocol_path", "protocol.path", f"missing {protocol['path']}")
            elif protocol["path"].endswith(".ts"):
                if protocol["symbol"] not in exported_symbols(protocol_path):
                    gap(gaps, capability, "protocol_symbol_unreachable", "protocol.symbol", f"generated export {protocol['symbol']!r} is absent")
            elif protocol["symbol"] not in protocol_path.read_text(encoding="utf-8"):
                gap(gaps, capability, "protocol_symbol_unreachable", "protocol.symbol", f"Rust protocol {protocol['symbol']!r} is absent")

        evidence_token(gaps, capability, item.get("provider"), "provider_unreachable", "provider")
        evidence_token(gaps, capability, item.get("delivery"), "delivery_unreachable", "delivery")
        for family in ("fields", "selectors", "quotas"):
            values = item.get(family, [])
            if not isinstance(values, list):
                gap(gaps, capability, f"invalid_{family}", family, f"{family} must be an array")
                continue
            names = [value.get("name") for value in values if isinstance(value, dict)]
            if names != sorted(names) or len(names) != len(set(names)):
                gap(gaps, capability, f"noncanonical_{family}", family, f"{family} names must be sorted and unique")
            for value_index, value in enumerate(values):
                evidence_token(
                    gaps,
                    capability,
                    value,
                    f"{family[:-1]}_unreachable",
                    f"{family}[{value_index}]",
                )

        if "bootstrapAdapter" in item:
            evidence_token(gaps, capability, item["bootstrapAdapter"], "bootstrap_adapter_unreachable", "bootstrapAdapter")
        if "providerCardinality" in item:
            evidence_token(gaps, capability, item["providerCardinality"], "provider_cardinality_unreachable", "providerCardinality")
        if "namespaceOwnership" in item:
            evidence_token(gaps, capability, item["namespaceOwnership"], "namespace_ownership_unreachable", "namespaceOwnership")
        bridge_name = item.get("bridgeOperation")
        if bridge_name is not None:
            operation = bridge.get(bridge_name)
            if operation is None:
                gap(gaps, capability, "bridge_operation_unreachable", "bridgeOperation", f"unknown bridge operation {bridge_name!r}")
            elif operation.get("surface") != "stable":
                gap(gaps, capability, "bridge_operation_not_stable", "bridgeOperation", f"bridge operation {bridge_name!r} is not stable")

        surface = item.get("publicSurface")
        if not isinstance(surface, dict):
            gap(gaps, capability, "missing_public_surface", "publicSurface", "public surface identity is required")
        elif surface.get("kind") == "typescript":
            if surface.get("identity") not in ts_public:
                gap(gaps, capability, "typescript_surface_unreachable", "publicSurface.identity", "TypeScript package is absent from the public catalog")
        elif surface.get("kind") == "rust":
            crate = rust_public.get(surface.get("identity"))
            if crate is None:
                gap(gaps, capability, "rust_surface_unreachable", "publicSurface.identity", "Rust crate is absent from the public catalog")
            elif surface.get("symbol") not in crate.get("exposes", []):
                gap(gaps, capability, "rust_symbol_unreachable", "publicSurface.symbol", "Rust symbol is absent from the declared public exports")
        else:
            gap(gaps, capability, "invalid_public_surface", "publicSurface.kind", "publicSurface kind must be typescript or rust")

        need = item.get("consumerNeed")
        expected_need = EXPECTED_CAPABILITIES.get(capability)
        if need != expected_need:
            gap(
                gaps, capability, "consumer_need_mismatch", "consumerNeed",
                f"capability must map to governed consumer need {expected_need!r}, found {need!r}",
            )
        need_contract = needs.get(need) if isinstance(need, str) else None
        if need is not None and need_contract is None:
            gap(gaps, capability, "consumer_need_unreachable", "consumerNeed", f"consumer need {need!r} is absent")
        elif need_contract is not None:
            allowed_need_kinds = COMPATIBLE_NEED_KINDS.get(item.get("kind"), set())
            if need_contract.get("kind") not in allowed_need_kinds:
                gap(
                    gaps, capability, "consumer_need_kind_mismatch", "consumerNeed",
                    f"capability kind {item.get('kind')!r} cannot prove need kind {need_contract.get('kind')!r}",
                )
            surface_identity = surface.get("identity") if isinstance(surface, dict) else None
            provider_identity = item.get("providerIdentity", surface_identity)
            required_provider = need_contract.get("provider")
            if required_provider is not None and provider_identity != required_provider:
                gap(
                    gaps, capability, "consumer_need_provider_mismatch", "providerIdentity",
                    f"need requires provider {required_provider!r}, found {provider_identity!r}",
                )
            semantic_check = semantic_need_checks.get(need)
            if semantic_check is not None and semantic_check.get("passed") is not True:
                gap(
                    gaps, capability, "consumer_need_semantic_proof_failed", "consumerNeed",
                    "the compiled consumer-needs conformance check did not pass",
                )

        if item.get("kind") == "gameplayRead":
            cardinality = item.get("providerCardinality")
            if not isinstance(cardinality, dict) or cardinality.get("constraint") != "exactlyOne":
                gap(
                    gaps, capability, "missing_provider_cardinality", "providerCardinality",
                    "gameplay reads must prove exactly one provider in the closed registry",
                )
            if "namespaceOwnership" not in item:
                gap(
                    gaps, capability, "missing_namespace_ownership", "namespaceOwnership",
                    "gameplay reads must prove provider namespace ownership",
                )
        results.append({
            "id": capability,
            "kind": item.get("kind"),
            "reachable": len(gaps) == before,
            "gapCount": len(gaps) - before,
        })

    for index, item in enumerate(exemptions):
        if not isinstance(item, dict):
            gap(gaps, "manifest", "invalid_exemption", f"internalExemptions[{index}]", "exemption must be an object")
            continue
        exemption_id = item.get("id", f"internalExemptions[{index}]")
        if not isinstance(item.get("reason"), str) or len(item["reason"].strip()) < 40:
            gap(gaps, exemption_id, "invalid_internal_reason", "reason", "internal-only exemption needs a specific reviewed reason")
        if not isinstance(item.get("owner"), str) or not item["owner"].strip():
            gap(gaps, exemption_id, "missing_internal_owner", "owner", "internal-only exemption needs an owner")
        evidence_token(gaps, exemption_id, item.get("evidence"), "internal_evidence_unreachable", "evidence")

    gaps.sort(key=lambda item: (item["capability"], item["code"], item["path"], item["message"]))
    catalog_paths = [
        ROOT / "engine-rs/crates/bridge/runtime-bridge-api/bridge-manifest.toml",
        ROOT / "harness/public-surface/ts-packages.json",
        ROOT / "harness/public-surface/rust-crates.json",
        ROOT / "harness/consumer-needs/validation-report.json",
        ROOT / "ts/packages/contracts/src/generated/gameExtension.ts",
    ]
    return {
        "schemaVersion": 1,
        "valid": not gaps,
        "manifest": relative(manifest_path),
        "manifestHash": digest(manifest_path),
        "catalogs": [
            {"path": relative(path), "hash": digest(path)} for path in catalog_paths
        ],
        "capabilityCount": len(capabilities),
        "catalogAssertionCount": len(assertions),
        "internalExemptionCount": len(exemptions),
        "catalogAssertions": assertion_results,
        "capabilities": results,
        "gaps": gaps,
    }


def encoded(report: dict[str, Any]) -> str:
    return json.dumps(report, indent=2, sort_keys=False) + "\n"


def check_adversarial_mutations() -> list[str]:
    source = load_json(DEFAULT_MANIFEST)
    cases: list[tuple[str, dict[str, Any], str]] = []

    omitted = json.loads(json.dumps(source))
    omitted["capabilities"].pop(0)
    cases.append(("omitted capability", omitted, "missing_required_capability"))

    unrelated_need = json.loads(json.dumps(source))
    unrelated_need["capabilities"][0]["consumerNeed"] = "asha-demo.audio-projection"
    cases.append(("unrelated consumer need", unrelated_need, "consumer_need_mismatch"))

    prose_evidence = json.loads(json.dumps(source))
    prose_evidence["capabilities"][0]["provider"] = {"path": "README.md", "token": "ASHA"}
    cases.append(("prose token evidence", prose_evidence, "invalid_evidence_source"))

    missing_cardinality = json.loads(json.dumps(source))
    gameplay_read = next(
        item for item in missing_cardinality["capabilities"]
        if item["id"] == "gameplay.module-named-read"
    )
    gameplay_read.pop("providerCardinality")
    cases.append(("missing provider cardinality", missing_cardinality, "missing_provider_cardinality"))

    failures = []
    with tempfile.TemporaryDirectory(prefix="asha-reachability-") as directory:
        for name, document, expected in cases:
            path = pathlib.Path(directory) / f"{name.replace(' ', '-')}.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            codes = {item["code"] for item in validate(path)["gaps"]}
            if expected not in codes:
                failures.append(f"{name}: expected {expected}, got {sorted(codes)}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=pathlib.Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--write-report", action="store_true")
    parser.add_argument("--check-fixtures", action="store_true")
    args = parser.parse_args()
    manifest_path = args.manifest.resolve()
    report = validate(manifest_path)
    if args.check_fixtures:
        fixture_dir = ROOT / "harness/reachability/fixtures"
        failures = []
        for fixture in sorted(fixture_dir.glob("*.json")):
            fixture_report = validate(fixture)
            expected = fixture.stem.split("__", 1)[0]
            codes = {item["code"] for item in fixture_report["gaps"]}
            if expected not in codes:
                failures.append(f"{fixture.name}: expected {expected}, got {sorted(codes)}")
        failures.extend(check_adversarial_mutations())
        if failures:
            print("\n".join(failures), file=sys.stderr)
            return 1
        print(f"reachability fixtures: OK ({len(list(fixture_dir.glob('*.json')))} negative fixtures)")
        return 0
    if args.write_report:
        DEFAULT_REPORT.write_text(encoded(report), encoding="utf-8")
    elif manifest_path == DEFAULT_MANIFEST:
        if not DEFAULT_REPORT.is_file() or DEFAULT_REPORT.read_text(encoding="utf-8") != encoded(report):
            print("reachability: validation-report.json is stale; run validate.py --write-report", file=sys.stderr)
            return 1
    if not report["valid"]:
        print(encoded(report), file=sys.stderr)
        return 1
    print(f"Reachability: OK ({report['capabilityCount']} capabilities, {report['internalExemptionCount']} internal exemptions)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
