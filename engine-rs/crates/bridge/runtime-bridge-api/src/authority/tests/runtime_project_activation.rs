use core_assets::{markers, AssetRef};
use core_scene::{
    FlatSceneDocument, NodeMetadata, SceneMetadata, SceneNodeKind, SceneNodeRecord, SceneTransform,
};
use gameplay_module_sdk::*;
use protocol_assets::{StoredAssetCatalog, StoredCatalogEntry};
use rule_gameplay_fabric::{SessionTickGameplayPayload, StandardGameplayEventKind};

use super::*;

const PROJECT_ID: u64 = 311;
const SCENE_ID: u64 = 912;
const VOXEL_PATH: &str = "assets/hand-authored-room.avxl.json";
const CATALOG_PATH: &str = "catalogs/materials.project-content.json";

struct TickProbeBehavior;

impl GameplayModuleBehavior for TickProbeBehavior {
    fn invoke(
        &self,
        context: &GameplayModuleContext<'_>,
    ) -> Result<GameplayModuleActions, GameplayModuleError> {
        let payload: SessionTickGameplayPayload = context.event_payload()?;
        let mut actions = context.actions();
        actions.trace(format!("stored-project.tick:{}", payload.tick));
        Ok(actions)
    }
}

fn build_provenance() -> GameplayModuleBuildProvenance {
    GameplayModuleBuildProvenance::from_build_inputs(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        &[include_bytes!("runtime_project_activation.rs")],
        include_bytes!("../../../../../../Cargo.lock"),
        &[],
    )
}

fn tick_probe_provider() -> GameplayStaticModuleProvider {
    let tick = StandardGameplayEventKind::SessionTick.contract();
    let mut manifest = GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: "fixture.stored-project-tick".to_owned(),
            namespace: "fixture.stored-project".to_owned(),
            version: "1.0.0".to_owned(),
            sdk_hash: "sha256:gameplay-sdk-v1".to_owned(),
            contract_hash: "sha256:stored-project-tick-contract".to_owned(),
            artifact_hash: "sha256:stored-project-tick-artifact".to_owned(),
            provider_id: "provider.fixture.stored-project".to_owned(),
        },
        published_events: Vec::new(),
        subscriptions: vec![GameplaySubscriptionDeclaration {
            subscription_id: "fixture.stored-project.tick".to_owned(),
            event: tick.clone(),
            invocation_id: "fixture.stored-project.observe-tick".to_owned(),
            selector: GameplayHeaderSelector {
                source: None,
                target: None,
                scope: Some("session".to_owned()),
                required_tags: vec!["tick".to_owned()],
            },
            max_deliveries_per_root: 1,
        }],
        invocations: vec![GameplayInvocationDescriptor {
            invocation_id: "fixture.stored-project.observe-tick".to_owned(),
            family: GameplayInvocationFamily::Observe,
            input_contract: tick.clone(),
            output_contract: tick,
            read_requirements: Vec::new(),
            max_outputs: 1,
            max_payload_bytes: 1_024,
        }],
        read_views: Vec::new(),
        proposal_kinds: Vec::new(),
        state_schemas: Vec::new(),
        fact_schemas: Vec::new(),
        ordering: Vec::new(),
        budget: GameplayExecutionBudget {
            max_waves: 1,
            max_events_per_root: 2,
            max_proposals_per_root: 1,
            max_invocations_per_root: 2,
            max_payload_bytes_per_root: 4_096,
        },
        deterministic_requirements: vec!["canonical-json".to_owned()],
        source_hash: "sha256:stored-project-tick-source".to_owned(),
    };
    let provenance = build_provenance();
    provenance.apply_to_manifest::<TickProbeBehavior>(&mut manifest);
    GameplayStaticModuleProvider::linked_from_manifest(manifest, &provenance, TickProbeBehavior)
}

fn static_composition() -> GameplayStaticComposition {
    let mut builder = GameplayStaticCompositionBuilder::new();
    builder
        .include_standard_owner_events()
        .add_provider(tick_probe_provider());
    builder.build().expect("static project composition")
}

fn stored_scene(asset_id: &str) -> FlatSceneDocument {
    let voxel_reference =
        AssetRef::<markers::VoxelVolume>::parse(asset_id, AssetVersionReq::Any, None)
            .expect("voxel reference")
            .erase();
    FlatSceneDocument {
        id: SceneId::new(SCENE_ID),
        schema_version: 4,
        metadata: SceneMetadata {
            name: Some("stored project entry".to_owned()),
            authoring_format_version: 4,
        },
        dependencies: vec![voxel_reference.clone()],
        nodes: vec![SceneNodeRecord {
            id: SceneNodeId::new(41),
            parent: None,
            child_order: 0,
            transform: SceneTransform {
                translation: Vec3::new(3.0, 2.0, -4.0),
                ..SceneTransform::IDENTITY
            },
            kind: SceneNodeKind::VoxelVolume(voxel_reference),
            metadata: NodeMetadata {
                label: Some("stored room".to_owned()),
                tags: vec!["entry".to_owned()],
            },
        }],
    }
}

fn material_catalog_artifact(
    composition: &GameplayStaticComposition,
    asset: &VoxelVolumeAsset,
) -> Vec<u8> {
    let document = ProjectContentDocumentDto::AssetCatalog {
        document_id: CATALOG_PATH.to_owned(),
        catalog: StoredAssetCatalog {
            entries: vec![StoredCatalogEntry {
                id: asset.asset_id.clone(),
                version: 1,
                hash: None,
                source_path: Some(VOXEL_PATH.to_owned()),
                label: Some("Hand-authored room".to_owned()),
                dependencies: Vec::new(),
                material: None,
            }],
        },
    };
    let gameplay = rule_project_bundle::GameplayProjectContentAdmission::new(
        composition.project_configuration_authority(),
    );
    let outcome = svc_project_content::validate_project_content_documents(
        vec![document],
        svc_project_content::ProjectContentValidationContext {
            scenes: &[],
            gameplay: &gameplay,
            reference_revision: 0,
        },
    );
    assert!(outcome.result.accepted, "{:?}", outcome.result.diagnostics);
    outcome.result.canonical_files[0]
        .canonical_json
        .as_bytes()
        .to_vec()
}

fn project_source_batch(
    bridge: &mut EngineBridge,
    composition: &GameplayStaticComposition,
    referenced_asset_id: &str,
) -> protocol_project_bundle::RuntimeProjectSourceBatch {
    let asset = hand_authored_voxel_volume_asset();
    let scene_bytes = core_scene::encode(&stored_scene(referenced_asset_id)).into_bytes();
    let asset_bytes = svc_voxel_asset::encode_asset(&asset)
        .expect("canonical voxel asset")
        .into_bytes();
    let catalog_bytes = material_catalog_artifact(composition, &asset);
    let lock_bytes = b"asset-lock-v1".to_vec();
    let manifest = svc_serialization::ProjectBundleManifest {
        bundle_schema_version: svc_serialization::BUNDLE_SCHEMA_VERSION,
        protocol_version: svc_serialization::SUPPORTED_PROTOCOL_VERSION,
        project: svc_serialization::ProjectSection {
            id: core_ids::ProjectId::new(PROJECT_ID),
            name: Some("stored-project-runtime".to_owned()),
        },
        entry_scene: SceneId::new(SCENE_ID),
        scenes: vec![svc_serialization::SceneSection {
            id: SceneId::new(SCENE_ID),
            schema_version: 4,
            artifact: "scenes/entry.scene.json".to_owned(),
        }],
        asset_lock: svc_serialization::AssetLockSection {
            artifact: "assets/lock.json".to_owned(),
            asset_count: 1,
        },
        generation_provenance: None,
        artifacts: vec![
            svc_serialization::ArtifactEntry::durable(
                "assets/lock.json",
                svc_serialization::ArtifactRole::AssetLock,
                &lock_bytes,
            ),
            svc_serialization::ArtifactEntry::durable(
                "scenes/entry.scene.json",
                svc_serialization::ArtifactRole::SceneDocument,
                &scene_bytes,
            ),
            svc_serialization::ArtifactEntry::durable(
                CATALOG_PATH,
                svc_serialization::ArtifactRole::MaterialCatalog,
                &catalog_bytes,
            ),
            svc_serialization::ArtifactEntry::durable(
                VOXEL_PATH,
                svc_serialization::ArtifactRole::VoxelVolumeAsset,
                &asset_bytes,
            ),
        ],
    };
    let manifest_json = svc_serialization::encode(&manifest);
    let transaction = bridge
        .begin_runtime_project_source_resources(&manifest_json)
        .expect("begin stored-project resources");
    let staged_asset = bridge
        .stage_runtime_project_source_resource(transaction, VOXEL_PATH, asset_bytes)
        .expect("stage canonical voxel asset");
    protocol_project_bundle::RuntimeProjectSourceBatch {
        manifest_json,
        resource_generation: Some(transaction.generation()),
        bodies: vec![
            protocol_project_bundle::ProjectSourceBody::Inline {
                path: "assets/lock.json".to_owned(),
                bytes: lock_bytes,
            },
            protocol_project_bundle::ProjectSourceBody::Inline {
                path: "scenes/entry.scene.json".to_owned(),
                bytes: scene_bytes,
            },
            protocol_project_bundle::ProjectSourceBody::Inline {
                path: CATALOG_PATH.to_owned(),
                bytes: catalog_bytes,
            },
            protocol_project_bundle::ProjectSourceBody::Resource {
                path: VOXEL_PATH.to_owned(),
                resource: protocol_project_bundle::StagedProjectResourceRef {
                    handle: staged_asset.handle.raw(),
                    generation: staged_asset.generation,
                    version: staged_asset.version,
                    byte_len: staged_asset.byte_len,
                },
            },
        ],
    }
}

fn admit(
    bridge: &mut EngineBridge,
    composition: &GameplayStaticComposition,
    referenced_asset_id: &str,
) {
    let batch = project_source_batch(bridge, composition, referenced_asset_id);
    let receipt = bridge
        .admit_runtime_project_source_batch(batch)
        .expect("source admission receipt");
    assert!(receipt.accepted, "{:?}", receipt.diagnostics);
}

#[test]
fn deferred_runtime_activation_is_atomic_and_lifecycle_bound() {
    let composition = static_composition();
    let mut bridge = DeferredRuntimeSessionBuilder::from_static_composition(composition.clone())
        .build_unloaded();
    assert!(bridge.bundle.engine.is_none());
    assert!(bridge.active_runtime_project().is_none());
    assert!(bridge.gameplay.static_project_content_admission.is_some());
    bridge.initialize_engine(EngineConfig { seed: 77 }).unwrap();

    admit(&mut bridge, &composition, "voxel-volume/missing-room");
    let rejected = bridge
        .activate_pending_runtime_project(RuntimeProjectLifecycleVersion::default())
        .expect_err("dangling stored reference must reject");
    assert!(matches!(rejected, RuntimeProjectLoadError::Admission(_)));
    assert!(bridge.active_runtime_project().is_none());
    assert_eq!(
        bridge.runtime_project_lifecycle_version(),
        RuntimeProjectLifecycleVersion::default()
    );
    assert!(bridge.pending_project_source().is_none());

    admit(&mut bridge, &composition, "voxel-volume/hand-authored-room");
    let loaded = bridge
        .activate_pending_runtime_project(RuntimeProjectLifecycleVersion::default())
        .expect("valid project activates");
    assert_eq!(loaded.project_id, PROJECT_ID);
    assert_eq!(loaded.entry_scene_id, SCENE_ID);
    assert_eq!(loaded.voxel_asset_count, 1);
    assert_eq!(loaded.lifecycle.generation, 1);
    assert!(bridge
        .initialize_engine(EngineConfig { seed: 999 })
        .is_err());
    assert_eq!(bridge.active_runtime_project(), Some(loaded.clone()));

    admit(&mut bridge, &composition, "voxel-volume/hand-authored-room");
    let prior = bridge.active_runtime_project().unwrap();
    assert!(matches!(
        bridge.activate_pending_runtime_project(loaded.lifecycle),
        Err(RuntimeProjectLoadError::AlreadyActive { .. })
    ));
    assert_eq!(bridge.active_runtime_project(), Some(prior.clone()));
    assert!(bridge.pending_project_source().is_none());

    let stale = RuntimeProjectLifecycleVersion {
        generation: loaded.lifecycle.generation,
        revision: loaded.lifecycle.revision.saturating_sub(1),
    };
    assert!(matches!(
        bridge.unload_runtime_project(stale),
        Err(RuntimeProjectLoadError::StaleLifecycle { .. })
    ));
    assert_eq!(bridge.active_runtime_project(), Some(prior));

    let unloaded = bridge
        .unload_runtime_project(loaded.lifecycle)
        .expect("explicit unload");
    assert_eq!(unloaded.lifecycle.generation, 1);
    assert_eq!(unloaded.lifecycle.revision, 2);
    assert!(bridge.active_runtime_project().is_none());

    admit(&mut bridge, &composition, "voxel-volume/hand-authored-room");
    let reloaded = bridge
        .activate_pending_runtime_project(unloaded.lifecycle)
        .expect("explicit reload");
    assert_eq!(reloaded.lifecycle.generation, 2);
    assert_eq!(reloaded.lifecycle.revision, 3);
}

#[test]
fn fresh_activation_derives_voxel_collision_projection_and_gameplay_from_stored_source() {
    let composition = static_composition();
    let mut bridge = DeferredRuntimeSessionBuilder::from_static_composition(composition.clone())
        .build_unloaded();
    bridge.initialize_engine(EngineConfig { seed: 88 }).unwrap();
    admit(&mut bridge, &composition, "voxel-volume/hand-authored-room");

    let loaded = bridge
        .activate_pending_runtime_project(RuntimeProjectLifecycleVersion::default())
        .expect("stored source activates without a generator registry");
    assert_eq!(loaded.voxel_asset_count, 1);
    assert_eq!(bridge.voxel.collision_world_offset, [3.0, 2.0, -4.0]);

    let projection = bridge.read_render_diffs(0).unwrap();
    assert!(projection.ops.iter().any(|operation| {
        matches!(
            operation,
            protocol_render::RenderDiff::Create { node, .. }
                if node.transform.translation == [3.0, 2.0, -4.0]
        )
    }));
    assert!(projection.ops.iter().any(|operation| {
        matches!(
            operation,
            protocol_render::RenderDiff::ReplaceMeshPayload { .. }
        )
    }));

    let reaction = bridge
        .with_static_gameplay_runtime("stored_project_tick", |host| host.tick(9))
        .unwrap()
        .expect("activated gameplay host");
    assert!(
        reaction.observe.accepted(),
        "{:?}",
        reaction.observe.diagnostics
    );
    assert_eq!(reaction.observe.invocations.len(), 1);
    assert_eq!(
        reaction.observe.invocations[0].module_id,
        "fixture.stored-project-tick"
    );
}
