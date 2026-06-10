#!/usr/bin/env bash
# Verifies that no TypeScript package imports a package listed under
# may_not_import in governance/ownership.toml.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

python3 - "$REPO_ROOT" <<'PYEOF'
import sys, tomllib, pathlib, json, re

repo = pathlib.Path(sys.argv[1])
ownership_path = repo / "governance" / "ownership.toml"
ts_packages = repo / "ts" / "packages"

with open(ownership_path, "rb") as f:
    ownership = tomllib.load(f)

packages = ownership.get("package", {})
failures = []

for pkg_dir in sorted(ts_packages.iterdir()):
    if not pkg_dir.is_dir():
        continue
    pkg_name = pkg_dir.name
    ownership_key = f"ts/packages/{pkg_name}"
    pkg_meta = packages.get(ownership_key, {})
    forbidden = pkg_meta.get("may_not_import", [])
    if not forbidden:
        continue

    # Collect all @asha/* imports from source files
    imports_found: set[str] = set()
    for src_file in pkg_dir.rglob("*.ts"):
        text = src_file.read_text()
        # Matches both `... from "@asha/x"` and side-effect `import "@asha/x"`.
        for match in re.finditer(r'(?:from|import)\s+["\'](@asha/[a-z-]+)["\']', text):
            imports_found.add(match.group(1))

    # Also check package.json dependencies
    pkg_json = pkg_dir / "package.json"
    if pkg_json.exists():
        data = json.loads(pkg_json.read_text())
        for section in ("dependencies", "devDependencies", "peerDependencies"):
            for dep in data.get(section, {}):
                if dep.startswith("@asha/"):
                    imports_found.add(dep)

    pkg_lane = pkg_meta.get("lane", "?")
    for fi in forbidden:
        if fi in imports_found:
            target_short = fi.split("/", 1)[-1]
            target_lane = packages.get(f"ts/packages/{target_short}", {}).get("lane", "?")
            failures.append(
                f"FAIL: {ownership_key} (lane {pkg_lane}) imports forbidden package "
                f"'{fi}' (lane {target_lane}).\n"
                f"      Route this through the contract border or move the dependency "
                f"into a {target_lane} package — do not relax the boundary."
            )

if failures:
    for msg in failures:
        print(msg)
    sys.exit(1)
else:
    print("TypeScript dep graph: OK")
PYEOF
