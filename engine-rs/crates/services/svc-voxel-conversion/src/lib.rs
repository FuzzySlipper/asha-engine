//! Rust authority service for bounded static-mesh to voxel conversion.
//!
//! # Lane
//!
//! `rust-service` — validates supported Asha static mesh/source assets and
//! produces deterministic voxel-conversion plans, previews, apply receipts, and
//! classified diagnostics. Studio and TypeScript consume the protocol DTOs; they
//! do not own conversion authority.
//!
//! # Current supported source shape
//!
//! This first slice accepts already-loaded static mesh source data: positions,
//! triangles, and source material slots. It intentionally does not import glTF,
//! read renderer buffers, or depend on Three.js/render protocol internals.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use core_voxel::VoxelValue;
use protocol_diagnostics::DiagnosticSeverity;
use protocol_voxel_conversion::{
    VoxelConversionApplyRequest, VoxelConversionBounds, VoxelConversionDiagnostic,
    VoxelConversionDiagnosticCode, VoxelConversionEvidenceKind, VoxelConversionEvidenceRef,
    VoxelConversionFitPolicy, VoxelConversionMode, VoxelConversionOriginPolicy,
    VoxelConversionPlan, VoxelConversionPlanRequest, VoxelConversionPreview,
    VoxelConversionPreviewRequest, VoxelConversionPreviewVoxel, VoxelConversionReceipt,
    VoxelConversionSourceRef, VoxelConversionTargetRef, VoxelConversionTextureSampleAsset,
    VoxelConversionTextureSourceRef,
};

pub const AUTHORITY_VERSION: &str = "svc-voxel-conversion.v0";
pub const MAX_SOURCE_VERTICES: usize = 1_000_000;
pub const MAX_SOURCE_TRIANGLES: usize = 2_000_000;
pub const MAX_RESOLUTION_AXIS: u32 = 4_096;
pub const MAX_RESOLUTION_CELLS: u64 = 512_000_000;
pub const MAX_REQUESTED_OUTPUT_VOXELS: u64 = 512_000_000;

const DEFAULT_RESOURCE_LIMITS: ConversionResourceLimits = ConversionResourceLimits {
    max_source_vertices: MAX_SOURCE_VERTICES,
    max_source_triangles: MAX_SOURCE_TRIANGLES,
    max_resolution_axis: MAX_RESOLUTION_AXIS,
    max_resolution_cells: MAX_RESOLUTION_CELLS,
    max_requested_output_voxels: MAX_REQUESTED_OUTPUT_VOXELS,
};

/// One supported static mesh source already loaded by Asha authority.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticMeshSource {
    pub asset_id: String,
    pub asset_kind: String,
    pub asset_version: u64,
    pub source_hash: String,
    pub mesh_primitive: Option<String>,
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<MeshTriangle>,
}

/// One triangle with a source material slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshTriangle {
    pub indices: [u32; 3],
    pub source_material_slot: u32,
}

/// Internal sparse authority voxel output. Absence is empty; present voxels are
/// always [`VoxelValue::Solid`] with a validated material id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertedVoxel {
    pub coord: protocol_voxel_conversion::VoxelConversionCoord,
    pub value: VoxelValue,
}

/// Full deterministic conversion output used by preview/apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionOutput {
    pub voxels: Vec<ConvertedVoxel>,
    pub bounds: Option<VoxelConversionBounds>,
    pub output_hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlannedConversion {
    pub plan: VoxelConversionPlan,
    pub output: Option<ConversionOutput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConversionResourceLimits {
    max_source_vertices: usize,
    max_source_triangles: usize,
    max_resolution_axis: u32,
    max_resolution_cells: u64,
    max_requested_output_voxels: u64,
}

pub fn plan_conversion(
    request: &VoxelConversionPlanRequest,
    source: &StaticMeshSource,
) -> PlannedConversion {
    let mut diagnostics = Vec::new();
    validate_source_ref(&request.source, source, &mut diagnostics);
    validate_settings(request, source, &mut diagnostics);

    let output = if diagnostics.is_empty() {
        build_output(request, source, &mut diagnostics)
    } else {
        None
    };

    let estimated_bounds = output.as_ref().and_then(|o| o.bounds);
    let estimated_output_voxels = output.as_ref().map_or(0, |o| o.voxels.len() as u64);
    let plan_id = stable_hash(&[
        "plan",
        &request.source.asset_id,
        &request.source.source_hash,
        &settings_fingerprint(request),
    ]);
    let settings_hash = stable_hash(&["settings", &settings_fingerprint(request)]);
    let authority_version = AUTHORITY_VERSION.to_string();
    let expected_source_hash = request.source.source_hash.clone();
    let plan_hash = plan_hash_from_parts(
        &plan_id,
        &expected_source_hash,
        &settings_hash,
        &authority_version,
    );
    let evidence = vec![evidence_ref(
        VoxelConversionEvidenceKind::Plan,
        format!("asha://voxel-conversion/plan/{plan_id}"),
        &stable_hash(&["plan-evidence", &plan_id, &settings_hash]),
    )];

    PlannedConversion {
        plan: VoxelConversionPlan {
            plan_id,
            source: request.source.clone(),
            target: request.target.clone(),
            settings: request.settings.clone(),
            authority_version,
            expected_source_hash,
            settings_hash,
            plan_hash,
            estimated_output_voxels,
            estimated_bounds,
            diagnostics,
            evidence,
        },
        output,
    }
}

pub fn preview_conversion(
    request: &VoxelConversionPreviewRequest,
    planned: &PlannedConversion,
) -> VoxelConversionPreview {
    let expected = plan_hash(&planned.plan);
    if request.plan_id != planned.plan.plan_id || request.expected_plan_hash != expected {
        return VoxelConversionPreview {
            plan_id: request.plan_id.clone(),
            output_hash: String::new(),
            output_voxel_count: 0,
            output_bounds: None,
            sample_voxels: Vec::new(),
            diagnostics: vec![diagnostic(
                VoxelConversionDiagnosticCode::StaleAuthoritySnapshot,
                DiagnosticSeverity::Error,
                "plan",
                "preview request did not match the current authority plan hash",
            )],
            evidence: Vec::new(),
        };
    }

    let Some(output) = &planned.output else {
        return VoxelConversionPreview {
            plan_id: planned.plan.plan_id.clone(),
            output_hash: String::new(),
            output_voxel_count: 0,
            output_bounds: None,
            sample_voxels: Vec::new(),
            diagnostics: planned.plan.diagnostics.clone(),
            evidence: planned.plan.evidence.clone(),
        };
    };

    VoxelConversionPreview {
        plan_id: planned.plan.plan_id.clone(),
        output_hash: output.output_hash.clone(),
        output_voxel_count: output.voxels.len() as u64,
        output_bounds: output.bounds,
        sample_voxels: output
            .voxels
            .iter()
            .map(|voxel| VoxelConversionPreviewVoxel {
                coord: voxel.coord,
                material: voxel
                    .value
                    .material()
                    .expect("converted voxels are solid")
                    .raw(),
            })
            .collect(),
        diagnostics: planned.plan.diagnostics.clone(),
        evidence: vec![evidence_ref(
            VoxelConversionEvidenceKind::Preview,
            format!("asha://voxel-conversion/preview/{}", planned.plan.plan_id),
            &output.output_hash,
        )],
    }
}

pub fn apply_conversion(
    request: &VoxelConversionApplyRequest,
    planned: &PlannedConversion,
) -> VoxelConversionReceipt {
    let preview = preview_conversion(
        &VoxelConversionPreviewRequest {
            plan_id: request.plan_id.clone(),
            expected_plan_hash: request.expected_plan_hash.clone(),
        },
        planned,
    );

    if !preview.diagnostics.is_empty() {
        return rejected_receipt(request.plan_id.clone(), preview.diagnostics);
    }
    if let Some(expected_preview_hash) = &request.expected_preview_hash {
        if expected_preview_hash != &preview.output_hash {
            return rejected_receipt(
                request.plan_id.clone(),
                vec![diagnostic(
                    VoxelConversionDiagnosticCode::ConversionReplayMismatch,
                    DiagnosticSeverity::Error,
                    "preview",
                    "apply request expected a different preview output hash",
                )],
            );
        }
    }

    VoxelConversionReceipt {
        plan_id: request.plan_id.clone(),
        applied: true,
        output_hash: Some(preview.output_hash.clone()),
        output_voxel_count: preview.output_voxel_count,
        output_bounds: preview.output_bounds,
        diagnostics: Vec::new(),
        evidence: vec![evidence_ref(
            VoxelConversionEvidenceKind::ApplyReceipt,
            format!("asha://voxel-conversion/apply/{}", request.plan_id),
            &stable_hash(&["apply", &request.plan_id, &preview.output_hash]),
        )],
    }
}

pub fn plan_hash(plan: &VoxelConversionPlan) -> String {
    plan_hash_from_parts(
        &plan.plan_id,
        &plan.expected_source_hash,
        &plan.settings_hash,
        &plan.authority_version,
    )
}

fn plan_hash_from_parts(
    plan_id: &str,
    expected_source_hash: &str,
    settings_hash: &str,
    authority_version: &str,
) -> String {
    stable_hash(&[
        "plan-hash",
        plan_id,
        expected_source_hash,
        settings_hash,
        authority_version,
    ])
}

fn validate_source_ref(
    reference: &VoxelConversionSourceRef,
    source: &StaticMeshSource,
    diagnostics: &mut Vec<VoxelConversionDiagnostic>,
) {
    if reference.asset_id != source.asset_id
        || reference.asset_kind != source.asset_kind
        || reference.asset_version != source.asset_version
        || reference.mesh_primitive != source.mesh_primitive
        || reference.asset_kind != "mesh"
    {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::UnsupportedSourceAsset,
            DiagnosticSeverity::Error,
            &reference.asset_id,
            "source reference does not match a supported loaded static mesh asset",
        ));
    }
    if reference.source_hash != source.source_hash {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::SourceHashMismatch,
            DiagnosticSeverity::Error,
            &reference.asset_id,
            "source hash does not match the loaded static mesh authority snapshot",
        ));
    }
    if source.triangles.is_empty() || source.positions.is_empty() {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::UnsupportedSourceAsset,
            DiagnosticSeverity::Error,
            &reference.asset_id,
            "static mesh source must contain positions and triangles",
        ));
    }
    for triangle in &source.triangles {
        if triangle
            .indices
            .iter()
            .any(|index| *index as usize >= source.positions.len())
        {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::UnsupportedSourceAsset,
                DiagnosticSeverity::Error,
                &reference.asset_id,
                "triangle index is outside the static mesh position buffer",
            ));
            break;
        }
    }
}

fn validate_settings(
    request: &VoxelConversionPlanRequest,
    source: &StaticMeshSource,
    diagnostics: &mut Vec<VoxelConversionDiagnostic>,
) {
    if request.settings.resolution.contains(&0)
        || !request.settings.voxel_size.is_finite()
        || request.settings.voxel_size <= 0.0
        || request
            .settings
            .transform
            .iter()
            .any(|value| !value.is_finite())
    {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::UnsupportedSourceAsset,
            DiagnosticSeverity::Error,
            "settings",
            "conversion settings contain non-finite values or zero resolution",
        ));
    }
    validate_resource_guardrails(request, source, DEFAULT_RESOURCE_LIMITS, diagnostics);
    if let Err(message) = validate_material_map(request, source) {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::InvalidMaterialMap,
            DiagnosticSeverity::Error,
            "materialMap",
            message,
        ));
    }
    validate_texture_sampling(request, source, diagnostics);
    if request.settings.mode == VoxelConversionMode::Solid {
        if let Err(message) = validate_solid_topology(source) {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::NonManifoldOrAmbiguousSolid,
                DiagnosticSeverity::Error,
                &request.source.asset_id,
                message,
            ));
        }
    }
}

fn validate_resource_guardrails(
    request: &VoxelConversionPlanRequest,
    source: &StaticMeshSource,
    limits: ConversionResourceLimits,
    diagnostics: &mut Vec<VoxelConversionDiagnostic>,
) {
    if source.positions.len() > limits.max_source_vertices {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::OutputLimitExceeded,
            DiagnosticSeverity::Error,
            "source.positions",
            format!(
                "source vertex count {} exceeds native conversion limit {}",
                source.positions.len(),
                limits.max_source_vertices
            ),
        ));
    }
    if source.triangles.len() > limits.max_source_triangles {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::OutputLimitExceeded,
            DiagnosticSeverity::Error,
            "source.triangles",
            format!(
                "source triangle count {} exceeds native conversion limit {}",
                source.triangles.len(),
                limits.max_source_triangles
            ),
        ));
    }
    if request
        .settings
        .resolution
        .iter()
        .any(|axis| *axis > limits.max_resolution_axis)
    {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::OutputLimitExceeded,
            DiagnosticSeverity::Error,
            "resolution",
            format!(
                "conversion resolution axis exceeds native conversion limit {}",
                limits.max_resolution_axis
            ),
        ));
    }
    match resolution_cells(request.settings.resolution) {
        Some(cells) if cells <= limits.max_resolution_cells => {}
        Some(cells) => diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::OutputLimitExceeded,
            DiagnosticSeverity::Error,
            "resolution",
            format!(
                "conversion resolution cell budget {cells} exceeds native conversion limit {}",
                limits.max_resolution_cells
            ),
        )),
        None => diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::OutputLimitExceeded,
            DiagnosticSeverity::Error,
            "resolution",
            "conversion resolution cell budget overflows u64",
        )),
    }
    if request.settings.max_output_voxels == 0
        || request.settings.max_output_voxels > limits.max_requested_output_voxels
    {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::OutputLimitExceeded,
            DiagnosticSeverity::Error,
            "maxOutputVoxels",
            format!(
                "requested max output voxels must be in 1..={}",
                limits.max_requested_output_voxels
            ),
        ));
    }
}

fn resolution_cells(resolution: [u32; 3]) -> Option<u64> {
    resolution
        .into_iter()
        .try_fold(1u64, |acc, axis| acc.checked_mul(u64::from(axis)))
}

fn validate_material_map(
    request: &VoxelConversionPlanRequest,
    source: &StaticMeshSource,
) -> Result<(), &'static str> {
    let mut map_slots = BTreeSet::new();
    for entry in &request.settings.material_map.entries {
        if !map_slots.insert(entry.source_material_slot) {
            return Err("duplicate source material slot in material map");
        }
    }
    if request
        .settings
        .material_map
        .default_voxel_material
        .is_none()
    {
        let texture_slots = texture_binding_slots(request);
        for slot in source_material_slots(source) {
            if !map_slots.contains(&slot) && !texture_slots.contains(&slot) {
                return Err("source material slot is unmapped and no default material is set");
            }
        }
    }
    Ok(())
}

fn validate_texture_sampling(
    request: &VoxelConversionPlanRequest,
    source: &StaticMeshSource,
    diagnostics: &mut Vec<VoxelConversionDiagnostic>,
) {
    let mut texture_keys = BTreeSet::new();
    for texture_asset in &request.settings.material_map.texture_assets {
        validate_texture_sample_asset(texture_asset, diagnostics);
        if !texture_keys.insert(texture_key(&texture_asset.texture)) {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::InvalidTextureMaterialRule,
                DiagnosticSeverity::Error,
                "materialMap.textureAssets",
                "duplicate texture sample asset identity",
            ));
        }
    }

    let source_slots = source_material_slots(source);
    let mut binding_slots = BTreeSet::new();
    for binding in &request.settings.material_map.texture_bindings {
        if !binding_slots.insert(binding.source_material_slot) {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::InvalidTextureMaterialRule,
                DiagnosticSeverity::Error,
                format!(
                    "materialMap.textureBindings.{}",
                    binding.source_material_slot
                ),
                "duplicate texture binding for source material slot",
            ));
        }
        if !source_slots.contains(&binding.source_material_slot) {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::InvalidTextureMaterialRule,
                DiagnosticSeverity::Error,
                format!(
                    "materialMap.textureBindings.{}",
                    binding.source_material_slot
                ),
                "texture binding references a source material slot not present in the mesh",
            ));
        }
        if binding.uv_attribute.attribute_name.is_empty()
            || binding.uv_attribute.source_hash.is_empty()
        {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::MissingUvAttribute,
                DiagnosticSeverity::Error,
                format!(
                    "materialMap.textureBindings.{}.uvAttribute",
                    binding.source_material_slot
                ),
                "texture binding requires an authority-visible UV attribute name and source hash",
            ));
        }
        if binding.sample_uv.iter().any(|value| !value.is_finite()) {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::InvalidTextureMaterialRule,
                DiagnosticSeverity::Error,
                format!(
                    "materialMap.textureBindings.{}.sampleUv",
                    binding.source_material_slot
                ),
                "texture binding sample UV must contain finite values",
            ));
        }
        if binding.sampling_policy != "nearest_texel" {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::UnsupportedSamplingPolicy,
                DiagnosticSeverity::Error,
                format!(
                    "materialMap.textureBindings.{}.samplingPolicy",
                    binding.source_material_slot
                ),
                "voxel conversion currently supports nearest_texel texture sampling only",
            ));
        }
        if binding.wrap_policy != "clamp_to_edge" {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::UnsupportedSamplingPolicy,
                DiagnosticSeverity::Error,
                format!(
                    "materialMap.textureBindings.{}.wrapPolicy",
                    binding.source_material_slot
                ),
                "voxel conversion currently supports clamp_to_edge texture wrapping only",
            ));
        }
        if binding.material_mode != "sample_palette_index" {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::InvalidTextureMaterialRule,
                DiagnosticSeverity::Error,
                format!(
                    "materialMap.textureBindings.{}.materialMode",
                    binding.source_material_slot
                ),
                "voxel conversion currently supports sample_palette_index material mapping only",
            ));
        }
        if binding.texture.color_space != "linear" && binding.texture.color_space != "srgb" {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::UnsupportedTextureFormat,
                DiagnosticSeverity::Error,
                format!(
                    "materialMap.textureBindings.{}.texture.colorSpace",
                    binding.source_material_slot
                ),
                "texture binding uses an unsupported color space",
            ));
        }
        if binding.texture.channel_layout != "palette_index_u16" {
            diagnostics.push(diagnostic(
                VoxelConversionDiagnosticCode::UnsupportedTextureFormat,
                DiagnosticSeverity::Error,
                format!(
                    "materialMap.textureBindings.{}.texture.channelLayout",
                    binding.source_material_slot
                ),
                "texture binding currently requires palette_index_u16 texel materials",
            ));
        }
        validate_texture_binding_source(
            request,
            binding.source_material_slot,
            &binding.texture,
            diagnostics,
        );
    }
}

fn validate_texture_sample_asset(
    texture_asset: &VoxelConversionTextureSampleAsset,
    diagnostics: &mut Vec<VoxelConversionDiagnostic>,
) {
    let texture = &texture_asset.texture;
    if texture.texture_asset_id.is_empty() || texture.content_hash.is_empty() {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::MissingTextureSource,
            DiagnosticSeverity::Error,
            "materialMap.textureAssets",
            "texture sample asset requires an asset id and content hash",
        ));
    }
    if texture.width == 0 || texture.height == 0 {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::UnsupportedTextureFormat,
            DiagnosticSeverity::Error,
            "materialMap.textureAssets",
            "texture sample asset dimensions must be non-zero",
        ));
    }
    if texture.color_space != "linear" && texture.color_space != "srgb" {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::UnsupportedTextureFormat,
            DiagnosticSeverity::Error,
            "materialMap.textureAssets",
            "texture sample asset uses an unsupported color space",
        ));
    }
    if texture.channel_layout != "palette_index_u16" {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::UnsupportedTextureFormat,
            DiagnosticSeverity::Error,
            "materialMap.textureAssets",
            "texture sample asset currently requires palette_index_u16 texel materials",
        ));
    }
    let expected_len = u64::from(texture.width) * u64::from(texture.height);
    if texture_asset.texel_materials.len() as u64 != expected_len {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::InvalidTextureMaterialRule,
            DiagnosticSeverity::Error,
            "materialMap.textureAssets.texelMaterials",
            "texture sample asset texel material count does not match width * height",
        ));
    }
}

fn validate_texture_binding_source(
    request: &VoxelConversionPlanRequest,
    source_slot: u32,
    texture: &VoxelConversionTextureSourceRef,
    diagnostics: &mut Vec<VoxelConversionDiagnostic>,
) {
    let matching_asset = request
        .settings
        .material_map
        .texture_assets
        .iter()
        .find(|asset| {
            asset.texture.texture_asset_id == texture.texture_asset_id
                && asset.texture.asset_version == texture.asset_version
        });
    match matching_asset {
        Some(asset) if asset.texture.content_hash == texture.content_hash => {}
        Some(_) => diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::TextureHashMismatch,
            DiagnosticSeverity::Error,
            format!("materialMap.textureBindings.{source_slot}.texture"),
            "texture binding content hash does not match the authority-visible texture snapshot",
        )),
        None => diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::MissingTextureSource,
            DiagnosticSeverity::Error,
            format!("materialMap.textureBindings.{source_slot}.texture"),
            "texture binding references no authority-visible texture snapshot",
        )),
    }
}

fn build_output(
    request: &VoxelConversionPlanRequest,
    source: &StaticMeshSource,
    diagnostics: &mut Vec<VoxelConversionDiagnostic>,
) -> Option<ConversionOutput> {
    let voxels = match request.settings.mode {
        VoxelConversionMode::Surface => surface_voxels(request, source),
        VoxelConversionMode::Solid => solid_voxels(request, source),
    };
    if voxels.len() as u64 > request.settings.max_output_voxels {
        diagnostics.push(diagnostic(
            VoxelConversionDiagnosticCode::OutputLimitExceeded,
            DiagnosticSeverity::Error,
            "maxOutputVoxels",
            "conversion output exceeds the requested maximum voxel count",
        ));
        return None;
    }
    let bounds = bounds_for(&voxels);
    let output_hash = output_hash(&voxels);
    Some(ConversionOutput {
        voxels,
        bounds,
        output_hash,
    })
}

fn surface_voxels(
    request: &VoxelConversionPlanRequest,
    source: &StaticMeshSource,
) -> Vec<ConvertedVoxel> {
    let mapper = CoordMapper::new(request, source);
    let material_map = material_resolver(request);
    let mut voxels = BTreeMap::new();
    for triangle in &source.triangles {
        let material = material_for(&material_map, request, triangle.source_material_slot);
        for index in triangle.indices {
            let coord = mapper.map(source.positions[index as usize]);
            voxels.insert(coord_key(coord), VoxelValue::solid_raw(material));
        }
    }
    voxels
        .into_iter()
        .map(|((x, y, z), value)| ConvertedVoxel {
            coord: protocol_voxel_conversion::VoxelConversionCoord { x, y, z },
            value,
        })
        .collect()
}

fn solid_voxels(
    request: &VoxelConversionPlanRequest,
    source: &StaticMeshSource,
) -> Vec<ConvertedVoxel> {
    let mapper = CoordMapper::new(request, source);
    let material_map = material_resolver(request);
    let material = source
        .triangles
        .first()
        .map(|triangle| material_for(&material_map, request, triangle.source_material_slot))
        .unwrap_or_else(|| {
            request
                .settings
                .material_map
                .default_voxel_material
                .unwrap_or(1)
        });

    let mapped_positions: Vec<_> = source
        .positions
        .iter()
        .map(|position| mapper.map(*position))
        .collect();
    let Some(bounds) = bounds_for_coords(&mapped_positions) else {
        return Vec::new();
    };
    let volume = ((bounds.max.x - bounds.min.x + 1) as usize)
        * ((bounds.max.y - bounds.min.y + 1) as usize)
        * ((bounds.max.z - bounds.min.z + 1) as usize);
    let mut voxels = Vec::with_capacity(volume);
    for z in bounds.min.z..=bounds.max.z {
        for y in bounds.min.y..=bounds.max.y {
            for x in bounds.min.x..=bounds.max.x {
                voxels.push(ConvertedVoxel {
                    coord: protocol_voxel_conversion::VoxelConversionCoord { x, y, z },
                    value: VoxelValue::solid_raw(material),
                });
            }
        }
    }
    voxels
}

fn source_material_slots(source: &StaticMeshSource) -> BTreeSet<u32> {
    source
        .triangles
        .iter()
        .map(|triangle| triangle.source_material_slot)
        .collect()
}

fn texture_binding_slots(request: &VoxelConversionPlanRequest) -> BTreeSet<u32> {
    request
        .settings
        .material_map
        .texture_bindings
        .iter()
        .map(|binding| binding.source_material_slot)
        .collect()
}

fn material_resolver(request: &VoxelConversionPlanRequest) -> BTreeMap<u32, u16> {
    let mut material_map: BTreeMap<u32, u16> = request
        .settings
        .material_map
        .entries
        .iter()
        .map(|entry| (entry.source_material_slot, entry.voxel_material))
        .collect();
    for binding in &request.settings.material_map.texture_bindings {
        if let Some(sampled) = sample_texture_material(request, binding) {
            material_map.insert(binding.source_material_slot, sampled);
        }
    }
    material_map
}

fn sample_texture_material(
    request: &VoxelConversionPlanRequest,
    binding: &protocol_voxel_conversion::VoxelConversionTextureBinding,
) -> Option<u16> {
    let texture = request
        .settings
        .material_map
        .texture_assets
        .iter()
        .find(|asset| asset.texture == binding.texture)?;
    let x = nearest_texel_axis(binding.sample_uv[0], texture.texture.width);
    let y = nearest_texel_axis(binding.sample_uv[1], texture.texture.height);
    let index = y * texture.texture.width as usize + x;
    texture.texel_materials.get(index).copied()
}

fn nearest_texel_axis(uv: f32, size: u32) -> usize {
    let max_index = size.saturating_sub(1) as f32;
    uv.clamp(0.0, 1.0).mul_add(max_index, 0.0).round() as usize
}

fn material_for(
    material_map: &BTreeMap<u32, u16>,
    request: &VoxelConversionPlanRequest,
    source_slot: u32,
) -> u16 {
    material_map
        .get(&source_slot)
        .copied()
        .or(request.settings.material_map.default_voxel_material)
        .expect("material map was validated before conversion")
}

fn texture_key(texture: &VoxelConversionTextureSourceRef) -> (String, u64) {
    (texture.texture_asset_id.clone(), texture.asset_version)
}

fn validate_solid_topology(source: &StaticMeshSource) -> Result<(), &'static str> {
    let mut faces = BTreeSet::<[u32; 3]>::new();
    let mut edges: BTreeMap<(u32, u32), Vec<(u32, u32)>> = BTreeMap::new();
    for triangle in &source.triangles {
        let [a, b, c] = triangle.indices;
        if a == b || b == c || c == a {
            return Err("solid conversion requires non-degenerate triangle indices");
        }
        if triangle_area_squared(source, triangle) <= f32::EPSILON {
            return Err("solid conversion requires non-degenerate triangle area");
        }
        let mut face = [a, b, c];
        face.sort_unstable();
        if !faces.insert(face) {
            return Err("solid conversion requires unique triangle faces");
        }
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let edge = if u <= v { (u, v) } else { (v, u) };
            edges.entry(edge).or_default().push((u, v));
        }
    }
    if edges.is_empty() {
        return Err("solid conversion requires closed manifold triangle edges");
    }
    if edges.values().any(|uses| uses.len() != 2) {
        return Err("solid conversion requires each undirected mesh edge to be used exactly twice");
    }
    if edges.values().any(|uses| uses[0] == uses[1]) {
        return Err("solid conversion requires paired edge uses to have opposite winding");
    }
    Ok(())
}

fn triangle_area_squared(source: &StaticMeshSource, triangle: &MeshTriangle) -> f32 {
    let [a, b, c] = triangle.indices;
    let a = source.positions[a as usize];
    let b = source.positions[b as usize];
    let c = source.positions[c as usize];
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]
}

fn bounds_for(voxels: &[ConvertedVoxel]) -> Option<VoxelConversionBounds> {
    let coords = voxels.iter().map(|voxel| voxel.coord).collect::<Vec<_>>();
    bounds_for_coords(&coords)
}

fn bounds_for_coords(
    coords: &[protocol_voxel_conversion::VoxelConversionCoord],
) -> Option<VoxelConversionBounds> {
    let first = *coords.first()?;
    let mut min = first;
    let mut max = first;
    for coord in coords.iter().skip(1) {
        min.x = min.x.min(coord.x);
        min.y = min.y.min(coord.y);
        min.z = min.z.min(coord.z);
        max.x = max.x.max(coord.x);
        max.y = max.y.max(coord.y);
        max.z = max.z.max(coord.z);
    }
    Some(VoxelConversionBounds { min, max })
}

fn output_hash(voxels: &[ConvertedVoxel]) -> String {
    let mut parts = Vec::with_capacity(voxels.len() * 4 + 1);
    parts.push("output".to_string());
    for voxel in voxels {
        parts.push(voxel.coord.x.to_string());
        parts.push(voxel.coord.y.to_string());
        parts.push(voxel.coord.z.to_string());
        parts.push(voxel.value.to_encoded().to_string());
    }
    stable_hash(&parts.iter().map(String::as_str).collect::<Vec<_>>())
}

fn settings_fingerprint(request: &VoxelConversionPlanRequest) -> String {
    let mut parts = vec![
        request.settings.mode.as_str().to_string(),
        request.settings.fit_policy.as_str().to_string(),
        request.settings.origin_policy.as_str().to_string(),
        format!("{:?}", request.settings.resolution),
        request.settings.voxel_size.to_bits().to_string(),
        request.settings.max_output_voxels.to_string(),
        request.target.grid.to_string(),
        format!(
            "{},{},{}",
            request.target.origin.x, request.target.origin.y, request.target.origin.z
        ),
    ];
    for value in request.settings.transform {
        parts.push(value.to_bits().to_string());
    }
    for entry in &request.settings.material_map.entries {
        parts.push(format!(
            "{}:{}",
            entry.source_material_slot, entry.voxel_material
        ));
    }
    for texture_asset in &request.settings.material_map.texture_assets {
        parts.push(format!(
            "texture:{}:{}:{}:{}:{}:{}:{}",
            texture_asset.texture.texture_asset_id,
            texture_asset.texture.asset_version,
            texture_asset.texture.content_hash,
            texture_asset.texture.width,
            texture_asset.texture.height,
            texture_asset.texture.color_space,
            texture_asset.texture.channel_layout
        ));
        parts.push(format!("{:?}", texture_asset.texel_materials));
    }
    for binding in &request.settings.material_map.texture_bindings {
        parts.push(format!(
            "texture-binding:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            binding.source_material_slot,
            binding.texture.texture_asset_id,
            binding.texture.asset_version,
            binding.texture.content_hash,
            binding.uv_attribute.attribute_name,
            binding.uv_attribute.source_hash,
            binding.sample_uv[0].to_bits(),
            binding.sample_uv[1].to_bits(),
            binding.sampling_policy,
            binding.material_mode
        ));
        parts.push(format!("wrap:{}", binding.wrap_policy));
    }
    if let Some(default) = request.settings.material_map.default_voxel_material {
        parts.push(format!("default:{default}"));
    }
    stable_hash(&parts.iter().map(String::as_str).collect::<Vec<_>>())
}

fn diagnostic(
    code: VoxelConversionDiagnosticCode,
    severity: DiagnosticSeverity,
    reference: impl Into<String>,
    message: impl Into<String>,
) -> VoxelConversionDiagnostic {
    VoxelConversionDiagnostic {
        code,
        severity,
        reference: reference.into(),
        message: message.into(),
    }
}

fn evidence_ref(
    kind: VoxelConversionEvidenceKind,
    uri: String,
    content_hash: &str,
) -> VoxelConversionEvidenceRef {
    VoxelConversionEvidenceRef {
        kind,
        uri,
        content_hash: content_hash.to_string(),
    }
}

fn rejected_receipt(
    plan_id: String,
    diagnostics: Vec<VoxelConversionDiagnostic>,
) -> VoxelConversionReceipt {
    VoxelConversionReceipt {
        plan_id,
        applied: false,
        output_hash: None,
        output_voxel_count: 0,
        output_bounds: None,
        diagnostics,
        evidence: Vec::new(),
    }
}

fn coord_key(coord: protocol_voxel_conversion::VoxelConversionCoord) -> (i64, i64, i64) {
    (coord.x, coord.y, coord.z)
}

struct CoordMapper {
    min: [f32; 3],
    target: VoxelConversionTargetRef,
    resolution: [u32; 3],
    voxel_size: f32,
    scale: [f32; 3],
    offset_voxels: [f32; 3],
    origin_policy: VoxelConversionOriginPolicy,
    transform: [f32; 16],
}

impl CoordMapper {
    fn new(request: &VoxelConversionPlanRequest, source: &StaticMeshSource) -> Self {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for position in &source.positions {
            let transformed = transform_position(request.settings.transform, *position);
            for axis in 0..3 {
                min[axis] = min[axis].min(transformed[axis]);
                max[axis] = max[axis].max(transformed[axis]);
            }
        }
        let span = [
            (max[0] - min[0]).max(0.0),
            (max[1] - min[1]).max(0.0),
            (max[2] - min[2]).max(0.0),
        ];
        let target_span =
            target_span_world(request.settings.resolution, request.settings.voxel_size);
        let scale = fit_scale(request.settings.fit_policy, span, target_span);
        let offset_voxels = origin_offset_voxels(
            request.settings.origin_policy,
            span,
            target_span,
            scale,
            request.settings.voxel_size,
        );
        Self {
            min,
            target: request.target.clone(),
            resolution: request.settings.resolution,
            voxel_size: request.settings.voxel_size,
            scale,
            offset_voxels,
            origin_policy: request.settings.origin_policy,
            transform: request.settings.transform,
        }
    }

    fn map(&self, position: [f32; 3]) -> protocol_voxel_conversion::VoxelConversionCoord {
        let transformed = transform_position(self.transform, position);
        let mut out = [0i64; 3];
        for axis in 0..3 {
            let anchored = match self.origin_policy {
                VoxelConversionOriginPolicy::SourceOrigin => transformed[axis] * self.scale[axis],
                VoxelConversionOriginPolicy::TargetMin | VoxelConversionOriginPolicy::Centered => {
                    (transformed[axis] - self.min[axis]) * self.scale[axis]
                }
            };
            let max_index = self.resolution[axis].saturating_sub(1) as f32;
            let local = (anchored / self.voxel_size) + self.offset_voxels[axis];
            out[axis] = local.round().clamp(0.0, max_index) as i64;
        }
        protocol_voxel_conversion::VoxelConversionCoord {
            x: self.target.origin.x + out[0],
            y: self.target.origin.y + out[1],
            z: self.target.origin.z + out[2],
        }
    }
}

fn transform_position(transform: [f32; 16], position: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = position;
    [
        transform[0] * x + transform[4] * y + transform[8] * z + transform[12],
        transform[1] * x + transform[5] * y + transform[9] * z + transform[13],
        transform[2] * x + transform[6] * y + transform[10] * z + transform[14],
    ]
}

fn target_span_world(resolution: [u32; 3], voxel_size: f32) -> [f32; 3] {
    [
        resolution[0].saturating_sub(1) as f32 * voxel_size,
        resolution[1].saturating_sub(1) as f32 * voxel_size,
        resolution[2].saturating_sub(1) as f32 * voxel_size,
    ]
}

fn fit_scale(
    fit_policy: VoxelConversionFitPolicy,
    source_span: [f32; 3],
    target_span: [f32; 3],
) -> [f32; 3] {
    let axis_ratios = [
        axis_fit_ratio(source_span[0], target_span[0]),
        axis_fit_ratio(source_span[1], target_span[1]),
        axis_fit_ratio(source_span[2], target_span[2]),
    ];
    match fit_policy {
        VoxelConversionFitPolicy::Stretch => [
            axis_ratios[0].unwrap_or(1.0),
            axis_ratios[1].unwrap_or(1.0),
            axis_ratios[2].unwrap_or(1.0),
        ],
        VoxelConversionFitPolicy::Contain => {
            let scale = uniform_fit_scale(axis_ratios, UniformFit::Contain);
            [scale, scale, scale]
        }
        VoxelConversionFitPolicy::Cover => {
            let scale = uniform_fit_scale(axis_ratios, UniformFit::Cover);
            [scale, scale, scale]
        }
    }
}

fn axis_fit_ratio(source_span: f32, target_span: f32) -> Option<f32> {
    if source_span > f32::EPSILON {
        Some(target_span / source_span)
    } else {
        None
    }
}

enum UniformFit {
    Contain,
    Cover,
}

fn uniform_fit_scale(axis_ratios: [Option<f32>; 3], fit: UniformFit) -> f32 {
    let mut ratios = axis_ratios.into_iter().flatten();
    let Some(first) = ratios.next() else {
        return 1.0;
    };
    ratios.fold(first, |acc, ratio| match fit {
        UniformFit::Contain => acc.min(ratio),
        UniformFit::Cover => acc.max(ratio),
    })
}

fn origin_offset_voxels(
    origin_policy: VoxelConversionOriginPolicy,
    source_span: [f32; 3],
    target_span: [f32; 3],
    scale: [f32; 3],
    voxel_size: f32,
) -> [f32; 3] {
    match origin_policy {
        VoxelConversionOriginPolicy::SourceOrigin | VoxelConversionOriginPolicy::TargetMin => {
            [0.0, 0.0, 0.0]
        }
        VoxelConversionOriginPolicy::Centered => [
            centered_axis_offset(source_span[0], target_span[0], scale[0], voxel_size),
            centered_axis_offset(source_span[1], target_span[1], scale[1], voxel_size),
            centered_axis_offset(source_span[2], target_span[2], scale[2], voxel_size),
        ],
    }
}

fn centered_axis_offset(source_span: f32, target_span: f32, scale: f32, voxel_size: f32) -> f32 {
    ((target_span - source_span * scale) / 2.0).max(0.0) / voxel_size
}

fn stable_hash(parts: &[&str]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_voxel_conversion::{
        VoxelConversionCoord, VoxelConversionFitPolicy, VoxelConversionMaterialMap,
        VoxelConversionMaterialMapEntry, VoxelConversionOriginPolicy, VoxelConversionSettings,
        VoxelConversionTextureBinding, VoxelConversionTextureSampleAsset,
        VoxelConversionTextureSourceRef, VoxelConversionUvAttributeRef,
    };
    use serde_json::json;

    fn identity() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    fn translation(x: f32, y: f32, z: f32) -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, y, z, 1.0,
        ]
    }

    fn quad_source() -> StaticMeshSource {
        StaticMeshSource {
            asset_id: "mesh/quad".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 1,
            source_hash: "sha256:quad".to_string(),
            mesh_primitive: None,
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            triangles: vec![
                MeshTriangle {
                    indices: [0, 1, 2],
                    source_material_slot: 0,
                },
                MeshTriangle {
                    indices: [0, 2, 3],
                    source_material_slot: 1,
                },
            ],
        }
    }

    fn rectangular_quad_source() -> StaticMeshSource {
        let mut source = quad_source();
        source.asset_id = "mesh/rect".to_string();
        source.source_hash = "sha256:rect".to_string();
        source.positions = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        source
    }

    fn shallow_box_source() -> StaticMeshSource {
        let mut source = cube_source();
        source.asset_id = "mesh/shallow-box".to_string();
        source.source_hash = "sha256:shallow-box".to_string();
        for position in &mut source.positions {
            position[2] *= 0.5;
        }
        source
    }

    fn tessellated_plane_source(size: u32) -> StaticMeshSource {
        let mut positions = Vec::new();
        for y in 0..=size {
            for x in 0..=size {
                positions.push([x as f32, y as f32, 0.0]);
            }
        }

        let row = size + 1;
        let mut triangles = Vec::new();
        for y in 0..size {
            for x in 0..size {
                let a = y * row + x;
                let b = a + 1;
                let c = a + row;
                let d = c + 1;
                let source_material_slot = (x + y) % 2;
                triangles.push(MeshTriangle {
                    indices: [a, b, d],
                    source_material_slot,
                });
                triangles.push(MeshTriangle {
                    indices: [a, d, c],
                    source_material_slot,
                });
            }
        }

        StaticMeshSource {
            asset_id: format!("mesh/tessellated-plane-{size}"),
            asset_kind: "mesh".to_string(),
            asset_version: 1,
            source_hash: format!("sha256:tessellated-plane-{size}"),
            mesh_primitive: None,
            positions,
            triangles,
        }
    }

    fn cube_source() -> StaticMeshSource {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let faces = [
            ([0, 1, 2], [0, 2, 3]),
            ([4, 6, 5], [4, 7, 6]),
            ([0, 4, 5], [0, 5, 1]),
            ([1, 5, 6], [1, 6, 2]),
            ([2, 6, 7], [2, 7, 3]),
            ([3, 7, 4], [3, 4, 0]),
        ];
        let triangles = faces
            .into_iter()
            .flat_map(|(a, b)| {
                [
                    MeshTriangle {
                        indices: a,
                        source_material_slot: 0,
                    },
                    MeshTriangle {
                        indices: b,
                        source_material_slot: 0,
                    },
                ]
            })
            .collect();
        StaticMeshSource {
            asset_id: "mesh/cube".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 1,
            source_hash: "sha256:cube".to_string(),
            mesh_primitive: None,
            positions,
            triangles,
        }
    }

    fn request_for(
        source: &StaticMeshSource,
        mode: VoxelConversionMode,
        resolution: [u32; 3],
        max_output_voxels: u64,
    ) -> VoxelConversionPlanRequest {
        VoxelConversionPlanRequest {
            source: VoxelConversionSourceRef {
                asset_id: source.asset_id.clone(),
                asset_kind: source.asset_kind.clone(),
                asset_version: source.asset_version,
                source_hash: source.source_hash.clone(),
                mesh_primitive: source.mesh_primitive.clone(),
            },
            target: VoxelConversionTargetRef {
                grid: 7,
                volume_asset_id: Some("voxel/generated".to_string()),
                origin: VoxelConversionCoord { x: 0, y: 0, z: 0 },
            },
            settings: VoxelConversionSettings {
                mode,
                fit_policy: VoxelConversionFitPolicy::Contain,
                origin_policy: VoxelConversionOriginPolicy::TargetMin,
                resolution,
                voxel_size: 1.0,
                max_output_voxels,
                transform: identity(),
                material_map: VoxelConversionMaterialMap {
                    entries: vec![
                        VoxelConversionMaterialMapEntry {
                            source_material_slot: 0,
                            source_material_id: Some("mat/a".to_string()),
                            voxel_material: 3,
                        },
                        VoxelConversionMaterialMapEntry {
                            source_material_slot: 1,
                            source_material_id: Some("mat/b".to_string()),
                            voxel_material: 5,
                        },
                    ],
                    texture_assets: Vec::new(),
                    texture_bindings: Vec::new(),
                    default_voxel_material: None,
                },
            },
        }
    }

    fn texture_source_ref(content_hash: &str) -> VoxelConversionTextureSourceRef {
        VoxelConversionTextureSourceRef {
            texture_asset_id: "texture/checker".to_string(),
            asset_version: 1,
            content_hash: content_hash.to_string(),
            width: 2,
            height: 1,
            color_space: "linear".to_string(),
            channel_layout: "palette_index_u16".to_string(),
        }
    }

    fn texture_sample_asset() -> VoxelConversionTextureSampleAsset {
        VoxelConversionTextureSampleAsset {
            texture: texture_source_ref("sha256:texture-checker"),
            texel_materials: vec![9, 11],
        }
    }

    fn texture_binding(
        source_material_slot: u32,
        sample_uv: [f32; 2],
    ) -> VoxelConversionTextureBinding {
        VoxelConversionTextureBinding {
            source_material_slot,
            texture: texture_source_ref("sha256:texture-checker"),
            uv_attribute: VoxelConversionUvAttributeRef {
                attribute_name: "TEXCOORD_0".to_string(),
                source_hash: "sha256:quad-uv0".to_string(),
            },
            sample_uv,
            sampling_policy: "nearest_texel".to_string(),
            wrap_policy: "clamp_to_edge".to_string(),
            material_mode: "sample_palette_index".to_string(),
        }
    }

    #[test]
    fn synthetic_quad_surface_plans_and_previews_two_material_slots() {
        let source = quad_source();
        let request = request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16);
        let planned = plan_conversion(&request, &source);
        assert!(planned.plan.diagnostics.is_empty());
        assert_eq!(planned.plan.estimated_output_voxels, 4);
        assert_eq!(planned.plan.estimated_bounds.unwrap().max.x, 3);
        assert_eq!(planned.plan.plan_hash, plan_hash(&planned.plan));

        let preview = preview_conversion(
            &VoxelConversionPreviewRequest {
                plan_id: planned.plan.plan_id.clone(),
                expected_plan_hash: planned.plan.plan_hash.clone(),
            },
            &planned,
        );
        assert_eq!(preview.output_voxel_count, 4);
        assert!(preview
            .sample_voxels
            .iter()
            .any(|voxel| voxel.material == 3));
        assert!(preview
            .sample_voxels
            .iter()
            .any(|voxel| voxel.material == 5));
    }

    #[test]
    fn synthetic_cube_solid_fills_resolution_volume() {
        let source = cube_source();
        let request = request_for(&source, VoxelConversionMode::Solid, [2, 2, 2], 8);
        let planned = plan_conversion(&request, &source);
        assert!(planned.plan.diagnostics.is_empty());
        assert_eq!(planned.plan.estimated_output_voxels, 8);
        assert_eq!(
            planned.output.as_ref().unwrap().voxels[0].value,
            VoxelValue::solid_raw(3)
        );
    }

    #[test]
    fn larger_tessellated_surface_fixture_has_stable_summary() {
        let source = tessellated_plane_source(4);
        let request = request_for(&source, VoxelConversionMode::Surface, [5, 5, 1], 32);

        let planned = plan_conversion(&request, &source);
        let preview = preview_conversion(
            &VoxelConversionPreviewRequest {
                plan_id: planned.plan.plan_id.clone(),
                expected_plan_hash: planned.plan.plan_hash.clone(),
            },
            &planned,
        );

        assert!(planned.plan.diagnostics.is_empty());
        assert_eq!(source.positions.len(), 25);
        assert_eq!(source.triangles.len(), 32);
        assert_eq!(planned.plan.estimated_output_voxels, 25);
        assert_eq!(bounds_label(planned.plan.estimated_bounds), "0,0,0..4,4,0");
        assert_eq!(preview.output_voxel_count, 25);
        assert_eq!(preview.output_hash, "fnv1a64:17b13dfeb6844321");
        assert_eq!(material_label(&preview.sample_voxels), "3,5");
    }

    #[test]
    fn surface_conversion_applies_transform_with_source_origin_anchor() {
        let source = quad_source();
        let mut request = request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16);
        request.settings.origin_policy = VoxelConversionOriginPolicy::SourceOrigin;
        request.settings.transform = translation(0.25, 0.0, 0.0);

        let planned = plan_conversion(&request, &source);

        assert!(planned.plan.diagnostics.is_empty());
        assert_eq!(bounds_label(planned.plan.estimated_bounds), "1,0,0..3,3,0");
        assert_ne!(
            planned.plan.settings_hash,
            plan_conversion(
                &request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16),
                &source
            )
            .plan
            .settings_hash
        );
    }

    #[test]
    fn surface_conversion_fit_policy_changes_mapped_coordinates() {
        let source = rectangular_quad_source();
        let contain = request_for(&source, VoxelConversionMode::Surface, [5, 5, 1], 16);
        let mut stretch = contain.clone();
        stretch.settings.fit_policy = VoxelConversionFitPolicy::Stretch;

        let contain_plan = plan_conversion(&contain, &source);
        let stretch_plan = plan_conversion(&stretch, &source);

        assert!(contain_plan.plan.diagnostics.is_empty());
        assert!(stretch_plan.plan.diagnostics.is_empty());
        assert_eq!(
            bounds_label(contain_plan.plan.estimated_bounds),
            "0,0,0..4,2,0"
        );
        assert_eq!(
            bounds_label(stretch_plan.plan.estimated_bounds),
            "0,0,0..4,4,0"
        );
        assert_ne!(
            contain_plan.plan.settings_hash,
            stretch_plan.plan.settings_hash
        );
        assert_ne!(
            contain_plan.output.as_ref().unwrap().output_hash,
            stretch_plan.output.as_ref().unwrap().output_hash
        );
    }

    #[test]
    fn centered_origin_places_contained_output_inside_target_bounds() {
        let source = rectangular_quad_source();
        let mut request = request_for(&source, VoxelConversionMode::Surface, [5, 5, 1], 16);
        request.settings.origin_policy = VoxelConversionOriginPolicy::Centered;

        let planned = plan_conversion(&request, &source);

        assert!(planned.plan.diagnostics.is_empty());
        assert_eq!(bounds_label(planned.plan.estimated_bounds), "0,1,0..4,3,0");
    }

    #[test]
    fn solid_conversion_fills_mapped_source_bounds_not_entire_resolution() {
        let source = shallow_box_source();
        let request = request_for(&source, VoxelConversionMode::Solid, [4, 4, 4], 64);

        let planned = plan_conversion(&request, &source);

        assert!(planned.plan.diagnostics.is_empty());
        assert_eq!(planned.plan.estimated_output_voxels, 48);
        assert_eq!(bounds_label(planned.plan.estimated_bounds), "0,0,0..3,3,2");
    }

    #[test]
    fn invalid_material_map_fails_closed_without_output() {
        let source = quad_source();
        let mut request = request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16);
        request.settings.material_map.entries.pop();
        let planned = plan_conversion(&request, &source);
        assert!(planned.output.is_none());
        assert_eq!(
            planned.plan.diagnostics[0].code,
            VoxelConversionDiagnosticCode::InvalidMaterialMap
        );
    }

    #[test]
    fn material_map_default_is_rust_authority_fallback() {
        let source = quad_source();
        let mut request = request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16);
        request.settings.material_map.entries = vec![VoxelConversionMaterialMapEntry {
            source_material_slot: 0,
            source_material_id: Some("mat/a".to_string()),
            voxel_material: 3,
        }];
        request.settings.material_map.default_voxel_material = Some(7);

        let planned = plan_conversion(&request, &source);
        let preview = preview_conversion(
            &VoxelConversionPreviewRequest {
                plan_id: planned.plan.plan_id.clone(),
                expected_plan_hash: planned.plan.plan_hash.clone(),
            },
            &planned,
        );

        assert!(planned.plan.diagnostics.is_empty());
        assert_eq!(preview.output_voxel_count, 4);
        assert_eq!(material_label(&preview.sample_voxels), "3,7");
    }

    #[test]
    fn texture_sampled_materials_are_rust_authority_output() {
        let source = quad_source();
        let mut request = request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16);
        request.settings.material_map.entries.clear();
        request.settings.material_map.texture_assets = vec![texture_sample_asset()];
        request.settings.material_map.texture_bindings = vec![
            texture_binding(0, [0.0, 0.0]),
            texture_binding(1, [1.0, 0.0]),
        ];

        let planned = plan_conversion(&request, &source);
        let preview = preview_conversion(
            &VoxelConversionPreviewRequest {
                plan_id: planned.plan.plan_id.clone(),
                expected_plan_hash: planned.plan.plan_hash.clone(),
            },
            &planned,
        );

        assert!(planned.plan.diagnostics.is_empty());
        assert_eq!(preview.output_voxel_count, 4);
        assert_eq!(material_label(&preview.sample_voxels), "9,11");
        assert_ne!(
            planned.plan.settings_hash,
            plan_conversion(
                &request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16),
                &source
            )
            .plan
            .settings_hash
        );
    }

    #[test]
    fn texture_binding_missing_snapshot_fails_closed() {
        let source = quad_source();
        let mut request = request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16);
        request.settings.material_map.entries.clear();
        request.settings.material_map.texture_bindings = vec![texture_binding(0, [0.0, 0.0])];

        let planned = plan_conversion(&request, &source);

        assert!(planned.output.is_none());
        assert!(planned.plan.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == VoxelConversionDiagnosticCode::MissingTextureSource
        }));
    }

    #[test]
    fn texture_binding_hash_mismatch_fails_closed() {
        let source = quad_source();
        let mut request = request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16);
        request.settings.material_map.entries.clear();
        request.settings.material_map.texture_assets = vec![texture_sample_asset()];
        let mut binding = texture_binding(0, [0.0, 0.0]);
        binding.texture.content_hash = "sha256:stale-texture".to_string();
        request.settings.material_map.texture_bindings = vec![binding];

        let planned = plan_conversion(&request, &source);

        assert!(planned.output.is_none());
        assert!(planned.plan.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == VoxelConversionDiagnosticCode::TextureHashMismatch
        }));
    }

    #[test]
    fn unsupported_texture_sampling_policy_fails_closed() {
        let source = quad_source();
        let mut request = request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16);
        request.settings.material_map.entries.clear();
        request.settings.material_map.texture_assets = vec![texture_sample_asset()];
        let mut binding = texture_binding(0, [0.0, 0.0]);
        binding.sampling_policy = "bilinear_level0".to_string();
        request.settings.material_map.texture_bindings = vec![binding];

        let planned = plan_conversion(&request, &source);

        assert!(planned.output.is_none());
        assert!(planned.plan.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == VoxelConversionDiagnosticCode::UnsupportedSamplingPolicy
        }));
    }

    #[test]
    fn unsupported_topology_rejects_solid_mode() {
        let source = quad_source();
        let request = request_for(&source, VoxelConversionMode::Solid, [2, 2, 2], 8);
        let planned = plan_conversion(&request, &source);
        assert!(planned.output.is_none());
        assert_eq!(
            planned.plan.diagnostics[0].code,
            VoxelConversionDiagnosticCode::NonManifoldOrAmbiguousSolid
        );
    }

    #[test]
    fn solid_topology_rejects_degenerate_duplicate_open_and_overused_edges() {
        let mut degenerate = cube_source();
        degenerate.triangles[0].indices = [0, 0, 1];
        assert_solid_topology_rejected(&degenerate);

        let mut duplicate_face = cube_source();
        duplicate_face.triangles.push(duplicate_face.triangles[0]);
        assert_solid_topology_rejected(&duplicate_face);

        let mut open_shell = cube_source();
        open_shell.triangles.pop();
        assert_solid_topology_rejected(&open_shell);

        let mut overused_edge = cube_source();
        overused_edge.triangles.push(MeshTriangle {
            indices: [0, 1, 4],
            source_material_slot: 0,
        });
        assert_solid_topology_rejected(&overused_edge);

        let mut flipped_winding = cube_source();
        flipped_winding.triangles[1].indices = [0, 3, 2];
        assert_solid_topology_rejected(&flipped_winding);
    }

    #[test]
    fn oversized_output_rejects_without_best_effort_output() {
        let source = cube_source();
        let request = request_for(&source, VoxelConversionMode::Solid, [2, 2, 2], 7);
        let planned = plan_conversion(&request, &source);
        assert!(planned.output.is_none());
        assert_eq!(
            planned.plan.diagnostics[0].code,
            VoxelConversionDiagnosticCode::OutputLimitExceeded
        );
    }

    #[test]
    fn larger_tessellated_surface_over_budget_rejects_without_output() {
        let source = tessellated_plane_source(4);
        let request = request_for(&source, VoxelConversionMode::Surface, [5, 5, 1], 24);

        let planned = plan_conversion(&request, &source);

        assert!(planned.output.is_none());
        assert_eq!(planned.plan.estimated_output_voxels, 0);
        assert_eq!(
            planned.plan.diagnostics[0].code,
            VoxelConversionDiagnosticCode::OutputLimitExceeded
        );
    }

    #[test]
    fn resolution_axis_guardrail_rejects_before_output_work() {
        let source = quad_source();
        let request = request_for(
            &source,
            VoxelConversionMode::Surface,
            [MAX_RESOLUTION_AXIS + 1, 4, 1],
            16,
        );

        let planned = plan_conversion(&request, &source);

        assert!(planned.output.is_none());
        assert_eq!(
            planned.plan.diagnostics[0].code,
            VoxelConversionDiagnosticCode::OutputLimitExceeded
        );
        assert_eq!(planned.plan.diagnostics[0].reference, "resolution");
    }

    #[test]
    fn resolution_cell_budget_guardrail_rejects_before_output_work() {
        let source = quad_source();
        let request = request_for(
            &source,
            VoxelConversionMode::Surface,
            [MAX_RESOLUTION_AXIS, MAX_RESOLUTION_AXIS, 33],
            16,
        );

        let planned = plan_conversion(&request, &source);

        assert!(planned.output.is_none());
        assert!(planned
            .plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code
                == VoxelConversionDiagnosticCode::OutputLimitExceeded
                && diagnostic.reference == "resolution"));
    }

    #[test]
    fn requested_output_guardrail_rejects_unbounded_requests() {
        let source = quad_source();
        let request = request_for(
            &source,
            VoxelConversionMode::Surface,
            [4, 4, 1],
            MAX_REQUESTED_OUTPUT_VOXELS + 1,
        );

        let planned = plan_conversion(&request, &source);

        assert!(planned.output.is_none());
        assert_eq!(
            planned.plan.diagnostics[0].code,
            VoxelConversionDiagnosticCode::OutputLimitExceeded
        );
        assert_eq!(planned.plan.diagnostics[0].reference, "maxOutputVoxels");
    }

    #[test]
    fn source_count_guardrails_are_typed_output_limit_diagnostics() {
        let source = quad_source();
        let request = request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16);
        let mut diagnostics = Vec::new();

        validate_resource_guardrails(
            &request,
            &source,
            ConversionResourceLimits {
                max_source_vertices: 3,
                max_source_triangles: 1,
                max_resolution_axis: MAX_RESOLUTION_AXIS,
                max_resolution_cells: MAX_RESOLUTION_CELLS,
                max_requested_output_voxels: MAX_REQUESTED_OUTPUT_VOXELS,
            },
            &mut diagnostics,
        );

        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.code == VoxelConversionDiagnosticCode::OutputLimitExceeded
        }));
        assert_eq!(diagnostics[0].reference, "source.positions");
        assert_eq!(diagnostics[1].reference, "source.triangles");
    }

    #[test]
    fn stale_source_hash_rejects_without_output() {
        let source = cube_source();
        let mut request = request_for(&source, VoxelConversionMode::Solid, [2, 2, 2], 8);
        request.source.source_hash = "sha256:stale".to_string();
        let planned = plan_conversion(&request, &source);
        assert!(planned.output.is_none());
        assert_eq!(
            planned.plan.diagnostics[0].code,
            VoxelConversionDiagnosticCode::SourceHashMismatch
        );
    }

    fn assert_solid_topology_rejected(source: &StaticMeshSource) {
        let request = request_for(source, VoxelConversionMode::Solid, [2, 2, 2], 8);
        let planned = plan_conversion(&request, source);
        assert!(planned.output.is_none());
        assert_eq!(
            planned.plan.diagnostics[0].code,
            VoxelConversionDiagnosticCode::NonManifoldOrAmbiguousSolid
        );
    }

    #[test]
    fn apply_receipt_is_replay_hash_checked() {
        let source = cube_source();
        let request = request_for(&source, VoxelConversionMode::Solid, [2, 2, 2], 8);
        let planned = plan_conversion(&request, &source);
        let preview = preview_conversion(
            &VoxelConversionPreviewRequest {
                plan_id: planned.plan.plan_id.clone(),
                expected_plan_hash: plan_hash(&planned.plan),
            },
            &planned,
        );
        let receipt = apply_conversion(
            &VoxelConversionApplyRequest {
                plan_id: planned.plan.plan_id.clone(),
                expected_plan_hash: plan_hash(&planned.plan),
                expected_preview_hash: Some(preview.output_hash),
            },
            &planned,
        );
        assert!(receipt.applied);
        assert_eq!(receipt.output_voxel_count, 8);
        assert!(receipt.output_hash.is_some());
    }

    #[test]
    fn committed_golden_summaries_cover_success_and_failure_cases() {
        assert_eq!(
            conversion_golden_summary().trim(),
            include_str!(
                "../../../../../harness/goldens/voxel-conversion/conversion-summary.golden"
            )
            .trim()
        );
    }

    #[test]
    fn committed_studio_consumer_proof_matches_rust_authority_output() {
        let generated = studio_consumer_proof_authority_json();
        if std::env::var_os("ASHA_DUMP_VOXEL_CONVERSION_PROOF").is_some() {
            println!("{generated}");
        }
        assert_eq!(
            generated.trim(),
            include_str!(
                "../../../../../harness/goldens/voxel-conversion/studio-consumer-proof-authority.golden.json"
            )
            .trim()
        );
    }

    fn conversion_golden_summary() -> String {
        let quad = quad_source();
        let quad_plan = plan_conversion(
            &request_for(&quad, VoxelConversionMode::Surface, [4, 4, 1], 16),
            &quad,
        );
        let quad_preview = preview_conversion(
            &VoxelConversionPreviewRequest {
                plan_id: quad_plan.plan.plan_id.clone(),
                expected_plan_hash: plan_hash(&quad_plan.plan),
            },
            &quad_plan,
        );

        let cube = cube_source();
        let cube_plan = plan_conversion(
            &request_for(&cube, VoxelConversionMode::Solid, [2, 2, 2], 8),
            &cube,
        );
        let oversized = plan_conversion(
            &request_for(&cube, VoxelConversionMode::Solid, [2, 2, 2], 7),
            &cube,
        );

        let mut stale_request = request_for(&cube, VoxelConversionMode::Solid, [2, 2, 2], 8);
        stale_request.source.source_hash = "sha256:stale".to_string();
        let stale = plan_conversion(&stale_request, &cube);

        format!(
            "quad.surface.voxels={}\nquad.surface.bounds={}\nquad.surface.materials={}\ncube.solid.voxels={}\ncube.solid.bounds={}\ncube.solid.materials={}\noversized.code={}\nstale.code={}\n",
            quad_preview.output_voxel_count,
            bounds_label(quad_preview.output_bounds),
            material_label(&quad_preview.sample_voxels),
            cube_plan.plan.estimated_output_voxels,
            bounds_label(cube_plan.plan.estimated_bounds),
            output_material_label(cube_plan.output.as_ref().unwrap()),
            oversized.plan.diagnostics[0].code.as_str(),
            stale.plan.diagnostics[0].code.as_str(),
        )
    }

    fn studio_consumer_proof_authority_json() -> String {
        let source = quad_source();
        let plan_request = request_for(&source, VoxelConversionMode::Surface, [4, 4, 1], 16);
        let planned = plan_conversion(&plan_request, &source);
        let plan_hash = plan_hash(&planned.plan);
        let preview_request = VoxelConversionPreviewRequest {
            plan_id: planned.plan.plan_id.clone(),
            expected_plan_hash: plan_hash.clone(),
        };
        let preview = preview_conversion(&preview_request, &planned);
        let apply_request = VoxelConversionApplyRequest {
            plan_id: planned.plan.plan_id.clone(),
            expected_plan_hash: plan_hash,
            expected_preview_hash: Some(preview.output_hash.clone()),
        };
        let receipt = apply_conversion(&apply_request, &planned);
        let evidence_export = [
            planned.plan.evidence.clone(),
            preview.evidence.clone(),
            receipt.evidence.clone(),
        ]
        .concat();
        let payload = json!({
            "schemaVersion": 1,
            "authorityVersion": AUTHORITY_VERSION,
            "sourceAssetId": source.asset_id,
            "planRequest": plan_request,
            "plan": planned.plan,
            "previewRequest": preview_request,
            "preview": preview,
            "applyRequest": apply_request,
            "receipt": receipt,
            "evidenceExport": evidence_export
        });
        format!("{}\n", serde_json::to_string_pretty(&payload).unwrap())
    }

    fn bounds_label(bounds: Option<VoxelConversionBounds>) -> String {
        let Some(bounds) = bounds else {
            return "none".to_string();
        };
        format!(
            "{},{},{}..{},{},{}",
            bounds.min.x, bounds.min.y, bounds.min.z, bounds.max.x, bounds.max.y, bounds.max.z
        )
    }

    fn material_label(voxels: &[VoxelConversionPreviewVoxel]) -> String {
        let materials: BTreeSet<u16> = voxels.iter().map(|voxel| voxel.material).collect();
        materials
            .into_iter()
            .map(|material| material.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn output_material_label(output: &ConversionOutput) -> String {
        let materials: BTreeSet<u16> = output
            .voxels
            .iter()
            .map(|voxel| voxel.value.material().unwrap().raw())
            .collect();
        materials
            .into_iter()
            .map(|material| material.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }
}
