#!/usr/bin/env python3
"""Validate the curated runtime bridge manifest against the documented schema.

Enforces the rules in
engine-rs/crates/bridge/runtime-bridge-api/MANIFEST-FORMAT.md §2. This is the
"manifest test" for task #2249 verification: it proves the manifest is a bounded,
typed, one-in/one-out surface and NOT a methodName+json RPC registry.

Run: python3 harness/bridge/validate-manifest.py
"""
import sys
import pathlib
import re
import tomllib

REPO = pathlib.Path(__file__).resolve().parents[2]
MANIFEST = REPO / "engine-rs/crates/bridge/runtime-bridge-api/bridge-manifest.toml"

# Type tokens that would re-open an opaque escape hatch.
FORBIDDEN_TYPE_TOKENS = [
    "serde_json::Value", "Value", "Json", "any", "unknown", "Box<dyn", "dyn ",
]
ALLOWED_BARE = {"Unit", "RuntimeBufferView"}
TYPE_REF_RE = re.compile(r"^protocol_[a-z_]+::[A-Za-z0-9_]+$")
NAME_RE = re.compile(r"^[a-z][a-z0-9_]*$")


def fail(errors, msg):
    errors.append(msg)


def valid_type_ref(t, handle_types):
    if t in ALLOWED_BARE or t in handle_types:
        return True
    return bool(TYPE_REF_RE.match(t))


def main():
    errors = []
    if not MANIFEST.exists():
        print(f"FAIL: manifest not found at {MANIFEST}")
        return 1

    with open(MANIFEST, "rb") as f:
        data = tomllib.load(f)

    m = data.get("manifest", {})
    error_type = m.get("error_type")
    handle_types = set(m.get("handle_types", []))
    if not error_type:
        fail(errors, "manifest.error_type is required")
    for req in ("owning_crate", "facade_package", "native_package", "version"):
        if req not in m:
            fail(errors, f"manifest.{req} is required")

    ops = data.get("operation", [])
    if not ops:
        fail(errors, "no operations defined")

    seen = set()
    for op in ops:
        name = op.get("name", "<unnamed>")
        # Rule 6: unique, snake_case
        if not NAME_RE.match(name):
            fail(errors, f"[{name}] name must be snake_case")
        if name in seen:
            fail(errors, f"[{name}] duplicate operation name")
        seen.add(name)

        # Rule 1: exactly one input + one output
        for field in ("input", "output"):
            if field not in op:
                fail(errors, f"[{name}] missing required '{field}'")
        if "args" in op or "params" in op or "method" in op:
            fail(errors, f"[{name}] variadic/methodName-style fields are forbidden")

        # Rule 2: typed refs only, no forbidden tokens
        for field in ("input", "output"):
            t = op.get(field, "")
            if not valid_type_ref(t, handle_types):
                fail(errors, f"[{name}] {field} '{t}' is not a protocol_*::Type, "
                             f"declared handle, RuntimeBufferView, or Unit")
            for bad in FORBIDDEN_TYPE_TOKENS:
                if bad in t:
                    fail(errors, f"[{name}] {field} contains forbidden token '{bad}'")

        # Rule 3: shared error type
        if op.get("errors") != error_type:
            fail(errors, f"[{name}] errors must equal manifest.error_type '{error_type}'")

        # Rule 4: surface + quarantine_reason
        surface = op.get("surface")
        if surface not in ("stable", "quarantined"):
            fail(errors, f"[{name}] surface must be 'stable' or 'quarantined'")
        if surface == "quarantined" and not op.get("quarantine_reason"):
            fail(errors, f"[{name}] quarantined surface requires quarantine_reason")

        # Rule 5: buffer-lending ops declare lifetime
        if name in ("get_buffer", "release_buffer") and not op.get("buffers"):
            fail(errors, f"[{name}] buffer operation must declare a 'buffers' lifetime note")

    # Parity: the TS facade operation registry must list exactly the manifest ops.
    # (Stands in for generated/conformance.json until the codegen emitter lands.)
    ops_ts = REPO / "ts/packages/runtime-bridge/src/operations.ts"
    if ops_ts.exists():
        ts_names = set(re.findall(r"manifestName:\s*'([a-z_]+)'", ops_ts.read_text()))
        manifest_names = {op.get("name") for op in ops}
        missing = manifest_names - ts_names
        extra = ts_names - manifest_names
        for n in sorted(missing):
            fail(errors, f"operations.ts is missing manifest op '{n}'")
        for n in sorted(extra):
            fail(errors, f"operations.ts lists '{n}' which is not in the manifest")

    if errors:
        print(f"Bridge manifest: {len(errors)} problem(s)")
        for e in errors:
            print(f"  FAIL: {e}")
        return 1
    print(f"Bridge manifest: OK ({len(ops)} operations, "
          f"{sum(1 for o in ops if o.get('surface') == 'stable')} stable / "
          f"{sum(1 for o in ops if o.get('surface') == 'quarantined')} quarantined)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
