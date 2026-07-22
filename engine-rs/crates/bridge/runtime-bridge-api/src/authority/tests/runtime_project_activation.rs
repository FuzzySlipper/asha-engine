use core_assets::{markers, AssetRef};
use core_scene::{
    FlatSceneDocument, NodeMetadata, SceneEntityInstance, SceneEntityReference, SceneMarker,
    SceneMetadata, SceneNodeKind, SceneNodeRecord, SceneTransform,
};
use gameplay_module_sdk::*;
use protocol_assets::{
    Rgba, StoredAssetCatalog, StoredCatalogEntry, StoredMaterialAuthority,
    StoredMaterialDefinition, StoredMaterialStyle,
};
use protocol_entity_authoring::{EntityDefinition, EntityDefinitionSourceTrace};
use protocol_input::{
    InputActionDefinition, InputActionPhase, InputBindingRecord, InputContextDefinition,
    InputValue, InputValueKind, PlatformInputKind, INPUT_BINDING_CATALOG_SCHEMA_VERSION,
};
use rule_gameplay_fabric::{SessionTickGameplayPayload, StandardGameplayEventKind};

use super::*;

const PROJECT_ID: u64 = 311;
const SCENE_ID: u64 = 912;
const VOXEL_PATH: &str = "assets/hand-authored-room.avxl.json";
const CATALOG_PATH: &str = "catalogs/materials.project-content.json";
const PRESENTATION_PATH: &str = "catalogs/presentation.project-content.json";
const AUDIO_PATH: &str = "assets/primary-fire.wav";
const PARTICLE_PATH: &str = "assets/primary-fire.svg";
const ANIMATED_MESH_PATH: &str = "assets/character.glb";
const AUDIO_BYTES: &[u8] = b"fixture-primary-fire-audio";
const PARTICLE_BYTES: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>";
const ANIMATED_MESH_BYTES: &[u8] = b"fixture-animated-mesh";
const FPS_PROJECT_BUNDLE: &str = "stored-fps-project";
const PLAYER_DEFINITION_DOCUMENT_ID: &str = "entity.demo-player";
const PLAYER_DEFINITION_PATH: &str = "entities/demo-player.project-content.json";
const ENEMY_DEFINITION_DOCUMENT_ID: &str = "entity.tunnel-enemy";
const ENEMY_DEFINITION_PATH: &str = "entities/tunnel-enemy.project-content.json";
const GAMEPLAY_PATH: &str = "gameplay/fps.project-content.json";

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

fn stored_fps_scene(asset_id: &str) -> FlatSceneDocument {
    let mut scene = stored_scene(asset_id);
    scene.metadata.name = Some("stored FPS project entry".to_owned());
    scene.nodes.extend([
        SceneNodeRecord {
            id: SceneNodeId::new(100),
            parent: None,
            child_order: 1,
            transform: SceneTransform {
                translation: Vec3::new(0.0, 0.0, -3.5),
                ..SceneTransform::IDENTITY
            },
            kind: SceneNodeKind::Marker(SceneMarker {
                marker_id: "spawn.enemy.primary".to_owned(),
            }),
            metadata: NodeMetadata::default(),
        },
        SceneNodeRecord {
            id: SceneNodeId::new(101),
            parent: None,
            child_order: 2,
            transform: SceneTransform {
                translation: Vec3::new(0.0, 1.62, 0.0),
                ..SceneTransform::IDENTITY
            },
            kind: SceneNodeKind::EntityInstance(SceneEntityInstance {
                instance_id: "demo.player".to_owned(),
                reference: SceneEntityReference::EntityDefinition {
                    stable_id: "actor/demo-player".to_owned(),
                },
                spawn_marker_id: None,
            }),
            metadata: NodeMetadata::default(),
        },
        SceneNodeRecord {
            id: SceneNodeId::new(102),
            parent: None,
            child_order: 3,
            transform: SceneTransform {
                translation: Vec3::new(0.0, 1.1, 0.0),
                ..SceneTransform::IDENTITY
            },
            kind: SceneNodeKind::EntityInstance(SceneEntityInstance {
                instance_id: "demo.enemy".to_owned(),
                reference: SceneEntityReference::EntityDefinition {
                    stable_id: "actor/generated-tunnel-enemy".to_owned(),
                },
                spawn_marker_id: Some("spawn.enemy.primary".to_owned()),
            }),
            metadata: NodeMetadata::default(),
        },
    ]);
    scene.canonical()
}

fn stored_fps_definition(
    stable_id: &str,
    player: bool,
    enemy_health: u32,
    include_player_weapon: bool,
) -> EntityDefinition {
    let source_path = if player {
        PLAYER_DEFINITION_PATH
    } else {
        ENEMY_DEFINITION_PATH
    };
    let mut capabilities = vec![
        EntityDefinitionCapability::Transform {
            transform: AuthoringTransform {
                translation: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            },
        },
        EntityDefinitionCapability::Bounds {
            min: if player {
                [-0.5, -1.4, -0.5]
            } else {
                [-0.7, -0.9, -0.7]
            },
            max: if player {
                [0.5, 1.4, 0.5]
            } else {
                [0.7, 0.9, 0.7]
            },
        },
        EntityDefinitionCapability::Collision {
            static_collider: false,
        },
        EntityDefinitionCapability::Render { visible: true },
        EntityDefinitionCapability::Health {
            current: if player { 100 } else { enemy_health },
            max: if player { 100 } else { enemy_health },
        },
        EntityDefinitionCapability::RenderProjection {
            projection_id: if player {
                "first_person_camera"
            } else {
                "target_cube"
            }
            .to_owned(),
            visible: true,
        },
        EntityDefinitionCapability::Faction {
            faction_id: if player { "player" } else { "hostile" }.to_owned(),
        },
    ];
    if player {
        capabilities.push(EntityDefinitionCapability::Controller {
            controller_id: "player_input".to_owned(),
        });
        if include_player_weapon {
            capabilities.push(EntityDefinitionCapability::WeaponMount {
                weapon_id: "weapon.demo.primary".to_owned(),
                damage: 40,
                range_units: 16,
                ammo: 2,
                cooldown_ticks_after_fire: 4,
            });
        }
    } else {
        capabilities.extend([
            EntityDefinitionCapability::Controller {
                controller_id: "enemy_policy".to_owned(),
            },
            EntityDefinitionCapability::PolicyBinding {
                binding_id: "actor/generated-tunnel-enemy:policy".to_owned(),
                policy_id: "policy.enemy.generated_tunnel.v0".to_owned(),
                view_kind: "runtime_session.fps.policy_view.v0".to_owned(),
                view_version: "v0".to_owned(),
                allowed_intents: vec![
                    "runtime.intent.move_direct_nav.v0".to_owned(),
                    "runtime.intent.primary_fire.v0".to_owned(),
                ],
                runtime_moment: "autonomous_policy_tick".to_owned(),
            },
            EntityDefinitionCapability::SpawnMarker {
                marker_id: "spawn.enemy.primary".to_owned(),
            },
        ]);
    }
    EntityDefinition {
        stable_id: stable_id.to_owned(),
        display_name: if player {
            "Demo Player"
        } else {
            "Tunnel Enemy"
        }
        .to_owned(),
        source: EntityDefinitionSourceTrace {
            project_bundle: FPS_PROJECT_BUNDLE.to_owned(),
            relative_path: source_path.to_owned(),
        },
        tags: Vec::new(),
        metadata: Vec::new(),
        capabilities,
    }
}

fn fps_content_artifacts(
    composition: &GameplayStaticComposition,
    scene: &FlatSceneDocument,
    enemy_health: u32,
    include_player_weapon: bool,
) -> Vec<(String, Vec<u8>)> {
    fps_content_artifacts_with(
        composition,
        scene,
        enemy_health,
        include_player_weapon,
        false,
        false,
    )
}

fn fps_content_artifacts_with(
    composition: &GameplayStaticComposition,
    scene: &FlatSceneDocument,
    enemy_health: u32,
    include_player_weapon: bool,
    enemy_controller_only_role: bool,
    player_missing_role: bool,
) -> Vec<(String, Vec<u8>)> {
    let mut player_definition = stored_fps_definition(
        "actor/demo-player",
        true,
        enemy_health,
        include_player_weapon,
    );
    if player_missing_role {
        player_definition.capabilities.retain(|capability| {
            !matches!(
                capability,
                EntityDefinitionCapability::Faction { .. }
                    | EntityDefinitionCapability::Controller { .. }
            )
        });
    }
    let mut enemy_definition = stored_fps_definition(
        "actor/generated-tunnel-enemy",
        false,
        enemy_health,
        include_player_weapon,
    );
    if enemy_controller_only_role {
        enemy_definition.capabilities.retain(|capability| {
            !matches!(
                capability,
                EntityDefinitionCapability::Faction { .. }
                    | EntityDefinitionCapability::PolicyBinding { .. }
            )
        });
    }
    let documents = vec![
        ProjectContentDocumentDto::EntityDefinition {
            document_id: PLAYER_DEFINITION_DOCUMENT_ID.to_owned(),
            definition: player_definition,
        },
        ProjectContentDocumentDto::EntityDefinition {
            document_id: ENEMY_DEFINITION_DOCUMENT_ID.to_owned(),
            definition: enemy_definition,
        },
        ProjectContentDocumentDto::GameplayConfiguration {
            document_id: GAMEPLAY_PATH.to_owned(),
            document: protocol_project_content::ProjectGameplayConfigurationDocumentDto {
                schema_version: protocol_project_content::PROJECT_CONTENT_SCHEMA_VERSION,
                configurations: Vec::new(),
                bindings: Vec::new(),
                overrides: Vec::new(),
                triggers: vec![protocol_project_bundle::GameplayTriggerDefinition {
                    schema_version:
                        protocol_project_bundle::GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
                    scene_instance_id: "demo.enemy".to_owned(),
                    scope: "encounter.primary".to_owned(),
                    tags: vec!["enemy".to_owned()],
                }],
            },
        },
    ];
    let gameplay = rule_project_bundle::GameplayProjectContentAdmission::new(
        composition.project_configuration_authority(),
    );
    let outcome = svc_project_content::validate_project_content_documents(
        documents,
        svc_project_content::ProjectContentValidationContext {
            scenes: &[svc_project_content::project_scene_document_dto(scene)],
            entry_scene_id: Some(scene.id),
            gameplay: &gameplay,
            reference_revision: 0,
        },
    );
    assert!(outcome.result.accepted, "{:?}", outcome.result.diagnostics);
    outcome
        .result
        .canonical_files
        .into_iter()
        .map(|file| {
            let path = match file.document_id.as_str() {
                PLAYER_DEFINITION_DOCUMENT_ID => PLAYER_DEFINITION_PATH,
                ENEMY_DEFINITION_DOCUMENT_ID => ENEMY_DEFINITION_PATH,
                _ => file.document_id.as_str(),
            };
            (path.to_owned(), file.canonical_json.into_bytes())
        })
        .collect()
}

fn material_catalog_document(asset: &VoxelVolumeAsset) -> ProjectContentDocumentDto {
    ProjectContentDocumentDto::AssetCatalog {
        document_id: CATALOG_PATH.to_owned(),
        catalog: StoredAssetCatalog {
            entries: vec![
                StoredCatalogEntry {
                    id: asset.asset_id.clone(),
                    version: 1,
                    hash: None,
                    source_path: Some(VOXEL_PATH.to_owned()),
                    label: Some("Hand-authored room".to_owned()),
                    dependencies: Vec::new(),
                    material: None,
                },
                StoredCatalogEntry {
                    id: "material/concrete".to_owned(),
                    version: 1,
                    hash: None,
                    source_path: None,
                    label: Some("Concrete".to_owned()),
                    dependencies: Vec::new(),
                    material: Some(StoredMaterialDefinition {
                        authority: StoredMaterialAuthority {
                            solid: true,
                            collidable: true,
                            occludes: true,
                            structural_class: "structural".to_owned(),
                        },
                        style: StoredMaterialStyle {
                            color: Rgba {
                                r: 0.45,
                                g: 0.48,
                                b: 0.52,
                                a: 1.0,
                            },
                            texture: None,
                            roughness: 0.85,
                            texture_tint: Rgba {
                                r: 1.0,
                                g: 1.0,
                                b: 1.0,
                                a: 1.0,
                            },
                            emission_color: Rgba {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            },
                            emissive: 0.0,
                            uv_strategy: "flat".to_owned(),
                        },
                    }),
                },
                StoredCatalogEntry {
                    id: "mesh/fixture-character".to_owned(),
                    version: 1,
                    hash: Some(svc_serialization::BundleHash::of(ANIMATED_MESH_BYTES).to_hex()),
                    source_path: Some(ANIMATED_MESH_PATH.to_owned()),
                    label: Some("Animated character".to_owned()),
                    dependencies: Vec::new(),
                    material: None,
                },
                StoredCatalogEntry {
                    id: "audio/fixture-primary-fire".to_owned(),
                    version: 1,
                    hash: Some(svc_serialization::BundleHash::of(AUDIO_BYTES).to_hex()),
                    source_path: Some(AUDIO_PATH.to_owned()),
                    label: Some("Primary fire audio".to_owned()),
                    dependencies: Vec::new(),
                    material: None,
                },
                StoredCatalogEntry {
                    id: "sprite/fixture-primary-fire".to_owned(),
                    version: 1,
                    hash: Some(svc_serialization::BundleHash::of(PARTICLE_BYTES).to_hex()),
                    source_path: Some(PARTICLE_PATH.to_owned()),
                    label: Some("Primary fire particle".to_owned()),
                    dependencies: Vec::new(),
                    material: None,
                },
            ],
        },
    }
}

fn presentation_catalog_document() -> ProjectContentDocumentDto {
    presentation_catalog_document_with_clips(vec![
        "idle".to_owned(),
        "run".to_owned(),
        "jump".to_owned(),
    ])
}

fn presentation_catalog_document_with_clips(
    animation_clip_ids: Vec<String>,
) -> ProjectContentDocumentDto {
    ProjectContentDocumentDto::PresentationCatalog {
        document_id: PRESENTATION_PATH.to_owned(),
        catalog: ProjectPresentationCatalogDto {
            schema_version: PROJECT_CONTENT_SCHEMA_VERSION,
            resources: vec![
                ProjectPresentationResourceDto {
                    resource_id: "fixture.primary-fire.animation".to_owned(),
                    kind: ProjectPresentationResourceKind::AnimatedMesh,
                    asset_id: "mesh/fixture-character".to_owned(),
                    source_path: ANIMATED_MESH_PATH.to_owned(),
                    content_hash: svc_serialization::BundleHash::of(ANIMATED_MESH_BYTES).to_hex(),
                    license_path: None,
                    clip_ids: animation_clip_ids,
                },
                ProjectPresentationResourceDto {
                    resource_id: "fixture.primary-fire.audio".to_owned(),
                    kind: ProjectPresentationResourceKind::Audio,
                    asset_id: "audio/fixture-primary-fire".to_owned(),
                    source_path: AUDIO_PATH.to_owned(),
                    content_hash: svc_serialization::BundleHash::of(AUDIO_BYTES).to_hex(),
                    license_path: None,
                    clip_ids: Vec::new(),
                },
                ProjectPresentationResourceDto {
                    resource_id: "fixture.primary-fire.particle".to_owned(),
                    kind: ProjectPresentationResourceKind::Particle,
                    asset_id: "sprite/fixture-primary-fire".to_owned(),
                    source_path: PARTICLE_PATH.to_owned(),
                    content_hash: svc_serialization::BundleHash::of(PARTICLE_BYTES).to_hex(),
                    license_path: None,
                    clip_ids: Vec::new(),
                },
            ],
            cues: vec![
                ProjectPresentationCueDto::Animation {
                    cue_id: presentation_catalog::PRIMARY_FIRE_ANIMATION_CUE.to_owned(),
                    resource_id: "fixture.primary-fire.animation".to_owned(),
                    clip_id: "jump".to_owned(),
                    looped: false,
                    at_seconds: 0.05,
                    signal: protocol_project_content::ProjectPresentationSignalDto {
                        domain: protocol_project_content::ProjectPresentationSignalDomain::Particle,
                        signal_id: "fixture.primary-fire.animation.particle".to_owned(),
                    },
                },
                ProjectPresentationCueDto::Audio {
                    cue_id: "fixture.primary-fire.audio".to_owned(),
                    signal_id: presentation_catalog::PRIMARY_FIRE_PRESENTATION_SIGNAL.to_owned(),
                    resource_id: "fixture.primary-fire.audio".to_owned(),
                    gain: 0.7,
                },
                ProjectPresentationCueDto::Particle {
                    cue_id: "fixture.primary-fire.particle".to_owned(),
                    signal_id: presentation_catalog::PRIMARY_FIRE_PRESENTATION_SIGNAL.to_owned(),
                    resource_id: "fixture.primary-fire.particle".to_owned(),
                    scale: 1.0,
                },
                ProjectPresentationCueDto::Particle {
                    cue_id: "fixture.primary-fire.animation.particle".to_owned(),
                    signal_id: "fixture.primary-fire.animation.particle".to_owned(),
                    resource_id: "fixture.primary-fire.particle".to_owned(),
                    scale: 0.8,
                },
            ],
        },
    }
}

fn catalog_artifacts_with_presentation(
    composition: &GameplayStaticComposition,
    asset: &VoxelVolumeAsset,
    presentation: ProjectContentDocumentDto,
) -> (Vec<u8>, Vec<u8>) {
    let gameplay = rule_project_bundle::GameplayProjectContentAdmission::new(
        composition.project_configuration_authority(),
    );
    let outcome = svc_project_content::validate_project_content_documents(
        vec![material_catalog_document(asset), presentation],
        svc_project_content::ProjectContentValidationContext {
            scenes: &[],
            entry_scene_id: None,
            gameplay: &gameplay,
            reference_revision: 0,
        },
    );
    assert!(outcome.result.accepted, "{:?}", outcome.result.diagnostics);
    let catalog = outcome
        .result
        .canonical_files
        .iter()
        .find(|file| file.document_id == CATALOG_PATH)
        .expect("material catalog")
        .canonical_json
        .as_bytes()
        .to_vec();
    let presentation = outcome
        .result
        .canonical_files
        .iter()
        .find(|file| file.document_id == PRESENTATION_PATH)
        .expect("presentation catalog")
        .canonical_json
        .as_bytes()
        .to_vec();
    (catalog, presentation)
}

fn project_source_batch(
    bridge: &mut EngineBridge,
    composition: &GameplayStaticComposition,
    referenced_asset_id: &str,
) -> protocol_project_bundle::RuntimeProjectSourceBatch {
    project_source_batch_for_scene(
        bridge,
        composition,
        stored_scene(referenced_asset_id),
        Vec::new(),
    )
}

fn project_source_batch_for_scene(
    bridge: &mut EngineBridge,
    composition: &GameplayStaticComposition,
    scene: FlatSceneDocument,
    content_artifacts: Vec<(String, Vec<u8>)>,
) -> protocol_project_bundle::RuntimeProjectSourceBatch {
    project_source_batch_for_scene_with_presentation(
        bridge,
        composition,
        scene,
        content_artifacts,
        presentation_catalog_document(),
    )
}

fn project_source_batch_for_scene_with_presentation(
    bridge: &mut EngineBridge,
    composition: &GameplayStaticComposition,
    scene: FlatSceneDocument,
    content_artifacts: Vec<(String, Vec<u8>)>,
    presentation: ProjectContentDocumentDto,
) -> protocol_project_bundle::RuntimeProjectSourceBatch {
    let asset = hand_authored_voxel_volume_asset();
    let scene_bytes = core_scene::encode(&scene).into_bytes();
    let asset_bytes = svc_voxel_asset::encode_asset(&asset)
        .expect("canonical voxel asset")
        .into_bytes();
    let (catalog_bytes, presentation_bytes) =
        catalog_artifacts_with_presentation(composition, &asset, presentation);
    let lock_bytes = serde_json::to_vec(&serde_json::json!({
            "entries": [{
            "id": asset.asset_id,
            "kind": "voxel-volume",
            "version": 1,
            "hash": null,
            "dependencies": []
        }, {
            "id": "mesh/fixture-character", "kind": "mesh", "version": 1,
            "hash": svc_serialization::BundleHash::of(ANIMATED_MESH_BYTES).to_hex(), "dependencies": []
        }, {
            "id": "audio/fixture-primary-fire", "kind": "audio", "version": 1,
            "hash": svc_serialization::BundleHash::of(AUDIO_BYTES).to_hex(), "dependencies": []
        }, {
            "id": "sprite/fixture-primary-fire", "kind": "sprite", "version": 1,
            "hash": svc_serialization::BundleHash::of(PARTICLE_BYTES).to_hex(), "dependencies": []
        }]
    }))
    .expect("asset lock serializes");
    let mut artifacts = vec![
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
            PRESENTATION_PATH,
            svc_serialization::ArtifactRole::ProjectContent,
            &presentation_bytes,
        ),
        svc_serialization::ArtifactEntry::durable(
            ANIMATED_MESH_PATH,
            svc_serialization::ArtifactRole::Resource("resource:animatedMesh".to_owned()),
            ANIMATED_MESH_BYTES,
        ),
        svc_serialization::ArtifactEntry::durable(
            AUDIO_PATH,
            svc_serialization::ArtifactRole::Resource("resource:audio".to_owned()),
            AUDIO_BYTES,
        ),
        svc_serialization::ArtifactEntry::durable(
            PARTICLE_PATH,
            svc_serialization::ArtifactRole::Resource("resource:particle".to_owned()),
            PARTICLE_BYTES,
        ),
        svc_serialization::ArtifactEntry::durable(
            VOXEL_PATH,
            svc_serialization::ArtifactRole::VoxelVolumeAsset,
            &asset_bytes,
        ),
    ];
    artifacts.extend(content_artifacts.iter().map(|(path, bytes)| {
        svc_serialization::ArtifactEntry::durable(
            path,
            if path == GAMEPLAY_PATH {
                svc_serialization::ArtifactRole::ProjectContent
            } else {
                svc_serialization::ArtifactRole::EntityDefinitionCatalog
            },
            bytes,
        )
    }));
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
            asset_count: 4,
        },
        generation_provenance: None,
        artifacts,
    };
    let manifest_json = svc_serialization::encode(&manifest);
    let transaction = bridge
        .begin_runtime_project_source_resources(&manifest_json)
        .expect("begin stored-project resources");
    let staged_asset = bridge
        .stage_runtime_project_source_resource(transaction, VOXEL_PATH, asset_bytes)
        .expect("stage canonical voxel asset");
    let staged_audio = bridge
        .stage_runtime_project_source_resource(transaction, AUDIO_PATH, AUDIO_BYTES.to_vec())
        .expect("stage canonical audio resource");
    let staged_animated_mesh = bridge
        .stage_runtime_project_source_resource(
            transaction,
            ANIMATED_MESH_PATH,
            ANIMATED_MESH_BYTES.to_vec(),
        )
        .expect("stage canonical animated mesh resource");
    let staged_particle = bridge
        .stage_runtime_project_source_resource(transaction, PARTICLE_PATH, PARTICLE_BYTES.to_vec())
        .expect("stage canonical particle resource");
    let mut bodies = vec![
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
        protocol_project_bundle::ProjectSourceBody::Inline {
            path: PRESENTATION_PATH.to_owned(),
            bytes: presentation_bytes,
        },
        protocol_project_bundle::ProjectSourceBody::Resource {
            path: ANIMATED_MESH_PATH.to_owned(),
            resource: protocol_project_bundle::StagedProjectResourceRef {
                handle: staged_animated_mesh.handle.raw(),
                generation: staged_animated_mesh.generation,
                version: staged_animated_mesh.version,
                byte_len: staged_animated_mesh.byte_len,
            },
        },
        protocol_project_bundle::ProjectSourceBody::Resource {
            path: AUDIO_PATH.to_owned(),
            resource: protocol_project_bundle::StagedProjectResourceRef {
                handle: staged_audio.handle.raw(),
                generation: staged_audio.generation,
                version: staged_audio.version,
                byte_len: staged_audio.byte_len,
            },
        },
        protocol_project_bundle::ProjectSourceBody::Resource {
            path: PARTICLE_PATH.to_owned(),
            resource: protocol_project_bundle::StagedProjectResourceRef {
                handle: staged_particle.handle.raw(),
                generation: staged_particle.generation,
                version: staged_particle.version,
                byte_len: staged_particle.byte_len,
            },
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
    ];
    bodies.extend(
        content_artifacts.into_iter().map(|(path, bytes)| {
            protocol_project_bundle::ProjectSourceBody::Inline { path, bytes }
        }),
    );
    protocol_project_bundle::RuntimeProjectSourceBatch {
        manifest_json,
        resource_generation: Some(transaction.generation()),
        bodies,
    }
}

fn fps_project_source_batch(
    bridge: &mut EngineBridge,
    composition: &GameplayStaticComposition,
) -> protocol_project_bundle::RuntimeProjectSourceBatch {
    fps_project_source_batch_with(bridge, composition, 40, true)
}

fn fps_project_source_batch_with(
    bridge: &mut EngineBridge,
    composition: &GameplayStaticComposition,
    enemy_health: u32,
    include_player_weapon: bool,
) -> protocol_project_bundle::RuntimeProjectSourceBatch {
    fps_project_source_batch_with_role_mode(
        bridge,
        composition,
        enemy_health,
        include_player_weapon,
        false,
    )
}

fn fps_project_source_batch_with_role_mode(
    bridge: &mut EngineBridge,
    composition: &GameplayStaticComposition,
    enemy_health: u32,
    include_player_weapon: bool,
    enemy_controller_only_role: bool,
) -> protocol_project_bundle::RuntimeProjectSourceBatch {
    let scene = stored_fps_scene("voxel-volume/hand-authored-room");
    let content = fps_content_artifacts_with(
        composition,
        &scene,
        enemy_health,
        include_player_weapon,
        enemy_controller_only_role,
        false,
    );
    project_source_batch_for_scene(bridge, composition, scene, content)
}

fn fps_project_source_batch_missing_player_role(
    bridge: &mut EngineBridge,
    composition: &GameplayStaticComposition,
) -> protocol_project_bundle::RuntimeProjectSourceBatch {
    let scene = stored_fps_scene("voxel-volume/hand-authored-room");
    let content = fps_content_artifacts_with(composition, &scene, 40, true, false, true);
    project_source_batch_for_scene(bridge, composition, scene, content)
}

fn fps_input_catalog() -> InputBindingCatalog {
    InputBindingCatalog {
        schema_version: INPUT_BINDING_CATALOG_SCHEMA_VERSION,
        actions: vec![InputActionDefinition {
            action_id: "game.move.forward".to_owned(),
            value_kind: InputValueKind::Button,
            accepted_phases: vec![InputActionPhase::Held],
        }],
        contexts: vec![InputContextDefinition {
            context_id: "gameplay".to_owned(),
            priority: 10,
            consumes_lower_priority: false,
        }],
        bindings: vec![InputBindingRecord {
            binding_id: "game.forward.w".to_owned(),
            action_id: "game.move.forward".to_owned(),
            context_id: "gameplay".to_owned(),
            platform_kind: PlatformInputKind::KeyboardKey,
            control: "KeyW".to_owned(),
            scale: 1.0,
            extension: None,
        }],
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
    assert!(bridge.runtime_project.engine.is_none());
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
fn fps_activation_rejects_incomplete_animation_graph_before_publication() {
    let composition = static_composition();
    let mut bridge = DeferredRuntimeSessionBuilder::from_static_composition(composition.clone())
        .with_project_domain(RuntimeProjectDomainAdapter::Fps)
        .build_unloaded();
    bridge
        .initialize_engine(EngineConfig { seed: 6060 })
        .unwrap();
    let scene = stored_fps_scene("voxel-volume/hand-authored-room");
    let content = fps_content_artifacts_with(&composition, &scene, 40, true, false, false);
    let source = project_source_batch_for_scene_with_presentation(
        &mut bridge,
        &composition,
        scene,
        content,
        presentation_catalog_document_with_clips(vec!["jump".to_owned()]),
    );
    let admission = bridge
        .admit_runtime_project_source_batch(source)
        .expect("structurally valid source reaches staged activation");
    assert!(admission.accepted, "{:?}", admission.diagnostics);

    let error = bridge
        .activate_pending_runtime_project(RuntimeProjectLifecycleVersion::default())
        .expect_err("incomplete built-in animation graph must reject before publication");
    assert!(matches!(error, RuntimeProjectLoadError::Resource(_)));
    assert!(error
        .to_string()
        .contains("FPS animation graph is incompatible"));
    assert!(bridge.active_runtime_project().is_none());
    assert!(bridge.scene.entities.snapshot().records.is_empty());
    assert_eq!(
        bridge.runtime_project_lifecycle_version(),
        RuntimeProjectLifecycleVersion::default()
    );
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

#[test]
fn statically_required_fps_domain_rejects_missing_semantics_before_publication() {
    use protocol_project_bundle::{
        RuntimeProjectLoadRequest, RuntimeProjectSourceAdapterInput,
        RuntimeProjectSourceAdapterKind,
    };

    let composition = static_composition();
    let mut bridge = DeferredRuntimeSessionBuilder::from_static_composition(composition.clone())
        .with_project_domain(RuntimeProjectDomainAdapter::Fps)
        .build_unloaded();
    bridge
        .initialize_engine(EngineConfig { seed: 6006 })
        .unwrap();
    admit(&mut bridge, &composition, "voxel-volume/hand-authored-room");
    let before_lifecycle = bridge.runtime_project_lifecycle_version();
    let before_entities = bridge.scene.entities.hash();

    let receipt = RuntimeBridge::load_runtime_project(
        &mut bridge,
        RuntimeProjectLoadRequest {
            source: RuntimeProjectSourceAdapterInput {
                kind: RuntimeProjectSourceAdapterKind::InMemory,
                identity: "fixture:required-fps-missing-semantics".to_owned(),
                materialization_hash: "fnv1a64:6006000000000000".to_owned(),
            },
            expected_lifecycle: RuntimeProjectLifecycleVersion::default(),
        },
    )
    .unwrap();

    assert!(!receipt.accepted);
    assert_eq!(receipt.diagnostics[0].code, "missingEntityDefinitions");
    assert_eq!(receipt.diagnostics[0].document_id.as_deref(), Some("912"));
    assert_eq!(receipt.diagnostics[0].path.as_deref(), Some("nodes"));
    assert!(receipt.diagnostics[0]
        .message
        .contains("requires at least one canonical entity definition"));
    assert!(bridge.active_runtime_project().is_none());
    assert_eq!(bridge.scene.entities.hash(), before_entities);
    assert_eq!(bridge.runtime_project_lifecycle_version(), before_lifecycle);
    assert!(RuntimeBridge::read_fps_runtime_session(&bridge).is_err());
}

#[test]
fn canonical_project_load_activates_playable_fps_authority_without_legacy_bootstrap() {
    use protocol_project_bundle::{
        RuntimeProjectLoadRequest, RuntimeProjectSourceAdapterInput,
        RuntimeProjectSourceAdapterKind,
    };
    use protocol_view::{
        CameraCollisionPolicy, CameraCollisionPolicyMode, CameraCollisionShape,
        CameraCreateRequest, CameraPose, CollisionConstrainedCameraInputEnvelope,
        FirstPersonCameraInput, FirstPersonMovementMode, PerspectiveProjection, ViewportSize,
    };

    let composition = static_composition();
    let mut bridge = DeferredRuntimeSessionBuilder::from_static_composition(composition.clone())
        .with_project_domain(RuntimeProjectDomainAdapter::Fps)
        .build_unloaded();
    bridge
        .initialize_engine(EngineConfig { seed: 6007 })
        .unwrap();
    let source = fps_project_source_batch(&mut bridge, &composition);
    let admission = bridge
        .admit_runtime_project_source_batch(source)
        .expect("canonical FPS source admission");
    assert!(admission.accepted, "{:?}", admission.diagnostics);

    let receipt = RuntimeBridge::load_runtime_project(
        &mut bridge,
        RuntimeProjectLoadRequest {
            source: RuntimeProjectSourceAdapterInput {
                kind: RuntimeProjectSourceAdapterKind::InMemory,
                identity: "fixture:canonical-fps".to_owned(),
                materialization_hash: "fnv1a64:6007000000000000".to_owned(),
            },
            expected_lifecycle: RuntimeProjectLifecycleVersion::default(),
        },
    )
    .expect("public canonical load operation");
    assert!(receipt.accepted, "{:?}", receipt.diagnostics);
    assert_eq!(receipt.active_project.as_ref().unwrap().entity_count, 2);
    assert_eq!(
        receipt.active_project.as_ref().unwrap().voxel_asset_count,
        1
    );
    let active_content = RuntimeBridge::read_active_runtime_project_content(&bridge)
        .expect("active content is projected from Rust authority");
    assert_eq!(active_content.project_id, PROJECT_ID);
    assert_eq!(active_content.content.documents.len(), 5);
    assert_eq!(active_content.entry_scene.id, SceneId::new(SCENE_ID));
    assert_eq!(active_content.active_domains.len(), 1);
    assert_eq!(
        active_content.active_domains[0].kind,
        ActiveRuntimeProjectDomainKind::Fps
    );
    assert_eq!(
        active_content.active_domains[0]
            .entity_roles
            .iter()
            .map(|entry| entry.role)
            .collect::<Vec<_>>(),
        vec![
            ActiveRuntimeProjectEntityRole::Player,
            ActiveRuntimeProjectEntityRole::Enemy,
        ]
    );

    let initial = RuntimeBridge::read_fps_runtime_session(&bridge)
        .expect("FPS authority is active immediately after loadProject");
    assert_ne!(initial.player_entity, initial.enemy_entity);
    assert_eq!(
        initial
            .health
            .iter()
            .find(|health| health.entity == initial.enemy_entity)
            .map(|health| (health.current, health.max)),
        Some((40, 40))
    );
    assert_eq!(initial.policy_bindings.len(), 1);

    let active = receipt.active_project.as_ref().unwrap();
    let collision_grid = active.voxel_bindings[0].grid;
    let camera = bridge
        .create_camera(CameraCreateRequest {
            initial_pose: CameraPose {
                position: [3.5, 2.5, -2.7],
                yaw_degrees: 0.0,
                pitch_degrees: 0.0,
            },
            projection: PerspectiveProjection {
                fov_y_degrees: 60.0,
                near: 0.1,
                far: 1000.0,
            },
            viewport: ViewportSize {
                width: 1280,
                height: 720,
            },
        })
        .unwrap();
    let collision = bridge
        .apply_collision_constrained_camera_input(CollisionConstrainedCameraInputEnvelope {
            camera: camera.camera,
            grid: collision_grid,
            movement_mode: FirstPersonMovementMode::Grounded,
            input: FirstPersonCameraInput {
                move_forward: 1.0,
                move_right: 0.0,
                move_up: 0.0,
                yaw_delta_degrees: 0.0,
                pitch_delta_degrees: 0.0,
                dt_seconds: 1.0,
                move_speed_units_per_second: 1.0,
            },
            tick: 1,
            shape: CameraCollisionShape {
                half_extents: [0.2, 0.2, 0.2],
            },
            policy: CameraCollisionPolicy {
                mode: CameraCollisionPolicyMode::AxisSeparableSlide,
                max_iterations: 3,
            },
        })
        .expect("collision camera consumes the canonical active voxel binding");
    assert!(collision.collision.collided);

    let pause = bridge
        .apply_time_control_command(TimeControlCommand::Pause)
        .expect("canonical runtime retains time authority");
    assert!(pause.accepted);
    assert_eq!(pause.after.mode, TimeControlMode::Paused);

    let input = bridge
        .configure_input_session(InputSessionConfigureRequest {
            catalog: fps_input_catalog(),
            initial_contexts: vec!["gameplay".to_owned()],
        })
        .expect("canonical runtime retains input authority");
    assert_eq!(
        input.context_state.active_contexts[0].context_id,
        "gameplay"
    );
    let resolved = bridge
        .submit_raw_input(RawInputSample {
            sequence: 1,
            platform_kind: PlatformInputKind::KeyboardKey,
            control: "KeyW".to_owned(),
            phase: InputActionPhase::Held,
            value: InputValue::Button { pressed: true },
        })
        .expect("stored project input resolves");
    assert_eq!(resolved.action.unwrap().action_id, "game.move.forward");

    let gameplay = bridge
        .with_static_gameplay_runtime("canonical_fps_tick", |host| {
            let before = host.readout();
            let tick = host.tick(9)?;
            Ok((before, tick))
        })
        .expect("canonical gameplay operation")
        .expect("canonical gameplay host");
    assert!(!gameplay.0.trigger_snapshot_hash.is_empty());
    assert_eq!(gameplay.1.observe.invocations.len(), 1);

    let enemy = EntityId::new(initial.enemy_entity);
    let enemy_position = bridge
        .scene
        .entities
        .transform(enemy)
        .expect("enemy transform")
        .transform
        .translation;
    let moved = bridge
        .apply_enemy_direct_nav_movement(EnemyDirectNavMovementRequest {
            entity: initial.enemy_entity,
            seed_position: enemy_position,
            target: Vec3::new(enemy_position.x + 0.1, enemy_position.y, enemy_position.z),
            max_step_units: 0.1,
        })
        .expect("enemy movement uses canonical EntityStore");
    assert_eq!(
        moved.authority_source,
        EnemyDirectNavAuthoritySource::RustEntityStore
    );

    let player_position = bridge
        .scene
        .entities
        .transform(EntityId::new(initial.player_entity))
        .expect("player transform")
        .transform
        .translation;
    let enemy_position = bridge
        .scene
        .entities
        .transform(enemy)
        .expect("moved enemy transform")
        .transform
        .translation;
    let miss = bridge
        .apply_fps_primary_fire(FpsPrimaryFireRequest {
            tick: 5,
            origin: [
                player_position.x as f64,
                player_position.y as f64,
                player_position.z as f64,
            ],
            direction: [0.0, 0.0, 1.0],
            shooter_role: None,
            target_role: None,
        })
        .expect("miss feedback is resolved by canonical FPS authority");
    assert_eq!(miss.target, None);
    assert_eq!(
        miss.target_health_after,
        Some(FpsBridgeHealth {
            current: 40,
            max: 40,
        })
    );
    let fire = bridge
        .apply_fps_primary_fire(FpsPrimaryFireRequest {
            tick: 10,
            origin: [
                player_position.x as f64,
                player_position.y as f64,
                player_position.z as f64,
            ],
            direction: [
                (enemy_position.x - player_position.x) as f64,
                (enemy_position.y - player_position.y) as f64,
                (enemy_position.z - player_position.z) as f64,
            ],
            shooter_role: None,
            target_role: None,
        })
        .expect("primary fire uses stored collision and actor capabilities");
    assert_eq!(fire.target, Some(initial.enemy_entity));
    assert_eq!(
        fire.target_health_after,
        Some(FpsBridgeHealth {
            current: 0,
            max: 40,
        })
    );
    assert!(matches!(
        fire.lifecycle_status,
        FpsBridgeLifecycleStatus::EnemyDefeated { .. }
    ));

    let restarted = bridge
        .restart_fps_runtime_session(FpsRuntimeSessionRestartRequest {
            expected_epoch: initial.session_epoch,
        })
        .expect("restart reuses the internal canonical seed");
    assert_eq!(restarted.session_epoch, initial.session_epoch + 1);
    assert_eq!(
        restarted
            .health
            .iter()
            .find(|health| health.entity == restarted.enemy_entity)
            .map(|health| health.current),
        Some(40)
    );
}

#[test]
fn rust_active_domain_projects_controller_only_enemy_role() {
    use protocol_project_bundle::{
        RuntimeProjectLoadRequest, RuntimeProjectSourceAdapterInput,
        RuntimeProjectSourceAdapterKind,
    };

    let composition = static_composition();
    let mut bridge = DeferredRuntimeSessionBuilder::from_static_composition(composition.clone())
        .with_project_domain(RuntimeProjectDomainAdapter::Fps)
        .build_unloaded();
    bridge
        .initialize_engine(EngineConfig { seed: 6011 })
        .unwrap();
    let source = fps_project_source_batch_with_role_mode(&mut bridge, &composition, 40, true, true);
    let admission = bridge
        .admit_runtime_project_source_batch(source)
        .expect("controller-only role source admission");
    assert!(admission.accepted, "{:?}", admission.diagnostics);
    let receipt = RuntimeBridge::load_runtime_project(
        &mut bridge,
        RuntimeProjectLoadRequest {
            source: RuntimeProjectSourceAdapterInput {
                kind: RuntimeProjectSourceAdapterKind::InMemory,
                identity: "fixture:canonical-fps-controller-role".to_owned(),
                materialization_hash: "fnv1a64:6007000000000003".to_owned(),
            },
            expected_lifecycle: RuntimeProjectLifecycleVersion::default(),
        },
    )
    .unwrap();
    assert!(receipt.accepted, "{:?}", receipt.diagnostics);

    let snapshot = RuntimeBridge::read_fps_runtime_session(&bridge).unwrap();
    assert!(snapshot.policy_bindings.is_empty());
    let content = RuntimeBridge::read_active_runtime_project_content(&bridge).unwrap();
    let enemy_role = content.active_domains[0]
        .entity_roles
        .iter()
        .find(|entry| entry.role == ActiveRuntimeProjectEntityRole::Enemy)
        .expect("Rust adapter projects the controller-only enemy role");
    assert_eq!(enemy_role.entity, snapshot.enemy_entity);
}

#[test]
fn canonical_fps_configuration_changes_next_run_and_invalid_topology_is_atomic() {
    use protocol_project_bundle::{
        RuntimeProjectLoadRequest, RuntimeProjectSourceAdapterInput,
        RuntimeProjectSourceAdapterKind,
    };

    let load_request = || RuntimeProjectLoadRequest {
        source: RuntimeProjectSourceAdapterInput {
            kind: RuntimeProjectSourceAdapterKind::InMemory,
            identity: "fixture:canonical-fps-configuration".to_owned(),
            materialization_hash: "fnv1a64:6007000000000001".to_owned(),
        },
        expected_lifecycle: RuntimeProjectLifecycleVersion::default(),
    };

    let composition = static_composition();
    let mut configured =
        DeferredRuntimeSessionBuilder::from_static_composition(composition.clone())
            .with_project_domain(RuntimeProjectDomainAdapter::Fps)
            .build_unloaded();
    configured
        .initialize_engine(EngineConfig { seed: 6008 })
        .unwrap();
    let source = fps_project_source_batch_with(&mut configured, &composition, 65, true);
    let admission = configured
        .admit_runtime_project_source_batch(source)
        .expect("configured source admission");
    assert!(admission.accepted, "{:?}", admission.diagnostics);
    let loaded = RuntimeBridge::load_runtime_project(&mut configured, load_request()).unwrap();
    assert!(loaded.accepted, "{:?}", loaded.diagnostics);
    let snapshot = RuntimeBridge::read_fps_runtime_session(&configured).unwrap();
    assert_eq!(
        snapshot
            .health
            .iter()
            .find(|health| health.entity == snapshot.enemy_entity)
            .map(|health| (health.current, health.max)),
        Some((65, 65)),
        "stored actor configuration, not a bridge default, owns the next run"
    );

    let composition = static_composition();
    let mut rejected = DeferredRuntimeSessionBuilder::from_static_composition(composition.clone())
        .with_project_domain(RuntimeProjectDomainAdapter::Fps)
        .build_unloaded();
    rejected
        .initialize_engine(EngineConfig { seed: 6009 })
        .unwrap();
    let source = fps_project_source_batch_with(&mut rejected, &composition, 40, false);
    let admission = rejected
        .admit_runtime_project_source_batch(source)
        .expect("structurally valid source reaches domain admission");
    assert!(admission.accepted, "{:?}", admission.diagnostics);
    let receipt = RuntimeBridge::load_runtime_project(&mut rejected, load_request()).unwrap();
    assert!(!receipt.accepted);
    assert_eq!(receipt.diagnostics[0].code, "missingPlayerWeapon");
    assert_eq!(
        receipt.diagnostics[0].document_id.as_deref(),
        Some(PLAYER_DEFINITION_DOCUMENT_ID)
    );
    assert_eq!(
        receipt.diagnostics[0].path.as_deref(),
        Some("entities/demo-player.project-content.json.document.definition.capabilities")
    );
    assert!(receipt.diagnostics[0].message.contains("actor/demo-player"));
    assert!(receipt.diagnostics[0].message.contains("weaponMount"));
    assert!(rejected.active_runtime_project().is_none());
    assert!(rejected.scene.entities.snapshot().records.is_empty());
    assert_eq!(
        rejected.runtime_project_lifecycle_version(),
        RuntimeProjectLifecycleVersion::default()
    );
    assert_eq!(
        RuntimeBridge::read_fps_runtime_session(&rejected)
            .expect_err("failed domain activation publishes no FPS authority")
            .kind,
        RuntimeBridgeErrorKind::NotInitialized
    );
}

#[test]
fn canonical_fps_missing_role_diagnostic_names_authored_document_and_field() {
    use protocol_project_bundle::{
        RuntimeProjectLoadRequest, RuntimeProjectSourceAdapterInput,
        RuntimeProjectSourceAdapterKind,
    };

    let composition = static_composition();
    let mut bridge = DeferredRuntimeSessionBuilder::from_static_composition(composition.clone())
        .with_project_domain(RuntimeProjectDomainAdapter::Fps)
        .build_unloaded();
    bridge
        .initialize_engine(EngineConfig { seed: 6012 })
        .unwrap();
    let source = fps_project_source_batch_missing_player_role(&mut bridge, &composition);
    let admission = bridge
        .admit_runtime_project_source_batch(source)
        .expect("structurally valid missing-role source reaches domain activation");
    assert!(admission.accepted, "{:?}", admission.diagnostics);

    let receipt = RuntimeBridge::load_runtime_project(
        &mut bridge,
        RuntimeProjectLoadRequest {
            source: RuntimeProjectSourceAdapterInput {
                kind: RuntimeProjectSourceAdapterKind::InMemory,
                identity: "fixture:canonical-fps-missing-player-role".to_owned(),
                materialization_hash: "fnv1a64:6007000000000004".to_owned(),
            },
            expected_lifecycle: RuntimeProjectLifecycleVersion::default(),
        },
    )
    .unwrap();

    assert!(!receipt.accepted);
    assert_eq!(receipt.diagnostics[0].code, "missingPlayerRole");
    assert_eq!(
        receipt.diagnostics[0].document_id.as_deref(),
        Some(PLAYER_DEFINITION_DOCUMENT_ID)
    );
    assert_eq!(
        receipt.diagnostics[0].path.as_deref(),
        Some("entities/demo-player.project-content.json.document.definition.capabilities")
    );
    assert!(receipt.diagnostics[0].message.contains("player_input"));
    assert!(bridge.active_runtime_project().is_none());
    assert!(bridge.scene.entities.snapshot().records.is_empty());
    assert_eq!(
        bridge.runtime_project_lifecycle_version(),
        RuntimeProjectLifecycleVersion::default()
    );
}

#[test]
fn canonical_fps_spawn_binding_mismatch_rejects_without_publishing_authority() {
    use protocol_project_bundle::{
        RuntimeProjectLoadRequest, RuntimeProjectSourceAdapterInput,
        RuntimeProjectSourceAdapterKind,
    };

    let composition = static_composition();
    let mut bridge = DeferredRuntimeSessionBuilder::from_static_composition(composition.clone())
        .with_project_domain(RuntimeProjectDomainAdapter::Fps)
        .build_unloaded();
    bridge
        .initialize_engine(EngineConfig { seed: 6010 })
        .unwrap();
    let mut scene = stored_fps_scene("voxel-volume/hand-authored-room");
    let enemy = scene
        .nodes
        .iter_mut()
        .find(|node| node.id == SceneNodeId::new(102))
        .expect("enemy scene instance");
    let SceneNodeKind::EntityInstance(instance) = &mut enemy.kind else {
        panic!("enemy node must be an entity instance");
    };
    instance.spawn_marker_id = None;
    let content = fps_content_artifacts(&composition, &scene, 40, true);
    let source = project_source_batch_for_scene(&mut bridge, &composition, scene, content);
    let admission = bridge
        .admit_runtime_project_source_batch(source)
        .expect("structurally valid mismatch reaches domain activation");
    assert!(admission.accepted, "{:?}", admission.diagnostics);

    let receipt = RuntimeBridge::load_runtime_project(
        &mut bridge,
        RuntimeProjectLoadRequest {
            source: RuntimeProjectSourceAdapterInput {
                kind: RuntimeProjectSourceAdapterKind::InMemory,
                identity: "fixture:canonical-fps-spawn-mismatch".to_owned(),
                materialization_hash: "fnv1a64:6007000000000002".to_owned(),
            },
            expected_lifecycle: RuntimeProjectLifecycleVersion::default(),
        },
    )
    .unwrap();
    assert!(!receipt.accepted);
    assert_eq!(receipt.diagnostics[0].code, "spawnMarkerMismatch");
    assert_eq!(
        receipt.diagnostics[0].document_id.as_deref(),
        Some(ENEMY_DEFINITION_DOCUMENT_ID)
    );
    assert_eq!(
        receipt.diagnostics[0].path.as_deref(),
        Some("entities/tunnel-enemy.project-content.json.document.definition.capabilities")
    );
    assert!(receipt.diagnostics[0]
        .message
        .contains("binds spawn marker"));
    assert!(bridge.active_runtime_project().is_none());
    assert!(bridge.scene.entities.snapshot().records.is_empty());
    assert!(RuntimeBridge::read_fps_runtime_session(&bridge).is_err());
}

#[test]
fn generated_public_facade_accepts_all_closed_source_adapter_kinds() {
    use protocol_project_bundle::{
        RuntimeProjectCloseRequest, RuntimeProjectLoadRequest, RuntimeProjectSourceAdapterInput,
        RuntimeProjectSourceAdapterKind,
    };

    for (index, kind) in [
        RuntimeProjectSourceAdapterKind::DevelopmentDirectory,
        RuntimeProjectSourceAdapterKind::PackagedProject,
        RuntimeProjectSourceAdapterKind::InMemory,
    ]
    .into_iter()
    .enumerate()
    {
        let composition = static_composition();
        let mut bridge =
            DeferredRuntimeSessionBuilder::from_static_composition(composition.clone())
                .build_unloaded();
        bridge
            .initialize_engine(EngineConfig {
                seed: 100 + index as u64,
            })
            .unwrap();
        admit(&mut bridge, &composition, "voxel-volume/hand-authored-room");

        let receipt = RuntimeBridge::load_runtime_project(
            &mut bridge,
            RuntimeProjectLoadRequest {
                source: RuntimeProjectSourceAdapterInput {
                    kind,
                    identity: format!("fixture:{index}"),
                    materialization_hash: format!("fnv1a64:{index:016x}"),
                },
                expected_lifecycle: RuntimeProjectLifecycleVersion::default(),
            },
        )
        .expect("generated load operation returns a typed receipt");
        assert!(receipt.accepted, "{:?}", receipt.diagnostics);
        let active = receipt.active_project.expect("accepted load identity");
        assert_eq!(active.project_id, PROJECT_ID);
        assert!(!active.content_set_hash.is_empty());
        assert!(!active.composition_hash.is_empty());
        assert_eq!(active.scene_count, 1);
        assert_eq!(active.entity_count, 0);

        let closed = RuntimeBridge::close_runtime_project(
            &mut bridge,
            RuntimeProjectCloseRequest {
                expected_lifecycle: receipt.lifecycle,
            },
        )
        .expect("generated close operation returns a typed receipt");
        assert!(closed.accepted, "{:?}", closed.diagnostics);
        assert_eq!(closed.lifecycle.revision, 2);
    }
}
