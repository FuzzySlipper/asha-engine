//! Protocol border for Asha-owned voxel conversion.
//!
//! # Lane
//!
//! `contract-steward` — owns the typed DTO surface for planning, previewing,
//! applying, and exporting evidence for conversion from supported Asha static
//! mesh/source assets into Asha voxel semantics.
//!
//! # Boundary posture
//!
//! These are inert data shapes and stable vocabularies only. Rust authority
//! crates own conversion planning, validation, apply, hashing, and receipts.
//! TypeScript and Studio may display these DTOs and submit requests through the
//! runtime facade, but they do not implement authoritative mesh voxelization.

#![forbid(unsafe_code)]

use protocol_diagnostics::DiagnosticSeverity;
use serde::{Deserialize, Serialize};

/// Stable supported conversion modes.
pub const VOXEL_CONVERSION_MODES: &[&str] = &["surface", "solid"];

/// Stable target-fit policies.
pub const VOXEL_CONVERSION_FIT_POLICIES: &[&str] = &["contain", "cover", "stretch"];

/// Stable origin-placement policies.
pub const VOXEL_CONVERSION_ORIGIN_POLICIES: &[&str] = &["source_origin", "target_min", "centered"];

/// Stable evidence-ref roles.
pub const VOXEL_CONVERSION_EVIDENCE_KINDS: &[&str] = &[
    "plan",
    "preview",
    "apply_receipt",
    "diagnostics",
    "source_snapshot",
    "output_snapshot",
];

/// Stable classified diagnostic/error codes. String values are the public
/// contract consumed by Studio and runtime facade callers.
pub const VOXEL_CONVERSION_DIAGNOSTIC_CODES: &[&str] = &[
    "voxel_conversion_unavailable",
    "operation_unimplemented",
    "unsupported_source_asset",
    "source_hash_mismatch",
    "invalid_material_map",
    "missing_texture_source",
    "texture_hash_mismatch",
    "missing_uv_attribute",
    "unsupported_texture_format",
    "unsupported_sampling_policy",
    "invalid_texture_material_rule",
    "output_limit_exceeded",
    "non_manifold_or_ambiguous_solid",
    "stale_authority_snapshot",
    "conversion_replay_mismatch",
];

/// Stable texture color-space names accepted by voxel conversion.
pub const VOXEL_CONVERSION_TEXTURE_COLOR_SPACES: &[&str] = &["linear", "srgb"];

/// Stable texture channel-layout names accepted by voxel conversion.
pub const VOXEL_CONVERSION_TEXTURE_CHANNEL_LAYOUTS: &[&str] = &["palette_index_u16", "grayscale8"];

/// Stable texture sampling policy names accepted by voxel conversion.
pub const VOXEL_CONVERSION_TEXTURE_SAMPLING_POLICIES: &[&str] = &["nearest_texel"];

/// Stable texture wrapping policy names accepted by voxel conversion.
pub const VOXEL_CONVERSION_TEXTURE_WRAP_POLICIES: &[&str] = &["clamp_to_edge"];

/// Stable texture-to-material mode names accepted by voxel conversion.
pub const VOXEL_CONVERSION_TEXTURE_MATERIAL_MODES: &[&str] = &["sample_palette_index"];

/// Conversion modes Rust authority may execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelConversionMode {
    /// Occupy cells intersecting source surfaces.
    Surface,
    /// Occupy a closed solid volume. Ambiguous/non-manifold inputs fail closed.
    Solid,
}

impl VoxelConversionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelConversionMode::Surface => "surface",
            VoxelConversionMode::Solid => "solid",
        }
    }
}

/// How the source bounds fit into the requested target resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelConversionFitPolicy {
    Contain,
    Cover,
    Stretch,
}

impl VoxelConversionFitPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelConversionFitPolicy::Contain => "contain",
            VoxelConversionFitPolicy::Cover => "cover",
            VoxelConversionFitPolicy::Stretch => "stretch",
        }
    }
}

/// How converted voxel coordinates are anchored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelConversionOriginPolicy {
    SourceOrigin,
    TargetMin,
    Centered,
}

impl VoxelConversionOriginPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelConversionOriginPolicy::SourceOrigin => "source_origin",
            VoxelConversionOriginPolicy::TargetMin => "target_min",
            VoxelConversionOriginPolicy::Centered => "centered",
        }
    }
}

/// Role of an exported evidence artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelConversionEvidenceKind {
    Plan,
    Preview,
    ApplyReceipt,
    Diagnostics,
    SourceSnapshot,
    OutputSnapshot,
}

impl VoxelConversionEvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelConversionEvidenceKind::Plan => "plan",
            VoxelConversionEvidenceKind::Preview => "preview",
            VoxelConversionEvidenceKind::ApplyReceipt => "apply_receipt",
            VoxelConversionEvidenceKind::Diagnostics => "diagnostics",
            VoxelConversionEvidenceKind::SourceSnapshot => "source_snapshot",
            VoxelConversionEvidenceKind::OutputSnapshot => "output_snapshot",
        }
    }
}

/// Classified conversion diagnostic/error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelConversionDiagnosticCode {
    VoxelConversionUnavailable,
    OperationUnimplemented,
    UnsupportedSourceAsset,
    SourceHashMismatch,
    InvalidMaterialMap,
    MissingTextureSource,
    TextureHashMismatch,
    MissingUvAttribute,
    UnsupportedTextureFormat,
    UnsupportedSamplingPolicy,
    InvalidTextureMaterialRule,
    OutputLimitExceeded,
    NonManifoldOrAmbiguousSolid,
    StaleAuthoritySnapshot,
    ConversionReplayMismatch,
}

impl VoxelConversionDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelConversionDiagnosticCode::VoxelConversionUnavailable => {
                "voxel_conversion_unavailable"
            }
            VoxelConversionDiagnosticCode::OperationUnimplemented => "operation_unimplemented",
            VoxelConversionDiagnosticCode::UnsupportedSourceAsset => "unsupported_source_asset",
            VoxelConversionDiagnosticCode::SourceHashMismatch => "source_hash_mismatch",
            VoxelConversionDiagnosticCode::InvalidMaterialMap => "invalid_material_map",
            VoxelConversionDiagnosticCode::MissingTextureSource => "missing_texture_source",
            VoxelConversionDiagnosticCode::TextureHashMismatch => "texture_hash_mismatch",
            VoxelConversionDiagnosticCode::MissingUvAttribute => "missing_uv_attribute",
            VoxelConversionDiagnosticCode::UnsupportedTextureFormat => "unsupported_texture_format",
            VoxelConversionDiagnosticCode::UnsupportedSamplingPolicy => {
                "unsupported_sampling_policy"
            }
            VoxelConversionDiagnosticCode::InvalidTextureMaterialRule => {
                "invalid_texture_material_rule"
            }
            VoxelConversionDiagnosticCode::OutputLimitExceeded => "output_limit_exceeded",
            VoxelConversionDiagnosticCode::NonManifoldOrAmbiguousSolid => {
                "non_manifold_or_ambiguous_solid"
            }
            VoxelConversionDiagnosticCode::StaleAuthoritySnapshot => "stale_authority_snapshot",
            VoxelConversionDiagnosticCode::ConversionReplayMismatch => "conversion_replay_mismatch",
        }
    }
}

/// Integer voxel coordinate at the DTO border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoxelConversionCoord {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

/// Inclusive voxel-space bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionBounds {
    pub min: VoxelConversionCoord,
    pub max: VoxelConversionCoord,
}

/// Source asset and authority snapshot identity for conversion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionSourceRef {
    pub asset_id: String,
    pub asset_kind: String,
    pub asset_version: u64,
    pub source_hash: String,
    pub mesh_primitive: Option<String>,
}

/// One static-mesh triangle registered as an authority-visible conversion source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionSourceTriangle {
    pub indices: [u32; 3],
    pub source_material_slot: u32,
}

/// One source material slot available on a registered conversion source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionSourceMaterialSlot {
    pub source_material_slot: u32,
    pub source_material_id: Option<String>,
}

/// Register inline static-mesh geometry as an authority-visible conversion source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionSourceRegistrationRequest {
    pub source: VoxelConversionSourceRef,
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<VoxelConversionSourceTriangle>,
    pub material_slots: Vec<VoxelConversionSourceMaterialSlot>,
}

/// A material-indexed triangle group inside a project mesh asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionMeshAssetGroup {
    pub material_slot: u32,
    pub start: u32,
    pub count: u32,
}

/// Project/catalog static-mesh data accepted by Rust voxel-conversion ingestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionMeshAsset {
    pub asset_id: String,
    pub source_path: Option<String>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub groups: Vec<VoxelConversionMeshAssetGroup>,
    pub material_slots: Vec<VoxelConversionSourceMaterialSlot>,
}

/// Register an authored project static-mesh asset as a conversion source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionMeshAssetRegistrationRequest {
    pub source: VoxelConversionSourceRef,
    pub mesh_asset: VoxelConversionMeshAsset,
}

/// Result of registering a conversion source; rejected inputs carry diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionSourceRegistration {
    pub source: VoxelConversionSourceRef,
    pub registered: bool,
    pub material_slots: Vec<VoxelConversionSourceMaterialSlot>,
    pub diagnostics: Vec<VoxelConversionDiagnostic>,
    pub evidence: Vec<VoxelConversionEvidenceRef>,
}

/// Target voxel grid/volume identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionTargetRef {
    pub grid: u64,
    pub volume_asset_id: Option<String>,
    pub origin: VoxelConversionCoord,
}

/// One source material slot mapped into an Asha voxel material id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionMaterialMapEntry {
    pub source_material_slot: u32,
    pub source_material_id: Option<String>,
    pub voxel_material: u16,
}

/// Authority-visible UV attribute identity used by texture sampling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionUvAttributeRef {
    pub attribute_name: String,
    pub source_hash: String,
}

/// Authority-visible texture snapshot identity for voxel material sampling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionTextureSourceRef {
    pub texture_asset_id: String,
    pub asset_version: u64,
    pub content_hash: String,
    pub width: u32,
    pub height: u32,
    pub color_space: String,
    pub channel_layout: String,
}

/// Texture snapshot data accepted by Rust authority for voxel material sampling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionTextureSampleAsset {
    pub texture: VoxelConversionTextureSourceRef,
    pub texel_materials: Vec<u16>,
}

/// Per-source-slot texture sampling request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionTextureBinding {
    pub source_material_slot: u32,
    pub texture: VoxelConversionTextureSourceRef,
    pub uv_attribute: VoxelConversionUvAttributeRef,
    pub sample_uv: [f32; 2],
    pub sampling_policy: String,
    pub wrap_policy: String,
    pub material_mode: String,
}

/// Material-map DTO. `default_voxel_material` is used only when authority
/// accepts unmapped source slots for the chosen conversion policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionMaterialMap {
    pub entries: Vec<VoxelConversionMaterialMapEntry>,
    pub texture_assets: Vec<VoxelConversionTextureSampleAsset>,
    pub texture_bindings: Vec<VoxelConversionTextureBinding>,
    pub default_voxel_material: Option<u16>,
}

/// A conversion request's tunable settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionSettings {
    pub mode: VoxelConversionMode,
    pub fit_policy: VoxelConversionFitPolicy,
    pub origin_policy: VoxelConversionOriginPolicy,
    pub resolution: [u32; 3],
    pub voxel_size: f32,
    pub max_output_voxels: u64,
    pub transform: [f32; 16],
    pub material_map: VoxelConversionMaterialMap,
}

/// One request to plan a conversion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionPlanRequest {
    pub source: VoxelConversionSourceRef,
    pub target: VoxelConversionTargetRef,
    pub settings: VoxelConversionSettings,
}

/// One classified diagnostic for a conversion operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionDiagnostic {
    pub code: VoxelConversionDiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub reference: String,
    pub message: String,
}

/// Reference to an inspectable artifact emitted by authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionEvidenceRef {
    pub kind: VoxelConversionEvidenceKind,
    pub uri: String,
    pub content_hash: String,
}

/// Deterministic conversion plan produced by Rust authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionPlan {
    pub plan_id: String,
    pub source: VoxelConversionSourceRef,
    pub target: VoxelConversionTargetRef,
    pub settings: VoxelConversionSettings,
    pub authority_version: String,
    pub expected_source_hash: String,
    pub settings_hash: String,
    pub plan_hash: String,
    pub estimated_output_voxels: u64,
    pub estimated_bounds: Option<VoxelConversionBounds>,
    pub diagnostics: Vec<VoxelConversionDiagnostic>,
    pub evidence: Vec<VoxelConversionEvidenceRef>,
}

/// Preview request for a previously produced plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionPreviewRequest {
    pub plan_id: String,
    pub expected_plan_hash: String,
}

/// One sampled/previewed output voxel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionPreviewVoxel {
    pub coord: VoxelConversionCoord,
    pub material: u16,
}

/// Bounded preview of conversion output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionPreview {
    pub plan_id: String,
    pub output_hash: String,
    pub output_voxel_count: u64,
    pub output_bounds: Option<VoxelConversionBounds>,
    pub sample_voxels: Vec<VoxelConversionPreviewVoxel>,
    pub diagnostics: Vec<VoxelConversionDiagnostic>,
    pub evidence: Vec<VoxelConversionEvidenceRef>,
}

/// Apply request for a planned conversion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionApplyRequest {
    pub plan_id: String,
    pub expected_plan_hash: String,
    pub expected_preview_hash: Option<String>,
}

/// Final apply receipt. Rejected requests never pretend to have applied output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionReceipt {
    pub plan_id: String,
    pub applied: bool,
    pub output_hash: Option<String>,
    pub output_voxel_count: u64,
    pub output_bounds: Option<VoxelConversionBounds>,
    pub diagnostics: Vec<VoxelConversionDiagnostic>,
    pub evidence: Vec<VoxelConversionEvidenceRef>,
}

/// Request for authority-owned model/volume readback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelModelInfoRequest {
    pub grid: u64,
    pub volume_asset_id: Option<String>,
    pub include_material_counts: bool,
}

/// Per-material voxel count derived from authority state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelModelMaterialCount {
    pub material: u16,
    pub voxel_count: u64,
}

/// Rich but bounded model/volume readback for Studio and agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelModelInfoReadout {
    pub request: VoxelModelInfoRequest,
    pub resident: bool,
    pub model_id: String,
    pub volume_asset_id: Option<String>,
    pub grid: u64,
    pub bounds: Option<VoxelConversionBounds>,
    pub voxel_count: u64,
    pub material_counts: Vec<VoxelModelMaterialCount>,
    pub source: Option<VoxelConversionSourceRef>,
    pub latest_plan_id: Option<String>,
    pub latest_output_hash: Option<String>,
    pub session_hash: String,
    pub replay_hash: String,
    pub evidence: Vec<VoxelConversionEvidenceRef>,
    pub diagnostics: Vec<VoxelConversionDiagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabulary_tables_match_enum_strings() {
        assert_eq!(
            VOXEL_CONVERSION_MODES,
            &[
                VoxelConversionMode::Surface.as_str(),
                VoxelConversionMode::Solid.as_str()
            ]
        );
        assert_eq!(
            VOXEL_CONVERSION_FIT_POLICIES,
            &[
                VoxelConversionFitPolicy::Contain.as_str(),
                VoxelConversionFitPolicy::Cover.as_str(),
                VoxelConversionFitPolicy::Stretch.as_str(),
            ]
        );
        assert_eq!(
            VOXEL_CONVERSION_ORIGIN_POLICIES,
            &[
                VoxelConversionOriginPolicy::SourceOrigin.as_str(),
                VoxelConversionOriginPolicy::TargetMin.as_str(),
                VoxelConversionOriginPolicy::Centered.as_str(),
            ]
        );
        assert_eq!(
            VOXEL_CONVERSION_EVIDENCE_KINDS,
            &[
                VoxelConversionEvidenceKind::Plan.as_str(),
                VoxelConversionEvidenceKind::Preview.as_str(),
                VoxelConversionEvidenceKind::ApplyReceipt.as_str(),
                VoxelConversionEvidenceKind::Diagnostics.as_str(),
                VoxelConversionEvidenceKind::SourceSnapshot.as_str(),
                VoxelConversionEvidenceKind::OutputSnapshot.as_str(),
            ]
        );
        assert_eq!(
            VOXEL_CONVERSION_DIAGNOSTIC_CODES,
            &[
                VoxelConversionDiagnosticCode::VoxelConversionUnavailable.as_str(),
                VoxelConversionDiagnosticCode::OperationUnimplemented.as_str(),
                VoxelConversionDiagnosticCode::UnsupportedSourceAsset.as_str(),
                VoxelConversionDiagnosticCode::SourceHashMismatch.as_str(),
                VoxelConversionDiagnosticCode::InvalidMaterialMap.as_str(),
                VoxelConversionDiagnosticCode::MissingTextureSource.as_str(),
                VoxelConversionDiagnosticCode::TextureHashMismatch.as_str(),
                VoxelConversionDiagnosticCode::MissingUvAttribute.as_str(),
                VoxelConversionDiagnosticCode::UnsupportedTextureFormat.as_str(),
                VoxelConversionDiagnosticCode::UnsupportedSamplingPolicy.as_str(),
                VoxelConversionDiagnosticCode::InvalidTextureMaterialRule.as_str(),
                VoxelConversionDiagnosticCode::OutputLimitExceeded.as_str(),
                VoxelConversionDiagnosticCode::NonManifoldOrAmbiguousSolid.as_str(),
                VoxelConversionDiagnosticCode::StaleAuthoritySnapshot.as_str(),
                VoxelConversionDiagnosticCode::ConversionReplayMismatch.as_str(),
            ]
        );
    }

    #[test]
    fn plan_request_serializes_with_camel_case_fields_and_snake_case_vocab() {
        let request = VoxelConversionPlanRequest {
            source: VoxelConversionSourceRef {
                asset_id: "mesh/test-cube".to_string(),
                asset_kind: "mesh".to_string(),
                asset_version: 7,
                source_hash: "sha256:source".to_string(),
                mesh_primitive: Some("primitive-0".to_string()),
            },
            target: VoxelConversionTargetRef {
                grid: 1,
                volume_asset_id: None,
                origin: VoxelConversionCoord { x: 0, y: 0, z: 0 },
            },
            settings: VoxelConversionSettings {
                mode: VoxelConversionMode::Solid,
                fit_policy: VoxelConversionFitPolicy::Contain,
                origin_policy: VoxelConversionOriginPolicy::TargetMin,
                resolution: [16, 16, 16],
                voxel_size: 0.25,
                max_output_voxels: 4096,
                transform: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
                material_map: VoxelConversionMaterialMap {
                    entries: vec![VoxelConversionMaterialMapEntry {
                        source_material_slot: 0,
                        source_material_id: Some("mat/stone".to_string()),
                        voxel_material: 3,
                    }],
                    texture_assets: Vec::new(),
                    texture_bindings: Vec::new(),
                    default_voxel_material: None,
                },
            },
        };

        let serialized = serde_json::to_value(&request).unwrap();
        assert_eq!(serialized["source"]["assetId"], "mesh/test-cube");
        assert_eq!(serialized["settings"]["mode"], "solid");
        assert_eq!(serialized["settings"]["fitPolicy"], "contain");
        assert_eq!(serialized["settings"]["originPolicy"], "target_min");
        assert_eq!(
            serialized["settings"]["materialMap"]["entries"][0]["voxelMaterial"],
            3
        );
    }

    #[test]
    fn texture_sampling_dtos_round_trip_with_camel_case_fields() {
        let texture = VoxelConversionTextureSourceRef {
            texture_asset_id: "texture/checker".to_string(),
            asset_version: 2,
            content_hash: "sha256:texture".to_string(),
            width: 2,
            height: 1,
            color_space: "linear".to_string(),
            channel_layout: "palette_index_u16".to_string(),
        };
        let texture_asset = VoxelConversionTextureSampleAsset {
            texture: texture.clone(),
            texel_materials: vec![3, 7],
        };
        let binding = VoxelConversionTextureBinding {
            source_material_slot: 4,
            texture,
            uv_attribute: VoxelConversionUvAttributeRef {
                attribute_name: "TEXCOORD_0".to_string(),
                source_hash: "sha256:uv".to_string(),
            },
            sample_uv: [1.0, 0.0],
            sampling_policy: "nearest_texel".to_string(),
            wrap_policy: "clamp_to_edge".to_string(),
            material_mode: "sample_palette_index".to_string(),
        };
        let map = VoxelConversionMaterialMap {
            entries: Vec::new(),
            texture_assets: vec![texture_asset],
            texture_bindings: vec![binding],
            default_voxel_material: None,
        };

        assert_round_trip(&map);

        let serialized = serde_json::to_value(&map).unwrap();
        assert_eq!(
            serialized["textureAssets"][0]["texture"]["textureAssetId"],
            "texture/checker"
        );
        assert_eq!(serialized["textureAssets"][0]["texelMaterials"][1], 7);
        assert_eq!(
            serialized["textureBindings"][0]["uvAttribute"]["attributeName"],
            "TEXCOORD_0"
        );
        assert_eq!(serialized["textureBindings"][0]["sampleUv"][0], 1.0);
        assert_eq!(
            serialized["textureBindings"][0]["samplingPolicy"],
            "nearest_texel"
        );
    }

    #[test]
    fn model_info_readout_round_trips_with_camel_case_fields() {
        let request = VoxelModelInfoRequest {
            grid: 7,
            volume_asset_id: Some("volume/demo-cave".to_string()),
            include_material_counts: true,
        };
        let material_count = VoxelModelMaterialCount {
            material: 11,
            voxel_count: 512,
        };
        let readout = VoxelModelInfoReadout {
            request: request.clone(),
            resident: true,
            model_id: "model/voxel-cave".to_string(),
            volume_asset_id: request.volume_asset_id.clone(),
            grid: request.grid,
            bounds: Some(VoxelConversionBounds {
                min: VoxelConversionCoord { x: -1, y: 0, z: 2 },
                max: VoxelConversionCoord { x: 14, y: 8, z: 17 },
            }),
            voxel_count: 640,
            material_counts: vec![material_count.clone()],
            source: Some(VoxelConversionSourceRef {
                asset_id: "mesh/cave-wall".to_string(),
                asset_kind: "static_mesh".to_string(),
                asset_version: 3,
                source_hash: "sha256:source".to_string(),
                mesh_primitive: Some("primitive-0".to_string()),
            }),
            latest_plan_id: Some("plan-123".to_string()),
            latest_output_hash: Some("sha256:output".to_string()),
            session_hash: "sha256:session".to_string(),
            replay_hash: "sha256:replay".to_string(),
            evidence: vec![VoxelConversionEvidenceRef {
                kind: VoxelConversionEvidenceKind::OutputSnapshot,
                uri: "asha://evidence/output".to_string(),
                content_hash: "sha256:evidence".to_string(),
            }],
            diagnostics: vec![VoxelConversionDiagnostic {
                code: VoxelConversionDiagnosticCode::StaleAuthoritySnapshot,
                severity: DiagnosticSeverity::Warning,
                reference: "grid:7".to_string(),
                message: "sample warning".to_string(),
            }],
        };

        assert_round_trip(&request);
        assert_round_trip(&material_count);
        assert_round_trip(&readout);

        let serialized = serde_json::to_value(&readout).unwrap();
        assert_eq!(serialized["request"]["includeMaterialCounts"], true);
        assert_eq!(serialized["volumeAssetId"], "volume/demo-cave");
        assert_eq!(serialized["materialCounts"][0]["voxelCount"], 512);
        assert_eq!(serialized["latestOutputHash"], "sha256:output");
        assert_eq!(serialized["evidence"][0]["kind"], "output_snapshot");
        assert_eq!(serialized["diagnostics"][0]["severity"], "warning");
    }

    fn assert_round_trip<T>(sample: &T)
    where
        T: Clone
            + PartialEq
            + std::fmt::Debug
            + serde::Serialize
            + for<'de> serde::Deserialize<'de>,
    {
        let serialized = serde_json::to_string(sample).unwrap();
        let deserialized: T = serde_json::from_str(&serialized).unwrap();
        assert_eq!(&deserialized, sample);
    }
}
