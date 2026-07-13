#!/usr/bin/env bash
# Verifies that each Rust crate's internal ASHA crate dependencies are listed
# under may_depend_on in governance/ownership.toml, and that explicit
# may_not_depend_on entries are not present.
set -euo pipefail

REPO_ROOT="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

python3 - "$REPO_ROOT" <<'PYEOF'
import sys, tomllib, pathlib

repo = pathlib.Path(sys.argv[1])
ownership_path = repo / "governance" / "ownership.toml"
engine_rs = repo / "engine-rs"

with open(ownership_path, "rb") as f:
    ownership = tomllib.load(f)

crates = ownership.get("crate", {})
workspace_toml = engine_rs / "Cargo.toml"
with open(workspace_toml, "rb") as f:
    workspace = tomllib.load(f)

failures = []
ownership_exempt = set(ownership.get("ownership_exempt", {}).get("crates", []))
workspace_members = workspace.get("workspace", {}).get("members", [])
internal_crates = {}
valid_implementation_statuses = {"active", "reserved"}

for rel_path in workspace_members:
    cargo_toml = engine_rs / rel_path / "Cargo.toml"
    if not cargo_toml.exists():
        continue
    with open(cargo_toml, "rb") as f:
        crate_cfg = tomllib.load(f)
    package_name = crate_cfg.get("package", {}).get("name")
    if package_name:
        internal_crates[package_name] = f"engine-rs/{rel_path}"

# Ownership also governs intentionally excluded standalone crates such as the
# napi addon. Include them in the name map and edge validation even though Cargo
# workspace membership is deliberately absent.
for ownership_key in crates:
    cargo_toml = repo / ownership_key / "Cargo.toml"
    if not cargo_toml.exists():
        continue
    with open(cargo_toml, "rb") as f:
        crate_cfg = tomllib.load(f)
    package_name = crate_cfg.get("package", {}).get("name")
    if package_name:
        internal_crates[package_name] = ownership_key

for ownership_key, crate_meta in crates.items():
    implementation_status = crate_meta.get("implementation_status", "active")
    if implementation_status not in valid_implementation_statuses:
        failures.append(
            f"FAIL: {ownership_key} has invalid Rust ownership implementation_status "
            f"'{implementation_status}'. Allowed values: "
            f"{', '.join(sorted(valid_implementation_statuses))}"
        )


def normalized(name: str) -> str:
    return name.replace("-", "_")


def configured_list(crate_meta: dict, key: str):
    value = crate_meta.get(key, [])
    if value == "unrestricted":
        return value
    return list(value)


def dependency_package_name(dep_name: str, dep_spec) -> str:
    if isinstance(dep_spec, dict):
        return dep_spec.get("package", dep_name)
    return dep_name


def internal_dependencies(crate_cfg: dict):
    deps = []
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for dep_name, dep_spec in crate_cfg.get(section, {}).items():
            package_name = dependency_package_name(dep_name, dep_spec)
            if package_name in internal_crates:
                deps.append((section, package_name))
    return deps


checked_ownership_keys = {f"engine-rs/{rel_path}" for rel_path in workspace_members}
checked_ownership_keys.update(
    ownership_key
    for ownership_key in crates
    if (repo / ownership_key / "Cargo.toml").exists()
)

for ownership_key in sorted(checked_ownership_keys):
    crate_path = repo / ownership_key

    if ownership_key not in crates and ownership_key not in ownership_exempt:
        failures.append(f"FAIL: {ownership_key} has no ownership entry in governance/ownership.toml")
        continue

    crate_meta = crates.get(ownership_key, {})
    cargo_toml = crate_path / "Cargo.toml"
    if not cargo_toml.exists():
        continue
    with open(cargo_toml, "rb") as f:
        crate_cfg = tomllib.load(f)

    crate_lane = crate_meta.get("lane", "?")
    allowed = configured_list(crate_meta, "may_depend_on")
    forbidden = set(crate_meta.get("may_not_depend_on", []))

    for section, dep in internal_dependencies(crate_cfg):
        dep_key = internal_crates.get(dep, "")
        dep_lane = crates.get(dep_key, {}).get("lane", "?")

        if dep in forbidden or any(normalized(dep) == normalized(fd) for fd in forbidden):
            failures.append(
                f"FAIL: {ownership_key} (lane {crate_lane}) depends on explicitly forbidden "
                f"internal crate '{dep}' from {section} (lane {dep_lane}).\n"
                f"      Route this through the approved boundary or move the code into a "
                f"{dep_lane} crate; do not relax the boundary casually."
            )
            continue

        if allowed != "unrestricted" and not any(normalized(dep) == normalized(ad) for ad in allowed):
            failures.append(
                f"FAIL: {ownership_key} (lane {crate_lane}) depends on unlisted internal "
                f"crate '{dep}' from {section} (lane {dep_lane}).\n"
                f"      Add it to governance/ownership.toml may_depend_on only if this is an "
                f"approved lane boundary; otherwise route through the existing border."
            )

if failures:
    for msg in failures:
        print(msg)
    sys.exit(1)
else:
    print("Rust dep graph: OK")
PYEOF
