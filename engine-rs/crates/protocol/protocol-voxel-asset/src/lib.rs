//! Protocol border for Asha-native stored voxel-volume assets.
//!
//! # Lane
//!
//! `contract-steward` — owns inert DTOs and stable vocabulary for durable
//! ProjectBundle/catalog voxel-volume assets. Rust authority validates,
//! serializes, hashes, imports, exports, and transitions these assets between
//! stored ProjectBundle data and runtime SessionState.
//!
//! # Boundary posture
//!
//! This is not a VoxelForge compatibility layer and does not define `.vforge`.
//! Studio and TypeScript may display these DTOs and submit them through public
//! facades, but they do not own validation or canonical serialization.

#![forbid(unsafe_code)]

use protocol_diagnostics::DiagnosticSeverity;
use serde::{Deserialize, Serialize};

/// Current supported Asha voxel-volume asset schema.
pub const VOXEL_ASSET_SCHEMA_VERSION: u32 = 1;

/// Canonical media type for the JSON envelope.
pub const VOXEL_ASSET_MEDIA_TYPE: &str = "application/vnd.asha.voxel-volume+json;version=1";

/// Canonical filename extension for this JSON envelope.
pub const VOXEL_ASSET_EXTENSION: &str = "avxl.json";

/// Stable representation tags.
pub const VOXEL_ASSET_REPRESENTATION_KINDS: &[&str] = &["sparse_runs"];

/// Stable provenance/evidence ref tags.
pub const VOXEL_ASSET_PROVENANCE_KINDS: &[&str] = &[
    "authored",
    "converted",
    "runtime_export",
    "imported_reference",
];

/// Stable classified validation diagnostics.
pub const VOXEL_ASSET_DIAGNOSTIC_CODES: &[&str] = &[
    "unsupported_schema_version",
    "unsupported_media_type",
    "invalid_asset_id",
    "invalid_grid",
    "invalid_bounds",
    "unsupported_representation",
    "invalid_sparse_run",
    "duplicate_voxel",
    "duplicate_material_binding",
    "invalid_material_reference",
    "unknown_voxel_material",
    "content_hash_mismatch",
    "runtime_model_unavailable",
    "export_limit_exceeded",
    "stale_runtime_snapshot",
];

/// Stored voxel representation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelAssetRepresentationKind {
    SparseRuns,
}

impl VoxelAssetRepresentationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelAssetRepresentationKind::SparseRuns => "sparse_runs",
        }
    }
}

/// Stored voxel-volume provenance kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelAssetProvenanceKind {
    Authored,
    Converted,
    RuntimeExport,
    ImportedReference,
}

impl VoxelAssetProvenanceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelAssetProvenanceKind::Authored => "authored",
            VoxelAssetProvenanceKind::Converted => "converted",
            VoxelAssetProvenanceKind::RuntimeExport => "runtime_export",
            VoxelAssetProvenanceKind::ImportedReference => "imported_reference",
        }
    }
}

/// Classified stored-voxel asset diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelAssetDiagnosticCode {
    UnsupportedSchemaVersion,
    UnsupportedMediaType,
    InvalidAssetId,
    InvalidGrid,
    InvalidBounds,
    UnsupportedRepresentation,
    InvalidSparseRun,
    DuplicateVoxel,
    DuplicateMaterialBinding,
    InvalidMaterialReference,
    UnknownVoxelMaterial,
    ContentHashMismatch,
    RuntimeModelUnavailable,
    ExportLimitExceeded,
    StaleRuntimeSnapshot,
}

impl VoxelAssetDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelAssetDiagnosticCode::UnsupportedSchemaVersion => "unsupported_schema_version",
            VoxelAssetDiagnosticCode::UnsupportedMediaType => "unsupported_media_type",
            VoxelAssetDiagnosticCode::InvalidAssetId => "invalid_asset_id",
            VoxelAssetDiagnosticCode::InvalidGrid => "invalid_grid",
            VoxelAssetDiagnosticCode::InvalidBounds => "invalid_bounds",
            VoxelAssetDiagnosticCode::UnsupportedRepresentation => "unsupported_representation",
            VoxelAssetDiagnosticCode::InvalidSparseRun => "invalid_sparse_run",
            VoxelAssetDiagnosticCode::DuplicateVoxel => "duplicate_voxel",
            VoxelAssetDiagnosticCode::DuplicateMaterialBinding => "duplicate_material_binding",
            VoxelAssetDiagnosticCode::InvalidMaterialReference => "invalid_material_reference",
            VoxelAssetDiagnosticCode::UnknownVoxelMaterial => "unknown_voxel_material",
            VoxelAssetDiagnosticCode::ContentHashMismatch => "content_hash_mismatch",
            VoxelAssetDiagnosticCode::RuntimeModelUnavailable => "runtime_model_unavailable",
            VoxelAssetDiagnosticCode::ExportLimitExceeded => "export_limit_exceeded",
            VoxelAssetDiagnosticCode::StaleRuntimeSnapshot => "stale_runtime_snapshot",
        }
    }
}

/// Integer coordinate in stored voxel space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VoxelAssetCoord {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

/// Inclusive stored voxel-space bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAssetBounds {
    pub min: VoxelAssetCoord,
    pub max: VoxelAssetCoord,
}

/// Grid placement metadata for stored voxel cells.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAssetGrid {
    pub origin: [f64; 3],
    pub cell_size: f64,
    pub coordinate_system: String,
}

/// One compact voxel-material binding to a named ProjectBundle catalog entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAssetMaterialBinding {
    pub voxel_material: u16,
    pub palette_entry_id: String,
    pub display_name: Option<String>,
    pub material_asset_id: String,
    pub material_catalog_binding_id: Option<String>,
}

/// One run of solid voxels along +X. Absence is empty space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAssetSparseRun {
    pub start: VoxelAssetCoord,
    pub length: u32,
    pub material: u16,
}

/// Stored voxel representation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAssetRepresentation {
    pub kind: VoxelAssetRepresentationKind,
    pub sparse_runs: Vec<VoxelAssetSparseRun>,
}

/// Provenance/evidence reference for stored voxel assets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAssetProvenanceRef {
    pub kind: VoxelAssetProvenanceKind,
    pub uri: String,
    pub content_hash: String,
}

/// Human/editor metadata that never owns runtime authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAssetAuthoringMetadata {
    pub label: Option<String>,
    pub created_by: Option<String>,
    pub source_tool: Option<String>,
}

/// Canonical hashes recorded with the stored asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAssetContentHashes {
    pub canonical_json: String,
    pub voxel_data: String,
}

/// One classified validation diagnostic for a stored voxel-volume asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAssetDiagnostic {
    pub code: VoxelAssetDiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub reference: String,
    pub message: String,
}

/// Per-material voxel count for stored/runtime voxel asset readbacks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAssetMaterialCount {
    pub material: u16,
    pub voxel_count: u64,
}

/// A complete Asha-native stored voxel-volume asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelVolumeAsset {
    pub asset_id: String,
    pub schema_version: u32,
    pub media_type: String,
    pub grid: VoxelAssetGrid,
    pub bounds: VoxelAssetBounds,
    pub representation: VoxelAssetRepresentation,
    pub material_palette: Vec<VoxelAssetMaterialBinding>,
    pub provenance: Vec<VoxelAssetProvenanceRef>,
    pub authoring: VoxelAssetAuthoringMetadata,
    pub validation_diagnostics: Vec<VoxelAssetDiagnostic>,
    pub content_hashes: VoxelAssetContentHashes,
}

/// Request to export a resident runtime voxel model into stored asset form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelVolumeAssetExportRequest {
    pub grid: u64,
    pub volume_asset_id: Option<String>,
    pub target_asset_id: String,
    pub label: Option<String>,
    pub created_by: Option<String>,
    pub source_tool: Option<String>,
    pub max_sparse_runs: u64,
    pub expected_session_hash: Option<String>,
}

/// Receipt for explicit runtime-to-stored voxel asset export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelVolumeAssetExportReceipt {
    pub request: VoxelVolumeAssetExportRequest,
    pub exported: bool,
    pub asset: Option<VoxelVolumeAsset>,
    pub canonical_json: Option<String>,
    pub canonical_json_hash: Option<String>,
    pub voxel_data_hash: Option<String>,
    pub diagnostics: Vec<VoxelAssetDiagnostic>,
}

/// Request to turn a resident runtime voxel model into an explicit stored asset
/// diff/save proposal for a ProjectBundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelVolumeAssetSaveRequest {
    pub export_request: VoxelVolumeAssetExportRequest,
    pub target_project_bundle: String,
    pub target_asset_path: String,
    pub representation_kind: String,
    pub expected_existing_canonical_json_hash: Option<String>,
    pub expected_canonical_json_hash: Option<String>,
    pub expected_voxel_data_hash: Option<String>,
}

/// Explicit stored-asset diff summary produced before the host writes content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelVolumeAssetStoredDiff {
    pub project_bundle: String,
    pub asset_id: String,
    pub asset_path: String,
    pub operation: String,
    pub previous_canonical_json_hash: Option<String>,
    pub next_canonical_json_hash: String,
    pub next_voxel_data_hash: String,
    pub representation_kind: VoxelAssetRepresentationKind,
    pub sparse_run_count: u64,
    pub voxel_count: u64,
    pub material_count: u64,
    pub provenance_count: u64,
    pub runtime_session_hash: String,
}

/// Receipt for an accepted/rejected runtime-to-stored voxel asset transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelVolumeAssetSaveReceipt {
    pub request: VoxelVolumeAssetSaveRequest,
    pub saved: bool,
    pub diff: Option<VoxelVolumeAssetStoredDiff>,
    pub asset: Option<VoxelVolumeAsset>,
    pub canonical_json: Option<String>,
    pub canonical_json_hash: Option<String>,
    pub voxel_data_hash: Option<String>,
    pub diagnostics: Vec<VoxelAssetDiagnostic>,
}

/// Explicit request to load a validated stored voxel-volume asset into runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelVolumeAssetLoadRequest {
    pub asset: VoxelVolumeAsset,
    pub target_grid: u64,
    pub target_volume_asset_id: Option<String>,
    pub replace_existing: bool,
    pub include_material_counts: bool,
}

/// Receipt/readback for loading a stored voxel-volume asset into runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelVolumeAssetLoadReceipt {
    pub request_asset_id: String,
    pub loaded: bool,
    pub model_id: String,
    pub volume_asset_id: Option<String>,
    pub grid: u64,
    pub bounds: Option<VoxelAssetBounds>,
    pub voxel_count: u64,
    pub material_counts: Vec<VoxelAssetMaterialCount>,
    pub provenance: Vec<VoxelAssetProvenanceRef>,
    pub canonical_json_hash: Option<String>,
    pub voxel_data_hash: Option<String>,
    pub session_hash: String,
    pub replay_hash: String,
    pub diagnostics: Vec<VoxelAssetDiagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabulary_tables_are_nonempty_and_unique() {
        for table in [
            VOXEL_ASSET_REPRESENTATION_KINDS,
            VOXEL_ASSET_PROVENANCE_KINDS,
            VOXEL_ASSET_DIAGNOSTIC_CODES,
        ] {
            assert!(!table.is_empty());
            let mut sorted = table.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), table.len(), "duplicate in {table:?}");
        }
    }

    #[test]
    fn enum_tables_match_public_strings() {
        assert_eq!(
            vec![VoxelAssetRepresentationKind::SparseRuns.as_str()],
            VOXEL_ASSET_REPRESENTATION_KINDS
        );
        assert_eq!(
            [
                VoxelAssetProvenanceKind::Authored,
                VoxelAssetProvenanceKind::Converted,
                VoxelAssetProvenanceKind::RuntimeExport,
                VoxelAssetProvenanceKind::ImportedReference,
            ]
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>(),
            VOXEL_ASSET_PROVENANCE_KINDS
        );
    }
}
