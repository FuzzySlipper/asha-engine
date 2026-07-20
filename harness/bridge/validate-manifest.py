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
NATIVE_ADDON_TS = REPO / "ts/packages/native-bridge/src/native-addon.ts"
NATIVE_RUST_SRC = REPO / "engine-rs/crates/bridge/native-bridge/src"

# Type tokens that would re-open an opaque escape hatch.
FORBIDDEN_TYPE_TOKENS = [
    "serde_json::Value", "Value", "Json", "any", "unknown", "Box<dyn", "dyn ",
]
ALLOWED_BARE = {"Unit", "RuntimeBufferView"}
TYPE_REF_RE = re.compile(r"^protocol_[a-z_]+::[A-Za-z0-9_]+(?:\[\])?$")
NAME_RE = re.compile(r"^[a-z][a-z0-9_]*$")
FACADE_TYPE_REF_RE = re.compile(r"^(?:bridge|contracts|session)::[A-Za-z0-9_]+(?:\[\])?$")
CAPABILITY_ID_RE = re.compile(r"^[a-z][a-z0-9_]*$")
TS_IDENTIFIER_RE = re.compile(r"^[A-Za-z][A-Za-z0-9]*$")
RUST_NAPI_EXPORT_RE = re.compile(
    r"#\[napi(?:\([^\]]*\))?\]\s*(?:#\[[^\]]+\]\s*)*"
    r"pub fn ([a-z][a-z0-9_]*)\s*\("
)

# Cross-transport adapters that are intentionally absent from the public bridge
# manifest and generated RuntimeBridge. Each entry must instead live in the
# separately checked NativeAddonAdapterBindings interface.
PRIVATE_NATIVE_ADAPTER_EXPORTS = {
    "open_workspace_authoring_adapter": "openWorkspaceAuthoringAdapter",
}

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
    "protocol_runtime::FpsRuntimeSessionRestartRequest",
    "protocol_runtime::FpsRuntimeSessionSnapshot",
    "protocol_runtime::GameRuleCatalogValidationReceipt",
    "protocol_runtime::GameRuleEffectIntentRequest",
    "protocol_runtime::GameRuleRuntimeReadout",
    "protocol_runtime::GameExtensionWeaponEffectInvocationRequest",
    "protocol_runtime::GameExtensionWeaponEffectInvocationResult",
    "protocol_runtime::ComposedRuntimeSessionReadout",
    "protocol_runtime::GameplayModuleViewRequest",
    "protocol_runtime::GameplayModuleViewSnapshot",
    "protocol_runtime::GameplayPrefabPartInteractionRequest",
    "protocol_runtime::GameplayPrefabPartInteractionReceipt",
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
    "protocol_game_rules": "gameRules.ts",
    "protocol_input": "input.ts",
    "protocol_time_control": "timeControl.ts",
    "protocol_render": "render.ts",
    "protocol_presentation": "presentation.ts",
    "protocol_replay": "replay.ts",
    "protocol_scene": "scene.ts",
    "protocol_view": "view.ts",
    "protocol_voxel": "voxel.ts",
    "protocol_voxel_annotation": "voxelAnnotation.ts",
    "protocol_voxel_asset": "voxelAsset.ts",
    "protocol_voxel_conversion": "voxelConversion.ts",
    "protocol_voxel_edit_history": "voxelEditHistory.ts",
    "protocol_project_bundle": "projectBundle.ts",
    "protocol_project_content": "projectContent.ts",
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


def snake_to_camel(name):
    head, *rest = name.split("_")
    return head + "".join(part.capitalize() for part in rest)


def facade_method(operation):
    return operation.get("facade_method") or snake_to_camel(operation["name"])


def native_wired(operation):
    return operation.get("native_wired", operation.get("surface") == "stable")


def interface_methods(path, interface_name):
    text = path.read_text(encoding="utf-8")
    marker = f"interface {interface_name} {{"
    start = text.find(marker)
    if start < 0:
        return [], f"{path.relative_to(REPO)} has no {interface_name} interface"
    cursor = start + len(marker)
    depth = 1
    while cursor < len(text) and depth > 0:
        depth += int(text[cursor] == "{") - int(text[cursor] == "}")
        cursor += 1
    if depth != 0:
        return [], f"{path.relative_to(REPO)} has an unterminated {interface_name} interface"
    body = text[start + len(marker):cursor - 1]
    return re.findall(r"^  ([a-z][A-Za-z0-9]*)\(", body, re.MULTILINE), None


def rust_napi_exports():
    exports = []
    for path in sorted(NATIVE_RUST_SRC.rglob("*.rs")):
        if "generated" in path.parts:
            continue
        exports.extend(RUST_NAPI_EXPORT_RE.findall(path.read_text(encoding="utf-8")))
    return exports


def validate_exact_inventory(errors, label, expected, actual):
    expected_set = set(expected)
    actual_set = set(actual)
    if len(actual) != len(actual_set):
        fail(errors, f"{label} contains duplicate entries")
    for name in sorted(expected_set - actual_set):
        fail(errors, f"{label} is missing required entry '{name}'")
    for name in sorted(actual_set - expected_set):
        fail(errors, f"{label} has non-manifest entry '{name}'")


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
    error_families = m.get("error_families", [])
    handle_types = set(m.get("handle_types", []))
    if not error_type:
        fail(errors, "manifest.error_type is required")
    for req in ("owning_crate", "facade_package", "native_package", "version"):
        if req not in m:
            fail(errors, f"manifest.{req} is required")
    if not error_families:
        fail(errors, "manifest.error_families must be nonempty")
    if len(error_families) != len(set(error_families)):
        fail(errors, "manifest.error_families must be unique")
    for family in error_families:
        if not NAME_RE.match(family):
            fail(errors, f"manifest error family '{family}' must be snake_case")

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

        # Facade overrides are explicit mappings to existing semantic DTO owners;
        # they may not introduce ambient or unqualified generated types.
        for field in ("facade_input", "facade_output"):
            if field in op and not FACADE_TYPE_REF_RE.match(op[field]):
                fail(errors, f"[{name}] {field} '{op[field]}' must use bridge::, contracts::, or session::")

        # Rule 3: shared error type
        if op.get("errors") != error_type:
            fail(errors, f"[{name}] errors must equal manifest.error_type '{error_type}'")

        # Rule 4: surface + quarantine_reason
        surface = op.get("surface")
        if surface not in ("stable", "quarantined"):
            fail(errors, f"[{name}] surface must be 'stable' or 'quarantined'")
        if surface == "quarantined" and not op.get("quarantine_reason"):
            fail(errors, f"[{name}] quarantined surface requires quarantine_reason")
        if "native_wired" in op and not isinstance(op["native_wired"], bool):
            fail(errors, f"[{name}] native_wired override must be boolean")
        if surface == "stable" and not native_wired(op):
            fail(errors, f"[{name}] stable surface must remain native wired")

        # Rule 5: buffer-lending ops declare lifetime
        if name in ("get_buffer", "release_buffer") and not op.get("buffers"):
            fail(errors, f"[{name}] buffer operation must declare a 'buffers' lifetime note")

    # Capability cells own generated TS grouping and lifecycle declarations.
    capabilities = data.get("capability", [])
    required_capability_fields = {
        "id", "interface", "property", "initialization", "runtime_project",
        "snapshot_hash", "resource_lifetime", "operations",
    }
    grouped_operations = []
    seen_capability_ids = set()
    seen_interfaces = set()
    seen_properties = set()
    for capability in capabilities:
        capability_id = capability.get("id", "<unnamed>")
        missing_fields = required_capability_fields - set(capability)
        if missing_fields:
            fail(errors, f"capability [{capability_id}] is missing {sorted(missing_fields)}")
        if not CAPABILITY_ID_RE.match(capability_id):
            fail(errors, f"capability id '{capability_id}' must be snake_case")
        if capability_id in seen_capability_ids:
            fail(errors, f"duplicate capability id '{capability_id}'")
        seen_capability_ids.add(capability_id)
        interface = capability.get("interface", "")
        property_name = capability.get("property", "")
        if not TS_IDENTIFIER_RE.match(interface):
            fail(errors, f"capability [{capability_id}] interface '{interface}' is not a TS identifier")
        if not TS_IDENTIFIER_RE.match(property_name):
            fail(errors, f"capability [{capability_id}] property '{property_name}' is not a TS identifier")
        if interface in seen_interfaces:
            fail(errors, f"duplicate capability interface '{interface}'")
        if property_name in seen_properties:
            fail(errors, f"duplicate capability property '{property_name}'")
        seen_interfaces.add(interface)
        seen_properties.add(property_name)
        if capability.get("initialization") not in {"requiresEngine", "createsEngine"}:
            fail(errors, f"capability [{capability_id}] has invalid initialization")
        if capability.get("runtime_project") not in {"retainedAcrossProjectChanges", "ownsProjectLifecycle"}:
            fail(errors, f"capability [{capability_id}] has invalid runtime_project")
        if capability.get("resource_lifetime") not in {"session", "frame", "mixedExplicitAndSession"}:
            fail(errors, f"capability [{capability_id}] has invalid resource_lifetime")
        grouped_operations.extend(capability.get("operations", []))
    manifest_names = {op.get("name") for op in ops}
    for name in sorted(manifest_names.intersection(PRIVATE_NATIVE_ADAPTER_EXPORTS)):
        fail(errors, f"private native adapter '{name}' must not enter the public manifest")
    grouped_names = set(grouped_operations)
    for name in sorted(manifest_names - grouped_names):
        fail(errors, f"operation '{name}' is missing a capability group")
    for name in sorted(grouped_names - manifest_names):
        fail(errors, f"capability group references unknown operation '{name}'")
    for name in sorted({name for name in grouped_operations if grouped_operations.count(name) > 1}):
        fail(errors, f"operation '{name}' appears in multiple capability groups")

    # Parity: the TS facade operation registry must list exactly the manifest ops.
    # (Stands in for generated/conformance.json until the codegen emitter lands.)
    ops_ts = REPO / "ts/packages/runtime-bridge/src/generated/operations.ts"
    if ops_ts.exists():
        ts_names = set(re.findall(r"manifestName:\s*'([a-z_]+)'", ops_ts.read_text()))
        missing = manifest_names - ts_names
        extra = ts_names - manifest_names
        for n in sorted(missing):
            fail(errors, f"operations.ts is missing manifest op '{n}'")
        for n in sorted(extra):
            fail(errors, f"operations.ts lists '{n}' which is not in the manifest")

    # The generated exact declaration is backed by one handwritten semantic
    # signature per stable operation and one concrete Rust #[napi] export. These
    # checks fail an unwired manifest addition before a runtime smoke can begin.
    native_operations = [op for op in ops if native_wired(op)]
    expected_ts_methods = {facade_method(op) for op in native_operations}
    if NATIVE_ADDON_TS.exists():
        actual_ts_methods, interface_error = interface_methods(NATIVE_ADDON_TS, "NativeAddonBindings")
        if interface_error:
            fail(errors, interface_error)
        validate_exact_inventory(
            errors, "NativeAddonBindings", expected_ts_methods, actual_ts_methods
        )
        adapter_ts_methods, adapter_interface_error = interface_methods(
            NATIVE_ADDON_TS, "NativeAddonAdapterBindings"
        )
        if adapter_interface_error:
            fail(errors, adapter_interface_error)
        validate_exact_inventory(
            errors,
            "NativeAddonAdapterBindings",
            set(PRIVATE_NATIVE_ADAPTER_EXPORTS.values()),
            adapter_ts_methods,
        )
    else:
        fail(errors, f"native addon declarations not found at {NATIVE_ADDON_TS}")

    expected_rust_exports = {op["name"] for op in native_operations}
    expected_rust_exports.update(PRIVATE_NATIVE_ADAPTER_EXPORTS)
    actual_rust_exports = rust_napi_exports()
    validate_exact_inventory(
        errors, "native-bridge #[napi] exports", expected_rust_exports, actual_rust_exports
    )

    if errors:
        print(f"Bridge manifest: {len(errors)} problem(s)")
        for e in errors:
            print(f"  FAIL: {e}")
        return 1
    print(f"Bridge manifest: OK ({len(ops)} operations in {len(capabilities)} capabilities, "
          f"{sum(1 for o in ops if o.get('surface') == 'stable')} stable / "
          f"{sum(1 for o in ops if o.get('surface') == 'quarantined')} quarantined)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
