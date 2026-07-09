//! Protocol border for ASHA-native voxel annotation layers.
//!
//! # Lane
//!
//! `contract-steward` - owns inert DTOs and stable vocabulary for durable
//! ProjectBundle/catalog voxel annotation layers plus future runtime
//! load/query/edit/export receipts.
//!
//! # Boundary posture
//!
//! These contracts describe semantic regions over voxel cells. They do not own
//! voxel occupancy, rendering, collision, gameplay authority, validation, or
//! runtime mutation. Rust services/rules will validate and apply these shapes in
//! later tasks; TypeScript may display and submit them only through public
//! generated contracts and runtime facades.

#![forbid(unsafe_code)]

use protocol_diagnostics::DiagnosticSeverity;
use serde::{Deserialize, Serialize};

/// Current supported ASHA voxel annotation layer schema.
pub const VOXEL_ANNOTATION_SCHEMA_VERSION: u32 = 1;

/// Canonical media type for the JSON annotation layer envelope.
pub const VOXEL_ANNOTATION_MEDIA_TYPE: &str =
    "application/vnd.asha.voxel-annotation+json;version=1";

/// Canonical filename extension for this JSON envelope.
pub const VOXEL_ANNOTATION_EXTENSION: &str = "avann.json";

/// Stable semantic region kind vocabulary.
pub const VOXEL_ANNOTATION_KINDS: &[&str] = &[
    "selection",
    "room",
    "portal",
    "spawn_area",
    "cover",
    "hazard",
    "nav_hint",
    "custom",
];

/// Stable provenance/evidence ref tags.
pub const VOXEL_ANNOTATION_PROVENANCE_KINDS: &[&str] = &[
    "authored",
    "imported_reference",
    "runtime_export",
    "generated",
];

/// Stable classified validation/runtime diagnostic codes.
pub const VOXEL_ANNOTATION_DIAGNOSTIC_CODES: &[&str] = &[
    "unsupported_schema_version",
    "unsupported_media_type",
    "invalid_layer_id",
    "invalid_target_voxel_volume_asset_id",
    "target_voxel_hash_mismatch",
    "invalid_bounds",
    "invalid_region_id",
    "duplicate_region_id",
    "unknown_parent_region",
    "parent_cycle",
    "unsupported_annotation_kind",
    "invalid_sparse_run",
    "duplicate_cell",
    "region_out_of_bounds",
    "quota_exceeded",
    "stale_layer_hash",
    "layer_not_loaded",
    "query_out_of_bounds",
    "edit_conflict",
];

/// Stable edit operations accepted by future annotation runtime authority.
pub const VOXEL_ANNOTATION_EDIT_OPERATIONS: &[&str] = &[
    "upsert_region",
    "remove_region",
    "add_runs",
    "remove_runs",
    "replace_selection",
    "set_parent",
    "set_tags",
    "set_label",
    "set_kind",
];

/// Stable query modes accepted by future annotation runtime authority.
pub const VOXEL_ANNOTATION_QUERY_MODES: &[&str] = &["cell", "bounds", "region", "layer_summary"];

/// Semantic region kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelAnnotationKind {
    Selection,
    Room,
    Portal,
    SpawnArea,
    Cover,
    Hazard,
    NavHint,
    Custom,
}

impl VoxelAnnotationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelAnnotationKind::Selection => "selection",
            VoxelAnnotationKind::Room => "room",
            VoxelAnnotationKind::Portal => "portal",
            VoxelAnnotationKind::SpawnArea => "spawn_area",
            VoxelAnnotationKind::Cover => "cover",
            VoxelAnnotationKind::Hazard => "hazard",
            VoxelAnnotationKind::NavHint => "nav_hint",
            VoxelAnnotationKind::Custom => "custom",
        }
    }
}

/// Voxel annotation provenance kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelAnnotationProvenanceKind {
    Authored,
    ImportedReference,
    RuntimeExport,
    Generated,
}

impl VoxelAnnotationProvenanceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelAnnotationProvenanceKind::Authored => "authored",
            VoxelAnnotationProvenanceKind::ImportedReference => "imported_reference",
            VoxelAnnotationProvenanceKind::RuntimeExport => "runtime_export",
            VoxelAnnotationProvenanceKind::Generated => "generated",
        }
    }
}

/// Classified voxel annotation diagnostic code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelAnnotationDiagnosticCode {
    UnsupportedSchemaVersion,
    UnsupportedMediaType,
    InvalidLayerId,
    InvalidTargetVoxelVolumeAssetId,
    TargetVoxelHashMismatch,
    InvalidBounds,
    InvalidRegionId,
    DuplicateRegionId,
    UnknownParentRegion,
    ParentCycle,
    UnsupportedAnnotationKind,
    InvalidSparseRun,
    DuplicateCell,
    RegionOutOfBounds,
    QuotaExceeded,
    StaleLayerHash,
    LayerNotLoaded,
    QueryOutOfBounds,
    EditConflict,
}

impl VoxelAnnotationDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelAnnotationDiagnosticCode::UnsupportedSchemaVersion => "unsupported_schema_version",
            VoxelAnnotationDiagnosticCode::UnsupportedMediaType => "unsupported_media_type",
            VoxelAnnotationDiagnosticCode::InvalidLayerId => "invalid_layer_id",
            VoxelAnnotationDiagnosticCode::InvalidTargetVoxelVolumeAssetId => {
                "invalid_target_voxel_volume_asset_id"
            }
            VoxelAnnotationDiagnosticCode::TargetVoxelHashMismatch => "target_voxel_hash_mismatch",
            VoxelAnnotationDiagnosticCode::InvalidBounds => "invalid_bounds",
            VoxelAnnotationDiagnosticCode::InvalidRegionId => "invalid_region_id",
            VoxelAnnotationDiagnosticCode::DuplicateRegionId => "duplicate_region_id",
            VoxelAnnotationDiagnosticCode::UnknownParentRegion => "unknown_parent_region",
            VoxelAnnotationDiagnosticCode::ParentCycle => "parent_cycle",
            VoxelAnnotationDiagnosticCode::UnsupportedAnnotationKind => {
                "unsupported_annotation_kind"
            }
            VoxelAnnotationDiagnosticCode::InvalidSparseRun => "invalid_sparse_run",
            VoxelAnnotationDiagnosticCode::DuplicateCell => "duplicate_cell",
            VoxelAnnotationDiagnosticCode::RegionOutOfBounds => "region_out_of_bounds",
            VoxelAnnotationDiagnosticCode::QuotaExceeded => "quota_exceeded",
            VoxelAnnotationDiagnosticCode::StaleLayerHash => "stale_layer_hash",
            VoxelAnnotationDiagnosticCode::LayerNotLoaded => "layer_not_loaded",
            VoxelAnnotationDiagnosticCode::QueryOutOfBounds => "query_out_of_bounds",
            VoxelAnnotationDiagnosticCode::EditConflict => "edit_conflict",
        }
    }
}

/// Runtime annotation edit operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelAnnotationEditOperation {
    UpsertRegion,
    RemoveRegion,
    AddRuns,
    RemoveRuns,
    ReplaceSelection,
    SetParent,
    SetTags,
    SetLabel,
    SetKind,
}

impl VoxelAnnotationEditOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelAnnotationEditOperation::UpsertRegion => "upsert_region",
            VoxelAnnotationEditOperation::RemoveRegion => "remove_region",
            VoxelAnnotationEditOperation::AddRuns => "add_runs",
            VoxelAnnotationEditOperation::RemoveRuns => "remove_runs",
            VoxelAnnotationEditOperation::ReplaceSelection => "replace_selection",
            VoxelAnnotationEditOperation::SetParent => "set_parent",
            VoxelAnnotationEditOperation::SetTags => "set_tags",
            VoxelAnnotationEditOperation::SetLabel => "set_label",
            VoxelAnnotationEditOperation::SetKind => "set_kind",
        }
    }
}

/// Runtime annotation query mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoxelAnnotationQueryMode {
    Cell,
    Bounds,
    Region,
    LayerSummary,
}

impl VoxelAnnotationQueryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            VoxelAnnotationQueryMode::Cell => "cell",
            VoxelAnnotationQueryMode::Bounds => "bounds",
            VoxelAnnotationQueryMode::Region => "region",
            VoxelAnnotationQueryMode::LayerSummary => "layer_summary",
        }
    }
}

/// Integer coordinate in stored voxel space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VoxelAnnotationCoord {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

/// Inclusive stored voxel-space bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationBounds {
    pub min: VoxelAnnotationCoord,
    pub max: VoxelAnnotationCoord,
}

/// One annotation membership run along +X. Absence means not selected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationSparseRun {
    pub start: VoxelAnnotationCoord,
    pub length: u32,
}

/// Compact annotation membership payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationSelection {
    pub sparse_runs: Vec<VoxelAnnotationSparseRun>,
}

/// Provenance/evidence reference for stored annotation layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationProvenanceRef {
    pub kind: VoxelAnnotationProvenanceKind,
    pub uri: String,
    pub content_hash: String,
}

/// Canonical hashes recorded with an annotation layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationContentHashes {
    pub canonical_json: String,
    pub membership_data: String,
}

/// One classified validation/runtime diagnostic for voxel annotations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationDiagnostic {
    pub code: VoxelAnnotationDiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub reference: String,
    pub message: String,
}

/// One semantic region inside a voxel annotation layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationRegion {
    pub region_id: String,
    pub label: String,
    pub kind: VoxelAnnotationKind,
    pub tags: Vec<String>,
    pub parent_region_id: Option<String>,
    pub bounds: VoxelAnnotationBounds,
    pub selection: VoxelAnnotationSelection,
}

/// A complete ASHA-native stored voxel annotation layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationLayer {
    pub layer_id: String,
    pub schema_version: u32,
    pub media_type: String,
    pub target_voxel_volume_asset_id: String,
    pub target_voxel_data_hash: String,
    pub target_bounds: VoxelAnnotationBounds,
    pub regions: Vec<VoxelAnnotationRegion>,
    pub provenance: Vec<VoxelAnnotationProvenanceRef>,
    pub content_hashes: VoxelAnnotationContentHashes,
    pub validation_diagnostics: Vec<VoxelAnnotationDiagnostic>,
}

/// Request to validate and canonicalize a stored annotation layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationLayerValidationRequest {
    pub layer: VoxelAnnotationLayer,
    pub expected_target_voxel_volume_asset_id: Option<String>,
    pub expected_target_voxel_data_hash: Option<String>,
    pub max_regions: u64,
    pub max_sparse_runs_per_region: u64,
    pub max_total_assigned_cells: u64,
}

/// Validation and canonicalization report for a stored annotation layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationLayerValidationReport {
    pub layer_id: String,
    pub valid: bool,
    pub canonical_json_hash: Option<String>,
    pub membership_data_hash: Option<String>,
    pub region_count: u64,
    pub sparse_run_count: u64,
    pub assigned_cell_count: u64,
    pub diagnostics: Vec<VoxelAnnotationDiagnostic>,
}

/// Explicit request to load a validated annotation layer into runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationLayerLoadRequest {
    pub layer: VoxelAnnotationLayer,
    pub target_grid: u64,
    pub replace_existing: bool,
    pub expected_session_hash: Option<String>,
}

/// Receipt/readback for loading an annotation layer into runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationLayerLoadReceipt {
    pub request_layer_id: String,
    pub loaded: bool,
    pub runtime_layer_id: Option<String>,
    pub target_voxel_volume_asset_id: String,
    pub target_voxel_data_hash: String,
    pub region_count: u64,
    pub assigned_cell_count: u64,
    pub layer_hash: Option<String>,
    pub session_hash: String,
    pub replay_hash: String,
    pub diagnostics: Vec<VoxelAnnotationDiagnostic>,
}

/// Request to query a loaded runtime annotation layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationQueryRequest {
    pub runtime_layer_id: Option<String>,
    pub layer_id: String,
    pub mode: VoxelAnnotationQueryMode,
    pub cell: Option<VoxelAnnotationCoord>,
    pub bounds: Option<VoxelAnnotationBounds>,
    pub region_id: Option<String>,
    pub max_regions: u64,
    pub expected_layer_hash: Option<String>,
}

/// Compact region readout returned by annotation queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationRegionReadout {
    pub region_id: String,
    pub label: String,
    pub kind: VoxelAnnotationKind,
    pub tags: Vec<String>,
    pub parent_region_id: Option<String>,
    pub bounds: VoxelAnnotationBounds,
    pub assigned_cell_count: u64,
}

/// Query readout for a loaded runtime annotation layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationQueryReadout {
    pub request: VoxelAnnotationQueryRequest,
    pub matched_regions: Vec<VoxelAnnotationRegionReadout>,
    pub region_count: u64,
    pub truncated: bool,
    pub layer_hash: Option<String>,
    pub diagnostics: Vec<VoxelAnnotationDiagnostic>,
}

/// Typed runtime annotation edit request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationEditRequest {
    pub runtime_layer_id: Option<String>,
    pub layer_id: String,
    pub expected_layer_hash: String,
    pub operation: VoxelAnnotationEditOperation,
    pub region_id: Option<String>,
    pub region: Option<VoxelAnnotationRegion>,
    pub sparse_runs: Vec<VoxelAnnotationSparseRun>,
    pub tags: Vec<String>,
    pub label: Option<String>,
    pub kind: Option<VoxelAnnotationKind>,
    pub parent_region_id: Option<String>,
}

/// Receipt for an accepted/rejected runtime annotation edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationEditReceipt {
    pub request: VoxelAnnotationEditRequest,
    pub edited: bool,
    pub layer_hash_before: String,
    pub layer_hash_after: Option<String>,
    pub region_count: u64,
    pub assigned_cell_count: u64,
    pub diagnostics: Vec<VoxelAnnotationDiagnostic>,
    pub replay_hash: String,
}

/// Request to export a runtime annotation layer back to stored DTO form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationLayerExportRequest {
    pub runtime_layer_id: Option<String>,
    pub layer_id: String,
    pub expected_layer_hash: String,
    pub include_diagnostics: bool,
}

/// Receipt for explicit runtime-to-stored annotation layer export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoxelAnnotationLayerExportReceipt {
    pub request: VoxelAnnotationLayerExportRequest,
    pub exported: bool,
    pub layer: Option<VoxelAnnotationLayer>,
    pub canonical_json: Option<String>,
    pub canonical_json_hash: Option<String>,
    pub membership_data_hash: Option<String>,
    pub diagnostics: Vec<VoxelAnnotationDiagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabulary_tables_are_nonempty_and_unique() {
        for table in [
            VOXEL_ANNOTATION_KINDS,
            VOXEL_ANNOTATION_PROVENANCE_KINDS,
            VOXEL_ANNOTATION_DIAGNOSTIC_CODES,
            VOXEL_ANNOTATION_EDIT_OPERATIONS,
            VOXEL_ANNOTATION_QUERY_MODES,
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
            [
                VoxelAnnotationKind::Selection,
                VoxelAnnotationKind::Room,
                VoxelAnnotationKind::Portal,
                VoxelAnnotationKind::SpawnArea,
                VoxelAnnotationKind::Cover,
                VoxelAnnotationKind::Hazard,
                VoxelAnnotationKind::NavHint,
                VoxelAnnotationKind::Custom,
            ]
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>(),
            VOXEL_ANNOTATION_KINDS
        );
        assert_eq!(
            [
                VoxelAnnotationProvenanceKind::Authored,
                VoxelAnnotationProvenanceKind::ImportedReference,
                VoxelAnnotationProvenanceKind::RuntimeExport,
                VoxelAnnotationProvenanceKind::Generated,
            ]
            .iter()
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>(),
            VOXEL_ANNOTATION_PROVENANCE_KINDS
        );
        assert_eq!(
            [
                VoxelAnnotationDiagnosticCode::UnsupportedSchemaVersion,
                VoxelAnnotationDiagnosticCode::UnsupportedMediaType,
                VoxelAnnotationDiagnosticCode::InvalidLayerId,
                VoxelAnnotationDiagnosticCode::InvalidTargetVoxelVolumeAssetId,
                VoxelAnnotationDiagnosticCode::TargetVoxelHashMismatch,
                VoxelAnnotationDiagnosticCode::InvalidBounds,
                VoxelAnnotationDiagnosticCode::InvalidRegionId,
                VoxelAnnotationDiagnosticCode::DuplicateRegionId,
                VoxelAnnotationDiagnosticCode::UnknownParentRegion,
                VoxelAnnotationDiagnosticCode::ParentCycle,
                VoxelAnnotationDiagnosticCode::UnsupportedAnnotationKind,
                VoxelAnnotationDiagnosticCode::InvalidSparseRun,
                VoxelAnnotationDiagnosticCode::DuplicateCell,
                VoxelAnnotationDiagnosticCode::RegionOutOfBounds,
                VoxelAnnotationDiagnosticCode::QuotaExceeded,
                VoxelAnnotationDiagnosticCode::StaleLayerHash,
                VoxelAnnotationDiagnosticCode::LayerNotLoaded,
                VoxelAnnotationDiagnosticCode::QueryOutOfBounds,
                VoxelAnnotationDiagnosticCode::EditConflict,
            ]
            .iter()
            .map(|code| code.as_str())
            .collect::<Vec<_>>(),
            VOXEL_ANNOTATION_DIAGNOSTIC_CODES
        );
        assert_eq!(
            [
                VoxelAnnotationEditOperation::UpsertRegion,
                VoxelAnnotationEditOperation::RemoveRegion,
                VoxelAnnotationEditOperation::AddRuns,
                VoxelAnnotationEditOperation::RemoveRuns,
                VoxelAnnotationEditOperation::ReplaceSelection,
                VoxelAnnotationEditOperation::SetParent,
                VoxelAnnotationEditOperation::SetTags,
                VoxelAnnotationEditOperation::SetLabel,
                VoxelAnnotationEditOperation::SetKind,
            ]
            .iter()
            .map(|operation| operation.as_str())
            .collect::<Vec<_>>(),
            VOXEL_ANNOTATION_EDIT_OPERATIONS
        );
        assert_eq!(
            [
                VoxelAnnotationQueryMode::Cell,
                VoxelAnnotationQueryMode::Bounds,
                VoxelAnnotationQueryMode::Region,
                VoxelAnnotationQueryMode::LayerSummary,
            ]
            .iter()
            .map(|mode| mode.as_str())
            .collect::<Vec<_>>(),
            VOXEL_ANNOTATION_QUERY_MODES
        );
    }

    #[test]
    fn layer_serializes_with_camel_case_fields_and_snake_case_vocabulary() {
        let layer = VoxelAnnotationLayer {
            layer_id: "voxel-annotation/demo/semantic.avann".to_string(),
            schema_version: VOXEL_ANNOTATION_SCHEMA_VERSION,
            media_type: VOXEL_ANNOTATION_MEDIA_TYPE.to_string(),
            target_voxel_volume_asset_id: "voxel-volume/demo/tunnel".to_string(),
            target_voxel_data_hash: "fnv1a64:target".to_string(),
            target_bounds: bounds(0, 0, 0, 3, 2, 1),
            regions: vec![VoxelAnnotationRegion {
                region_id: "region/spawn".to_string(),
                label: "Spawn".to_string(),
                kind: VoxelAnnotationKind::SpawnArea,
                tags: vec!["demo".to_string()],
                parent_region_id: None,
                bounds: bounds(0, 0, 0, 1, 0, 0),
                selection: VoxelAnnotationSelection {
                    sparse_runs: vec![VoxelAnnotationSparseRun {
                        start: coord(0, 0, 0),
                        length: 2,
                    }],
                },
            }],
            provenance: vec![VoxelAnnotationProvenanceRef {
                kind: VoxelAnnotationProvenanceKind::Authored,
                uri: "file://project/annotations/spawn.avann.json".to_string(),
                content_hash: "sha256:source".to_string(),
            }],
            content_hashes: VoxelAnnotationContentHashes {
                canonical_json: "sha256:canonical".to_string(),
                membership_data: "sha256:membership".to_string(),
            },
            validation_diagnostics: vec![VoxelAnnotationDiagnostic {
                code: VoxelAnnotationDiagnosticCode::QuotaExceeded,
                severity: DiagnosticSeverity::Warning,
                reference: "regions[0]".to_string(),
                message: "near quota".to_string(),
            }],
        };

        let value = serde_json::to_value(&layer).expect("serializes");
        assert_eq!(value["layerId"], "voxel-annotation/demo/semantic.avann");
        assert_eq!(
            value["targetVoxelVolumeAssetId"],
            "voxel-volume/demo/tunnel"
        );
        assert_eq!(value["targetVoxelDataHash"], "fnv1a64:target");
        assert_eq!(value["regions"][0]["kind"], "spawn_area");
        assert_eq!(
            value["regions"][0]["selection"]["sparseRuns"][0]["length"],
            2
        );
        assert_eq!(
            value["contentHashes"]["membershipData"],
            "sha256:membership"
        );
        assert_eq!(value["validationDiagnostics"][0]["code"], "quota_exceeded");
    }

    fn coord(x: i64, y: i64, z: i64) -> VoxelAnnotationCoord {
        VoxelAnnotationCoord { x, y, z }
    }

    fn bounds(
        min_x: i64,
        min_y: i64,
        min_z: i64,
        max_x: i64,
        max_y: i64,
        max_z: i64,
    ) -> VoxelAnnotationBounds {
        VoxelAnnotationBounds {
            min: coord(min_x, min_y, min_z),
            max: coord(max_x, max_y, max_z),
        }
    }
}
