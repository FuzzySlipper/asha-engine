#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use asha_runtime_session_composition::{
    AssetReferenceDto, AssetVersionReqDto, FlatSceneDocumentDto, ProjectContentDecodeRequestDto,
    ProjectContentDocumentKind, ProjectContentSourceDto, RuntimeBridge,
    SceneDocumentDecodeRequestDto, SceneDocumentEncodeRequestDto, SceneEntityInstanceDto,
    SceneEntityReferenceDto, SceneLightDto, SceneLightShadowIntentDto, SceneMetadataDto,
    SceneNodeKindDto, SceneNodeRecordDto, SceneTransformDto, StaticProjectAuthoringBuilder,
    WorkspaceAuthoringOpenRequest, WorkspaceAuthoringProjectBundleRef,
    WorkspaceAuthoringProjectIdentity,
};
use core_ids::{ProjectId, SceneId, SceneNodeId};
use protocol_voxel_asset::{
    VoxelAssetAuthoringMetadata, VoxelAssetBounds, VoxelAssetContentHashes, VoxelAssetCoord,
    VoxelAssetGrid, VoxelAssetMaterialBinding, VoxelAssetProvenanceKind, VoxelAssetProvenanceRef,
    VoxelAssetRepresentation, VoxelAssetRepresentationKind, VoxelAssetSparseRun, VoxelVolumeAsset,
    VOXEL_ASSET_MEDIA_TYPE, VOXEL_ASSET_SCHEMA_VERSION,
};
use svc_serialization::{
    ArtifactEntry, ArtifactRole, AssetLockSection, ProjectBundleManifest, ProjectSection,
    SceneSection, BUNDLE_SCHEMA_VERSION, SUPPORTED_PROTOCOL_VERSION,
};

const PROJECT_ID: u64 = 5997;
const ENTRY_SCENE_ID: u64 = 701;
const SECONDARY_SCENE_ID: u64 = 702;
const HOUSE_ASSET_ID: &str = "voxel-volume/demo-house";
const HOUSE_ASSET_PATH: &str = "assets/demo-house.avxl.json";

struct GeneratedProjectFile {
    role: ArtifactRole,
    bytes: Vec<u8>,
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = root.join("project");
    let artifacts = generate_project();
    if std::env::args().any(|argument| argument == "--check") {
        let actual = read_project_tree(&output);
        assert_eq!(
            actual, artifacts,
            "committed canonical project fixture drifted"
        );
        println!("canonical project consumer fixture is current");
        return;
    }
    if output.exists() {
        std::fs::remove_dir_all(&output).expect("remove prior canonical project fixture");
    }
    for (path, bytes) in artifacts {
        let target = output.join(&path);
        std::fs::create_dir_all(target.parent().expect("artifact has parent"))
            .expect("create canonical project directory");
        std::fs::write(&target, bytes).expect("write canonical project artifact");
        println!("wrote {}", target.display());
    }
}

fn read_project_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).expect("read committed project directory") {
            let entry = entry.expect("read committed project entry");
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, files);
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .expect("project file is under project root")
                .to_string_lossy()
                .replace('\\', "/");
            let prior = files.insert(
                relative.clone(),
                std::fs::read(&path).expect("read committed project file"),
            );
            assert!(
                prior.is_none(),
                "duplicate committed project path {relative}"
            );
        }
    }

    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

fn generate_project() -> BTreeMap<String, Vec<u8>> {
    let composition = asha_gameplay_module_fixture::composed_static_composition(4);
    let mut bridge = StaticProjectAuthoringBuilder::from_static_composition(composition).build();
    bridge
        .open_workspace_authoring(WorkspaceAuthoringOpenRequest {
            authoring_id: "canonical-project-consumer.generator".to_owned(),
            seed: 5997,
            project: WorkspaceAuthoringProjectIdentity {
                game_id: PROJECT_ID.to_string(),
                workspace_id: "canonical-project-consumer.generator".to_owned(),
            },
            project_bundle: WorkspaceAuthoringProjectBundleRef {
                bundle_schema_version: BUNDLE_SCHEMA_VERSION,
                protocol_version: SUPPORTED_PROTOCOL_VERSION,
                scene_id: ENTRY_SCENE_ID,
            },
        })
        .expect("open Rust project authoring cell");

    let scenes = [
        ("scenes/entry.scene.json", entry_scene()),
        ("scenes/secondary.scene.json", secondary_scene()),
    ];
    let mut files = BTreeMap::new();
    for (path, scene) in &scenes {
        let encoded = bridge
            .encode_scene_document(SceneDocumentEncodeRequestDto {
                document: scene.clone(),
            })
            .expect("encode scene through Rust");
        assert!(encoded.accepted, "{:?}", encoded.diagnostics);
        let canonical = encoded.canonical_json.expect("accepted scene is canonical");
        let reopened = bridge
            .decode_scene_document(SceneDocumentDecodeRequestDto {
                source_text: canonical.clone(),
            })
            .expect("install scene through Rust");
        assert!(reopened.accepted, "{:?}", reopened.diagnostics);
        insert_artifact(
            &mut files,
            path,
            ArtifactRole::SceneDocument,
            canonical.into_bytes(),
        );
    }

    let project_content = bridge
        .decode_project_content(ProjectContentDecodeRequestDto {
            sources: project_content_sources(),
        })
        .expect("decode ProjectContent through linked provider authority");
    assert!(
        project_content.accepted,
        "{:?}",
        project_content.diagnostics
    );
    for file in project_content.canonical_files {
        insert_artifact(
            &mut files,
            &file.document_id,
            ArtifactRole::ProjectContent,
            file.canonical_json.into_bytes(),
        );
    }

    let house = house_asset();
    let house_json = svc_voxel_asset::encode_asset(&house).expect("encode canonical house asset");
    insert_artifact(
        &mut files,
        HOUSE_ASSET_PATH,
        ArtifactRole::VoxelVolumeAsset,
        house_json.into_bytes(),
    );
    insert_artifact(
        &mut files,
        "assets/lock.json",
        ArtifactRole::AssetLock,
        b"{\"assets\":[]}".to_vec(),
    );

    let manifest = ProjectBundleManifest {
        bundle_schema_version: BUNDLE_SCHEMA_VERSION,
        protocol_version: SUPPORTED_PROTOCOL_VERSION,
        project: ProjectSection {
            id: ProjectId::new(PROJECT_ID),
            name: Some("Canonical project consumer".to_owned()),
        },
        entry_scene: SceneId::new(ENTRY_SCENE_ID),
        scenes: vec![
            SceneSection {
                id: SceneId::new(ENTRY_SCENE_ID),
                schema_version: 4,
                artifact: scenes[0].0.to_owned(),
            },
            SceneSection {
                id: SceneId::new(SECONDARY_SCENE_ID),
                schema_version: 4,
                artifact: scenes[1].0.to_owned(),
            },
        ],
        asset_lock: AssetLockSection {
            artifact: "assets/lock.json".to_owned(),
            asset_count: 1,
        },
        generation_provenance: None,
        artifacts: files
            .iter()
            .map(|(path, file)| {
                ArtifactEntry::durable(path.clone(), file.role.clone(), &file.bytes)
            })
            .collect(),
    }
    .canonical();
    manifest
        .validate()
        .expect("generated ProjectBundle is valid");
    let mut output = files
        .into_iter()
        .map(|(path, file)| (path, file.bytes))
        .collect::<BTreeMap<_, _>>();
    output.insert(
        svc_serialization::PROJECT_BUNDLE_MANIFEST_PATH.to_owned(),
        svc_serialization::encode(&manifest).into_bytes(),
    );
    output
}

fn insert_artifact(
    files: &mut BTreeMap<String, GeneratedProjectFile>,
    path: &str,
    role: ArtifactRole,
    bytes: Vec<u8>,
) {
    let replaced = files.insert(path.to_owned(), GeneratedProjectFile { role, bytes });
    assert!(
        replaced.is_none(),
        "duplicate generated project path {path}"
    );
}

fn project_content_sources() -> Vec<ProjectContentSourceDto> {
    let registry = asha_gameplay_module_fixture::binding_registry(4);
    let configuration = &registry.configurations[0];
    let module_json = serde_json::to_value(&configuration.module).expect("module ref serializes");
    let binding_json = serde_json::to_value(&registry.bindings[0]).expect("binding serializes");
    let schema_id = format!(
        "{}.{}.v{}",
        configuration.configuration.namespace,
        configuration.configuration.name,
        configuration.configuration.version
    );
    vec![
        source(
            "entities/demo-actor.json",
            ProjectContentDocumentKind::EntityDefinition,
            serde_json::json!({
                "kind": "EntityDefinition",
                "stableId": "demo.actor",
                "displayName": "Demo Actor",
                "source": {"projectBundle": "canonical-project-consumer", "relativePath": "entities/demo-actor.json"},
                "tags": [],
                "metadata": [],
                "capabilities": [
                    {"kind": "transform", "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}},
                    {"kind": "bounds", "min": [-0.4, 0, -0.4], "max": [0.4, 1.8, 0.4]},
                    {"kind": "collision", "staticCollider": false},
                    {"kind": "render", "visible": true}
                ]
            }),
        ),
        source(
            "catalogs/demo-assets.json",
            ProjectContentDocumentKind::AssetCatalog,
            serde_json::json!({
                "entries": [{
                    "id": HOUSE_ASSET_ID,
                    "version": 1,
                    "hash": null,
                    "sourcePath": HOUSE_ASSET_PATH,
                    "label": "Demo House",
                    "dependencies": [],
                    "material": null
                }]
            }),
        ),
        source(
            "prefabs/demo-registry.json",
            ProjectContentDocumentKind::PrefabRegistry,
            serde_json::json!({
                "schemaVersion": 1,
                "definitions": [{
                    "id": 700,
                    "schemaVersion": 1,
                    "displayName": "Demo Actor Prefab",
                    "parts": [{
                        "id": 1,
                        "namespace": "body",
                        "displayName": "Body",
                        "parent": null,
                        "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]},
                        "source": {"kind": "entityDefinition", "stableId": "demo.actor"}
                    }],
                    "partRoles": [{"role": "actor/body", "part": 1}],
                    "variant": null
                }]
            }),
        ),
        source(
            "gameplay/pulse.json",
            ProjectContentDocumentKind::GameplayConfiguration,
            serde_json::json!({
                "schemaVersion": 1,
                "configurations": [{
                    "configurationId": configuration.configuration_id,
                    "module": module_json,
                    "schemaId": schema_id,
                    "values": [{"fieldId": "multiplier", "value": {"kind": "integer", "value": 4}}]
                }],
                "bindings": [binding_json],
                "overrides": [],
                "triggers": []
            }),
        ),
        source(
            "presentation/demo-cues.json",
            ProjectContentDocumentKind::PresentationCatalog,
            serde_json::json!({"schemaVersion": 1, "resources": [], "cues": []}),
        ),
        source(
            "presentation/delete-me.json",
            ProjectContentDocumentKind::PresentationCatalog,
            serde_json::json!({"schemaVersion": 1, "resources": [], "cues": []}),
        ),
        source(
            "catalogs/split-source.json",
            ProjectContentDocumentKind::AssetCatalog,
            serde_json::json!({"entries": []}),
        ),
    ]
}

fn source(
    document_id: &str,
    kind: ProjectContentDocumentKind,
    value: serde_json::Value,
) -> ProjectContentSourceDto {
    ProjectContentSourceDto {
        document_id: document_id.to_owned(),
        kind,
        source_text: serde_json::to_string(&value).expect("ProjectContent source serializes"),
    }
}

fn entry_scene() -> FlatSceneDocumentDto {
    let house = voxel_reference();
    FlatSceneDocumentDto {
        schema_version: 4,
        id: SceneId::new(ENTRY_SCENE_ID),
        metadata: SceneMetadataDto {
            name: Some("Entry plaza".to_owned()),
            authoring_format_version: 4,
        },
        dependencies: vec![house.clone()],
        nodes: vec![
            node(
                1,
                0,
                "Demo house",
                [3.0, 0.0, -4.0],
                SceneNodeKindDto::VoxelVolume(house),
            ),
            node(
                2,
                1,
                "Direct actor",
                [0.0, 1.0, 0.0],
                SceneNodeKindDto::EntityInstance {
                    instance: SceneEntityInstanceDto {
                        instance_id: "entry.direct-actor".to_owned(),
                        reference: SceneEntityReferenceDto::EntityDefinition {
                            stable_id: "demo.actor".to_owned(),
                        },
                        spawn_marker_id: None,
                    },
                },
            ),
            node(
                3,
                2,
                "Prefab actor",
                [2.0, 1.0, 0.0],
                SceneNodeKindDto::EntityInstance {
                    instance: SceneEntityInstanceDto {
                        instance_id: "entry.prefab-actor".to_owned(),
                        reference: SceneEntityReferenceDto::Prefab {
                            prefab_id: 700,
                            variant_id: None,
                            instantiation_seed: 5997,
                        },
                        spawn_marker_id: None,
                    },
                },
            ),
            node(
                4,
                3,
                "Sun",
                [0.0, 8.0, 0.0],
                SceneNodeKindDto::Light(SceneLightDto::Directional {
                    color: [1.0, 0.95, 0.85],
                    intensity: 2.0,
                    enabled: true,
                    shadow_intent: SceneLightShadowIntentDto::Requested,
                }),
            ),
        ],
    }
}

fn secondary_scene() -> FlatSceneDocumentDto {
    FlatSceneDocumentDto {
        schema_version: 4,
        id: SceneId::new(SECONDARY_SCENE_ID),
        metadata: SceneMetadataDto {
            name: Some("Secondary room".to_owned()),
            authoring_format_version: 4,
        },
        dependencies: Vec::new(),
        nodes: vec![node(
            20,
            0,
            "Secondary actor",
            [-2.0, 1.0, 1.0],
            SceneNodeKindDto::EntityInstance {
                instance: SceneEntityInstanceDto {
                    instance_id: "secondary.direct-actor".to_owned(),
                    reference: SceneEntityReferenceDto::EntityDefinition {
                        stable_id: "demo.actor".to_owned(),
                    },
                    spawn_marker_id: None,
                },
            },
        )],
    }
}

fn node(
    id: u64,
    child_order: u32,
    label: &str,
    translation: [f32; 3],
    kind: SceneNodeKindDto,
) -> SceneNodeRecordDto {
    SceneNodeRecordDto {
        id: SceneNodeId::new(id),
        parent: None,
        child_order,
        label: Some(label.to_owned()),
        tags: Vec::new(),
        transform: SceneTransformDto {
            translation,
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        },
        kind,
    }
}

fn voxel_reference() -> AssetReferenceDto {
    AssetReferenceDto {
        id: HOUSE_ASSET_ID.to_owned(),
        version: AssetVersionReqDto::Any,
        hash: None,
    }
}

fn house_asset() -> VoxelVolumeAsset {
    let mut runs = Vec::new();
    for z in 0..=4 {
        runs.push(run(0, 0, z, 5, 1));
        runs.push(run(0, 4, z, 5, 2));
    }
    for y in 1..=3 {
        for z in 0..=4 {
            if z == 0 {
                if y <= 2 {
                    runs.push(run(0, y, z, 2, 1));
                    runs.push(run(3, y, z, 2, 1));
                } else {
                    runs.push(run(0, y, z, 5, 1));
                }
            } else if z == 4 {
                runs.push(run(0, y, z, 5, 1));
            } else {
                runs.push(run(0, y, z, 1, 1));
                runs.push(run(4, y, z, 1, 1));
            }
        }
    }
    let asset = VoxelVolumeAsset {
        asset_id: HOUSE_ASSET_ID.to_owned(),
        schema_version: VOXEL_ASSET_SCHEMA_VERSION,
        media_type: VOXEL_ASSET_MEDIA_TYPE.to_owned(),
        grid: VoxelAssetGrid {
            origin: [0.0, 0.0, 0.0],
            cell_size: 1.0,
            coordinate_system: svc_voxel_asset::VOXEL_ASSET_COORDINATE_SYSTEM.to_owned(),
        },
        bounds: VoxelAssetBounds {
            min: VoxelAssetCoord { x: 0, y: 0, z: 0 },
            max: VoxelAssetCoord { x: 4, y: 4, z: 4 },
        },
        representation: VoxelAssetRepresentation {
            kind: VoxelAssetRepresentationKind::SparseRuns,
            sparse_runs: runs,
        },
        material_palette: vec![
            VoxelAssetMaterialBinding {
                voxel_material: 1,
                palette_entry_id: "voxel-material/brick".to_owned(),
                display_name: Some("Brick".to_owned()),
                material_asset_id: "material/brick".to_owned(),
                material_catalog_binding_id: Some("catalog-binding/brick".to_owned()),
            },
            VoxelAssetMaterialBinding {
                voxel_material: 2,
                palette_entry_id: "voxel-material/roof".to_owned(),
                display_name: Some("Roof".to_owned()),
                material_asset_id: "material/roof".to_owned(),
                material_catalog_binding_id: Some("catalog-binding/roof".to_owned()),
            },
        ],
        provenance: vec![VoxelAssetProvenanceRef {
            kind: VoxelAssetProvenanceKind::Authored,
            uri: "asha://canonical-project-consumer/assets/demo-house".to_owned(),
            content_hash: "fnv1a64:canonical-consumer-house".to_owned(),
        }],
        authoring: VoxelAssetAuthoringMetadata {
            label: Some("Demo house".to_owned()),
            created_by: Some("generate-canonical-project".to_owned()),
            source_tool: Some("asha canonical project consumer".to_owned()),
        },
        validation_diagnostics: Vec::new(),
        content_hashes: VoxelAssetContentHashes {
            canonical_json: String::new(),
            voxel_data: String::new(),
        },
    };
    let asset = svc_voxel_asset::with_computed_hashes(&asset);
    let report = svc_voxel_asset::validate_asset(&asset);
    assert!(report.is_valid(), "{:?}", report.diagnostics);
    asset
}

fn run(x: i64, y: i64, z: i64, length: u32, material: u16) -> VoxelAssetSparseRun {
    VoxelAssetSparseRun {
        start: VoxelAssetCoord { x, y, z },
        length,
        material,
    }
}
