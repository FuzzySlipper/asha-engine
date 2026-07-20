//! Public-height prefab bootstrap and inspection for downstream gameplay hosts.
//!
//! Canonical project admission compiles registry JSON and placement commands.
//! Validation and entity creation remain in the existing serialization and
//! ProjectBundle rule owners.

use std::collections::BTreeSet;

use core_entity::EntityStore;
use core_ids::{PrefabId, PrefabInstanceId};
use rule_project_bundle::{
    InstantiatePrefabCommand, PrefabInstantiationCatalog, PrefabPlacementOrigin,
    ProjectBundleLoadResult, ValidatedGameplayPrefabLineage,
};
use serde::{Deserialize, Serialize};
use svc_serialization::{
    load_prefab_registry, PrefabInstanceRecord, PrefabOverride, PrefabOverrideValue,
    PrefabRegistryValidationContext, PrefabTransform, ValidatedPrefabRegistry,
};

use crate::GameplayRuntimeHostError;

type AppliedPrefabBootstrap = (
    ValidatedPrefabRegistry,
    Vec<(String, PrefabInstanceId, ValidatedGameplayPrefabLineage)>,
);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimePrefabCatalog {
    pub asset_ids: Vec<String>,
    pub entity_definition_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimePrefabBootstrap {
    pub registry_json: String,
    pub catalog: GameplayRuntimePrefabCatalog,
    pub placements: Vec<GameplayRuntimePrefabPlacement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimePrefabPlacement {
    pub command_id: String,
    /// Stable scene-authored identity used by configuration overrides.
    pub scene_instance_id: String,
    pub origin: GameplayRuntimePrefabPlacementOrigin,
    pub instance: u64,
    /// Base prefab selected by the stored scene reference. `prefab` may name a
    /// concrete variant definition used for expansion.
    pub authored_prefab: u64,
    pub prefab: u64,
    pub seed: u64,
    pub transform: GameplayRuntimePrefabTransform,
    pub overrides: Vec<GameplayRuntimePrefabOverride>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GameplayRuntimePrefabPlacementOrigin {
    Authored,
    Player,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimePrefabTransform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl GameplayRuntimePrefabTransform {
    pub const IDENTITY: Self = Self {
        translation: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    };
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "field",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameplayRuntimePrefabOverride {
    Transform {
        target_role: String,
        transform: GameplayRuntimePrefabTransform,
    },
    EntityDefinition {
        target_role: String,
        stable_id: String,
    },
    Asset {
        target_role: String,
        asset: String,
    },
    Material {
        target_role: String,
        asset: String,
    },
    Activation {
        target_role: String,
        active: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimePrefabReadout {
    pub state_hash: String,
    pub instances: Vec<GameplayRuntimePrefabInstanceReadout>,
    pub accepted_commands: Vec<GameplayRuntimePrefabCommandReadout>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimePrefabCommandReadout {
    pub command_id: String,
    pub instance: u64,
    pub prefab: u64,
    pub origin: GameplayRuntimePrefabPlacementOrigin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimePrefabInstanceReadout {
    pub instance: u64,
    pub prefab: u64,
    pub origin: GameplayRuntimePrefabPlacementOrigin,
    pub provenance_hash: String,
    pub override_count: u32,
    pub parts: Vec<GameplayRuntimePrefabPartReadout>,
    pub roles: Vec<GameplayRuntimePrefabRoleReadout>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimePrefabPartReadout {
    pub part: u64,
    pub namespace: String,
    pub entity: u64,
    pub parent_entity: Option<u64>,
    pub translation: [f32; 3],
    pub source_kind: String,
    pub active: bool,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayRuntimePrefabRoleReadout {
    pub role: String,
    pub entity: u64,
}

pub(crate) fn apply_prefab_bootstrap(
    bundle: &mut ProjectBundleLoadResult,
    bootstrap: GameplayRuntimePrefabBootstrap,
) -> Result<AppliedPrefabBootstrap, GameplayRuntimeHostError> {
    let validation_context = PrefabRegistryValidationContext {
        asset_ids: bootstrap
            .catalog
            .asset_ids
            .into_iter()
            .collect::<BTreeSet<_>>(),
        entity_definition_ids: bootstrap
            .catalog
            .entity_definition_ids
            .into_iter()
            .collect::<BTreeSet<_>>(),
    };
    let registry = load_prefab_registry(&bootstrap.registry_json, &validation_context)
        .map_err(|error| GameplayRuntimeHostError::Prefab(error.to_string()))?;
    let catalog = PrefabInstantiationCatalog::from(&validation_context);
    let entities = bundle.runtime_entities.get_or_insert_with(EntityStore::new);
    let mut scene_instances = Vec::with_capacity(bootstrap.placements.len());
    let mut stable_ids = BTreeSet::new();
    for placement in bootstrap.placements {
        if placement.scene_instance_id.trim().is_empty()
            || !stable_ids.insert(placement.scene_instance_id.clone())
        {
            return Err(GameplayRuntimeHostError::Prefab(
                "prefab placements require non-empty unique sceneInstanceId values".to_owned(),
            ));
        }
        let runtime_instance = PrefabInstanceId::new(placement.instance);
        let expanded_prefab = PrefabId::new(placement.prefab);
        let lineage = ValidatedGameplayPrefabLineage::from_registry(
            &registry,
            expanded_prefab,
            PrefabId::new(placement.authored_prefab),
        )
        .map_err(|error| GameplayRuntimeHostError::Prefab(error.to_string()))?;
        bundle
            .prefab_instances
            .instantiate(
                entities,
                &registry,
                &catalog,
                InstantiatePrefabCommand {
                    command_id: placement.command_id,
                    origin: placement.origin.into(),
                    record: PrefabInstanceRecord {
                        instance: runtime_instance,
                        prefab: expanded_prefab,
                        seed: placement.seed,
                        transform: placement.transform.into(),
                        overrides: placement.overrides.into_iter().map(Into::into).collect(),
                    },
                },
            )
            .map_err(|error| GameplayRuntimeHostError::Prefab(error.to_string()))?;
        scene_instances.push((placement.scene_instance_id, runtime_instance, lineage));
    }
    Ok((registry, scene_instances))
}

pub(crate) fn prefab_readout(bundle: &ProjectBundleLoadResult) -> GameplayRuntimePrefabReadout {
    let empty = EntityStore::new();
    let entities = bundle.runtime_entities.as_ref().unwrap_or(&empty);
    let snapshot = bundle.prefab_instances.snapshot(entities);
    GameplayRuntimePrefabReadout {
        state_hash: snapshot.state_hash,
        accepted_commands: snapshot
            .accepted_commands
            .iter()
            .map(|command| GameplayRuntimePrefabCommandReadout {
                command_id: command.command_id.clone(),
                instance: command.record.instance.raw(),
                prefab: command.record.prefab.raw(),
                origin: command.origin.into(),
            })
            .collect(),
        instances: snapshot
            .instances
            .iter()
            .map(|instance| GameplayRuntimePrefabInstanceReadout {
                instance: instance.record.instance.raw(),
                prefab: instance.record.prefab.raw(),
                origin: instance.origin.into(),
                provenance_hash: instance.provenance_hash.clone(),
                override_count: u32::try_from(instance.record.overrides.len()).unwrap_or(u32::MAX),
                parts: instance
                    .parts
                    .iter()
                    .map(|part| GameplayRuntimePrefabPartReadout {
                        part: part.part.raw(),
                        namespace: part.namespace.clone(),
                        entity: part.entity.raw(),
                        parent_entity: part.parent_entity.map(|entity| entity.raw()),
                        translation: part.transform.translation,
                        source_kind: match &part.source {
                            svc_serialization::PrefabPartSource::Scene { .. } => "scene",
                            svc_serialization::PrefabPartSource::EntityDefinition { .. } => {
                                "entityDefinition"
                            }
                            svc_serialization::PrefabPartSource::VoxelObject { .. } => {
                                "voxelObject"
                            }
                        }
                        .to_owned(),
                        active: part.active,
                        roles: part.roles.clone(),
                    })
                    .collect(),
                roles: instance
                    .role_map
                    .iter()
                    .map(|role| GameplayRuntimePrefabRoleReadout {
                        role: role.reference.role.clone(),
                        entity: role.entity.raw(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

impl From<GameplayRuntimePrefabPlacementOrigin> for PrefabPlacementOrigin {
    fn from(value: GameplayRuntimePrefabPlacementOrigin) -> Self {
        match value {
            GameplayRuntimePrefabPlacementOrigin::Authored => Self::Authored,
            GameplayRuntimePrefabPlacementOrigin::Player => Self::Player,
        }
    }
}

impl From<PrefabPlacementOrigin> for GameplayRuntimePrefabPlacementOrigin {
    fn from(value: PrefabPlacementOrigin) -> Self {
        match value {
            PrefabPlacementOrigin::Authored => Self::Authored,
            PrefabPlacementOrigin::Player => Self::Player,
        }
    }
}

impl From<GameplayRuntimePrefabTransform> for PrefabTransform {
    fn from(value: GameplayRuntimePrefabTransform) -> Self {
        Self {
            translation: value.translation,
            rotation: value.rotation,
            scale: value.scale,
        }
    }
}

impl From<GameplayRuntimePrefabOverride> for PrefabOverride {
    fn from(value: GameplayRuntimePrefabOverride) -> Self {
        match value {
            GameplayRuntimePrefabOverride::Transform {
                target_role,
                transform,
            } => Self {
                target_role,
                value: PrefabOverrideValue::Transform {
                    transform: transform.into(),
                },
            },
            GameplayRuntimePrefabOverride::EntityDefinition {
                target_role,
                stable_id,
            } => Self {
                target_role,
                value: PrefabOverrideValue::EntityDefinition { stable_id },
            },
            GameplayRuntimePrefabOverride::Asset { target_role, asset } => Self {
                target_role,
                value: PrefabOverrideValue::Asset { asset },
            },
            GameplayRuntimePrefabOverride::Material { target_role, asset } => Self {
                target_role,
                value: PrefabOverrideValue::Material { asset },
            },
            GameplayRuntimePrefabOverride::Activation {
                target_role,
                active,
            } => Self {
                target_role,
                value: PrefabOverrideValue::Activation { active },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GameplayRuntimeSchedulerDefinition;
    use crate::{
        BundleArtifacts, GameplayBindingEntityTargets, GameplayRuntimeDeclaredReadPlan,
        GameplayRuntimeHost, GameplayRuntimeSpatialEntity, GameplayTriggerDefinition, LoadPlan,
        LoadStep, RuntimeProjectActivationInput, GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
    };
    use core_ids::{EntityId, PrefabId, PrefabInstanceId, RuntimeSessionId, SceneId, SceneNodeId};
    use core_scene::{
        encode, SceneEntityInstance, SceneEntityReference, SceneMetadata, SceneNode, SceneNodeKind,
        SceneTree,
    };
    use gameplay_module_sdk::*;
    use rule_gameplay_fabric::gameplay_payload_hash;

    struct PrefabReadBehavior;

    impl GameplayModuleBehavior for PrefabReadBehavior {
        fn invoke(
            &self,
            context: &GameplayModuleContext<'_>,
        ) -> Result<GameplayModuleActions, GameplayModuleError> {
            Ok(context.actions())
        }
    }

    fn read_contract(name: &str) -> GameplayContractRef {
        gameplay_contract(
            "fixture.prefab-read",
            name,
            1,
            &read_schema_descriptor(name),
        )
    }

    fn read_schema_descriptor(name: &str) -> String {
        format!("fixture:fixture.prefab-read.{name};canonical-json-v1")
    }

    fn test_provenance() -> GameplayModuleBuildProvenance {
        GameplayModuleBuildProvenance::from_build_inputs(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            &[include_bytes!("prefab.rs")],
            include_bytes!("../../../../Cargo.lock"),
            &[],
        )
    }

    fn prefab_read_provider() -> GameplayStaticModuleProvider {
        let event = read_contract("inspect");
        let prefab_view = read_contract("prefab-part-view");
        let scope_view = read_contract("scope-view");
        let provider_id = "provider.fixture-prefab-read".to_owned();
        let mut manifest = GameplayModuleManifest {
            module_ref: GameplayModuleRef {
                module_id: "fixture.prefab-read.module".to_owned(),
                namespace: "fixture.prefab-read".to_owned(),
                version: "1.0.0".to_owned(),
                sdk_hash: "sha256:gameplay-sdk-v1".to_owned(),
                contract_hash: "sha256:fixture-prefab-read-contract".to_owned(),
                artifact_hash: "sha256:fixture-prefab-read-artifact".to_owned(),
                provider_id: provider_id.clone(),
            },
            published_events: vec![GameplayEventSchemaDeclaration {
                codec_id: gameplay_canonical_codec_id(&event.schema_hash),
                event: event.clone(),
            }],
            subscriptions: vec![GameplaySubscriptionDeclaration {
                subscription_id: "fixture.prefab-read.inspect".to_owned(),
                event: event.clone(),
                invocation_id: "fixture.prefab-read.inspect".to_owned(),
                selector: GameplayHeaderSelector {
                    source: None,
                    target: None,
                    scope: None,
                    required_tags: Vec::new(),
                },
                max_deliveries_per_root: 1,
            }],
            invocations: vec![GameplayInvocationDescriptor {
                invocation_id: "fixture.prefab-read.inspect".to_owned(),
                family: GameplayInvocationFamily::Observe,
                input_contract: event.clone(),
                output_contract: read_contract("inspect-result"),
                read_requirements: vec![
                    GameplayInvocationReadRequirement {
                        request_id: "console-sensor".to_owned(),
                        view: prefab_view.clone(),
                    },
                    GameplayInvocationReadRequirement {
                        request_id: "console-scope".to_owned(),
                        view: scope_view.clone(),
                    },
                ],
                max_outputs: 1,
                max_payload_bytes: 1_024,
            }],
            read_views: vec![
                GameplayReadViewRequirement {
                    view: prefab_view.clone(),
                    provider_id: provider_id.clone(),
                    kind: GameplayReadViewKind::PrefabPart,
                    fields: vec!["entity".to_owned(), "part".to_owned(), "role".to_owned()],
                    selector_capabilities: vec![GameplayReadSelectorCapability::PrefabPartRole],
                    max_items: 1,
                },
                GameplayReadViewRequirement {
                    view: scope_view.clone(),
                    provider_id: provider_id.clone(),
                    kind: GameplayReadViewKind::Selection,
                    fields: vec!["entities".to_owned()],
                    selector_capabilities: vec![GameplayReadSelectorCapability::ScopeSelection],
                    max_items: 4,
                },
            ],
            proposal_kinds: Vec::new(),
            state_schemas: Vec::new(),
            fact_schemas: Vec::new(),
            ordering: Vec::new(),
            budget: GameplayExecutionBudget {
                max_waves: 2,
                max_events_per_root: 4,
                max_proposals_per_root: 1,
                max_invocations_per_root: 2,
                max_payload_bytes_per_root: 4_096,
            },
            deterministic_requirements: vec!["frozen-read-wave".to_owned()],
            source_hash: "sha256:fixture-prefab-read-source".to_owned(),
        };
        test_provenance().apply_to_manifest::<PrefabReadBehavior>(&mut manifest);
        GameplayStaticModuleProvider::linked_from_manifest(
            manifest,
            &test_provenance(),
            PrefabReadBehavior,
        )
        .event_codec(GameplayEventCodecRegistration::typed(
            TypedGameplayEventCodec::new(
                GameplayEventSchemaDeclaration {
                    codec_id: gameplay_canonical_codec_id(&event.schema_hash),
                    event,
                },
                read_schema_descriptor("inspect"),
                |payload: &u64| serde_json::to_vec(payload).map_err(|error| error.to_string()),
                |bytes| serde_json::from_slice(bytes).map_err(|error| error.to_string()),
            ),
        ))
        .read_view_provider(GameplayReadViewProviderRegistration {
            view: prefab_view,
            provider_id: provider_id.clone(),
            kind: GameplayReadViewKind::PrefabPart,
            fields: vec!["entity".to_owned(), "part".to_owned(), "role".to_owned()],
            selector_capabilities: vec![GameplayReadSelectorCapability::PrefabPartRole],
            max_items: 1,
            ordering: "singlePart".to_owned(),
        })
        .read_view_provider(GameplayReadViewProviderRegistration {
            view: scope_view,
            provider_id,
            kind: GameplayReadViewKind::Selection,
            fields: vec!["entities".to_owned()],
            selector_capabilities: vec![GameplayReadSelectorCapability::ScopeSelection],
            max_items: 4,
            ordering: "entityIdAscending".to_owned(),
        })
    }

    fn prefab_project_input() -> RuntimeProjectActivationInput {
        let scene = SceneTree {
            id: SceneId::new(44),
            schema_version: 4,
            metadata: SceneMetadata {
                name: Some("public-prefab-host".to_owned()),
                authoring_format_version: 4,
            },
            dependencies: Vec::new(),
            roots: vec![SceneNode::leaf(
                SceneNodeId::new(900),
                SceneNodeKind::EntityInstance(SceneEntityInstance {
                    instance_id: "fixture.console.trigger".to_owned(),
                    reference: SceneEntityReference::EntityDefinition {
                        stable_id: "fixture/console-trigger".to_owned(),
                    },
                    spawn_marker_id: None,
                }),
            )],
        };
        let mut composition = GameplayStaticCompositionBuilder::new();
        composition.include_standard_owner_events();
        RuntimeProjectActivationInput {
            load_plan: LoadPlan {
                steps: vec![
                    LoadStep::ValidateVersions {
                        bundle_schema_version: 2,
                        protocol_version: 1,
                    },
                    LoadStep::LoadAssetLock {
                        artifact: "assets/lock.json".to_owned(),
                        asset_count: 0,
                    },
                    LoadStep::LoadSceneDocument {
                        artifact: "scene/scene.json".to_owned(),
                        scene: SceneId::new(44),
                    },
                    LoadStep::BootstrapScene {
                        scene: SceneId::new(44),
                        runtime_session: RuntimeSessionId::new(44),
                    },
                    LoadStep::ValidateFinalState,
                ],
            },
            artifacts: BundleArtifacts::new()
                .with_artifact("assets/lock.json", "{\"entries\":[]}")
                .with_artifact("scene/scene.json", encode(&scene.to_flat())),
            bootstrap_resolution: core_scene::BootstrapResolutionContext {
                entity_definition_ids: ["fixture/console-trigger".to_owned()].into_iter().collect(),
                ..Default::default()
            },
            composition: composition.build().unwrap(),
            composition_requirement: None,
            bindings: GameplayModuleBindingRegistryBuilder::new().build(),
            entity_targets: GameplayBindingEntityTargets::new(),
            spatial_entities: Vec::new(),
            declared_reads: Vec::new(),
            triggers: Vec::new(),
            scheduler: GameplayRuntimeSchedulerDefinition::new(
                GameplayOwnerRef {
                    owner_id: "authority.prefab-scheduler".to_owned(),
                    provider_id: "provider.prefab-scheduler".to_owned(),
                },
                Vec::new(),
                Vec::new(),
            ),
        }
    }

    fn prefab_bootstrap() -> GameplayRuntimePrefabBootstrap {
        GameplayRuntimePrefabBootstrap {
            registry_json: r#"{
  "schemaVersion": 1,
  "definitions": [{
    "id": 70,
    "schemaVersion": 1,
    "displayName": "Public console",
    "parts": [
      {
        "id": 1,
        "namespace": "body",
        "displayName": "Body",
        "parent": null,
        "transform": { "translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1] },
        "source": { "kind": "entityDefinition", "stableId": "fixture.console.body" }
      },
      {
        "id": 2,
        "namespace": "sensor",
        "displayName": "Sensor",
        "parent": 1,
        "transform": { "translation": [0, 1, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1] },
        "source": { "kind": "entityDefinition", "stableId": "fixture.console.sensor" }
      }
    ],
    "partRoles": [
      { "role": "console/body", "part": 1 },
      { "role": "interaction/sensor", "part": 2 }
    ],
    "variant": null
  }, {
    "id": 71,
    "schemaVersion": 1,
    "displayName": "Public console blue",
    "parts": [],
    "partRoles": [],
    "variant": {
      "variantId": "blue",
      "base": 70,
      "removedRoles": [],
      "overrides": [{
        "targetRole": "console/body",
        "value": { "field": "entityDefinition", "stableId": "fixture.console.body.blue" }
      }]
    }
  }, {
    "id": 72,
    "schemaVersion": 1,
    "displayName": "Public console red",
    "parts": [],
    "partRoles": [],
    "variant": {
      "variantId": "red",
      "base": 70,
      "removedRoles": [],
      "overrides": [{
        "targetRole": "console/body",
        "value": { "field": "entityDefinition", "stableId": "fixture.console.body.red" }
      }]
    }
  }]
}"#
            .to_owned(),
            catalog: GameplayRuntimePrefabCatalog {
                asset_ids: Vec::new(),
                entity_definition_ids: vec![
                    "fixture.console.body".to_owned(),
                    "fixture.console.body.blue".to_owned(),
                    "fixture.console.body.red".to_owned(),
                    "fixture.console.sensor".to_owned(),
                ],
            },
            placements: vec![
                GameplayRuntimePrefabPlacement {
                    command_id: "place-console-authored".to_owned(),
                    scene_instance_id: "fixture.console.blue".to_owned(),
                    origin: GameplayRuntimePrefabPlacementOrigin::Authored,
                    instance: 700,
                    authored_prefab: 70,
                    prefab: 70,
                    seed: 11,
                    transform: GameplayRuntimePrefabTransform::IDENTITY,
                    overrides: vec![GameplayRuntimePrefabOverride::EntityDefinition {
                        target_role: "console/body".to_owned(),
                        stable_id: "fixture.console.body.blue".to_owned(),
                    }],
                },
                GameplayRuntimePrefabPlacement {
                    command_id: "place-console-player".to_owned(),
                    scene_instance_id: "fixture.console.red".to_owned(),
                    origin: GameplayRuntimePrefabPlacementOrigin::Player,
                    instance: 701,
                    authored_prefab: 70,
                    prefab: 70,
                    seed: 12,
                    transform: GameplayRuntimePrefabTransform {
                        translation: [4.0, 0.0, 0.0],
                        ..GameplayRuntimePrefabTransform::IDENTITY
                    },
                    overrides: vec![GameplayRuntimePrefabOverride::EntityDefinition {
                        target_role: "console/body".to_owned(),
                        stable_id: "fixture.console.body.red".to_owned(),
                    }],
                },
            ],
        }
    }

    fn prefab_read_project_input() -> RuntimeProjectActivationInput {
        let mut input = prefab_project_input();
        let mut composition = GameplayStaticCompositionBuilder::new();
        composition.add_provider(prefab_read_provider());
        input.composition = composition.build().unwrap();
        input.spatial_entities = vec![GameplayRuntimeSpatialEntity {
            entity: EntityId::new(900),
            translation: [0.0, 0.0, 0.0],
            half_extents: [0.5, 0.5, 0.5],
            static_collider: false,
        }];
        input.triggers = vec![GameplayTriggerDefinition {
            schema_version: GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
            scene_instance_id: "fixture.console.trigger".to_owned(),
            scope: "zone.console".to_owned(),
            tags: vec!["console".to_owned()],
        }];
        input.declared_reads = vec![GameplayRuntimeDeclaredReadPlan {
            module_id: "fixture.prefab-read.module".to_owned(),
            invocation_id: "fixture.prefab-read.inspect".to_owned(),
            requests: vec![
                GameplayReadRequest {
                    request_id: "console-sensor".to_owned(),
                    view: read_contract("prefab-part-view"),
                    fields: vec!["entity".to_owned(), "part".to_owned(), "role".to_owned()],
                    selector: GameplayReadSelector::PrefabPart {
                        instance: PrefabInstanceId::new(700),
                        reference: PrefabPartReference {
                            prefab: PrefabId::new(70),
                            role: "interaction/sensor".to_owned(),
                        },
                    },
                },
                GameplayReadRequest {
                    request_id: "console-scope".to_owned(),
                    view: read_contract("scope-view"),
                    fields: vec!["entities".to_owned()],
                    selector: GameplayReadSelector::Scope {
                        scope: "zone.console".to_owned(),
                        max_items: 4,
                    },
                },
            ],
        }];
        input
    }

    fn prefab_read_event() -> GameplayEventEnvelope {
        let payload = serde_json::to_vec(&1_u64).unwrap();
        GameplayEventEnvelope {
            event_id: "inspect-console".to_owned(),
            event: read_contract("inspect"),
            tick: 1,
            root_sequence: 1,
            wave: 0,
            event_sequence: 0,
            phase: GameplayEventPhase::PostCommit,
            emitter: GameplayEmitterRef::Owner {
                owner_id: "fixture.prefab-read".to_owned(),
            },
            causation: GameplayCausationRef {
                root_id: "inspect-console".to_owned(),
                parent_event_id: None,
                decision_id: None,
            },
            source: None,
            subjects: Vec::new(),
            targets: Vec::new(),
            scope: Some("zone.console".to_owned()),
            tags: Vec::new(),
            payload_hash: gameplay_payload_hash(&payload),
            canonical_payload: payload,
        }
    }

    #[test]
    fn public_prefab_bootstrap_places_resolves_and_restores_multiple_instances() {
        let host = GameplayRuntimeHost::activate_project_with_prefabs(
            prefab_project_input(),
            prefab_bootstrap(),
        )
        .unwrap();
        let readout = host.prefab_readout();
        assert_eq!(readout.instances.len(), 2);
        assert_eq!(readout.accepted_commands.len(), 2);
        assert_eq!(
            readout.accepted_commands[1].origin,
            GameplayRuntimePrefabPlacementOrigin::Player
        );
        let first_sensor = readout.instances[0]
            .roles
            .iter()
            .find(|role| role.role == "interaction/sensor")
            .unwrap();
        let second_sensor = readout.instances[1]
            .roles
            .iter()
            .find(|role| role.role == "interaction/sensor")
            .unwrap();
        assert_ne!(first_sensor.entity, second_sensor.entity);
        assert_eq!(readout.instances[0].override_count, 1);
        assert_eq!(readout.instances[1].override_count, 1);

        let snapshot = host.compose_snapshot().unwrap();
        let restored = GameplayRuntimeHost::restore_project_with_prefabs(
            prefab_project_input(),
            prefab_bootstrap(),
            &snapshot.text,
        )
        .unwrap();
        assert_eq!(restored.prefab_readout(), readout);
        assert_eq!(
            restored.readout().runtime_host_hash,
            host.readout().runtime_host_hash
        );
    }

    #[test]
    fn public_prefab_bootstrap_rejects_unproven_authored_lineage_before_activation() {
        let mut bootstrap = prefab_bootstrap();
        bootstrap.placements[0].prefab = 71;
        bootstrap.placements[0].authored_prefab = 72;
        let error = match GameplayRuntimeHost::activate_project_with_prefabs(
            prefab_project_input(),
            bootstrap,
        ) {
            Ok(_) => {
                panic!("a concrete blue variant cannot claim the red variant as its authored base")
            }
            Err(error) => error,
        };
        assert!(matches!(error, GameplayRuntimeHostError::Prefab(_)));
        assert!(error.to_string().contains("AuthoredPrefabMismatch"));
    }

    #[test]
    fn public_host_resolves_loaded_prefab_roles_and_populated_scopes() {
        let mut host = GameplayRuntimeHost::activate_project_with_prefabs(
            prefab_read_project_input(),
            prefab_bootstrap(),
        )
        .unwrap();
        let expected_sensor = host.prefab_readout().instances[0]
            .roles
            .iter()
            .find(|role| role.role == "interaction/sensor")
            .unwrap()
            .entity;

        let receipt = host.observe(prefab_read_event()).unwrap();
        assert!(
            receipt.observe.accepted(),
            "{:?}",
            receipt.observe.diagnostics
        );
        let reads = receipt.observe.invocations[0]
            .declared_reads
            .as_ref()
            .expect("public invocation receives its declared reads");
        let prefab = reads
            .reads
            .iter()
            .find(|read| read.request_id == "console-sensor")
            .unwrap();
        assert_eq!(
            prefab.value,
            GameplayReadValue::PrefabPart {
                instance: 700,
                prefab: 70,
                role: "interaction/sensor".to_owned(),
                part: 2,
                entity: expected_sensor,
            }
        );
        let scope = reads
            .reads
            .iter()
            .find(|read| read.request_id == "console-scope")
            .unwrap();
        assert_eq!(
            scope.value,
            GameplayReadValue::EntitySelection {
                entities: vec![900]
            }
        );
    }
}
