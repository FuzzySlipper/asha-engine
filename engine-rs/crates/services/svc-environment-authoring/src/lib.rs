//! Pure Rust materialization of procedural environment recipes into canonical
//! stored scene and voxel-volume artifacts.
//!
//! This service owns the closed provider registry and deterministic generation
//! transaction. It does not own a RuntimeSession, a renderer, workspace
//! revisions, or host file writes; the bridge binds immutable candidates to
//! those lifecycle concerns.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use core_assets::{AssetId, AssetKind, AssetReference, AssetVersionReq};
use core_ids::SceneNodeId;
use core_math::Vec3;
use core_scene::{
    FlatSceneDocument, NodeMetadata, Quat, SceneMarker, SceneNodeKind, SceneNodeRecord,
    SceneTransform,
};
use core_space::{VoxelCoord, WorldVec};
use protocol_project_content::{
    ProceduralEnvironmentDiagnosticCode, ProceduralEnvironmentDiagnosticDto,
    ProceduralEnvironmentLimitsDto, ProceduralEnvironmentMarkerReadoutDto,
    ProceduralEnvironmentMarkerTargetDto, ProceduralEnvironmentProvenanceDto,
    ProceduralEnvironmentSourceReadoutDto,
};
use protocol_voxel_asset::{
    VoxelAssetAuthoringMetadata, VoxelAssetBounds, VoxelAssetContentHashes, VoxelAssetCoord,
    VoxelAssetGrid, VoxelAssetMaterialBinding, VoxelAssetProvenanceKind, VoxelAssetProvenanceRef,
    VoxelAssetRepresentation, VoxelAssetRepresentationKind, VoxelAssetSparseRun, VoxelVolumeAsset,
    VOXEL_ASSET_MEDIA_TYPE, VOXEL_ASSET_SCHEMA_VERSION,
};
use svc_levelgen::{generate_tunnel, GeneratedTunnel, TunnelGeneratorConfig, TUNNEL_GENERATOR_ID};
use svc_serialization::BundleHash;
use svc_spatial::VoxelWorld;

pub const MAX_GENERATED_VOXELS: u64 = 1_000_000;
pub const MAX_GENERATED_SPARSE_RUNS: u64 = 65_536;
pub const MAX_GENERATED_MARKERS: u64 = 128;
const GENERATED_PROVENANCE_URI_PREFIX: &str = "asha-generator://";

#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentTarget {
    pub scene_path: String,
    pub asset_id: String,
    pub asset_path: String,
    pub voxel_node_id: SceneNodeId,
    pub voxel_parent_id: Option<SceneNodeId>,
    pub voxel_child_order: u32,
    pub voxel_label: Option<String>,
    pub voxel_transform: SceneTransform,
    pub marker_targets: Vec<ProceduralEnvironmentMarkerTargetDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentMaterializationInput {
    pub provider_id: String,
    pub preset_id: String,
    pub seed: u64,
    /// Canonical asset already validated and loaded into this authoring
    /// generation. It is used only to authorize replacement after the stored
    /// scene has correctly discarded its runtime generator binding.
    pub replacement_asset: Option<VoxelVolumeAsset>,
    pub target: EnvironmentTarget,
    pub material_palette: Vec<VoxelAssetMaterialBinding>,
    pub authoring: VoxelAssetAuthoringMetadata,
    pub limits: ProceduralEnvironmentLimitsDto,
}

#[derive(Debug, Clone)]
pub struct MaterializedEnvironment {
    pub scene: FlatSceneDocument,
    pub scene_json: String,
    pub scene_hash: String,
    pub asset: VoxelVolumeAsset,
    pub asset_json: String,
    pub world: VoxelWorld,
    pub instance_transform: SceneTransform,
    pub provenance: ProceduralEnvironmentProvenanceDto,
    pub markers: Vec<ProceduralEnvironmentMarkerReadoutDto>,
    pub sources: ProceduralEnvironmentSourceReadoutDto,
    pub artifact_set_hash: String,
    pub candidate_hash: String,
}

pub type MaterializationResult =
    Result<MaterializedEnvironment, Vec<ProceduralEnvironmentDiagnosticDto>>;

/// Resolve a recipe through the statically closed registry and materialize it
/// into local-space voxel data plus authored scene placement.
pub fn materialize_environment(
    current_scene: &FlatSceneDocument,
    input: &EnvironmentMaterializationInput,
) -> MaterializationResult {
    let mut diagnostics = validate_request(current_scene, input);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let generated = match (input.provider_id.as_str(), input.preset_id.as_str()) {
        (TUNNEL_GENERATOR_ID, "tiny-enclosed") => {
            generate_tunnel(TunnelGeneratorConfig::tiny_enclosed(input.seed)).map_err(|error| {
                vec![diagnostic(
                    ProceduralEnvironmentDiagnosticCode::InvalidTarget,
                    "recipe",
                    error.to_string(),
                )]
            })?
        }
        _ => unreachable!("closed provider registry validated before dispatch"),
    };

    let resident = resident_voxels(&generated.world);
    let solid_voxel_count = resident.len() as u64;
    let sparse_runs = sparse_runs(&resident);
    if solid_voxel_count > input.limits.max_voxels.min(MAX_GENERATED_VOXELS) {
        diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::LimitExceeded,
            "limits.maxVoxels",
            format!("generated {solid_voxel_count} solid voxels"),
        ));
    }
    if sparse_runs.len() as u64 > input.limits.max_sparse_runs.min(MAX_GENERATED_SPARSE_RUNS) {
        diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::LimitExceeded,
            "limits.maxSparseRuns",
            format!("generated {} sparse runs", sparse_runs.len()),
        ));
    }
    if generated.spawn_markers.len() as u64 > input.limits.max_markers.min(MAX_GENERATED_MARKERS) {
        diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::LimitExceeded,
            "limits.maxMarkers",
            format!("generated {} markers", generated.spawn_markers.len()),
        ));
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let bounds = voxel_bounds(&resident).ok_or_else(|| {
        vec![diagnostic(
            ProceduralEnvironmentDiagnosticCode::InvalidGeneratedAsset,
            "asset.bounds",
            "provider generated no solid voxels",
        )]
    })?;
    let output_hash = format!("fnv1a64:{:016x}", generated.record.output_hash);
    let config_hash = format!("fnv1a64:{:016x}", generated.record.config_hash);
    let provenance = ProceduralEnvironmentProvenanceDto {
        provider_id: generated.record.generator_id.to_owned(),
        provider_version: generated.record.generator_version,
        preset_id: generated.record.preset.to_owned(),
        seed: generated.record.seed,
        config_hash: config_hash.clone(),
        output_hash: output_hash.clone(),
    };
    let asset = VoxelVolumeAsset {
        asset_id: input.target.asset_id.clone(),
        schema_version: VOXEL_ASSET_SCHEMA_VERSION,
        media_type: VOXEL_ASSET_MEDIA_TYPE.to_owned(),
        grid: VoxelAssetGrid {
            origin: [0.0, 0.0, 0.0],
            cell_size: generated.grid.voxel_size(),
            coordinate_system: svc_voxel_asset::VOXEL_ASSET_COORDINATE_SYSTEM.to_owned(),
        },
        bounds,
        representation: VoxelAssetRepresentation {
            kind: VoxelAssetRepresentationKind::SparseRuns,
            sparse_runs,
        },
        material_palette: input.material_palette.clone(),
        provenance: vec![generated_provenance_ref(&provenance)],
        authoring: input.authoring.clone(),
        validation_diagnostics: Vec::new(),
        content_hashes: VoxelAssetContentHashes {
            canonical_json: String::new(),
            voxel_data: String::new(),
        },
    };
    let asset = svc_voxel_asset::with_computed_hashes(&asset);
    let asset_report = svc_voxel_asset::validate_asset(&asset);
    if !asset_report.is_valid() {
        return Err(asset_report
            .diagnostics
            .into_iter()
            .map(|entry| {
                diagnostic(
                    ProceduralEnvironmentDiagnosticCode::InvalidGeneratedAsset,
                    entry.reference,
                    entry.message,
                )
            })
            .collect());
    }
    let asset_json = svc_voxel_asset::encode_asset(&asset).map_err(|report| {
        report
            .diagnostics
            .into_iter()
            .map(|entry| {
                diagnostic(
                    ProceduralEnvironmentDiagnosticCode::InvalidGeneratedAsset,
                    entry.reference,
                    entry.message,
                )
            })
            .collect::<Vec<_>>()
    })?;

    let (scene, markers) = materialized_scene(current_scene, input, &generated, &asset)?;
    let scene_report = core_scene::validate(&scene);
    if !scene_report.is_ok() {
        return Err(scene_report
            .errors
            .into_iter()
            .map(|error| {
                diagnostic(
                    ProceduralEnvironmentDiagnosticCode::InvalidGeneratedScene,
                    "scene.nodes",
                    format!("{}: {error:?}", error.label()),
                )
            })
            .collect());
    }
    let instance_transform =
        core_scene::composed_world_transforms(&scene)[&input.target.voxel_node_id.raw()];
    let scene_json = core_scene::encode(&scene);
    let scene_hash = hash_label(&scene_json);
    let collision = svc_collision::CollisionProjection::build_with_offset(
        &generated.world,
        WorldVec::new(
            f64::from(instance_transform.translation.x),
            f64::from(instance_transform.translation.y),
            f64::from(instance_transform.translation.z),
        ),
    );
    let collision_identity = collision.identity(&generated.world);
    let navigation = svc_pathfinding::build_nav_projection(
        &generated.world,
        svc_pathfinding::NavProjectionConfig::default(),
    )
    .map_err(|error| {
        vec![diagnostic(
            ProceduralEnvironmentDiagnosticCode::InvalidGeneratedAsset,
            "navigation",
            format!("navigation projection failed: {error:?}"),
        )]
    })?;
    let sources = ProceduralEnvironmentSourceReadoutDto {
        voxel_data_hash: asset.content_hashes.voxel_data.clone(),
        collision_source_hash: format!("fnv1a64:{}", collision_identity.source_hash_hex()),
        navigation_source_hash: format!("fnv1a64:{:016x}", navigation.projection_hash()),
        solid_voxel_count,
        walkable_voxel_count: navigation.walkable_len() as u64,
    };
    let artifact_set_hash = hash_label(&format!(
        "environment-artifacts-v1|{}|{}|{}|{}",
        input.target.scene_path,
        scene_hash,
        input.target.asset_path,
        asset.content_hashes.canonical_json
    ));
    let candidate_hash = hash_label(&format!(
        "environment-candidate-v1|{}|{}|{}|{}|{}|{}",
        artifact_set_hash, input.provider_id, input.preset_id, input.seed, config_hash, output_hash
    ));

    Ok(MaterializedEnvironment {
        scene,
        scene_json,
        scene_hash,
        asset,
        asset_json,
        world: generated.world,
        instance_transform,
        provenance,
        markers,
        sources,
        artifact_set_hash,
        candidate_hash,
    })
}

fn validate_request(
    current_scene: &FlatSceneDocument,
    input: &EnvironmentMaterializationInput,
) -> Vec<ProceduralEnvironmentDiagnosticDto> {
    let mut diagnostics = Vec::new();
    if input.provider_id != TUNNEL_GENERATOR_ID {
        diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::UnknownProvider,
            "providerId",
            format!(
                "provider {:?} is not in the closed registry",
                input.provider_id
            ),
        ));
    } else if input.preset_id != "tiny-enclosed" {
        diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::UnknownPreset,
            "presetId",
            format!(
                "preset {:?} is not registered by the provider",
                input.preset_id
            ),
        ));
    }
    if input.limits.max_voxels == 0
        || input.limits.max_sparse_runs == 0
        || input.limits.max_markers == 0
    {
        diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::LimitExceeded,
            "limits",
            "all generation limits must be greater than zero",
        ));
    }
    if !valid_project_relative_path(&input.target.scene_path)
        || !valid_project_relative_path(&input.target.asset_path)
        || input.target.scene_path == input.target.asset_path
    {
        diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::InvalidTarget,
            "target",
            "scene and asset paths must be distinct project-relative paths",
        ));
    }
    match AssetId::parse(&input.target.asset_id) {
        Ok(asset_id) if asset_id.kind() == AssetKind::VoxelVolume => {}
        _ => diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::InvalidTarget,
            "target.assetId",
            "assetId must be a valid voxel-volume asset identity",
        )),
    }
    if input.target.voxel_transform.validate().is_err() {
        diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::InvalidTarget,
            "target.voxelTransform",
            "voxel instance transform is invalid",
        ));
    }
    if input
        .target
        .voxel_parent_id
        .is_some_and(|parent| !current_scene.nodes.iter().any(|record| record.id == parent))
    {
        diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::InvalidTarget,
            "target.voxelParentId",
            "voxel parent does not exist in the current scene",
        ));
    }

    if let Err(recipe_diagnostic) = validate_authoring_recipe(current_scene, input) {
        diagnostics.push(recipe_diagnostic);
    }

    let source_ids = input
        .target
        .marker_targets
        .iter()
        .map(|target| target.source_marker_id.as_str())
        .collect::<BTreeSet<_>>();
    let marker_ids = input
        .target
        .marker_targets
        .iter()
        .map(|target| target.marker_id.as_str())
        .collect::<BTreeSet<_>>();
    let node_ids = input
        .target
        .marker_targets
        .iter()
        .map(|target| target.node_id.raw())
        .collect::<BTreeSet<_>>();
    if source_ids.len() != input.target.marker_targets.len()
        || marker_ids.len() != input.target.marker_targets.len()
        || node_ids.len() != input.target.marker_targets.len()
        || node_ids.contains(&input.target.voxel_node_id.raw())
    {
        diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::InvalidTarget,
            "target.markerTargets",
            "marker source ids, marker ids, and node ids must be unique",
        ));
    }
    for target in &input.target.marker_targets {
        if !matches!(
            target.source_marker_id.as_str(),
            "player_start" | "exit_hint"
        ) {
            diagnostics.push(diagnostic(
                ProceduralEnvironmentDiagnosticCode::InvalidTarget,
                "target.markerTargets.sourceMarkerId",
                format!("unknown generated marker {:?}", target.source_marker_id),
            ));
        }
    }
    if input.target.marker_targets.len() != 2 {
        diagnostics.push(diagnostic(
            ProceduralEnvironmentDiagnosticCode::InvalidTarget,
            "target.markerTargets",
            "tiny-enclosed requires explicit player_start and exit_hint marker targets",
        ));
    }
    diagnostics
}

fn validate_authoring_recipe(
    current_scene: &FlatSceneDocument,
    input: &EnvironmentMaterializationInput,
) -> Result<(), ProceduralEnvironmentDiagnosticDto> {
    let stored_recipe = current_scene.nodes.iter().find_map(|record| {
        let SceneNodeKind::Bootstrap(bindings) = &record.kind else {
            return None;
        };
        bindings.generator.as_ref()
    });
    if let Some(recipe) = stored_recipe {
        if recipe.provider_id == input.provider_id
            && recipe.preset_id == input.preset_id
            && recipe.seed == input.seed
        {
            return Ok(());
        }
        return Err(diagnostic(
            ProceduralEnvironmentDiagnosticCode::RecipeMismatch,
            "scene.bootstrap.generator",
            "request provider, preset, and seed must match the stored scene recipe",
        ));
    }

    let target_node = current_scene
        .nodes
        .iter()
        .find(|record| record.id == input.target.voxel_node_id);
    let target_matches_asset = target_node.is_some_and(|record| {
        matches!(
            &record.kind,
            SceneNodeKind::VoxelVolume(reference)
                if reference.id().as_str() == input.target.asset_id
        )
    });
    if !target_matches_asset {
        return Err(diagnostic(
            ProceduralEnvironmentDiagnosticCode::InvalidTarget,
            "target.voxelNodeId",
            "generator-free replacement must target the existing voxel-volume node for the requested asset",
        ));
    }

    let Some(asset) = input.replacement_asset.as_ref() else {
        return Err(diagnostic(
            ProceduralEnvironmentDiagnosticCode::RecipeMismatch,
            "replacementAsset.provenance",
            "generator-free replacement requires the canonical target asset loaded in this authoring generation",
        ));
    };
    if asset.asset_id != input.target.asset_id || !svc_voxel_asset::validate_asset(asset).is_valid()
    {
        return Err(diagnostic(
            ProceduralEnvironmentDiagnosticCode::InvalidTarget,
            "replacementAsset",
            "loaded replacement asset is invalid or does not match target.assetId",
        ));
    }
    let generated = asset
        .provenance
        .iter()
        .filter(|reference| reference.kind == VoxelAssetProvenanceKind::Generated)
        .collect::<Vec<_>>();
    if asset.provenance.len() != 1 || generated.len() != 1 {
        return Err(diagnostic(
            ProceduralEnvironmentDiagnosticCode::RecipeMismatch,
            "replacementAsset.provenance",
            "replacement asset must contain exactly one unambiguous generated provenance record",
        ));
    }
    parse_stored_generated_recipe(generated[0]).ok_or_else(|| {
        diagnostic(
            ProceduralEnvironmentDiagnosticCode::RecipeMismatch,
            "replacementAsset.provenance[0]",
            "replacement asset generated provenance is malformed",
        )
    })?;
    Ok(())
}

fn parse_stored_generated_recipe(reference: &VoxelAssetProvenanceRef) -> Option<()> {
    if reference.kind != VoxelAssetProvenanceKind::Generated {
        return None;
    }
    let suffix = reference
        .uri
        .strip_prefix(GENERATED_PROVENANCE_URI_PREFIX)?;
    let (path, query) = suffix.split_once('?')?;
    let mut segments = path.split('/');
    let provider_id = segments.next()?;
    let preset_id = segments.next()?;
    let provider_version: u32 = segments.next()?.strip_prefix('v')?.parse().ok()?;
    if provider_id.is_empty()
        || preset_id.is_empty()
        || segments.next().is_some()
        || !reference.content_hash.starts_with("fnv1a64:")
    {
        return None;
    }
    let mut fields = BTreeMap::new();
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=')?;
        if value.is_empty() || fields.insert(key, value).is_some() {
            return None;
        }
    }
    if fields.len() != 2 {
        return None;
    }
    let _seed: u64 = fields.get("seed")?.parse().ok()?;
    let config_hash = fields.get("configHash")?;
    if provider_version == 0 || !config_hash.starts_with("fnv1a64:") {
        return None;
    }
    Some(())
}

fn materialized_scene(
    current_scene: &FlatSceneDocument,
    input: &EnvironmentMaterializationInput,
    generated: &GeneratedTunnel,
    asset: &VoxelVolumeAsset,
) -> Result<
    (
        FlatSceneDocument,
        Vec<ProceduralEnvironmentMarkerReadoutDto>,
    ),
    Vec<ProceduralEnvironmentDiagnosticDto>,
> {
    let replaced_ids = input
        .target
        .marker_targets
        .iter()
        .map(|target| target.node_id.raw())
        .chain(std::iter::once(input.target.voxel_node_id.raw()))
        .collect::<BTreeSet<_>>();
    for record in &current_scene.nodes {
        if !replaced_ids.contains(&record.id.raw()) {
            continue;
        }
        let compatible = if record.id == input.target.voxel_node_id {
            matches!(record.kind, SceneNodeKind::VoxelVolume(_))
        } else {
            matches!(record.kind, SceneNodeKind::Marker(_))
        };
        if !compatible {
            return Err(vec![diagnostic(
                ProceduralEnvironmentDiagnosticCode::InvalidTarget,
                "target",
                format!(
                    "target node {} is already used by another scene kind",
                    record.id.raw()
                ),
            )]);
        }
    }

    let asset_id = AssetId::parse(&asset.asset_id).expect("validated asset id");
    let asset_reference = AssetReference::new(asset_id, AssetVersionReq::Any, None);
    let mut scene = current_scene.clone();
    for record in &mut scene.nodes {
        let SceneNodeKind::Bootstrap(bindings) = &mut record.kind else {
            continue;
        };
        if bindings.generator.as_ref().is_some_and(|recipe| {
            recipe.provider_id == input.provider_id
                && recipe.preset_id == input.preset_id
                && recipe.seed == input.seed
        }) {
            // The recipe has now produced canonical stored content. Catalogs
            // remain runtime bootstrap inputs, but generator identity is
            // authoring provenance and must not survive as a runtime
            // dependency.
            bindings.generator = None;
        }
    }
    scene
        .nodes
        .retain(|record| !replaced_ids.contains(&record.id.raw()));
    scene
        .dependencies
        .retain(|reference| reference.id().as_str() != asset.asset_id);
    scene.dependencies.push(asset_reference.clone());
    scene.nodes.push(SceneNodeRecord {
        id: input.target.voxel_node_id,
        parent: input.target.voxel_parent_id,
        child_order: input.target.voxel_child_order,
        transform: input.target.voxel_transform,
        kind: SceneNodeKind::VoxelVolume(asset_reference),
        metadata: NodeMetadata {
            label: input.target.voxel_label.clone(),
            tags: vec!["procedural-environment".to_owned()],
        },
    });

    let targets = input
        .target
        .marker_targets
        .iter()
        .map(|target| (target.source_marker_id.as_str(), target))
        .collect::<BTreeMap<_, _>>();
    let mut markers = Vec::new();
    for marker in &generated.spawn_markers {
        let target = targets[marker.id];
        let half_yaw = (marker.yaw_degrees as f32).to_radians() * 0.5;
        let transform = SceneTransform {
            translation: Vec3::new(
                marker.world.x as f32,
                marker.world.y as f32,
                marker.world.z as f32,
            ),
            rotation: Quat::new(0.0, half_yaw.sin(), 0.0, half_yaw.cos()),
            scale: Vec3::ONE,
        };
        scene.nodes.push(SceneNodeRecord {
            id: target.node_id,
            parent: Some(input.target.voxel_node_id),
            child_order: target.child_order,
            transform,
            kind: SceneNodeKind::Marker(SceneMarker {
                marker_id: target.marker_id.clone(),
            }),
            metadata: NodeMetadata {
                label: Some(marker.kind.to_owned()),
                tags: vec!["generated-marker".to_owned()],
            },
        });
        markers.push(ProceduralEnvironmentMarkerReadoutDto {
            source_marker_id: marker.id.to_owned(),
            marker_id: target.marker_id.clone(),
            node_id: target.node_id,
            local_position: transform.translation.to_array(),
            yaw_degrees: marker.yaw_degrees,
        });
    }
    Ok((scene, markers))
}

fn generated_provenance_ref(
    provenance: &ProceduralEnvironmentProvenanceDto,
) -> VoxelAssetProvenanceRef {
    VoxelAssetProvenanceRef {
        kind: VoxelAssetProvenanceKind::Generated,
        uri: format!(
            "{}{}/{}/v{}?seed={}&configHash={}",
            GENERATED_PROVENANCE_URI_PREFIX,
            provenance.provider_id,
            provenance.preset_id,
            provenance.provider_version,
            provenance.seed,
            provenance.config_hash,
        ),
        content_hash: provenance.output_hash.clone(),
    }
}

fn resident_voxels(world: &VoxelWorld) -> BTreeMap<VoxelCoord, u16> {
    let grid = world.grid();
    let mut resident = BTreeMap::new();
    for (chunk_coord, chunk) in world.resident_chunks() {
        for (local, value) in chunk.iter() {
            if let Some(material) = value.material() {
                resident.insert(
                    grid.chunk_local_to_voxel(chunk_coord, local),
                    material.raw(),
                );
            }
        }
    }
    resident
}

fn sparse_runs(resident: &BTreeMap<VoxelCoord, u16>) -> Vec<VoxelAssetSparseRun> {
    let mut voxels = resident
        .iter()
        .map(|(coord, material)| (*coord, *material))
        .collect::<Vec<_>>();
    voxels.sort_by_key(|(coord, material)| (coord.z, coord.y, coord.x, *material));
    let mut runs: Vec<VoxelAssetSparseRun> = Vec::new();
    for (coord, material) in voxels {
        if let Some(last) = runs.last_mut() {
            if last.start.y == coord.y
                && last.start.z == coord.z
                && last.material == material
                && last.start.x + i64::from(last.length) == coord.x
            {
                last.length += 1;
                continue;
            }
        }
        runs.push(VoxelAssetSparseRun {
            start: VoxelAssetCoord {
                x: coord.x,
                y: coord.y,
                z: coord.z,
            },
            length: 1,
            material,
        });
    }
    runs
}

fn voxel_bounds(resident: &BTreeMap<VoxelCoord, u16>) -> Option<VoxelAssetBounds> {
    let mut coords = resident.keys();
    let first = *coords.next()?;
    let mut min = first;
    let mut max = first;
    for coord in coords {
        min = VoxelCoord::new(min.x.min(coord.x), min.y.min(coord.y), min.z.min(coord.z));
        max = VoxelCoord::new(max.x.max(coord.x), max.y.max(coord.y), max.z.max(coord.z));
    }
    Some(VoxelAssetBounds {
        min: VoxelAssetCoord {
            x: min.x,
            y: min.y,
            z: min.z,
        },
        max: VoxelAssetCoord {
            x: max.x,
            y: max.y,
            z: max.z,
        },
    })
}

fn valid_project_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !value.starts_with('/')
        && !value.contains('\\')
        && value
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "..")
}

fn diagnostic(
    code: ProceduralEnvironmentDiagnosticCode,
    path: impl Into<String>,
    message: impl Into<String>,
) -> ProceduralEnvironmentDiagnosticDto {
    ProceduralEnvironmentDiagnosticDto {
        code,
        path: path.into(),
        message: message.into(),
    }
}

fn hash_label(value: &str) -> String {
    format!("fnv1a64:{}", BundleHash::of_str(value).to_hex())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_ids::SceneId;
    use core_scene::{SceneBootstrapBindings, SceneGeneratorBinding, SceneMetadata};

    fn base_scene() -> FlatSceneDocument {
        FlatSceneDocument {
            schema_version: 4,
            id: SceneId::new(7),
            metadata: SceneMetadata {
                name: Some("materialization".into()),
                authoring_format_version: 4,
            },
            dependencies: Vec::new(),
            nodes: vec![SceneNodeRecord {
                id: SceneNodeId::new(1),
                parent: None,
                child_order: 0,
                transform: SceneTransform::IDENTITY,
                kind: SceneNodeKind::Bootstrap(SceneBootstrapBindings {
                    generator: Some(SceneGeneratorBinding {
                        provider_id: TUNNEL_GENERATOR_ID.into(),
                        preset_id: "tiny-enclosed".into(),
                        seed: 42,
                    }),
                    catalogs: Vec::new(),
                }),
                metadata: NodeMetadata::default(),
            }],
        }
    }

    fn input() -> EnvironmentMaterializationInput {
        EnvironmentMaterializationInput {
            provider_id: TUNNEL_GENERATOR_ID.into(),
            preset_id: "tiny-enclosed".into(),
            seed: 42,
            replacement_asset: None,
            target: EnvironmentTarget {
                scene_path: "scenes/tunnel.scene.json".into(),
                asset_id: "voxel-volume/generated-tunnel".into(),
                asset_path: "assets/generated-tunnel.avxl.json".into(),
                voxel_node_id: SceneNodeId::new(10),
                voxel_parent_id: None,
                voxel_child_order: 1,
                voxel_label: Some("Generated tunnel".into()),
                voxel_transform: SceneTransform {
                    translation: Vec3::new(-3.5, -1.0, -5.5),
                    ..SceneTransform::IDENTITY
                },
                marker_targets: vec![
                    ProceduralEnvironmentMarkerTargetDto {
                        source_marker_id: "player_start".into(),
                        node_id: SceneNodeId::new(11),
                        marker_id: "spawn/player".into(),
                        child_order: 0,
                    },
                    ProceduralEnvironmentMarkerTargetDto {
                        source_marker_id: "exit_hint".into(),
                        node_id: SceneNodeId::new(12),
                        marker_id: "navigation/exit".into(),
                        child_order: 1,
                    },
                ],
            },
            material_palette: [1u16, 2, 3]
                .into_iter()
                .map(|material| VoxelAssetMaterialBinding {
                    voxel_material: material,
                    palette_entry_id: format!("voxel-material/tunnel-{material}"),
                    display_name: None,
                    material_asset_id: format!("material/tunnel-{material}"),
                    material_catalog_binding_id: Some(format!("catalog-binding/tunnel-{material}")),
                })
                .collect(),
            authoring: VoxelAssetAuthoringMetadata {
                label: Some("Generated tunnel".into()),
                created_by: Some("test".into()),
                source_tool: Some("svc-environment-authoring".into()),
            },
            limits: ProceduralEnvironmentLimitsDto {
                max_voxels: 10_000,
                max_sparse_runs: 10_000,
                max_markers: 8,
            },
        }
    }

    #[test]
    fn materializes_repeatable_local_asset_and_scene_placement() {
        let first = materialize_environment(&base_scene(), &input()).unwrap();
        let second = materialize_environment(&base_scene(), &input()).unwrap();
        assert_eq!(first.candidate_hash, second.candidate_hash);
        assert_eq!(first.asset.grid.origin, [0.0; 3]);
        assert_eq!(first.asset.content_hashes, second.asset.content_hashes);
        assert_eq!(first.scene_hash, second.scene_hash);
        assert_eq!(first.markers.len(), 2);
        assert!(first.sources.solid_voxel_count > 0);
        assert!(core_scene::validate(&first.scene).is_ok());
        assert!(first.scene.nodes.iter().all(|record| {
            !matches!(
                &record.kind,
                SceneNodeKind::Bootstrap(bindings) if bindings.generator.is_some()
            )
        }));

        let decoded_asset = svc_voxel_asset::decode_asset(&first.asset_json).unwrap();
        assert_eq!(
            decoded_asset.provenance,
            vec![generated_provenance_ref(&first.provenance)]
        );
        assert_eq!(
            decoded_asset.provenance[0].uri,
            format!(
                "asha-generator://{}/{}/v{}?seed={}&configHash={}",
                first.provenance.provider_id,
                first.provenance.preset_id,
                first.provenance.provider_version,
                first.provenance.seed,
                first.provenance.config_hash,
            )
        );
        assert_eq!(
            decoded_asset.provenance[0].content_hash,
            first.provenance.output_hash
        );
    }

    #[test]
    fn generator_free_scene_replacement_requires_matching_loaded_asset_provenance() {
        let first = materialize_environment(&base_scene(), &input()).unwrap();
        let mut replacement = input();
        replacement.replacement_asset = Some(first.asset.clone());

        let replaced = materialize_environment(&first.scene, &replacement).unwrap();
        assert_eq!(
            replaced
                .scene
                .nodes
                .iter()
                .filter(|record| matches!(record.kind, SceneNodeKind::VoxelVolume(_)))
                .count(),
            1
        );
        assert!(replaced.scene.nodes.iter().any(|record| {
            record.id == replacement.target.voxel_node_id
                && matches!(record.kind, SceneNodeKind::VoxelVolume(_))
        }));

        let mut changed_seed = replacement.clone();
        changed_seed.seed += 1;
        let changed = materialize_environment(&first.scene, &changed_seed).unwrap();
        assert_eq!(changed.provenance.seed, 43);
        assert_ne!(changed.asset.content_hashes, first.asset.content_hashes);

        let mut missing_asset = replacement;
        missing_asset.replacement_asset = None;
        let diagnostics = materialize_environment(&first.scene, &missing_asset).unwrap_err();
        assert!(diagnostics.iter().any(|entry| {
            entry.code == ProceduralEnvironmentDiagnosticCode::RecipeMismatch
                && entry.path == "replacementAsset.provenance"
        }));

        let mut malformed_asset = first.asset.clone();
        malformed_asset.provenance[0].uri = "asha-generator://malformed".into();
        let malformed_asset = svc_voxel_asset::with_computed_hashes(&malformed_asset);
        let mut malformed = input();
        malformed.replacement_asset = Some(malformed_asset);
        let diagnostics = materialize_environment(&first.scene, &malformed).unwrap_err();
        assert!(diagnostics.iter().any(|entry| {
            entry.code == ProceduralEnvironmentDiagnosticCode::RecipeMismatch
                && entry.path == "replacementAsset.provenance[0]"
        }));

        let mut foreign_asset = first.asset.clone();
        foreign_asset.provenance[0].kind = VoxelAssetProvenanceKind::Authored;
        let foreign_asset = svc_voxel_asset::with_computed_hashes(&foreign_asset);
        let mut foreign = input();
        foreign.replacement_asset = Some(foreign_asset);
        let diagnostics = materialize_environment(&first.scene, &foreign).unwrap_err();
        assert!(diagnostics.iter().any(|entry| {
            entry.code == ProceduralEnvironmentDiagnosticCode::RecipeMismatch
                && entry.path == "replacementAsset.provenance"
        }));
    }

    #[test]
    fn scene_parent_translation_is_composed_for_runtime_sources() {
        let mut scene = base_scene();
        scene.nodes.push(SceneNodeRecord {
            id: SceneNodeId::new(2),
            parent: None,
            child_order: 1,
            transform: SceneTransform {
                translation: Vec3::new(10.0, 2.0, -4.0),
                ..SceneTransform::IDENTITY
            },
            kind: SceneNodeKind::EmptyGroup,
            metadata: NodeMetadata::default(),
        });
        let mut request = input();
        request.target.voxel_parent_id = Some(SceneNodeId::new(2));

        let materialized = materialize_environment(&scene, &request).unwrap();

        assert_eq!(
            materialized.instance_transform.translation,
            Vec3::new(6.5, 1.0, -9.5)
        );
    }

    #[test]
    fn rejects_unknown_recipe_and_bounded_requests_atomically() {
        let mut unknown = input();
        unknown.provider_id = "unknown.provider".into();
        assert!(materialize_environment(&base_scene(), &unknown).is_err());

        let mut bounded = input();
        bounded.limits.max_voxels = 1;
        let diagnostics = materialize_environment(&base_scene(), &bounded).unwrap_err();
        assert!(diagnostics
            .iter()
            .any(|entry| entry.code == ProceduralEnvironmentDiagnosticCode::LimitExceeded));
    }
}
