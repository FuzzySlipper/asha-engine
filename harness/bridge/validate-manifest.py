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
TYPE_REF_RE = re.compile(r"^protocol_[a-z_]+::[A-Za-z0-9_]+(?:\[\])?$")
NAME_RE = re.compile(r"^[a-z][a-z0-9_]*$")

# Known prototype manifest refs that predate generated contract ownership. Keep
# this list explicit so newly-added protocol_* refs cannot silently point at
# nonexistent generated DTOs.
KNOWN_TRANSITIONAL_PROTOCOL_REFS = {
    "protocol_runtime::EngineConfig",
    "protocol_runtime::EnemyDirectNavMovementRequest",
    "protocol_runtime::EnemyDirectNavMovementResult",
    "protocol_runtime::FpsEncounterDirectorSnapshot",
    "protocol_runtime::FpsEncounterLifecycleInput",
    "protocol_runtime::FpsEncounterTransitionRequest",
    "protocol_runtime::FpsEncounterTransitionResult",
    "protocol_runtime::FpsPrimaryFireRequest",
    "protocol_runtime::FpsPrimaryFireResult",
    "protocol_runtime::FpsRuntimeSessionLoadRequest",
    "protocol_runtime::FpsRuntimeSessionRestartRequest",
    "protocol_runtime::FpsRuntimeSessionSnapshot",
    "protocol_runtime::StepInputEnvelope",
    "protocol_runtime::StepResult",
    "protocol_render::RenderFrameDiffDescriptor",
    "protocol_render::VoxelMeshEvidenceRequest",
    "protocol_render::VoxelMeshEvidenceSnapshot",
    "protocol_replay::ReplayFixture",
    "protocol_replay::ReplayStepReport",
}
PROTOCOL_MODULE_TO_GENERATED_FILE = {
    "protocol_diagnostics": "diagnostics.ts",
    "protocol_render": "render.ts",
    "protocol_replay": "replay.ts",
    "protocol_scene": "scene.ts",
    "protocol_view": "view.ts",
    "protocol_voxel": "voxel.ts",
    "protocol_voxel_conversion": "voxelConversion.ts",
    "protocol_world_bundle": "worldBundle.ts",
}
EXPORT_RE = re.compile(r"export (?:interface|type|const|enum) ([A-Za-z0-9_]+)")


def fail(errors, msg):
    errors.append(msg)


def valid_type_ref(t, handle_types):
    if t in ALLOWED_BARE or t in handle_types:
        return True
    return bool(TYPE_REF_RE.match(t))


def generated_contract_exports():
    exports = {}
    generated_dir = REPO / "ts/packages/contracts/src/generated"
    for protocol_module, filename in PROTOCOL_MODULE_TO_GENERATED_FILE.items():
        path = generated_dir / filename
        if path.exists():
            exports[protocol_module] = set(EXPORT_RE.findall(path.read_text()))
        else:
            exports[protocol_module] = set()
    return exports


def validate_protocol_ref(errors, op_name, field, ref, exports):
    if not ref.startswith("protocol_") or ref in KNOWN_TRANSITIONAL_PROTOCOL_REFS:
        return
    module, type_name = ref.split("::", 1)
    type_name = type_name.removesuffix("[]")
    if module not in PROTOCOL_MODULE_TO_GENERATED_FILE:
        fail(errors, f"[{op_name}] {field} uses unknown protocol module '{module}'")
        return
    if type_name not in exports.get(module, set()):
        fail(errors, f"[{op_name}] {field} protocol ref '{ref}' has no generated @asha/contracts export")


def main():
    errors = []
    if not MANIFEST.exists():
        print(f"FAIL: manifest not found at {MANIFEST}")
        return 1

    with open(MANIFEST, "rb") as f:
        data = tomllib.load(f)

    contract_exports = generated_contract_exports()

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
            validate_protocol_ref(errors, name, field, t, contract_exports)
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
    ops_ts = REPO / "ts/packages/runtime-bridge/src/generated/operations.ts"
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
