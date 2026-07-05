#!/usr/bin/env python3
"""Validate engine-owned public TypeScript surface metadata."""
from __future__ import annotations

import json
import pathlib
import re
import sys
import tomllib
from typing import Any, NoReturn, cast

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
MANIFEST_PATH = REPO_ROOT / "harness" / "public-surface" / "ts-packages.json"
RAW_TRANSPORT_PACKAGES = {"@asha/native-bridge", "@asha/wasm-replay-bridge"}
VALID_STATUSES = {"public", "unstable", "internal"}


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}")
    sys.exit(1)


def read_json(path: pathlib.Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def package_dir_from_name(package_name: str) -> str:
    prefix = "@asha/"
    if not package_name.startswith(prefix):
        fail(f"public surface manifest package is not an @asha package: {package_name}")
    return package_name.removeprefix(prefix)


def package_root_from_specifier(specifier: str) -> str:
    parts = specifier.split("/")
    if len(parts) < 2 or parts[0] != "@asha":
        fail(f"public surface specifier is not an @asha package specifier: {specifier}")
    return "/".join(parts[:2])


def package_json(package_name: str) -> dict[str, Any]:
    return read_json(REPO_ROOT / "ts" / "packages" / package_dir_from_name(package_name) / "package.json")


def actual_package_names() -> set[str]:
    names: set[str] = set()
    for package_json_path in sorted((REPO_ROOT / "ts" / "packages").glob("*/package.json")):
        data = read_json(package_json_path)
        name = data.get("name")
        if isinstance(name, str):
            names.add(name)
    return names


def require_root_only_export(pkg_name: str, pkg: dict[str, Any], record: dict[str, Any]) -> None:
    exports_value = pkg.get("exports")
    if not isinstance(exports_value, dict):
        fail(f"{pkg_name} must define package exports")
    exports: dict[str, Any] = cast(dict[str, Any], exports_value)
    allowed_subpaths = record.get("allowedExportSubpaths", [])
    if not isinstance(allowed_subpaths, list) or not all(isinstance(path, str) for path in allowed_subpaths):
        fail(f"{pkg_name} allowedExportSubpaths must be a string array when present")
    expected_exports = [".", *cast(list[str], allowed_subpaths)]
    if sorted(exports.keys()) != sorted(expected_exports):
        fail(f"{pkg_name} must expose only approved exports; got {sorted(exports.keys())}, expected {sorted(expected_exports)}")


def github_anchor(heading: str) -> str:
    slug = heading.strip().lower()
    slug = re.sub(r"`([^`]*)`", r"\1", slug)
    slug = re.sub(r"[^a-z0-9 _-]", "", slug)
    slug = re.sub(r"\s+", "-", slug)
    slug = re.sub(r"-+", "-", slug)
    return slug.strip("-")


def markdown_anchors(path: pathlib.Path) -> set[str]:
    anchors: set[str] = set()
    for line in path.read_text().splitlines():
        if not line.startswith("#"):
            continue
        heading = line.lstrip("#").strip()
        if heading:
            anchors.add(github_anchor(heading))
    return anchors


def require_doc_anchor(ref: str, context: str) -> None:
    if "#" not in ref:
        fail(f"{context} must point at a document anchor; got {ref!r}")
    path_text, anchor = ref.split("#", 1)
    doc_path = REPO_ROOT / path_text
    if not doc_path.is_file():
        fail(f"{context} document is missing: {path_text}")
    if anchor not in markdown_anchors(doc_path):
        fail(f"{context} anchor is missing: {ref}")


def compatibility_block(pkg: dict[str, Any]) -> dict[str, Any] | None:
    asha = pkg.get("asha")
    if isinstance(asha, dict) and isinstance(asha.get("compatibility"), dict):
        return cast(dict[str, Any], asha["compatibility"])
    compatibility = pkg.get("compatibility")
    if isinstance(compatibility, dict):
        return cast(dict[str, Any], compatibility)
    return None


def compatibility_marker(pkg_name: str, pkg: dict[str, Any], block: dict[str, Any]) -> str:
    direct_version = block.get("version")
    metadata_file_value = block.get("metadataFile")
    metadata_marker = None
    if isinstance(metadata_file_value, str) and metadata_file_value:
        metadata_path = REPO_ROOT / "ts" / "packages" / package_dir_from_name(pkg_name) / metadata_file_value
        if not metadata_path.is_file():
            fail(f"{pkg_name} compatibility metadata file is missing: {metadata_file_value}")
        metadata = json.loads(metadata_path.read_text())
        if isinstance(metadata, dict):
            metadata_marker_value = metadata.get("compatibilityVersion")
            if isinstance(metadata_marker_value, str) and metadata_marker_value:
                metadata_marker = metadata_marker_value
            if metadata.get("surface") is not None and metadata.get("surface") != pkg_name:
                fail(f"{pkg_name} compatibility metadata surface drifted: {metadata.get('surface')}")
            if metadata.get("packageVersion") is not None and metadata.get("packageVersion") != pkg.get("version"):
                fail(
                    f"{pkg_name} compatibility packageVersion must match package.json version "
                    f"{pkg.get('version')!r}; got {metadata.get('packageVersion')!r}"
                )

    if isinstance(direct_version, str) and direct_version:
        if metadata_marker is not None and metadata_marker != direct_version:
            fail(
                f"{pkg_name} package compatibility version {direct_version!r} "
                f"does not match metadata file marker {metadata_marker!r}"
            )
        return direct_version
    if metadata_marker is not None:
        return metadata_marker
    fail(f"{pkg_name} compatibility metadata must declare version or metadataFile with compatibilityVersion")


def load_ownership() -> dict[str, Any]:
    ownership_path = REPO_ROOT / "governance" / "ownership.toml"
    return tomllib.loads(ownership_path.read_text())


def check_runtime_bridge_api_crate() -> None:
    cargo_path = REPO_ROOT / "engine-rs" / "crates" / "bridge" / "runtime-bridge-api" / "Cargo.toml"
    cargo = tomllib.loads(cargo_path.read_text())
    package_meta = cargo.get("package", {}).get("metadata", {}).get("asha", {})
    public_surface = package_meta.get("public-surface")
    if not isinstance(public_surface, dict):
        fail("runtime-bridge-api Cargo.toml must declare package.metadata.asha.public-surface")
    if public_surface.get("tier") != 1:
        fail("runtime-bridge-api must be marked Tier 1 public bridge boundary")
    if public_surface.get("role") != "rust-bridge-boundary":
        fail("runtime-bridge-api public-surface role must be rust-bridge-boundary")
    if public_surface.get("direct-consumer-import") is not False:
        fail("runtime-bridge-api must document that downstream consumers should not import it directly")


def check_consumer_policies(manifest: dict[str, Any], records_by_package: dict[str, dict[str, Any]]) -> None:
    policies = manifest.get("consumerPolicies")
    if not isinstance(policies, list) or not policies:
        fail("public surface manifest must declare consumerPolicies")

    seen_roles: set[str] = set()
    for policy in policies:
        if not isinstance(policy, dict):
            fail("consumer policy records must be objects")
        role = policy.get("consumerRole")
        if not isinstance(role, str) or not role:
            fail("consumer policy must declare consumerRole")
        if role in seen_roles:
            fail(f"consumer policy duplicates role {role}")
        seen_roles.add(role)

        approved = policy.get("approvedPackageRoots")
        approved_subpaths = policy.get("approvedPackageSubpaths", [])
        forbidden = policy.get("forbiddenPackageRoots")
        patterns = policy.get("forbiddenSpecifierPatterns")
        if not isinstance(approved, list) or not all(isinstance(pkg, str) for pkg in approved):
            fail(f"{role} consumer policy approvedPackageRoots must be a string array")
        if not isinstance(approved_subpaths, list) or not all(isinstance(pkg, str) for pkg in approved_subpaths):
            fail(f"{role} consumer policy approvedPackageSubpaths must be a string array when present")
        if not isinstance(forbidden, list) or not all(isinstance(pkg, str) for pkg in forbidden):
            fail(f"{role} consumer policy forbiddenPackageRoots must be a string array")
        if not isinstance(patterns, list) or not all(isinstance(pattern, str) for pattern in patterns):
            fail(f"{role} consumer policy forbiddenSpecifierPatterns must be a string array")

        approved_set = set(cast(list[str], approved))
        forbidden_set = set(cast(list[str], forbidden))
        overlap = sorted(approved_set & forbidden_set)
        if overlap:
            fail(f"{role} consumer policy approves and forbids package(s): {', '.join(overlap)}")

        for pkg_name in sorted(approved_set):
            record = records_by_package.get(pkg_name)
            if record is None:
                fail(f"{role} consumer policy approves unknown package {pkg_name}")
            if record.get("status") not in {"public", "unstable"}:
                fail(f"{role} consumer policy approves non-public package {pkg_name}")
            allowed_roles = record.get("allowedConsumerRoles")
            if not isinstance(allowed_roles, list) or role not in allowed_roles:
                fail(f"{role} consumer policy approves {pkg_name}, but package manifest does not allow that role")

        for specifier in cast(list[str], approved_subpaths):
            root = package_root_from_specifier(specifier)
            if root not in approved_set:
                fail(f"{role} consumer policy approves subpath {specifier}, but does not approve package root {root}")
            pkg = package_json(root)
            allowed_export_subpaths = (
                pkg.get("asha", {})
                .get("publicSurface", {})
                .get("allowedExportSubpaths", [])
            )
            if not isinstance(allowed_export_subpaths, list):
                allowed_export_subpaths = []
            expected_subpath = f"./{specifier.removeprefix(root + '/')}"
            if expected_subpath not in allowed_export_subpaths:
                fail(f"{role} consumer policy approves {specifier}, but {root} does not expose {expected_subpath}")

        for pkg_name, record in sorted(records_by_package.items()):
            allowed_roles = record.get("allowedConsumerRoles")
            if isinstance(allowed_roles, list) and role in allowed_roles and pkg_name not in approved_set:
                fail(f"{role} is allowed by {pkg_name}, but the consumer policy does not approve that root")

        for pkg_name in sorted(forbidden_set):
            record = records_by_package.get(pkg_name)
            if record is None:
                fail(f"{role} consumer policy forbids unknown package {pkg_name}")
            allowed_roles = record.get("allowedConsumerRoles")
            if isinstance(allowed_roles, list) and role in allowed_roles:
                fail(f"{role} consumer policy forbids {pkg_name}, but package manifest allows that role")

        if not any("*" in pattern for pattern in cast(list[str], patterns)):
            fail(f"{role} consumer policy must include glob-like forbidden specifier patterns")


def check_manifest() -> None:
    manifest = read_json(MANIFEST_PATH)
    if manifest.get("schemaVersion") != 1:
        fail("public surface manifest schemaVersion must be 1")

    records = manifest.get("packages")
    if not isinstance(records, list):
        fail("public surface manifest must contain a packages array")

    ownership_packages = load_ownership().get("package", {})
    seen_packages: set[str] = set()
    manifest_names: set[str] = set()
    records_by_package: dict[str, dict[str, Any]] = {}

    for record in records:
        if not isinstance(record, dict):
            fail("public surface manifest package records must be objects")
        pkg_name = record.get("package")
        if not isinstance(pkg_name, str):
            fail("public surface manifest package record missing package")
        if pkg_name in seen_packages:
            fail(f"public surface manifest duplicates {pkg_name}")
        seen_packages.add(pkg_name)
        manifest_names.add(pkg_name)
        records_by_package[pkg_name] = record

        status = record.get("status")
        if status not in VALID_STATUSES:
            fail(f"{pkg_name} has invalid public surface status {status!r}")
        if pkg_name in RAW_TRANSPORT_PACKAGES and status != "internal":
            fail(f"{pkg_name} is a raw transport and must remain internal")

        ownership_key = record.get("ownershipKey")
        expected_ownership_key = f"ts/packages/{package_dir_from_name(pkg_name)}"
        if ownership_key != expected_ownership_key:
            fail(f"{pkg_name} ownershipKey must be {expected_ownership_key}, got {ownership_key!r}")
        if ownership_key not in ownership_packages:
            fail(f"{pkg_name} ownershipKey {ownership_key} is missing from governance/ownership.toml")

        consumer_role = record.get("intendedConsumerRole")
        if not isinstance(consumer_role, str) or not consumer_role:
            fail(f"{pkg_name} must declare intendedConsumerRole")
        allowed_roles = record.get("allowedConsumerRoles")
        if not isinstance(allowed_roles, list) or not all(isinstance(role, str) for role in allowed_roles):
            fail(f"{pkg_name} allowedConsumerRoles must be a string array")
        if status in {"public", "unstable"} and not allowed_roles:
            fail(f"{pkg_name} {status} surface must declare at least one allowedConsumerRole")

        pkg = package_json(pkg_name)
        if pkg.get("name") != pkg_name:
            fail(f"{pkg_name} package name drifted: {pkg.get('name')}")
        require_root_only_export(pkg_name, pkg, record)

        changelog = record.get("changelog")
        if status in {"public", "unstable"}:
            if not isinstance(changelog, str) or not changelog:
                fail(f"{pkg_name} {status} surface must declare a changelog anchor")
            require_doc_anchor(changelog, f"{pkg_name} public surface changelog")

        block = compatibility_block(pkg)
        if block is None:
            if "compatibilityMarker" in record:
                fail(f"{pkg_name} declares compatibilityMarker but has no package compatibility metadata")
            continue

        marker = compatibility_marker(pkg_name, pkg, block)
        if record.get("compatibilityMarker") != marker:
            fail(
                f"{pkg_name} public surface manifest compatibilityMarker must be {marker!r}, "
                f"got {record.get('compatibilityMarker')!r}"
            )
        metadata_file = block.get("metadataFile")
        if isinstance(metadata_file, str):
            metadata_path = REPO_ROOT / "ts" / "packages" / package_dir_from_name(pkg_name) / metadata_file
            if not metadata_path.is_file():
                fail(f"{pkg_name} compatibility metadata file is missing: {metadata_file}")
        package_changelog = block.get("changelog")
        if not isinstance(package_changelog, str) or not package_changelog:
            fail(f"{pkg_name} compatibility metadata must declare changelog")
        require_doc_anchor(package_changelog, f"{pkg_name} package compatibility changelog")
        if package_changelog != changelog:
            fail(f"{pkg_name} package compatibility changelog must match public surface manifest")

    actual_names = actual_package_names()
    missing = sorted(actual_names - manifest_names)
    extra = sorted(manifest_names - actual_names)
    if missing:
        fail(f"public surface manifest is missing package(s): {', '.join(missing)}")
    if extra:
        fail(f"public surface manifest references missing package(s): {', '.join(extra)}")
    check_consumer_policies(manifest, records_by_package)


check_manifest()
check_runtime_bridge_api_crate()
print("Public engine boundary metadata: OK")
