#!/usr/bin/env python3
"""Deterministic structural validator for ASHA consumer-needs manifests."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
MANIFEST_DIR = ROOT / "harness/consumer-needs/manifests"
REPORT_PATH = ROOT / "harness/consumer-needs/validation-report.json"
TS_PUBLIC_PATH = ROOT / "harness/public-surface/ts-packages.json"
RUST_PUBLIC_PATH = ROOT / "harness/public-surface/rust-crates.json"
GAMEPLAY_MANIFEST_PATH = MANIFEST_DIR / "gameplay-module-fixture.json"
GAMEPLAY_CONFORMANCE_MANIFEST = (
    ROOT / "harness/fixtures/gameplay-module-sdk/downstream-module/Cargo.toml"
)

KINDS = {
    "typescriptPackage", "runtimeOperation", "runtimeReadout", "generatedType",
    "projectionChannel", "conformanceEntrypoint", "rustCrate", "gameplayModule",
    "gameplayEventPublish", "gameplayEventSubscribe", "gameplayInvocation",
    "gameplayRead", "gameplayProposal", "gameplayOwner", "gameplayStateSchema",
    "gameplayFactSchema", "gameplayBindingSchema", "bootstrapAdapter",
    "projectBundleArtifact", "prefabPart", "serviceQuery",
}
LEVELS = ("type", "provider", "selector", "delivery")
COMMON_KEYS = {
    "id", "kind", "identity", "provider", "symbols", "fields", "selectors",
    "values", "quota", "ordering", "target", "artifactRole", "requiredLevel",
    "evidence",
}
FORBIDDEN_PREFAB_TERMS = ("displayname", "hierarchy", "scenenodescan", "privateregistry")


@dataclass(frozen=True, order=True)
class Gap:
    manifest: str
    requirement: str
    code: str
    path: str
    message: str

    def json(self) -> dict[str, str]:
        return {
            "manifest": self.manifest,
            "requirement": self.requirement,
            "code": self.code,
            "path": self.path,
            "message": self.message,
        }


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"consumer-needs: cannot read {path}: {error}") from error


def add_gap(
    gaps: list[Gap], manifest: Path, requirement: str, code: str, path: str, message: str
) -> None:
    gaps.append(Gap(manifest.relative_to(ROOT).as_posix(), requirement, code, path, message))


def role_indexes() -> tuple[dict[str, set[str]], dict[str, set[str]], dict[str, set[str]]]:
    ts = read_json(TS_PUBLIC_PATH)
    rust = read_json(RUST_PUBLIC_PATH)
    ts_roles = {
        item["consumerRole"]: set(item["approvedPackageRoots"])
        for item in ts["consumerPolicies"]
    }
    rust_roles = {
        item["consumerRole"]: set(item["approvedCrates"])
        for item in rust["consumerPolicies"]
    }
    rust_exports = {
        item["crate"]: set(item.get("exposes", [])) for item in rust["crates"]
    }
    return ts_roles, rust_roles, rust_exports


def validate_sorted_strings(
    gaps: list[Gap], manifest: Path, requirement_id: str, path: str, value: Any
) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        add_gap(gaps, manifest, requirement_id, "invalid_string_list", path, "must be a string array")
        return []
    if value != sorted(set(value)):
        add_gap(
            gaps, manifest, requirement_id, "noncanonical_list", path,
            "must be unique and sorted lexicographically",
        )
    return value


def validate_evidence(
    gaps: list[Gap], manifest: Path, requirement_id: str, required_level: str, value: Any
) -> None:
    if not isinstance(value, dict) or set(value) != set(LEVELS):
        add_gap(
            gaps, manifest, requirement_id, "invalid_evidence_shape", "evidence",
            "must contain exactly type, provider, selector, and delivery arrays",
        )
        return
    for level in LEVELS:
        refs = validate_sorted_strings(
            gaps, manifest, requirement_id, f"evidence.{level}", value[level]
        )
        if LEVELS.index(level) <= LEVELS.index(required_level) and not refs:
            add_gap(
                gaps, manifest, requirement_id, f"missing_{level}_evidence",
                f"evidence.{level}", f"{required_level} proof requires {level} evidence",
        )
        for ref in refs:
            if "://" in ref:
                continue
            evidence_path = (ROOT / ref).resolve()
            if not evidence_path.exists():
                add_gap(
                    gaps, manifest, requirement_id, "missing_evidence_ref",
                    f"evidence.{level}", f"repository evidence path does not exist: {ref}",
                )


def validate_requirement(
    gaps: list[Gap], manifest: Path, role: str, item: Any,
    ts_roles: dict[str, set[str]], rust_roles: dict[str, set[str]],
    rust_exports: dict[str, set[str]],
) -> str:
    if not isinstance(item, dict):
        add_gap(gaps, manifest, "<unknown>", "invalid_requirement", "requirements", "must be an object")
        return "<unknown>"
    requirement_id = item.get("id") if isinstance(item.get("id"), str) else "<unknown>"
    unknown = sorted(set(item) - COMMON_KEYS)
    if unknown:
        add_gap(
            gaps, manifest, requirement_id, "unknown_requirement_field", requirement_id,
            f"unknown fields: {', '.join(unknown)}",
        )
    for field in ("id", "kind", "identity", "requiredLevel"):
        if not isinstance(item.get(field), str) or not item[field]:
            add_gap(gaps, manifest, requirement_id, "missing_field", field, "must be a non-empty string")
    kind = item.get("kind")
    if kind not in KINDS:
        add_gap(gaps, manifest, requirement_id, "unknown_kind", "kind", f"unknown kind: {kind!r}")
    required_level = item.get("requiredLevel")
    if required_level not in LEVELS:
        add_gap(
            gaps, manifest, requirement_id, "invalid_proof_level", "requiredLevel",
            f"must be one of {', '.join(LEVELS)}",
        )
        required_level = "type"
    validate_evidence(gaps, manifest, requirement_id, required_level, item.get("evidence"))

    for field in ("symbols", "fields", "selectors", "values"):
        if field in item:
            validate_sorted_strings(gaps, manifest, requirement_id, field, item[field])
    if "provider" in item and (not isinstance(item["provider"], str) or not item["provider"]):
        add_gap(gaps, manifest, requirement_id, "invalid_provider", "provider", "must be non-empty")

    if kind == "typescriptPackage":
        package = item.get("identity")
        if package not in ts_roles.get(role, set()):
            add_gap(
                gaps, manifest, requirement_id, "role_package_not_allowed", "identity",
                f"{package!r} is not approved for consumer role {role!r}",
            )
        if not item.get("symbols"):
            add_gap(gaps, manifest, requirement_id, "missing_symbols", "symbols", "package need must name symbols")

    if kind == "rustCrate":
        crate = item.get("identity")
        if crate not in rust_roles.get(role, set()):
            add_gap(
                gaps, manifest, requirement_id, "role_crate_not_allowed", "identity",
                f"{crate!r} is not approved for consumer role {role!r}",
            )
        for symbol in item.get("symbols", []):
            if symbol not in rust_exports.get(crate, set()):
                add_gap(
                    gaps, manifest, requirement_id, "rust_symbol_not_exposed", "symbols",
                    f"{crate!r} does not declare public symbol {symbol!r}",
                )

    if kind in {"runtimeOperation", "runtimeReadout", "generatedType", "projectionChannel"}:
        provider = item.get("provider")
        if not isinstance(provider, str):
            add_gap(gaps, manifest, requirement_id, "missing_provider", "provider", "surface need requires a provider")
        elif provider.startswith("@asha/") and provider not in ts_roles.get(role, set()):
            add_gap(
                gaps, manifest, requirement_id, "role_provider_not_allowed", "provider",
                f"provider {provider!r} is not approved for consumer role {role!r}",
            )

    if kind == "runtimeReadout" and not item.get("fields"):
        add_gap(gaps, manifest, requirement_id, "missing_readout_fields", "fields", "runtime readout must name fields")
    if kind == "generatedType" and not item.get("fields") and not item.get("values"):
        add_gap(
            gaps, manifest, requirement_id, "missing_generated_shape", "fields",
            "generated type must name required fields or values",
        )

    if kind.startswith("gameplay") or kind in {"bootstrapAdapter", "serviceQuery"}:
        if kind != "gameplayOwner" and not item.get("provider"):
            add_gap(gaps, manifest, requirement_id, "missing_provider", "provider", "gameplay need requires a provider")

    if kind == "gameplayRead":
        if not item.get("fields"):
            add_gap(gaps, manifest, requirement_id, "missing_read_fields", "fields", "gameplay read must name fields")
        if not item.get("selectors"):
            add_gap(gaps, manifest, requirement_id, "missing_read_selectors", "selectors", "gameplay read must name selectors")
        quota = item.get("quota")
        if not isinstance(quota, dict) or not isinstance(quota.get("maxItems"), int) or quota["maxItems"] < 1:
            add_gap(gaps, manifest, requirement_id, "missing_read_quota", "quota.maxItems", "must be a positive integer")
        if not item.get("ordering"):
            add_gap(gaps, manifest, requirement_id, "missing_read_ordering", "ordering", "gameplay read must state ordering")

    if kind == "gameplayModule" and not item.get("fields"):
        add_gap(
            gaps, manifest, requirement_id, "missing_module_identity_fields", "fields",
            "module need must name the identity/hash fields it consumes",
        )
    if kind == "gameplayEventSubscribe":
        if not item.get("selectors"):
            add_gap(gaps, manifest, requirement_id, "missing_subscription_selectors", "selectors", "subscription must name selectors")
        if not isinstance(item.get("quota"), dict) or not item["quota"].get("maxDeliveries"):
            add_gap(gaps, manifest, requirement_id, "missing_delivery_quota", "quota.maxDeliveries", "subscription must be bounded")
    if kind == "gameplayInvocation":
        if not item.get("values"):
            add_gap(gaps, manifest, requirement_id, "missing_invocation_family", "values", "invocation must name a family")
        if not isinstance(item.get("quota"), dict) or not item["quota"].get("maxPayloadBytes"):
            add_gap(gaps, manifest, requirement_id, "missing_payload_quota", "quota.maxPayloadBytes", "invocation payload must be bounded")
        if not item.get("ordering"):
            add_gap(gaps, manifest, requirement_id, "missing_invocation_ordering", "ordering", "invocation must state ordering")
    if kind == "gameplayBindingSchema":
        if not item.get("fields"):
            add_gap(gaps, manifest, requirement_id, "missing_binding_fields", "fields", "binding schema must name configuration fields")
        if not isinstance(item.get("target"), dict):
            add_gap(gaps, manifest, requirement_id, "missing_binding_target", "target", "binding schema must name a target scope")
    if kind == "serviceQuery":
        if not item.get("fields") or not item.get("selectors"):
            add_gap(gaps, manifest, requirement_id, "incomplete_service_query", "fields", "service query must name fields and selectors")
        if not isinstance(item.get("quota"), dict) or not item["quota"].get("maxItems"):
            add_gap(gaps, manifest, requirement_id, "missing_query_quota", "quota.maxItems", "service query must be bounded")

    if "quota" in item:
        quota = item["quota"]
        allowed = {"maxItems", "maxPayloadBytes", "maxDeliveries"}
        if not isinstance(quota, dict) or set(quota) - allowed:
            add_gap(gaps, manifest, requirement_id, "invalid_quota", "quota", "contains unknown quota fields")
        elif any(not isinstance(value, int) or value < 1 for value in quota.values()):
            add_gap(gaps, manifest, requirement_id, "invalid_quota", "quota", "quota values must be positive integers")

    if "target" in item and kind != "prefabPart":
        target = item["target"]
        allowed_target = {"prefab", "role", "scope"}
        if not isinstance(target, dict) or not target or set(target) - allowed_target:
            add_gap(gaps, manifest, requirement_id, "invalid_target", "target", "target contains unsupported fields")
        elif "scope" in target and (not isinstance(target["scope"], str) or not target["scope"]):
            add_gap(gaps, manifest, requirement_id, "invalid_target_scope", "target.scope", "scope must be non-empty")

    if kind == "prefabPart":
        target = item.get("target")
        if not isinstance(target, dict) or set(target) != {"prefab", "role"}:
            add_gap(
                gaps, manifest, requirement_id, "invalid_prefab_part_target", "target",
                "must contain exactly numeric prefab and stable role",
            )
        elif not isinstance(target["prefab"], int) or target["prefab"] < 1 or not isinstance(target["role"], str):
            add_gap(gaps, manifest, requirement_id, "invalid_prefab_part_target", "target", "prefab and role are invalid")
        serialized = json.dumps(item, sort_keys=True).lower()
        for term in FORBIDDEN_PREFAB_TERMS:
            if term in serialized:
                add_gap(
                    gaps, manifest, requirement_id, "forbidden_prefab_selector", "target",
                    f"prefab needs cannot use {term}",
                )

    if kind == "projectBundleArtifact" and item.get("artifactRole") != "prefabRegistry":
        add_gap(
            gaps, manifest, requirement_id, "unsupported_project_bundle_artifact", "artifactRole",
            "the first ProjectBundle artifact need is prefabRegistry",
        )
    return requirement_id


def validate_manifest(path: Path) -> tuple[list[Gap], dict[str, Any]]:
    value = read_json(path)
    gaps: list[Gap] = []
    manifest_hash = "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()
    summary = {
        "manifest": path.relative_to(ROOT).as_posix(),
        "manifestHash": manifest_hash,
        "consumerId": "<unknown>",
        "consumerRole": "<unknown>",
        "requirementCount": 0,
    }
    if not isinstance(value, dict) or set(value) != {"schemaVersion", "consumer", "requirements"}:
        add_gap(gaps, path, "<manifest>", "invalid_manifest_shape", "$", "must contain exactly schemaVersion, consumer, requirements")
        return gaps, summary
    if value.get("schemaVersion") != 1:
        add_gap(gaps, path, "<manifest>", "unsupported_schema", "schemaVersion", "must be 1")
    consumer = value.get("consumer")
    if not isinstance(consumer, dict) or set(consumer) != {"id", "role", "source"}:
        add_gap(gaps, path, "<manifest>", "invalid_consumer", "consumer", "must contain exactly id, role, source")
        return gaps, summary
    role = consumer.get("role")
    summary.update({"consumerId": consumer.get("id", "<unknown>"), "consumerRole": role})
    ts_roles, rust_roles, rust_exports = role_indexes()
    if role not in set(ts_roles) | set(rust_roles):
        add_gap(gaps, path, "<manifest>", "unknown_consumer_role", "consumer.role", f"unknown role: {role!r}")
    requirements = value.get("requirements")
    if not isinstance(requirements, list):
        add_gap(gaps, path, "<manifest>", "invalid_requirements", "requirements", "must be an array")
        return gaps, summary
    summary["requirementCount"] = len(requirements)
    ids = [validate_requirement(gaps, path, role, item, ts_roles, rust_roles, rust_exports) for item in requirements]
    duplicates = sorted({item for item in ids if ids.count(item) > 1})
    for duplicate in duplicates:
        add_gap(gaps, path, duplicate, "duplicate_requirement", "id", "requirement ids must be unique")
    return gaps, summary


def build_report(paths: list[Path]) -> dict[str, Any]:
    gaps: list[Gap] = []
    manifests = []
    for path in sorted(paths):
        manifest_gaps, summary = validate_manifest(path)
        gaps.extend(manifest_gaps)
        manifests.append(summary)
    rendered_gaps = [gap.json() for gap in sorted(set(gaps))]
    return {
        "schemaVersion": 1,
        "valid": not rendered_gaps,
        "manifestCount": len(manifests),
        "requirementCount": sum(item["requirementCount"] for item in manifests),
        "manifests": sorted(manifests, key=lambda item: item["consumerId"]),
        "gaps": rendered_gaps,
    }


def run_gameplay_semantic_validation() -> tuple[dict[str, Any], list[dict[str, str]]]:
    """Join the authored needs to the compiled provider and delivered-frame proof."""
    with tempfile.TemporaryDirectory(prefix="asha-consumer-needs-") as directory:
        report_path = Path(directory) / "gameplay-conformance.json"
        command = [
            "cargo", "run", "--quiet", "--manifest-path",
            str(GAMEPLAY_CONFORMANCE_MANIFEST), "--bin", "conformance", "--",
            "--json", str(report_path),
        ]
        result = subprocess.run(
            command,
            cwd=ROOT,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if not report_path.exists():
            detail = result.stderr.strip() or result.stdout.strip() or "runner produced no report"
            raise SystemExit(f"consumer-needs: gameplay semantic validation failed: {detail}")
        value = read_json(report_path)

    checks = [
        check for check in value.get("checks", [])
        if isinstance(check.get("id"), str) and check["id"].startswith("consumerNeed.")
    ]
    summary = {
        "runner": "harness/fixtures/gameplay-module-sdk/downstream-module/src/bin/conformance.rs",
        "valid": value.get("valid") is True and result.returncode == 0,
        "consumerNeedsManifestHash": value.get("consumerNeedsManifestHash"),
        "registryDigest": value.get("registryDigest"),
        "bindingRegistryHash": value.get("bindingRegistryHash"),
        "checks": checks,
    }
    gaps = [
        {
            "manifest": GAMEPLAY_MANIFEST_PATH.relative_to(ROOT).as_posix(),
            "requirement": gap.get("path", "<semantic>"),
            "code": gap.get("code", "semantic_validation_failed"),
            "path": gap.get("path", "<semantic>"),
            "message": gap.get("message", "compiled semantic validation failed"),
        }
        for gap in value.get("gaps", [])
    ]
    if result.returncode != 0 and not gaps:
        gaps.append({
            "manifest": GAMEPLAY_MANIFEST_PATH.relative_to(ROOT).as_posix(),
            "requirement": "<semantic>",
            "code": "semantic_runner_failed",
            "path": "<semantic>",
            "message": result.stderr.strip() or "compiled semantic runner failed",
        })
    return summary, gaps


def canonical_json(value: Any) -> str:
    return json.dumps(value, indent=2, sort_keys=False) + "\n"


def check_negative_fixtures() -> list[str]:
    expectations = {
        "invalid-missing-delivery.json": "missing_delivery_evidence",
        "invalid-prefab-selector.json": "invalid_prefab_part_target",
        "invalid-role-package.json": "role_package_not_allowed",
    }
    failures = []
    fixture_dir = ROOT / "harness/consumer-needs/fixtures"
    for name, expected in expectations.items():
        report = build_report([fixture_dir / name])
        codes = {gap["code"] for gap in report["gaps"]}
        if expected not in codes:
            failures.append(f"{name}: expected {expected}, got {sorted(codes)}")
    for name in (
        "valid-gameplay-read.json",
        "valid-input-needs.json",
        "valid-prefab-needs.json",
    ):
        report = build_report([fixture_dir / name])
        if not report["valid"]:
            codes = sorted({gap["code"] for gap in report["gaps"]})
            failures.append(f"{name}: expected valid, got {codes}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-report", action="store_true")
    parser.add_argument("--check-fixtures", action="store_true")
    args = parser.parse_args()
    paths = sorted(MANIFEST_DIR.glob("*.json"))
    report = build_report(paths)
    semantic_validation, semantic_gaps = run_gameplay_semantic_validation()
    report["semanticValidation"] = semantic_validation
    report["gaps"].extend(semantic_gaps)
    report["gaps"] = sorted(
        report["gaps"],
        key=lambda gap: (gap["manifest"], gap["requirement"], gap["code"], gap["path"]),
    )
    report["valid"] = report["valid"] and semantic_validation["valid"] and not semantic_gaps
    rendered = canonical_json(report)
    if args.write_report:
        REPORT_PATH.write_text(rendered, encoding="utf-8")
    elif not REPORT_PATH.exists() or REPORT_PATH.read_text(encoding="utf-8") != rendered:
        print("consumer-needs: validation-report.json is stale; run validate.py --write-report", file=sys.stderr)
        return 1
    if args.check_fixtures:
        failures = check_negative_fixtures()
        if failures:
            print("\n".join(failures), file=sys.stderr)
            return 1
    if not report["valid"]:
        print(rendered, file=sys.stderr)
        return 1
    print(f"Consumer needs: OK ({report['manifestCount']} manifests, {report['requirementCount']} requirements)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
