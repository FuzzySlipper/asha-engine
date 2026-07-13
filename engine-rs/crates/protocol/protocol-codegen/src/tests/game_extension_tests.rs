use super::*;

pub(super) fn extend_round_trip_coverage(coverage: &mut BTreeSet<String>) {
    coverage.extend(
        [
            "GameplayContractRef",
            "GameplayModuleRef",
            "GameplayModuleConfiguration",
            "GameplayModuleBinding",
            "GameplayModuleBindingOverride",
            "GameplayModuleBindingRegistry",
            "GameplayModuleBindingDiagnostic",
            "GameplayModuleBindingReadout",
            "GameplayModuleBindingActivationReceipt",
            "GameplayOwnerRef",
            "GameplayEventSchemaDeclaration",
            "GameplayEntityRef",
            "GameplayCausationRef",
            "GameplayEventEnvelope",
            "GameplayHeaderSelector",
            "GameplaySubscriptionDeclaration",
            "GameplayInvocationReadRequirement",
            "GameplayInvocationDescriptor",
            "GameplayProposalDeclaration",
            "GameplayProposalEnvelope",
            "GameplayReadViewRequirement",
            "GameplayReadViewProviderReadout",
            "GameplayOwnedSchemaDeclaration",
            "GameplayOrderingConstraint",
            "GameplayExecutionBudget",
            "GameplayModuleManifest",
            "GameplayRegistryDiagnostic",
            "GameplayTopologyEdge",
            "GameplayRegistryReadout",
        ]
        .map(|item| interface_coverage_key("gameExtension", item)),
    );
    coverage.extend(
        [
            ("GameplayEmitterRef", "owner"),
            ("GameplayEmitterRef", "module"),
            ("GameplayEmitterRef", "scheduler"),
            ("GameplayModuleBindingTarget", "session"),
            ("GameplayModuleBindingTarget", "entityDefinition"),
            ("GameplayModuleBindingTarget", "prefab"),
            ("GameplayModuleBindingTarget", "prefabPart"),
            ("GameplayRegistryValidationOutcome", "valid"),
            ("GameplayRegistryValidationOutcome", "invalid"),
        ]
        .map(|(item, tag)| variant_coverage_key("gameExtension", item, tag)),
    );
}

/// Focused behavior test for the `gameExtension` family: stable hook/proposal/
/// receipt/diagnostic vocabularies are sourced from `protocol-game-extension`,
/// while manifests, hook requests, proposals, receipts, and replay evidence are
/// generated and publicly re-exported. Guard for #4516.
#[test]
fn game_extension_family_emits_vocab_and_shapes() {
    let ext = file("gameExtension.ts");
    for kind in protocol_game_extension::GAME_EXTENSION_HOOK_KINDS {
        assert!(
            ext.contains(&format!("'{kind}'")),
            "missing hook kind {kind}"
        );
    }
    for kind in protocol_game_extension::GAME_EXTENSION_PROPOSAL_KINDS {
        assert!(
            ext.contains(&format!("'{kind}'")),
            "missing proposal kind {kind}"
        );
    }
    for status in protocol_game_extension::GAME_EXTENSION_RECEIPT_STATUSES {
        assert!(
            ext.contains(&format!("'{status}'")),
            "missing receipt status {status}"
        );
    }
    for code in protocol_game_extension::GAME_EXTENSION_DIAGNOSTIC_CODES {
        assert!(
            ext.contains(&format!("'{code}'")),
            "missing diagnostic {code}"
        );
    }
    for family in protocol_game_extension::GAMEPLAY_INVOCATION_FAMILIES {
        assert!(
            ext.contains(&format!("'{family}'")),
            "missing family {family}"
        );
    }
    for phase in protocol_game_extension::GAMEPLAY_EVENT_PHASES {
        assert!(ext.contains(&format!("'{phase}'")), "missing phase {phase}");
    }
    for code in protocol_game_extension::GAMEPLAY_REGISTRY_DIAGNOSTIC_CODES {
        assert!(
            ext.contains(&format!("'{code}'")),
            "missing registry diagnostic {code}"
        );
    }

    assert!(ext.contains("import type { EntityId } from './ids.js';"));
    assert!(ext.contains("import type { DiagnosticSeverity } from './diagnostics.js';"));
    assert!(ext.contains("export interface GameRuleModuleManifest {"));
    assert!(ext.contains("export interface WeaponEffectHookRequest {"));
    assert!(ext.contains("export type GameExtensionProposal ="));
    assert!(ext.contains("readonly kind: 'damageModifier'"));
    assert!(ext.contains("export interface GameExtensionReplayEvidence {"));
    assert!(ext.contains("export interface GameplayModuleManifest {"));
    assert!(ext.contains("export type GameplayEmitterRef ="));
    assert!(ext.contains("export type GameplayRegistryValidationOutcome ="));
}

#[test]
fn gameplay_module_binding_serialization_matches_ir_shape() {
    use core_ids::{PrefabId, PrefabInstanceId};
    use protocol_game_extension::*;
    use protocol_project_bundle::PrefabPartReference;

    let game_extension = module("gameExtension");
    let contract = |name: &str| GameplayContractRef {
        namespace: "game.fixture".to_owned(),
        name: name.to_owned(),
        version: 1,
        schema_hash: format!("sha256:{name}"),
    };
    let module_ref = GameplayModuleRef {
        module_id: "game.fixture.module".to_owned(),
        namespace: "game.fixture".to_owned(),
        version: "1.0.0".to_owned(),
        sdk_hash: "sha256:sdk".to_owned(),
        contract_hash: "sha256:contract".to_owned(),
        artifact_hash: "sha256:artifact".to_owned(),
        provider_id: "provider.game.fixture".to_owned(),
    };
    let configuration = GameplayModuleConfiguration {
        configuration_id: "fixture.default".to_owned(),
        module: module_ref,
        configuration: contract("configuration"),
        codec_id: "codec.game.fixture.configuration".to_owned(),
        canonical_config: br#"{"amount":4}"#.to_vec(),
        config_hash: "fnv1a64:configuration".to_owned(),
    };
    let targets = [
        GameplayModuleBindingTarget::Session,
        GameplayModuleBindingTarget::EntityDefinition {
            stable_id: "entity.fixture".to_owned(),
        },
        GameplayModuleBindingTarget::Prefab {
            prefab: PrefabId::new(4),
        },
        GameplayModuleBindingTarget::PrefabPart {
            part: PrefabPartReference {
                prefab: PrefabId::new(4),
                role: "weapon/muzzle".to_owned(),
            },
        },
    ];
    for (target, tag) in targets
        .iter()
        .zip(["session", "entityDefinition", "prefab", "prefabPart"])
    {
        compare_object_to_variant(
            &game_extension,
            "GameplayModuleBindingTarget",
            tag,
            &serde_json::to_value(target).unwrap(),
        )
        .unwrap();
    }
    let binding = GameplayModuleBinding {
        binding_id: "binding.fixture".to_owned(),
        module_id: "game.fixture.module".to_owned(),
        configuration_id: "fixture.default".to_owned(),
        state_schema: contract("state"),
        target: targets[3].clone(),
        required_reads: Vec::new(),
        output_contracts: vec![contract("result")],
        enabled: true,
    };
    let override_layer = GameplayModuleBindingOverride {
        binding_id: binding.binding_id.clone(),
        prefab_instance: PrefabInstanceId::new(8),
        configuration_id: None,
        enabled: Some(false),
    };
    let registry = GameplayModuleBindingRegistry {
        schema_version: GAMEPLAY_MODULE_BINDING_SCHEMA_VERSION,
        configurations: vec![configuration.clone()],
        bindings: vec![binding.clone()],
        overrides: vec![override_layer.clone()],
        registry_hash: "fnv1a64:registry".to_owned(),
    };
    let diagnostic = GameplayModuleBindingDiagnostic {
        code: GameplayModuleBindingDiagnosticCode::InvalidOverride,
        path: "overrides[0]".to_owned(),
        message: "invalid fixture override".to_owned(),
    };
    let readout = GameplayModuleBindingReadout {
        binding_id: binding.binding_id.clone(),
        module_id: binding.module_id.clone(),
        configuration_id: configuration.configuration_id.clone(),
        target: binding.target.clone(),
        resolved_scopes: vec!["entity:4".to_owned()],
        active: true,
        provenance_hash: "fnv1a64:provenance".to_owned(),
    };
    let receipt = GameplayModuleBindingActivationReceipt {
        binding_registry_hash: registry.registry_hash.clone(),
        gameplay_registry_digest: "fnv1a64:gameplay".to_owned(),
        readouts: vec![readout.clone()],
        module_state_hash: "fnv1a64:state".to_owned(),
        receipt_hash: "fnv1a64:receipt".to_owned(),
    };
    for (name, value) in [
        (
            "GameplayModuleConfiguration",
            serde_json::to_value(configuration).unwrap(),
        ),
        (
            "GameplayModuleBinding",
            serde_json::to_value(binding).unwrap(),
        ),
        (
            "GameplayModuleBindingOverride",
            serde_json::to_value(override_layer).unwrap(),
        ),
        (
            "GameplayModuleBindingRegistry",
            serde_json::to_value(registry).unwrap(),
        ),
        (
            "GameplayModuleBindingDiagnostic",
            serde_json::to_value(diagnostic).unwrap(),
        ),
        (
            "GameplayModuleBindingReadout",
            serde_json::to_value(readout).unwrap(),
        ),
        (
            "GameplayModuleBindingActivationReceipt",
            serde_json::to_value(receipt).unwrap(),
        ),
    ] {
        compare_object_to_interface(&game_extension, name, &value).unwrap();
    }
}

#[test]
fn game_extension_rust_serialization_matches_ir_shape() {
    use core_ids::EntityId;
    use protocol_diagnostics::DiagnosticSeverity;
    use protocol_game_extension::{
        GameExtensionDiagnostic, GameExtensionDiagnosticCode, GameExtensionHookKind,
        GameExtensionHookReceipt, GameExtensionProposal, GameExtensionReceiptStatus,
        GameExtensionReplayEvidence, GameExtensionTraceEntry, GameRuleHookDeclaration,
        GameRuleModuleManifest, GameRuleModuleRef, WeaponEffectHookRequest,
    };

    let game_extension = module("gameExtension");
    let module_ref = GameRuleModuleRef {
        module_id: "demo.primary_fire_effect".to_string(),
        version: "0.1.0".to_string(),
        contract_hash: "sha256:contract".to_string(),
    };
    let hook = GameRuleHookDeclaration {
        hook_id: "weapon.primary".to_string(),
        kind: GameExtensionHookKind::WeaponEffect,
        input_contract: "WeaponEffectHookRequest.v0".to_string(),
        output_contract: "GameExtensionProposal.v0".to_string(),
        required_capabilities: vec!["health".to_string(), "weaponMount".to_string()],
    };
    let manifest = GameRuleModuleManifest {
        module_ref: module_ref.clone(),
        declared_hooks: vec![hook.clone()],
        deterministic_requirements: vec![
            "no-wall-clock".to_string(),
            "no-ambient-random".to_string(),
            "no-ts-callback".to_string(),
        ],
        source_hash: "sha256:module-source".to_string(),
    };
    let diagnostic = GameExtensionDiagnostic {
        code: GameExtensionDiagnosticCode::InvalidProposal,
        severity: DiagnosticSeverity::Error,
        path: "proposal".to_string(),
        message: "proposal is invalid".to_string(),
    };
    let request = WeaponEffectHookRequest {
        module_ref: module_ref.clone(),
        hook_id: "weapon.primary".to_string(),
        request_id: "request-1".to_string(),
        tick: 42,
        source: EntityId::new(1),
        target: Some(EntityId::new(2)),
        base_damage: -8,
        range_millimeters: 400,
        tags: vec!["primary-fire".to_string()],
        input_hash: "fnv1a64:input".to_string(),
    };
    let damage = GameExtensionProposal::DamageModifier {
        proposal_id: "proposal.damage".to_string(),
        target: EntityId::new(2),
        channel_id: "value.health".to_string(),
        amount_delta: -2,
        tags: vec!["close-range".to_string()],
        proposal_hash: "fnv1a64:damage".to_string(),
    };
    let bundle = GameExtensionProposal::EffectBundle {
        proposal_id: "proposal.bundle".to_string(),
        bundle_id: "bundle.poisoned-impact".to_string(),
        tags: vec!["poison".to_string()],
        proposal_hash: "fnv1a64:bundle".to_string(),
    };
    let rejected = GameExtensionProposal::Reject {
        proposal_id: "proposal.reject".to_string(),
        code: GameExtensionDiagnosticCode::InvalidProposal,
        message: "module rejected".to_string(),
        proposal_hash: "fnv1a64:reject".to_string(),
    };
    let noop = GameExtensionProposal::Noop {
        proposal_id: "proposal.noop".to_string(),
        proposal_hash: "fnv1a64:noop".to_string(),
    };
    let trace = GameExtensionTraceEntry {
        step: 1,
        code: "module.proposed".to_string(),
        message: "module returned a typed proposal".to_string(),
        refs: vec!["proposal.damage".to_string()],
    };
    let receipt = GameExtensionHookReceipt {
        module_ref: module_ref.clone(),
        hook_id: "weapon.primary".to_string(),
        request_id: "request-1".to_string(),
        status: GameExtensionReceiptStatus::Proposed,
        input_hash: "fnv1a64:input".to_string(),
        proposal: Some(damage.clone()),
        diagnostics: vec![diagnostic.clone()],
        trace: vec![trace.clone()],
        proposal_hash: "fnv1a64:damage".to_string(),
    };
    let evidence = GameExtensionReplayEvidence {
        module_ref: module_ref.clone(),
        hook_id: "weapon.primary".to_string(),
        request_id: "request-1".to_string(),
        input_hash: "fnv1a64:input".to_string(),
        proposal_hash: "fnv1a64:damage".to_string(),
        validation_status: "accepted".to_string(),
        event_hashes: vec!["fnv1a64:event".to_string()],
        rejection_hashes: Vec::new(),
        replay_hash: "fnv1a64:replay".to_string(),
    };

    compare_object_to_interface(
        &game_extension,
        "GameRuleModuleRef",
        &serde_json::to_value(&module_ref).unwrap(),
    )
    .unwrap();
    compare_object_to_interface(
        &game_extension,
        "GameRuleHookDeclaration",
        &serde_json::to_value(&hook).unwrap(),
    )
    .unwrap();
    compare_object_to_interface(
        &game_extension,
        "GameRuleModuleManifest",
        &serde_json::to_value(&manifest).unwrap(),
    )
    .unwrap();
    compare_object_to_interface(
        &game_extension,
        "GameExtensionDiagnostic",
        &serde_json::to_value(&diagnostic).unwrap(),
    )
    .unwrap();
    compare_object_to_interface(
        &game_extension,
        "WeaponEffectHookRequest",
        &serde_json::to_value(&request).unwrap(),
    )
    .unwrap();
    compare_object_to_variant(
        &game_extension,
        "GameExtensionProposal",
        "damageModifier",
        &serde_json::to_value(&damage).unwrap(),
    )
    .unwrap();
    compare_object_to_variant(
        &game_extension,
        "GameExtensionProposal",
        "effectBundle",
        &serde_json::to_value(&bundle).unwrap(),
    )
    .unwrap();
    compare_object_to_variant(
        &game_extension,
        "GameExtensionProposal",
        "reject",
        &serde_json::to_value(&rejected).unwrap(),
    )
    .unwrap();
    compare_object_to_variant(
        &game_extension,
        "GameExtensionProposal",
        "noop",
        &serde_json::to_value(&noop).unwrap(),
    )
    .unwrap();
    compare_object_to_interface(
        &game_extension,
        "GameExtensionTraceEntry",
        &serde_json::to_value(&trace).unwrap(),
    )
    .unwrap();
    compare_object_to_interface(
        &game_extension,
        "GameExtensionHookReceipt",
        &serde_json::to_value(&receipt).unwrap(),
    )
    .unwrap();
    compare_object_to_interface(
        &game_extension,
        "GameExtensionReplayEvidence",
        &serde_json::to_value(&evidence).unwrap(),
    )
    .unwrap();
}

#[test]
fn gameplay_fabric_rust_serialization_matches_ir_shape() {
    use core_ids::EntityId;
    use protocol_diagnostics::DiagnosticSeverity;
    use protocol_game_extension::{
        GameplayCausationRef, GameplayContractRef, GameplayEmitterRef, GameplayEntityRef,
        GameplayEventEnvelope, GameplayEventPhase, GameplayEventSchemaDeclaration,
        GameplayExecutionBudget, GameplayHeaderSelector, GameplayInvocationDescriptor,
        GameplayInvocationFamily, GameplayInvocationReadRequirement, GameplayModuleManifest,
        GameplayModuleRef, GameplayOrderingConstraint, GameplayOwnedSchemaDeclaration,
        GameplayOwnerRef, GameplayProposalDeclaration, GameplayProposalEnvelope,
        GameplayReadSelectorCapability, GameplayReadViewKind, GameplayReadViewProviderReadout,
        GameplayReadViewRequirement, GameplayRegistryDiagnostic, GameplayRegistryDiagnosticCode,
        GameplayRegistryReadout, GameplayRegistryValidationOutcome,
        GameplaySubscriptionDeclaration, GameplayTopologyEdge,
    };

    let game_extension = module("gameExtension");
    let event = GameplayContractRef {
        namespace: "game.combat".into(),
        name: "damage-applied".into(),
        version: 1,
        schema_hash: "sha256:event".into(),
    };
    let output = GameplayContractRef {
        namespace: "game.feedback".into(),
        name: "damage-cue".into(),
        version: 1,
        schema_hash: "sha256:cue".into(),
    };
    let module_ref = GameplayModuleRef {
        module_id: "game.feedback".into(),
        namespace: "game.feedback".into(),
        version: "1.0.0".into(),
        sdk_hash: "sha256:sdk".into(),
        contract_hash: "sha256:contract".into(),
        artifact_hash: "sha256:artifact".into(),
        provider_id: "provider.feedback".into(),
    };
    let owner = GameplayOwnerRef {
        owner_id: "authority.feedback".into(),
        provider_id: "provider.feedback".into(),
    };
    let event_declaration = GameplayEventSchemaDeclaration {
        event: event.clone(),
        codec_id: "asha.canonical-json-v1".into(),
    };
    let entity = GameplayEntityRef {
        entity: EntityId::new(7),
    };
    let emitters = [
        GameplayEmitterRef::Owner {
            owner_id: owner.owner_id.clone(),
        },
        GameplayEmitterRef::Module {
            module_id: module_ref.module_id.clone(),
        },
        GameplayEmitterRef::Scheduler {
            scheduler_id: "scheduler.main".into(),
        },
    ];
    let causation = GameplayCausationRef {
        root_id: "root-1".into(),
        parent_event_id: None,
        decision_id: Some("decision-1".into()),
    };
    let envelope = GameplayEventEnvelope {
        event_id: "event-1".into(),
        event: event.clone(),
        tick: 42,
        root_sequence: 3,
        wave: 1,
        event_sequence: 2,
        phase: GameplayEventPhase::PostCommit,
        emitter: emitters[1].clone(),
        causation: causation.clone(),
        source: Some(entity.clone()),
        subjects: vec![entity.clone()],
        targets: vec![entity.clone()],
        scope: Some("combat".into()),
        tags: vec!["damage".into()],
        canonical_payload: vec![1, 2, 3],
        payload_hash: "sha256:payload".into(),
    };
    let selector = GameplayHeaderSelector {
        source: None,
        target: Some(entity.clone()),
        scope: Some("combat".into()),
        required_tags: vec!["damage".into()],
    };
    let subscription = GameplaySubscriptionDeclaration {
        subscription_id: "feedback.observe-damage".into(),
        event: event.clone(),
        invocation_id: "observe-damage".into(),
        selector: selector.clone(),
        max_deliveries_per_root: 16,
    };
    let invocation = GameplayInvocationDescriptor {
        invocation_id: "observe-damage".into(),
        family: GameplayInvocationFamily::Observe,
        input_contract: event.clone(),
        output_contract: output.clone(),
        read_requirements: vec![GameplayInvocationReadRequirement {
            request_id: "damage-source".into(),
            view: event.clone(),
        }],
        max_outputs: 4,
        max_payload_bytes: 4_096,
    };
    let proposal = GameplayProposalDeclaration {
        proposal: output.clone(),
        owner: owner.clone(),
    };
    let proposal_envelope = GameplayProposalEnvelope {
        proposal_id: "proposal-1".into(),
        proposal: output.clone(),
        tick: 42,
        root_sequence: 3,
        wave: 1,
        proposal_sequence: 1,
        emitter: emitters[1].clone(),
        causation: causation.clone(),
        originating_event_id: Some(envelope.event_id.clone()),
        source: Some(entity.clone()),
        targets: vec![entity.clone()],
        canonical_payload: vec![4, 5, 6],
        payload_hash: "sha256:proposal-payload".into(),
    };
    let read_view = GameplayReadViewRequirement {
        view: event.clone(),
        provider_id: "provider.combat-view".into(),
        kind: GameplayReadViewKind::EntityCapability,
        fields: vec!["amount".into()],
        selector_capabilities: vec![
            GameplayReadSelectorCapability::EventTarget,
            GameplayReadSelectorCapability::LifecycleCapability,
        ],
        max_items: 32,
    };
    let owned = GameplayOwnedSchemaDeclaration {
        schema: output.clone(),
        owner: owner.clone(),
    };
    let ordering = GameplayOrderingConstraint {
        before_module: "game.combat".into(),
        after_module: module_ref.module_id.clone(),
    };
    let budget = GameplayExecutionBudget {
        max_waves: 8,
        max_events_per_root: 64,
        max_proposals_per_root: 32,
        max_invocations_per_root: 64,
        max_payload_bytes_per_root: 65_536,
    };
    let manifest = GameplayModuleManifest {
        module_ref: module_ref.clone(),
        published_events: vec![event_declaration.clone()],
        subscriptions: vec![subscription.clone()],
        invocations: vec![invocation.clone()],
        read_views: vec![read_view.clone()],
        proposal_kinds: vec![proposal.clone()],
        state_schemas: vec![owned.clone()],
        fact_schemas: vec![owned.clone()],
        ordering: vec![ordering.clone()],
        budget: budget.clone(),
        deterministic_requirements: vec!["canonical-input-order".into()],
        source_hash: "sha256:source".into(),
    };
    let diagnostic = GameplayRegistryDiagnostic {
        code: GameplayRegistryDiagnosticCode::MissingCodec,
        severity: DiagnosticSeverity::Error,
        path: "codecs".into(),
        message: "published event has no codec".into(),
    };
    let topology = GameplayTopologyEdge {
        kind: "subscription".into(),
        from: module_ref.module_id.clone(),
        to: event.key(),
        contract: Some(subscription.invocation_id.clone()),
    };
    let provider_readout = GameplayReadViewProviderReadout {
        view: event.key(),
        provider_id: read_view.provider_id.clone(),
        kind: read_view.kind,
        fields: read_view.fields.clone(),
        selector_capabilities: read_view.selector_capabilities.clone(),
        max_items: read_view.max_items,
        ordering: "entityIdAscending".into(),
        provider_hash: "fnv1a64:provider".into(),
    };
    let readout = GameplayRegistryReadout {
        registry_digest: "fnv1a64:registry".into(),
        module_ids: vec![module_ref.module_id.clone()],
        event_kinds: vec![event.key()],
        subscription_ids: vec![subscription.subscription_id.clone()],
        proposal_owners: vec![format!("{}={}", output.key(), owner.owner_id)],
        read_view_providers: vec![format!("{}={}", event.key(), read_view.provider_id)],
        read_view_provider_details: vec![provider_readout.clone()],
        state_owners: vec![format!("{}={}", output.key(), owner.owner_id)],
        ordering: vec![ordering.clone()],
        topology: vec![topology.clone()],
        topology_dump: "module game.feedback\n".into(),
    };
    let valid = GameplayRegistryValidationOutcome::Valid {
        readout: Box::new(readout.clone()),
    };
    let invalid = GameplayRegistryValidationOutcome::Invalid {
        diagnostics: vec![diagnostic.clone()],
    };

    for (name, value) in [
        ("GameplayContractRef", serde_json::to_value(&event).unwrap()),
        (
            "GameplayModuleRef",
            serde_json::to_value(&module_ref).unwrap(),
        ),
        ("GameplayOwnerRef", serde_json::to_value(&owner).unwrap()),
        (
            "GameplayEventSchemaDeclaration",
            serde_json::to_value(&event_declaration).unwrap(),
        ),
        ("GameplayEntityRef", serde_json::to_value(&entity).unwrap()),
        (
            "GameplayCausationRef",
            serde_json::to_value(&causation).unwrap(),
        ),
        (
            "GameplayEventEnvelope",
            serde_json::to_value(&envelope).unwrap(),
        ),
        (
            "GameplayHeaderSelector",
            serde_json::to_value(&selector).unwrap(),
        ),
        (
            "GameplaySubscriptionDeclaration",
            serde_json::to_value(&subscription).unwrap(),
        ),
        (
            "GameplayInvocationReadRequirement",
            serde_json::to_value(&invocation.read_requirements[0]).unwrap(),
        ),
        (
            "GameplayInvocationDescriptor",
            serde_json::to_value(&invocation).unwrap(),
        ),
        (
            "GameplayProposalDeclaration",
            serde_json::to_value(&proposal).unwrap(),
        ),
        (
            "GameplayProposalEnvelope",
            serde_json::to_value(&proposal_envelope).unwrap(),
        ),
        (
            "GameplayReadViewRequirement",
            serde_json::to_value(&read_view).unwrap(),
        ),
        (
            "GameplayReadViewProviderReadout",
            serde_json::to_value(&provider_readout).unwrap(),
        ),
        (
            "GameplayOwnedSchemaDeclaration",
            serde_json::to_value(&owned).unwrap(),
        ),
        (
            "GameplayOrderingConstraint",
            serde_json::to_value(&ordering).unwrap(),
        ),
        (
            "GameplayExecutionBudget",
            serde_json::to_value(&budget).unwrap(),
        ),
        (
            "GameplayModuleManifest",
            serde_json::to_value(&manifest).unwrap(),
        ),
        (
            "GameplayRegistryDiagnostic",
            serde_json::to_value(&diagnostic).unwrap(),
        ),
        (
            "GameplayTopologyEdge",
            serde_json::to_value(&topology).unwrap(),
        ),
        (
            "GameplayRegistryReadout",
            serde_json::to_value(&readout).unwrap(),
        ),
    ] {
        compare_object_to_interface(&game_extension, name, &value).unwrap();
    }
    for (variant, value) in ["owner", "module", "scheduler"].into_iter().zip(
        emitters
            .iter()
            .map(|value| serde_json::to_value(value).unwrap()),
    ) {
        compare_object_to_variant(&game_extension, "GameplayEmitterRef", variant, &value).unwrap();
    }
    compare_object_to_variant(
        &game_extension,
        "GameplayRegistryValidationOutcome",
        "valid",
        &serde_json::to_value(&valid).unwrap(),
    )
    .unwrap();
    compare_object_to_variant(
        &game_extension,
        "GameplayRegistryValidationOutcome",
        "invalid",
        &serde_json::to_value(&invalid).unwrap(),
    )
    .unwrap();
}
