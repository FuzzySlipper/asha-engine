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
    "output_limit_exceeded",
    "non_manifold_or_ambiguous_solid",
    "stale_authority_snapshot",
    "conversion_replay_mismatch",
];

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

/// Material-map DTO. `default_voxel_material` is used only when authority
/// accepts unmapped source slots for the chosen conversion policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelConversionMaterialMap {
    pub entries: Vec<VoxelConversionMaterialMapEntry>,
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
}
