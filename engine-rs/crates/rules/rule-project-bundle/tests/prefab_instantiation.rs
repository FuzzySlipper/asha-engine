use core_entity::{decode_snapshot, encode_snapshot, EntityLifecycle, EntitySource, EntityStore};
use core_ids::{PrefabId, PrefabInstanceId, PrefabPartId, RuntimeSessionId, SceneId, SceneNodeId};
use core_scene::{encode, SceneMetadata, SceneNode, SceneNodeKind, SceneTree};
use rule_project_bundle::{
    execute_load_plan, BundleArtifacts, InstantiatePrefabCommand, LoadExecutionError,
    PrefabInstanceAuthority, PrefabInstantiationCatalog, PrefabInstantiationError,
    PrefabPlacementOrigin, ProjectBundlePrefabError, ProjectBundleStage,
    SESSION_STATE_SNAPSHOT_PATH,
};
use svc_serialization::{
    LoadPlan, LoadStep, PrefabDefinition, PrefabInstanceRecord, PrefabOverride,
    PrefabOverrideValue, PrefabPart, PrefabPartReference, PrefabPartRoleBinding, PrefabPartSource,
    PrefabRegistry, PrefabRegistryValidationContext, PrefabTransform, PrefabVariantDelta,
    ValidatedPrefabRegistry, PREFAB_DEFINITION_SCHEMA_VERSION, PREFAB_REGISTRY_SCHEMA_VERSION,
};

fn transform(x: f32, y: f32, z: f32) -> PrefabTransform {
    PrefabTransform {
        translation: [x, y, z],
        ..PrefabTransform::IDENTITY
    }
}

fn registry(display_name: &str) -> ValidatedPrefabRegistry {
    let base = PrefabDefinition {
        id: PrefabId::new(10),
        schema_version: PREFAB_DEFINITION_SCHEMA_VERSION,
        display_name: display_name.to_owned(),
        parts: vec![
            PrefabPart {
                id: PrefabPartId::new(1),
                namespace: "body".to_owned(),
                display_name: "Body".to_owned(),
                parent: None,
                transform: transform(1.0, 0.0, 0.0),
                source: PrefabPartSource::EntityDefinition {
                    stable_id: "machine.assembler".to_owned(),
                },
            },
            PrefabPart {
                id: PrefabPartId::new(2),
                namespace: "tool/muzzle".to_owned(),
                display_name: "Muzzle".to_owned(),
                parent: Some(PrefabPartId::new(1)),
                transform: transform(0.0, 2.0, 0.0),
                source: PrefabPartSource::VoxelObject {
                    asset: "voxel-object/assembler-tool".to_owned(),
                },
            },
        ],
        part_roles: vec![
            PrefabPartRoleBinding {
                role: "machine/body".to_owned(),
                part: PrefabPartId::new(1),
            },
            PrefabPartRoleBinding {
                role: "weapon/muzzle".to_owned(),
                part: PrefabPartId::new(2),
            },
            PrefabPartRoleBinding {
                role: "weapon/output".to_owned(),
                part: PrefabPartId::new(2),
            },
        ],
        variant: None,
    };
    let variant = PrefabDefinition {
        id: PrefabId::new(11),
        schema_version: PREFAB_DEFINITION_SCHEMA_VERSION,
        display_name: "Fast assembler".to_owned(),
        parts: Vec::new(),
        part_roles: Vec::new(),
        variant: Some(PrefabVariantDelta {
            variant_id: "damaged".into(),
            base: PrefabId::new(10),
            removed_roles: Vec::new(),
            overrides: vec![PrefabOverride {
                target_role: "weapon/muzzle".to_owned(),
                value: PrefabOverrideValue::Asset {
                    asset: "voxel-object/fast-tool".to_owned(),
                },
            }],
        }),
    };
    let body_only_variant = PrefabDefinition {
        id: PrefabId::new(12),
        schema_version: PREFAB_DEFINITION_SCHEMA_VERSION,
        display_name: "Assembler body only".to_owned(),
        parts: Vec::new(),
        part_roles: Vec::new(),
        variant: Some(PrefabVariantDelta {
            variant_id: "body-only".into(),
            base: PrefabId::new(10),
            removed_roles: vec!["weapon/muzzle".to_owned(), "weapon/output".to_owned()],
            overrides: Vec::new(),
        }),
    };
    let context = PrefabRegistryValidationContext {
        asset_ids: [
            "voxel-object/assembler-tool",
            "voxel-object/fast-tool",
            "material/steel",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        entity_definition_ids: ["machine.assembler", "machine.assembler-heavy"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
    };
    ValidatedPrefabRegistry::new(
        PrefabRegistry {
            schema_version: PREFAB_REGISTRY_SCHEMA_VERSION,
            definitions: vec![body_only_variant, variant, base],
        },
        &context,
    )
    .expect("valid prefab registry")
}

fn catalog() -> PrefabInstantiationCatalog {
    PrefabInstantiationCatalog {
        asset_ids: [
            "voxel-object/assembler-tool",
            "voxel-object/fast-tool",
            "material/steel",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        entity_definition_ids: ["machine.assembler", "machine.assembler-heavy"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
    }
}

fn command(
    command_id: &str,
    origin: PrefabPlacementOrigin,
    prefab: u64,
    instance: u64,
    overrides: Vec<PrefabOverride>,
) -> InstantiatePrefabCommand {
    InstantiatePrefabCommand {
        command_id: command_id.to_owned(),
        origin,
        record: PrefabInstanceRecord {
            instance: PrefabInstanceId::new(instance),
            prefab: PrefabId::new(prefab),
            seed: 991,
            transform: transform(10.0, 0.0, -4.0),
            overrides,
        },
    }
}

fn loaded_stage() -> ProjectBundleStage {
    let scene = SceneTree {
        id: SceneId::new(90),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("prefab-stage".into()),
            authoring_format_version: 1,
        },
        dependencies: vec![],
        roots: vec![SceneNode::leaf(
            SceneNodeId::new(1),
            SceneNodeKind::EmptyGroup,
        )],
    };
    let plan = LoadPlan {
        steps: vec![
            LoadStep::ValidateVersions {
                bundle_schema_version: 2,
                protocol_version: 1,
            },
            LoadStep::LoadAssetLock {
                artifact: "assets/lock.json".into(),
                asset_count: 0,
            },
            LoadStep::LoadSceneDocument {
                artifact: "scene/scene.json".into(),
                scene: SceneId::new(90),
            },
            LoadStep::BootstrapScene {
                scene: SceneId::new(90),
                runtime_session: RuntimeSessionId::new(5),
            },
            LoadStep::ValidateFinalState,
        ],
    };
    let artifacts = BundleArtifacts::new()
        .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
        .with_artifact("scene/scene.json", encode(&scene.to_flat()));
    let mut stage = ProjectBundleStage::empty();
    stage.load_and_commit(&plan, &artifacts).unwrap();
    stage
}

#[test]
fn same_prefab_seed_and_placement_produce_identical_ids_roles_and_hashes() {
    let registry = registry("Assembler");
    let command = command("place-1", PrefabPlacementOrigin::Authored, 10, 20, vec![]);
    let mut left = PrefabInstanceAuthority::new();
    let mut right = PrefabInstanceAuthority::new();
    let mut left_entities = EntityStore::new();
    let mut right_entities = EntityStore::new();
    let left_receipt = left
        .instantiate(&mut left_entities, &registry, &catalog(), command.clone())
        .unwrap();
    let right_receipt = right
        .instantiate(&mut right_entities, &registry, &catalog(), command)
        .unwrap();

    assert_eq!(left_receipt, right_receipt);
    assert_eq!(left_receipt.facts.len(), 3);
    assert_eq!(
        left.state_hash(&left_entities),
        right.state_hash(&right_entities)
    );
    assert_eq!(
        left.instance(PrefabInstanceId::new(20)),
        right.instance(PrefabInstanceId::new(20))
    );
    let muzzle = left
        .resolve_part(
            PrefabInstanceId::new(20),
            &PrefabPartReference {
                prefab: PrefabId::new(10),
                role: "weapon/muzzle".to_owned(),
            },
        )
        .unwrap();
    let alias = left
        .resolve_part(
            PrefabInstanceId::new(20),
            &PrefabPartReference {
                prefab: PrefabId::new(10),
                role: "weapon/output".to_owned(),
            },
        )
        .unwrap();
    assert_eq!(muzzle.entity, alias.entity);
    assert_eq!(muzzle.node, alias.node);
    let instance = left.instance(PrefabInstanceId::new(20)).unwrap();
    let body = instance
        .parts
        .iter()
        .find(|part| part.part == PrefabPartId::new(1))
        .unwrap();
    let muzzle_part = instance
        .parts
        .iter()
        .find(|part| part.part == PrefabPartId::new(2))
        .unwrap();
    assert_eq!(muzzle_part.parent_entity, Some(body.entity));
    assert_eq!(body.transform.translation, [11.0, 0.0, -4.0]);
    assert_eq!(muzzle_part.transform.translation, [11.0, 2.0, -4.0]);
}

#[test]
fn display_rename_and_placement_origin_do_not_change_part_identity() {
    let mut authored = PrefabInstanceAuthority::new();
    let mut authored_entities = EntityStore::new();
    authored
        .instantiate(
            &mut authored_entities,
            &registry("Assembler"),
            &catalog(),
            command("authored", PrefabPlacementOrigin::Authored, 10, 20, vec![]),
        )
        .unwrap();
    let mut player = PrefabInstanceAuthority::new();
    let mut player_entities = EntityStore::new();
    player
        .instantiate(
            &mut player_entities,
            &registry("Renamed in the editor"),
            &catalog(),
            command("player", PrefabPlacementOrigin::Player, 10, 20, vec![]),
        )
        .unwrap();
    let authored_instance = authored.instance(PrefabInstanceId::new(20)).unwrap();
    let player_instance = player.instance(PrefabInstanceId::new(20)).unwrap();
    assert_eq!(authored_instance.parts, player_instance.parts);
    assert_eq!(authored_instance.role_map, player_instance.role_map);
}

#[test]
fn variant_and_instance_overrides_remain_separate_and_provenance_is_entity_owned() {
    let registry = registry("Assembler");
    let definition_override = PrefabOverride {
        target_role: "machine/body".to_owned(),
        value: PrefabOverrideValue::EntityDefinition {
            stable_id: "machine.assembler-heavy".to_owned(),
        },
    };
    let mut authority = PrefabInstanceAuthority::new();
    let mut entities = EntityStore::new();
    authority
        .instantiate(
            &mut entities,
            &registry,
            &catalog(),
            command(
                "variant-1",
                PrefabPlacementOrigin::Player,
                11,
                21,
                vec![
                    definition_override.clone(),
                    PrefabOverride {
                        target_role: "weapon/muzzle".to_owned(),
                        value: PrefabOverrideValue::Material {
                            asset: "material/steel".to_owned(),
                        },
                    },
                    PrefabOverride {
                        target_role: "weapon/muzzle".to_owned(),
                        value: PrefabOverrideValue::Activation { active: false },
                    },
                ],
            ),
        )
        .unwrap();
    let instance = authority.instance(PrefabInstanceId::new(21)).unwrap();
    assert_eq!(instance.record.overrides[0], definition_override);
    assert_eq!(instance.effective_overrides.len(), 4);
    assert!(matches!(
        instance.parts[0].source,
        PrefabPartSource::EntityDefinition { ref stable_id }
            if stable_id == "machine.assembler-heavy"
    ));
    assert!(matches!(
        instance.parts[1].source,
        PrefabPartSource::VoxelObject { ref asset } if asset == "voxel-object/fast-tool"
    ));
    assert_eq!(
        instance.parts[1].material_override.as_deref(),
        Some("material/steel")
    );
    assert!(!instance.parts[1].active);
    assert_eq!(
        entities.core(instance.parts[1].entity).unwrap().lifecycle,
        EntityLifecycle::Disabled
    );
    let source = &entities.core(instance.parts[1].entity).unwrap().source;
    assert!(matches!(
        source,
        EntitySource::PrefabInstance {
            prefab,
            instance,
            part,
            role: Some(role)
        } if *prefab == PrefabId::new(11)
            && *instance == PrefabInstanceId::new(21)
            && *part == PrefabPartId::new(2)
            && role == "weapon/muzzle"
    ));
    let encoded = encode_snapshot(&entities.snapshot());
    let restored_entities = EntityStore::from_snapshot(decode_snapshot(&encoded).unwrap());
    assert_eq!(
        restored_entities
            .core(instance.parts[1].entity)
            .unwrap()
            .source,
        source.clone()
    );
    assert_eq!(restored_entities.hash(), entities.hash());
}

#[test]
fn malformed_or_alias_conflicting_override_rejects_without_partial_state() {
    let registry = registry("Assembler");
    let mut authority = PrefabInstanceAuthority::new();
    let mut entities = EntityStore::new();
    let before = authority.state_hash(&entities);
    let conflict = vec![
        PrefabOverride {
            target_role: "weapon/muzzle".to_owned(),
            value: PrefabOverrideValue::Transform {
                transform: transform(1.0, 0.0, 0.0),
            },
        },
        PrefabOverride {
            target_role: "weapon/output".to_owned(),
            value: PrefabOverrideValue::Transform {
                transform: transform(2.0, 0.0, 0.0),
            },
        },
    ];
    let error = authority
        .instantiate(
            &mut entities,
            &registry,
            &catalog(),
            command("bad-alias", PrefabPlacementOrigin::Player, 10, 22, conflict),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        PrefabInstantiationError::DuplicateEffectiveOverride { .. }
    ));
    assert_eq!(authority.state_hash(&entities), before);
    assert!(authority.instance(PrefabInstanceId::new(22)).is_none());
    assert_eq!(entities.total_count(), 0);
}

#[test]
fn one_level_variant_removal_drops_the_part_and_every_alias_role() {
    let registry = registry("Assembler");
    let mut authority = PrefabInstanceAuthority::new();
    let mut entities = EntityStore::new();
    authority
        .instantiate(
            &mut entities,
            &registry,
            &catalog(),
            command("body-only", PrefabPlacementOrigin::Authored, 12, 23, vec![]),
        )
        .unwrap();
    let instance = authority.instance(PrefabInstanceId::new(23)).unwrap();
    assert_eq!(instance.parts.len(), 1);
    assert_eq!(instance.parts[0].part, PrefabPartId::new(1));
    assert_eq!(instance.role_map.len(), 1);
    assert_eq!(instance.role_map[0].reference.role, "machine/body");
    assert!(authority
        .resolve_part(
            PrefabInstanceId::new(23),
            &PrefabPartReference {
                prefab: PrefabId::new(12),
                role: "weapon/output".to_owned(),
            },
        )
        .is_none());
}

#[test]
fn snapshot_restore_replays_commands_and_rejects_tampered_hash() {
    let registry = registry("Assembler");
    let mut authority = PrefabInstanceAuthority::new();
    let mut entities = EntityStore::new();
    authority
        .instantiate(
            &mut entities,
            &registry,
            &catalog(),
            command("one", PrefabPlacementOrigin::Authored, 10, 30, vec![]),
        )
        .unwrap();
    authority
        .instantiate(
            &mut entities,
            &registry,
            &catalog(),
            command("two", PrefabPlacementOrigin::Player, 11, 31, vec![]),
        )
        .unwrap();
    let snapshot = authority.snapshot(&entities);
    let (restored, restored_entities) =
        PrefabInstanceAuthority::restore(&registry, &catalog(), &snapshot).unwrap();
    assert_eq!(
        restored.state_hash(&restored_entities),
        authority.state_hash(&entities)
    );
    assert_eq!(restored.snapshot(&restored_entities), snapshot);

    let mut tampered = snapshot;
    tampered.state_hash = "fnv1a64:0000000000000000".to_owned();
    assert_eq!(
        PrefabInstanceAuthority::restore(&registry, &catalog(), &tampered).unwrap_err(),
        PrefabInstantiationError::SnapshotDiverged
    );
}

#[test]
fn invalid_role_and_missing_prefab_are_atomic() {
    let registry = registry("Assembler");
    let mut authority = PrefabInstanceAuthority::new();
    let mut entities = EntityStore::new();
    let before = authority.state_hash(&entities);
    let invalid = PrefabOverride {
        target_role: "display-name/muzzle".to_owned(),
        value: PrefabOverrideValue::Transform {
            transform: PrefabTransform::IDENTITY,
        },
    };
    assert!(matches!(
        authority.instantiate(
            &mut entities,
            &registry,
            &catalog(),
            command(
                "bad-role",
                PrefabPlacementOrigin::Player,
                10,
                40,
                vec![invalid]
            )
        ),
        Err(PrefabInstantiationError::UnknownOverrideRole(_))
    ));
    assert!(matches!(
        authority.instantiate(
            &mut entities,
            &registry,
            &catalog(),
            command("missing", PrefabPlacementOrigin::Player, 999, 41, vec![])
        ),
        Err(PrefabInstantiationError::MissingPrefab(_))
    ));
    assert_eq!(authority.state_hash(&entities), before);
    assert_eq!(entities.total_count(), 0);
}

#[test]
fn project_bundle_stage_commits_prefab_and_entity_authority_atomically() {
    let registry = registry("Assembler");
    let mut empty = ProjectBundleStage::empty();
    assert_eq!(
        empty
            .instantiate_prefab(
                &registry,
                &catalog(),
                command("no-live", PrefabPlacementOrigin::Player, 10, 69, vec![]),
            )
            .unwrap_err(),
        ProjectBundlePrefabError::NoLiveSession
    );
    let mut stage = loaded_stage();
    let receipt = stage
        .instantiate_prefab(
            &registry,
            &catalog(),
            command("stage-good", PrefabPlacementOrigin::Player, 10, 70, vec![]),
        )
        .unwrap();
    assert_eq!(receipt.part_count, 2);
    let live = stage.live().unwrap();
    let entities = live.runtime_entities.as_ref().unwrap();
    assert_eq!(entities.total_count(), 2);
    assert!(live
        .prefab_instances
        .instance(PrefabInstanceId::new(70))
        .is_some());
    let before_entity_hash = entities.hash();
    let before_prefab_hash = live.prefab_instances.state_hash(entities);

    let error = stage
        .instantiate_prefab(
            &registry,
            &catalog(),
            command("stage-bad", PrefabPlacementOrigin::Player, 999, 71, vec![]),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ProjectBundlePrefabError::Instantiate(PrefabInstantiationError::MissingPrefab(_))
    ));
    let live = stage.live().unwrap();
    let entities = live.runtime_entities.as_ref().unwrap();
    assert_eq!(entities.hash(), before_entity_hash);
    assert_eq!(
        live.prefab_instances.state_hash(entities),
        before_prefab_hash
    );
    assert!(live
        .prefab_instances
        .instance(PrefabInstanceId::new(71))
        .is_none());
}

#[test]
fn project_bundle_session_save_reload_preserves_roles_overrides_and_provenance() {
    let registry = registry("Assembler");
    let mut stage = loaded_stage();
    stage
        .instantiate_prefab(
            &registry,
            &catalog(),
            command(
                "save-prefab",
                PrefabPlacementOrigin::Player,
                10,
                80,
                vec![
                    PrefabOverride {
                        target_role: "weapon/muzzle".to_owned(),
                        value: PrefabOverrideValue::Material {
                            asset: "material/steel".to_owned(),
                        },
                    },
                    PrefabOverride {
                        target_role: "weapon/muzzle".to_owned(),
                        value: PrefabOverrideValue::Activation { active: false },
                    },
                ],
            ),
        )
        .unwrap();
    let before = stage.live().unwrap();
    let before_entities = before.runtime_entities.as_ref().unwrap();
    let before_entity_hash = before_entities.hash();
    let before_prefab_hash = before.prefab_instances.state_hash(before_entities);
    let artifact = before.compose_session_state_snapshot().unwrap();

    let scene = SceneTree {
        id: SceneId::new(90),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("prefab-stage".into()),
            authoring_format_version: 1,
        },
        dependencies: vec![],
        roots: vec![SceneNode::leaf(
            SceneNodeId::new(1),
            SceneNodeKind::EmptyGroup,
        )],
    };
    let plan = LoadPlan {
        steps: vec![
            LoadStep::ValidateVersions {
                bundle_schema_version: 2,
                protocol_version: 1,
            },
            LoadStep::LoadAssetLock {
                artifact: "assets/lock.json".into(),
                asset_count: 0,
            },
            LoadStep::LoadSceneDocument {
                artifact: "scene/scene.json".into(),
                scene: SceneId::new(90),
            },
            LoadStep::BootstrapScene {
                scene: SceneId::new(90),
                runtime_session: RuntimeSessionId::new(5),
            },
            LoadStep::RestoreSessionState {
                artifact: SESSION_STATE_SNAPSHOT_PATH.into(),
            },
            LoadStep::ValidateFinalState,
        ],
    };
    let artifacts = BundleArtifacts::new()
        .with_artifact("assets/lock.json", "{ \"entries\": [] }\n")
        .with_artifact("scene/scene.json", encode(&scene.to_flat()))
        .with_artifact(SESSION_STATE_SNAPSHOT_PATH, artifact.text.clone());
    let restored = execute_load_plan(&plan, &artifacts).unwrap();
    let restored_entities = restored.runtime_entities.as_ref().unwrap();
    assert_eq!(restored_entities.hash(), before_entity_hash);
    assert_eq!(
        restored.prefab_instances.state_hash(restored_entities),
        before_prefab_hash
    );
    let output = restored
        .prefab_instances
        .resolve_part(
            PrefabInstanceId::new(80),
            &PrefabPartReference {
                prefab: PrefabId::new(10),
                role: "weapon/output".to_owned(),
            },
        )
        .expect("alias role survives reload");
    assert_eq!(
        restored_entities.core(output.entity).unwrap().lifecycle,
        EntityLifecycle::Disabled
    );
    let instance = restored
        .prefab_instances
        .instance(PrefabInstanceId::new(80))
        .unwrap();
    assert_eq!(instance.record.overrides.len(), 2);
    assert_eq!(instance.effective_overrides.len(), 2);

    let tampered = artifact
        .text
        .replace(&before_prefab_hash, "fnv1a64:0000000000000000");
    let error = execute_load_plan(
        &plan,
        &artifacts.with_artifact(SESSION_STATE_SNAPSHOT_PATH, tampered),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        LoadExecutionError::PrefabSessionStateDiverged { .. }
    ));
}
