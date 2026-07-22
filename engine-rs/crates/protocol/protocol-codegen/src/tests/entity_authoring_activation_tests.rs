use super::*;

pub(super) fn extend_round_trip_coverage(coverage: &mut BTreeSet<String>) {
    coverage.extend(
        [
            "CapabilityActivationRequest",
            "CapabilityActivationEvent",
            "CapabilityActivationReadout",
            "CapabilityActivationDiagnostic",
            "EntityAppearanceBinding",
        ]
        .map(|item| interface_coverage_key("entityAuthoring", item)),
    );
    coverage
        .extend(["accepted", "rejected", "forbidden"].map(|tag| {
            variant_coverage_key("entityAuthoring", "CapabilityActivationOutcome", tag)
        }));
}

#[test]
fn capability_activation_family_emits_closed_vocab_and_projection_shapes() {
    let generated = file("entityAuthoring.ts");
    for value in protocol_entity_authoring::ACTIVATABLE_CAPABILITY_KINDS
        .iter()
        .chain(protocol_entity_authoring::CAPABILITY_ACTIVATION_ACTIONS)
        .chain(protocol_entity_authoring::CAPABILITY_ACTIVATION_PRESENCE_VALUES)
        .chain(protocol_entity_authoring::CAPABILITY_ACTIVATION_ENTITY_LIFECYCLES)
        .chain(protocol_entity_authoring::CAPABILITY_ACTIVATION_DIAGNOSTIC_CODES)
    {
        assert!(generated.contains(&format!("'{value}'")), "missing {value}");
    }
    assert!(generated.contains("export interface CapabilityActivationRequest {"));
    assert!(generated.contains("export interface CapabilityActivationReadout {"));
    assert!(generated.contains("export type CapabilityActivationOutcome ="));
    assert!(generated.contains("readonly status: 'forbidden'"));
}

#[test]
fn capability_activation_samples_match_generated_ir_shapes() {
    let module = module("entityAuthoring");
    let request = json!({
        "entity": 7,
        "capability": "collision",
        "action": "deactivate"
    });
    let event = json!({
        "entity": 7,
        "capability": "collision",
        "from": "active",
        "to": "inactive"
    });
    let readout = json!({
        "entity": 7,
        "capability": "collision",
        "presence": "inactive",
        "entityLifecycle": "active",
        "effectiveActive": false
    });
    let diagnostic = json!({
        "code": "forbiddenOwner",
        "entity": 7,
        "capability": "collision",
        "message": "wrong owner"
    });
    let appearance = json!({
        "resourceId": "presentation/enemy",
        "initialClipId": "idle",
        "modelScale": [1.0, 1.0, 1.0]
    });

    for (name, value) in [
        ("CapabilityActivationRequest", &request),
        ("CapabilityActivationEvent", &event),
        ("CapabilityActivationReadout", &readout),
        ("CapabilityActivationDiagnostic", &diagnostic),
        ("EntityAppearanceBinding", &appearance),
    ] {
        compare_object_to_interface(&module, name, value).unwrap();
    }
    compare_object_to_variant(
        &module,
        "CapabilityActivationOutcome",
        "accepted",
        &json!({ "status": "accepted", "event": event, "readout": readout }),
    )
    .unwrap();
    for status in ["rejected", "forbidden"] {
        compare_object_to_variant(
            &module,
            "CapabilityActivationOutcome",
            status,
            &json!({ "status": status, "diagnostic": diagnostic }),
        )
        .unwrap();
    }
}
