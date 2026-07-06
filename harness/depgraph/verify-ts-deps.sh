#!/usr/bin/env bash
# Verifies that each TypeScript package's internal @asha/* imports are listed
# under may_import in governance/ownership.toml, and that every package has an
# ownership entry unless explicitly exempted.
set -euo pipefail

REPO_ROOT="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

python3 - "$REPO_ROOT" <<'PYEOF'
import sys, tomllib, pathlib, json, re

repo = pathlib.Path(sys.argv[1])
ownership_path = repo / "governance" / "ownership.toml"
ts_packages = repo / "ts" / "packages"
public_surface_manifest_path = repo / "harness" / "public-surface" / "ts-packages.json"

with open(ownership_path, "rb") as f:
    ownership = tomllib.load(f)

packages = ownership.get("package", {})
failures = []
ownership_exempt = set(ownership.get("ownership_exempt", {}).get("packages", []))
valid_package_types = {"lib", "shell", "testing", "tool"}
valid_package_layers = {
    "protocol",
    "transport",
    "domain",
    "renderer",
    "components",
    "shell",
    "testing-fixtures",
    "tool",
}
valid_implementation_statuses = {"active", "reserved"}
approved_bare_three_packages = {"ts/packages/renderer-three"}

actual_packages: dict[str, tuple[str, pathlib.Path, dict]] = {}
for pkg_dir in sorted(ts_packages.iterdir()):
    if not pkg_dir.is_dir():
        continue
    pkg_json = pkg_dir / "package.json"
    if not pkg_json.exists():
        continue
    data = json.loads(pkg_json.read_text())
    package_name = data.get("name")
    if package_name:
        actual_packages[f"ts/packages/{pkg_dir.name}"] = (package_name, pkg_dir, data)


def package_name_for_key(ownership_key: str) -> str:
    package_dir_name = ownership_key.rsplit("/", 1)[-1]
    return f"@asha/{package_dir_name}"


known_asha_packages = {name for name, _pkg_dir, _data in actual_packages.values()}
known_asha_packages.update(package_name_for_key(key) for key in packages)


def load_approved_export_subpaths() -> dict[str, set[str]]:
    if not public_surface_manifest_path.exists():
        return {}
    manifest = json.loads(public_surface_manifest_path.read_text())
    approved: dict[str, set[str]] = {}
    for record in manifest.get("packages", []):
        if not isinstance(record, dict):
            continue
        package_name = record.get("package")
        if not isinstance(package_name, str):
            continue
        subpaths = set()
        for subpath in record.get("allowedExportSubpaths", []):
            if isinstance(subpath, str):
                subpaths.add(subpath)
        approved[package_name] = subpaths
    return approved


approved_export_subpaths = load_approved_export_subpaths()


def approved_subpath_specifiers(package_name: str) -> set[str]:
    specifiers = set()
    for subpath in approved_export_subpaths.get(package_name, set()):
        if subpath.startswith("./"):
            specifiers.add(f"{package_name}/{subpath[2:]}")
    return specifiers


def collect_source_imports(pkg_dir: pathlib.Path, package_name: str) -> tuple[set[str], set[str]]:
    imports_found: set[str] = set()
    deep_imports_found: set[str] = set()
    src_dir = pkg_dir / "src"
    if not src_dir.exists():
        return imports_found, deep_imports_found

    import_re = re.compile(
        r"(?:from\s+|import\s+(?:type\s+)?|import\s*\(\s*)"
        r"[\"'](@asha/[a-z0-9-]+)(/[^\"']*)?[\"']"
    )
    for src_file in src_dir.rglob("*.ts"):
        text = src_file.read_text()
        for match in import_re.finditer(text):
            imported_package = match.group(1)
            imported_suffix = match.group(2)
            if imported_package != package_name and imported_package in known_asha_packages:
                imports_found.add(imported_package)
                if imported_suffix:
                    deep_imports_found.add(f"{imported_package}{imported_suffix}")
    return imports_found, deep_imports_found


def collect_manifest_imports(pkg_dir: pathlib.Path, package_name: str) -> set[str]:
    imports_found: set[str] = set()
    pkg_json = pkg_dir / "package.json"
    if not pkg_json.exists():
        return imports_found
    data = json.loads(pkg_json.read_text())
    for section in ("dependencies", "devDependencies", "peerDependencies"):
        for dep in data.get(section, {}):
            if dep.startswith("@asha/") and dep != package_name:
                imports_found.add(dep)
    return imports_found


def collect_bare_three_references(pkg_dir: pathlib.Path, pkg_data: dict) -> list[str]:
    references: list[str] = []
    for section in ("dependencies", "devDependencies", "peerDependencies"):
        for dep in pkg_data.get(section, {}):
            if dep in {"three", "@types/three"}:
                references.append(f"package.json {section}.{dep}")

    src_dir = pkg_dir / "src"
    if not src_dir.exists():
        return references
    import_re = re.compile(r"(?:from\s+|import\s+(?:type\s+)?|import\s*\(\s*)[\"']three(?:/[^\"']*)?[\"']")
    concrete_api_re = re.compile(r"\bTHREE\.|\bnew\s+WebGLRenderer\b")
    for src_file in src_dir.rglob("*.ts"):
        text = src_file.read_text()
        rel = src_file.relative_to(repo).as_posix()
        if import_re.search(text):
            references.append(f"{rel} imports bare three")
        if concrete_api_re.search(text):
            references.append(f"{rel} references concrete Three.js API")
    return references


def has_root_export(pkg_data: dict) -> bool:
    exports = pkg_data.get("exports")
    return isinstance(exports, dict) and "." in exports


def non_root_export_keys(pkg_data: dict) -> list[str]:
    exports = pkg_data.get("exports")
    if not isinstance(exports, dict):
        return []
    return sorted(key for key in exports if key != ".")


for ownership_key, (package_name, pkg_dir, pkg_data) in actual_packages.items():
    if ownership_key not in packages and ownership_key not in ownership_exempt:
        failures.append(f"FAIL: {ownership_key} has no ownership entry in governance/ownership.toml")
        continue

    pkg_meta = packages.get(ownership_key, {})
    pkg_lane = pkg_meta.get("lane", "?")
    allowed = set(pkg_meta.get("may_import", []))
    forbidden = set(pkg_meta.get("may_not_import", []))
    imports_found, deep_imports_found = collect_source_imports(pkg_dir, package_name)
    imports_found.update(collect_manifest_imports(pkg_dir, package_name))
    bare_three_references = collect_bare_three_references(pkg_dir, pkg_data)

    if not has_root_export(pkg_data):
        failures.append(f"FAIL: {ownership_key} package.json must expose root package API via exports['.']")
    extra_export_keys = [
        key for key in non_root_export_keys(pkg_data)
        if key not in approved_export_subpaths.get(package_name, set())
    ]
    if extra_export_keys:
        failures.append(
            f"FAIL: {ownership_key} package.json exposes non-root export(s): "
            f"{', '.join(extra_export_keys)}. ASHA packages are consumed through root barrels."
        )
    if not (pkg_dir / "src" / "index.ts").exists():
        failures.append(f"FAIL: {ownership_key} must expose its source root barrel at src/index.ts")

    if bare_three_references and ownership_key not in approved_bare_three_packages:
        failures.append(
            f"FAIL: {ownership_key} references bare Three.js backend APIs outside an approved renderer backend package:\n"
            + "\n".join(f"      - {reference}" for reference in bare_three_references)
        )

    for dep in sorted(allowed & forbidden):
        failures.append(f"FAIL: {ownership_key} lists '{dep}' in both may_import and may_not_import.")

    for specifier in sorted(deep_imports_found):
        if specifier in approved_subpath_specifiers(specifier.split("/", 2)[0] + "/" + specifier.split("/", 2)[1]):
            continue
        failures.append(
            f"FAIL: {ownership_key} imports deep sibling package path '{specifier}'.\n"
            f"      Import the package root barrel instead (for example '@asha/name'), "
            f"and re-export approved API from that package's src/index.ts."
        )

    for dep in sorted(imports_found):
        target_short = dep.split("/", 1)[-1]
        target_lane = packages.get(f"ts/packages/{target_short}", {}).get("lane", "?")
        if dep in forbidden:
            failures.append(
                f"FAIL: {ownership_key} (lane {pkg_lane}) imports forbidden package "
                f"'{dep}' (lane {target_lane}).\n"
                f"      Route this through the contract border or move the dependency "
                f"into a {target_lane} package — do not relax the boundary."
            )
            continue
        if dep not in allowed:
            failures.append(
                f"FAIL: {ownership_key} (lane {pkg_lane}) imports unlisted internal "
                f"package '{dep}' (lane {target_lane}).\n"
                f"      Add it to governance/ownership.toml may_import only if this is an "
                f"approved package boundary; otherwise route through the existing public API."
            )

for ownership_key in sorted(packages):
    pkg_meta = packages[ownership_key]
    package_type = pkg_meta.get("type")
    package_layer = pkg_meta.get("layer")
    implementation_status = pkg_meta.get("implementation_status", "active")
    if package_type is None:
        failures.append(f"FAIL: {ownership_key} is missing required TypeScript ownership field 'type'")
    elif package_type not in valid_package_types:
        failures.append(
            f"FAIL: {ownership_key} has invalid TypeScript ownership type '{package_type}'. "
            f"Allowed values: {', '.join(sorted(valid_package_types))}"
        )
    if package_layer is None:
        failures.append(f"FAIL: {ownership_key} is missing required TypeScript ownership field 'layer'")
    elif package_layer not in valid_package_layers:
        failures.append(
            f"FAIL: {ownership_key} has invalid TypeScript ownership layer '{package_layer}'. "
            f"Allowed values: {', '.join(sorted(valid_package_layers))}"
        )
    if implementation_status not in valid_implementation_statuses:
        failures.append(
            f"FAIL: {ownership_key} has invalid TypeScript ownership implementation_status "
            f"'{implementation_status}'. Allowed values: "
            f"{', '.join(sorted(valid_implementation_statuses))}"
        )

if failures:
    for msg in failures:
        print(msg)
    sys.exit(1)
else:
    print("TypeScript dep graph: OK")
PYEOF
