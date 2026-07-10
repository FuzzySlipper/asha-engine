//! Rust authority service for Asha-native voxel annotation layers.
//!
//! # Lane
//!
//! `rust-service` - validates, canonicalizes, hashes, decodes, and queries
//! stored voxel annotation DTOs. Runtime bridge verbs and Studio UI are outside
//! this crate; consumers submit typed protocol shapes and receive typed reports.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use core_assets::{AssetId, AssetKind};
use protocol_diagnostics::DiagnosticSeverity;
use protocol_voxel_annotation::{
    VoxelAnnotationBounds, VoxelAnnotationContentHashes, VoxelAnnotationCoord,
    VoxelAnnotationDiagnostic, VoxelAnnotationDiagnosticCode, VoxelAnnotationLayer,
    VoxelAnnotationLayerDraft, VoxelAnnotationLayerValidationInput,
    VoxelAnnotationLayerValidationReport, VoxelAnnotationLayerValidationRequest,
    VoxelAnnotationQueryMode, VoxelAnnotationQueryReadout, VoxelAnnotationQueryRequest,
    VoxelAnnotationRegion, VoxelAnnotationRegionReadout, VoxelAnnotationSparseRun,
    VOXEL_ANNOTATION_KINDS, VOXEL_ANNOTATION_MEDIA_TYPE, VOXEL_ANNOTATION_SCHEMA_VERSION,
};

pub const DEFAULT_MAX_REGIONS: u64 = 4096;
pub const DEFAULT_MAX_SPARSE_RUNS_PER_REGION: u64 = 16_384;
pub const DEFAULT_MAX_TOTAL_ASSIGNED_CELLS: u64 = 8_388_608;
pub const DEFAULT_MAX_TAGS_PER_REGION: usize = 32;
pub const DEFAULT_MAX_LABEL_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoxelAnnotationDecodeError {
    Json(String),
    Invalid(Box<VoxelAnnotationLayerValidationReport>),
}

impl std::fmt::Display for VoxelAnnotationDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoxelAnnotationDecodeError::Json(message) => {
                write!(f, "invalid voxel annotation JSON: {message}")
            }
            VoxelAnnotationDecodeError::Invalid(report) => {
                write!(
                    f,
                    "voxel annotation layer failed validation with {} diagnostic(s)",
                    report.diagnostics.len()
                )
            }
        }
    }
}

impl std::error::Error for VoxelAnnotationDecodeError {}

/// Validate a stored annotation layer and compute authority hashes.
pub fn validate_layer(
    request: &VoxelAnnotationLayerValidationRequest,
) -> VoxelAnnotationLayerValidationReport {
    let (layer, validate_hashes) = validation_input_layer(&request.input);
    let mut diagnostics = Vec::new();

    validate_version_and_media(&layer, &mut diagnostics);
    validate_layer_id(&layer, &mut diagnostics);
    validate_target(request, &layer, &mut diagnostics);
    validate_bounds("targetBounds", &layer.target_bounds, &mut diagnostics);

    let mut region_ids = BTreeSet::new();
    let mut parent_by_region = BTreeMap::new();
    let mut sparse_run_count = 0u64;
    let mut assigned_cell_count = 0u64;

    validate_region_quota(request, &layer, &mut diagnostics);
    for (index, region) in layer.regions.iter().enumerate() {
        validate_region_header(index, region, &mut region_ids, &mut diagnostics);
        parent_by_region.insert(region.region_id.clone(), region.parent_region_id.clone());
        validate_region_bounds(index, &layer, region, &mut diagnostics);
        validate_region_selection(
            index,
            request,
            &layer,
            region,
            &mut sparse_run_count,
            &mut assigned_cell_count,
            &mut diagnostics,
        );
    }
    validate_parent_tree(&parent_by_region, &mut diagnostics);
    if validate_hashes {
        validate_content_hashes(&layer, &mut diagnostics);
    }

    let valid = diagnostics_are_valid(&diagnostics);
    let normalized_layer = valid.then(|| with_computed_hashes(&layer));
    let canonical_json_hash = normalized_layer
        .as_ref()
        .map(|layer| layer.content_hashes.canonical_json.clone());
    let membership_data_hash = normalized_layer
        .as_ref()
        .map(|layer| layer.content_hashes.membership_data.clone());

    VoxelAnnotationLayerValidationReport {
        layer_id: layer.layer_id.clone(),
        valid,
        normalized_layer,
        canonical_json_hash,
        membership_data_hash,
        region_count: layer.regions.len() as u64,
        sparse_run_count,
        assigned_cell_count,
        diagnostics,
    }
}

/// Return a copy with authority-computed hashes populated.
pub fn with_computed_hashes(layer: &VoxelAnnotationLayer) -> VoxelAnnotationLayer {
    let mut normalized = layer.clone();
    normalized.content_hashes = VoxelAnnotationContentHashes {
        canonical_json: String::new(),
        membership_data: String::new(),
    };
    normalized.content_hashes = VoxelAnnotationContentHashes {
        canonical_json: canonical_json_hash(&normalized),
        membership_data: membership_data_hash(&normalized),
    };
    normalized
}

fn validation_input_layer(
    input: &VoxelAnnotationLayerValidationInput,
) -> (VoxelAnnotationLayer, bool) {
    match input {
        VoxelAnnotationLayerValidationInput::Draft { draft } => (layer_from_draft(draft), false),
        VoxelAnnotationLayerValidationInput::Finalized { layer } => (layer.clone(), true),
    }
}

fn layer_from_draft(draft: &VoxelAnnotationLayerDraft) -> VoxelAnnotationLayer {
    VoxelAnnotationLayer {
        layer_id: draft.layer_id.clone(),
        schema_version: draft.schema_version,
        media_type: draft.media_type.clone(),
        target_voxel_volume_asset_id: draft.target_voxel_volume_asset_id.clone(),
        target_voxel_data_hash: draft.target_voxel_data_hash.clone(),
        target_bounds: draft.target_bounds,
        regions: draft.regions.clone(),
        provenance: draft.provenance.clone(),
        content_hashes: VoxelAnnotationContentHashes {
            canonical_json: String::new(),
            membership_data: String::new(),
        },
        validation_diagnostics: Vec::new(),
    }
}

/// Encode canonical JSON after validation.
pub fn encode_layer(
    request: &VoxelAnnotationLayerValidationRequest,
) -> Result<String, Box<VoxelAnnotationLayerValidationReport>> {
    let report = validate_layer(request);
    if !report.valid {
        return Err(Box::new(report));
    }
    Ok(canonical_json(
        report
            .normalized_layer
            .as_ref()
            .expect("valid validation report has a normalized layer"),
    ))
}

/// Decode JSON and validate before returning the annotation layer.
pub fn decode_layer(
    text: &str,
    expected_target_voxel_volume_asset_id: Option<String>,
    expected_target_voxel_data_hash: Option<String>,
) -> Result<VoxelAnnotationLayer, VoxelAnnotationDecodeError> {
    let layer: VoxelAnnotationLayer =
        serde_json::from_str(text).map_err(|e| VoxelAnnotationDecodeError::Json(e.to_string()))?;
    let request = VoxelAnnotationLayerValidationRequest {
        input: VoxelAnnotationLayerValidationInput::Finalized { layer },
        expected_target_voxel_volume_asset_id,
        expected_target_voxel_data_hash,
        max_regions: DEFAULT_MAX_REGIONS,
        max_sparse_runs_per_region: DEFAULT_MAX_SPARSE_RUNS_PER_REGION,
        max_total_assigned_cells: DEFAULT_MAX_TOTAL_ASSIGNED_CELLS,
    };
    let report = validate_layer(&request);
    if report.valid {
        Ok(report
            .normalized_layer
            .expect("valid validation report has a normalized layer"))
    } else {
        Err(VoxelAnnotationDecodeError::Invalid(Box::new(report)))
    }
}

/// Query one validated annotation layer for cells, bounds, region ids, or summary.
pub fn query_layer(
    layer: &VoxelAnnotationLayer,
    request: &VoxelAnnotationQueryRequest,
) -> VoxelAnnotationQueryReadout {
    let validation_request = VoxelAnnotationLayerValidationRequest {
        input: VoxelAnnotationLayerValidationInput::Finalized {
            layer: layer.clone(),
        },
        expected_target_voxel_volume_asset_id: Some(layer.target_voxel_volume_asset_id.clone()),
        expected_target_voxel_data_hash: Some(layer.target_voxel_data_hash.clone()),
        max_regions: DEFAULT_MAX_REGIONS,
        max_sparse_runs_per_region: DEFAULT_MAX_SPARSE_RUNS_PER_REGION,
        max_total_assigned_cells: DEFAULT_MAX_TOTAL_ASSIGNED_CELLS,
    };
    let validation = validate_layer(&validation_request);
    let layer_hash = canonical_json_hash(layer);
    let mut diagnostics = validation.diagnostics;
    validate_query_request(layer, request, &layer_hash, &mut diagnostics);

    let mut matched_regions = if diagnostics_are_valid(&diagnostics) {
        match request.mode {
            VoxelAnnotationQueryMode::Cell => match request.cell {
                Some(cell) => layer
                    .regions
                    .iter()
                    .filter(|region| region_contains_cell(region, cell))
                    .map(region_readout)
                    .collect(),
                None => Vec::new(),
            },
            VoxelAnnotationQueryMode::Bounds => match request.bounds {
                Some(bounds) => layer
                    .regions
                    .iter()
                    .filter(|region| region_intersects_bounds(region, &bounds))
                    .map(region_readout)
                    .collect(),
                None => Vec::new(),
            },
            VoxelAnnotationQueryMode::Region => match request.region_id.as_deref() {
                Some(region_id) => layer
                    .regions
                    .iter()
                    .filter(|region| region.region_id == region_id)
                    .map(region_readout)
                    .collect(),
                None => Vec::new(),
            },
            VoxelAnnotationQueryMode::LayerSummary => {
                layer.regions.iter().map(region_readout).collect()
            }
        }
    } else {
        Vec::new()
    };

    let truncated = request.max_regions > 0 && matched_regions.len() as u64 > request.max_regions;
    if truncated {
        matched_regions.truncate(request.max_regions as usize);
    }

    VoxelAnnotationQueryReadout {
        request: request.clone(),
        matched_regions,
        region_count: layer.regions.len() as u64,
        truncated,
        layer_hash: Some(layer_hash),
        diagnostics,
    }
}

fn validate_version_and_media(
    layer: &VoxelAnnotationLayer,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    if layer.schema_version != VOXEL_ANNOTATION_SCHEMA_VERSION {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::UnsupportedSchemaVersion,
            "schemaVersion",
            format!(
                "schema version {} is not supported; expected {}",
                layer.schema_version, VOXEL_ANNOTATION_SCHEMA_VERSION
            ),
        ));
    }
    if layer.media_type != VOXEL_ANNOTATION_MEDIA_TYPE {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::UnsupportedMediaType,
            "mediaType",
            format!(
                "media type {:?} is not supported; expected {VOXEL_ANNOTATION_MEDIA_TYPE}",
                layer.media_type
            ),
        ));
    }
}

fn validate_layer_id(
    layer: &VoxelAnnotationLayer,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    if !layer.layer_id.starts_with("voxel-annotation/") || layer.layer_id.len() <= 17 {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::InvalidLayerId,
            "layerId",
            "annotation layer id must use the voxel-annotation/ prefix",
        ));
    }
}

fn validate_target(
    request: &VoxelAnnotationLayerValidationRequest,
    layer: &VoxelAnnotationLayer,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    match AssetId::parse(&layer.target_voxel_volume_asset_id) {
        Ok(id) if id.kind() == AssetKind::VoxelVolume => {}
        Ok(id) => diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::InvalidTargetVoxelVolumeAssetId,
            "targetVoxelVolumeAssetId",
            format!(
                "target asset id {:?} has kind {}; expected voxel-volume",
                layer.target_voxel_volume_asset_id,
                id.kind()
            ),
        )),
        Err(e) => diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::InvalidTargetVoxelVolumeAssetId,
            "targetVoxelVolumeAssetId",
            format!(
                "target asset id {:?} is invalid: {e}",
                layer.target_voxel_volume_asset_id
            ),
        )),
    }
    if let Some(expected) = &request.expected_target_voxel_volume_asset_id {
        if expected != &layer.target_voxel_volume_asset_id {
            diagnostics.push(diagnostic(
                VoxelAnnotationDiagnosticCode::InvalidTargetVoxelVolumeAssetId,
                "expectedTargetVoxelVolumeAssetId",
                "expected target voxel volume asset id does not match the layer",
            ));
        }
    }
    if layer.target_voxel_data_hash.is_empty() {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::TargetVoxelHashMismatch,
            "targetVoxelDataHash",
            "target voxel-data hash is required",
        ));
    }
    if let Some(expected) = &request.expected_target_voxel_data_hash {
        if expected != &layer.target_voxel_data_hash {
            diagnostics.push(diagnostic(
                VoxelAnnotationDiagnosticCode::TargetVoxelHashMismatch,
                "expectedTargetVoxelDataHash",
                "target voxel-data hash does not match the authority-visible voxel asset",
            ));
        }
    }
}

fn validate_bounds(
    reference: impl Into<String>,
    bounds: &VoxelAnnotationBounds,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    if bounds.min.x > bounds.max.x || bounds.min.y > bounds.max.y || bounds.min.z > bounds.max.z {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::InvalidBounds,
            reference,
            "inclusive bounds require min <= max on every axis",
        ));
    }
}

fn validate_region_quota(
    request: &VoxelAnnotationLayerValidationRequest,
    layer: &VoxelAnnotationLayer,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    if layer.regions.len() as u64 > request.max_regions {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::QuotaExceeded,
            "regions",
            format!(
                "region count {} exceeds maxRegions {}",
                layer.regions.len(),
                request.max_regions
            ),
        ));
    }
}

fn validate_region_header(
    index: usize,
    region: &VoxelAnnotationRegion,
    region_ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    if region.region_id.trim().is_empty() {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::InvalidRegionId,
            format!("regions[{index}].regionId"),
            "region id must be non-empty",
        ));
    } else if !region_ids.insert(region.region_id.clone()) {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::DuplicateRegionId,
            format!("regions[{index}].regionId"),
            format!("region id {:?} appears more than once", region.region_id),
        ));
    }
    if region.label.is_empty() || region.label.len() > DEFAULT_MAX_LABEL_BYTES {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::QuotaExceeded,
            format!("regions[{index}].label"),
            format!("label must be 1..={DEFAULT_MAX_LABEL_BYTES} UTF-8 bytes"),
        ));
    }
    if region.tags.len() > DEFAULT_MAX_TAGS_PER_REGION {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::QuotaExceeded,
            format!("regions[{index}].tags"),
            format!("tag count exceeds {DEFAULT_MAX_TAGS_PER_REGION}"),
        ));
    }
    let mut sorted_tags = region.tags.clone();
    sorted_tags.sort();
    sorted_tags.dedup();
    if sorted_tags != region.tags {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::InvalidRegionId,
            format!("regions[{index}].tags"),
            "tags must be sorted and unique",
        ));
    }
    if !VOXEL_ANNOTATION_KINDS.contains(&region.kind.as_str()) {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::UnsupportedAnnotationKind,
            format!("regions[{index}].kind"),
            format!("unsupported annotation kind {:?}", region.kind),
        ));
    }
}

fn validate_region_bounds(
    index: usize,
    layer: &VoxelAnnotationLayer,
    region: &VoxelAnnotationRegion,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    validate_bounds(
        format!("regions[{index}].bounds"),
        &region.bounds,
        diagnostics,
    );
    if !bounds_contains_bounds(&layer.target_bounds, &region.bounds) {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::RegionOutOfBounds,
            format!("regions[{index}].bounds"),
            "region bounds must stay inside targetBounds",
        ));
    }
}

fn validate_region_selection(
    index: usize,
    request: &VoxelAnnotationLayerValidationRequest,
    layer: &VoxelAnnotationLayer,
    region: &VoxelAnnotationRegion,
    sparse_run_count: &mut u64,
    assigned_cell_count: &mut u64,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    if region.selection.sparse_runs.len() as u64 > request.max_sparse_runs_per_region {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::QuotaExceeded,
            format!("regions[{index}].selection.sparseRuns"),
            "region sparse run count exceeds maxSparseRunsPerRegion",
        ));
    }
    let mut occupied = BTreeSet::new();
    let mut previous_key: Option<(i64, i64, i64)> = None;
    for (run_index, run) in region.selection.sparse_runs.iter().enumerate() {
        *sparse_run_count += 1;
        let reference = format!("regions[{index}].selection.sparseRuns[{run_index}]");
        validate_sparse_run_order(run, previous_key, &reference, diagnostics);
        previous_key = Some(run_key(run));
        let Some(end_x) = run
            .start
            .x
            .checked_add(i64::from(run.length).saturating_sub(1))
        else {
            diagnostics.push(diagnostic(
                VoxelAnnotationDiagnosticCode::InvalidSparseRun,
                reference,
                "sparse run end coordinate overflowed",
            ));
            continue;
        };
        if run.length == 0 {
            diagnostics.push(diagnostic(
                VoxelAnnotationDiagnosticCode::InvalidSparseRun,
                reference,
                "sparse run length must be greater than zero",
            ));
            continue;
        }
        let run_bounds = VoxelAnnotationBounds {
            min: run.start,
            max: VoxelAnnotationCoord {
                x: end_x,
                y: run.start.y,
                z: run.start.z,
            },
        };
        if !bounds_contains_bounds(&layer.target_bounds, &run_bounds) {
            diagnostics.push(diagnostic(
                VoxelAnnotationDiagnosticCode::RegionOutOfBounds,
                &reference,
                "sparse run must stay inside targetBounds",
            ));
        }
        if !bounds_contains_bounds(&region.bounds, &run_bounds) {
            diagnostics.push(diagnostic(
                VoxelAnnotationDiagnosticCode::InvalidBounds,
                &reference,
                "region bounds must enclose every selected cell",
            ));
        }
        for x in run.start.x..=end_x {
            let coord = VoxelAnnotationCoord {
                x,
                y: run.start.y,
                z: run.start.z,
            };
            if !occupied.insert(coord) {
                diagnostics.push(diagnostic(
                    VoxelAnnotationDiagnosticCode::DuplicateCell,
                    &reference,
                    format!(
                        "voxel coordinate ({}, {}, {}) is assigned more than once in region {:?}",
                        coord.x, coord.y, coord.z, region.region_id
                    ),
                ));
            }
        }
        *assigned_cell_count = assigned_cell_count.saturating_add(u64::from(run.length));
    }
    if *assigned_cell_count > request.max_total_assigned_cells {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::QuotaExceeded,
            "regions.selection",
            "total assigned cell count exceeds maxTotalAssignedCells",
        ));
    }
}

fn validate_sparse_run_order(
    run: &VoxelAnnotationSparseRun,
    previous_key: Option<(i64, i64, i64)>,
    reference: &str,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    let key = run_key(run);
    if previous_key.is_some_and(|previous| key < previous) {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::InvalidSparseRun,
            reference,
            "sparse runs must be sorted by z, then y, then x",
        ));
    }
}

fn validate_parent_tree(
    parent_by_region: &BTreeMap<String, Option<String>>,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    for (region_id, parent) in parent_by_region {
        let Some(parent_id) = parent else {
            continue;
        };
        if !parent_by_region.contains_key(parent_id) {
            diagnostics.push(diagnostic(
                VoxelAnnotationDiagnosticCode::UnknownParentRegion,
                format!("regions[{region_id}].parentRegionId"),
                format!("parent region {:?} is not in this layer", parent_id),
            ));
            continue;
        }
        let mut seen = BTreeSet::new();
        let mut cursor = Some(region_id.as_str());
        while let Some(current) = cursor {
            if !seen.insert(current.to_string()) {
                diagnostics.push(diagnostic(
                    VoxelAnnotationDiagnosticCode::ParentCycle,
                    format!("regions[{region_id}].parentRegionId"),
                    "parentRegionId chain contains a cycle",
                ));
                break;
            }
            cursor = parent_by_region
                .get(current)
                .and_then(|parent| parent.as_deref());
        }
    }
}

fn validate_content_hashes(
    layer: &VoxelAnnotationLayer,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    let canonical_json_hash = canonical_json_hash(layer);
    let membership_data_hash = membership_data_hash(layer);
    if layer.content_hashes.canonical_json.is_empty() {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::StaleLayerHash,
            "contentHashes.canonicalJson",
            "canonical JSON hash is required",
        ));
    } else if layer.content_hashes.canonical_json != canonical_json_hash {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::StaleLayerHash,
            "contentHashes.canonicalJson",
            "canonical JSON hash does not match authority-computed hash",
        ));
    }
    if layer.content_hashes.membership_data.is_empty() {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::StaleLayerHash,
            "contentHashes.membershipData",
            "membership data hash is required",
        ));
    } else if layer.content_hashes.membership_data != membership_data_hash {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::StaleLayerHash,
            "contentHashes.membershipData",
            "membership data hash does not match authority-computed hash",
        ));
    }
}

fn validate_query_request(
    layer: &VoxelAnnotationLayer,
    request: &VoxelAnnotationQueryRequest,
    layer_hash: &str,
    diagnostics: &mut Vec<VoxelAnnotationDiagnostic>,
) {
    if request.layer_id != layer.layer_id {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::LayerNotLoaded,
            "layerId",
            "query layer id does not match the loaded annotation layer",
        ));
    }
    if request
        .expected_layer_hash
        .as_deref()
        .is_some_and(|expected| expected != layer_hash)
    {
        diagnostics.push(diagnostic(
            VoxelAnnotationDiagnosticCode::StaleLayerHash,
            "expectedLayerHash",
            "query expected a different annotation layer hash",
        ));
    }
    match request.mode {
        VoxelAnnotationQueryMode::Cell => match request.cell {
            Some(cell) if bounds_contains_coord(&layer.target_bounds, cell) => {}
            Some(_) => diagnostics.push(diagnostic(
                VoxelAnnotationDiagnosticCode::QueryOutOfBounds,
                "cell",
                "cell query must stay inside targetBounds",
            )),
            None => diagnostics.push(diagnostic(
                VoxelAnnotationDiagnosticCode::QueryOutOfBounds,
                "cell",
                "cell query requires cell",
            )),
        },
        VoxelAnnotationQueryMode::Bounds => match request.bounds {
            Some(bounds) if bounds_contains_bounds(&layer.target_bounds, &bounds) => {}
            Some(_) => diagnostics.push(diagnostic(
                VoxelAnnotationDiagnosticCode::QueryOutOfBounds,
                "bounds",
                "bounds query must stay inside targetBounds",
            )),
            None => diagnostics.push(diagnostic(
                VoxelAnnotationDiagnosticCode::QueryOutOfBounds,
                "bounds",
                "bounds query requires bounds",
            )),
        },
        VoxelAnnotationQueryMode::Region => {
            if request.region_id.is_none() {
                diagnostics.push(diagnostic(
                    VoxelAnnotationDiagnosticCode::InvalidRegionId,
                    "regionId",
                    "region query requires regionId",
                ));
            }
        }
        VoxelAnnotationQueryMode::LayerSummary => {}
    }
}

fn canonical_json_hash(layer: &VoxelAnnotationLayer) -> String {
    let mut normalized = layer.clone();
    normalized.content_hashes.canonical_json.clear();
    normalized.content_hashes.membership_data.clear();
    format!(
        "fnv1a64:{:016x}",
        fnv1a64(canonical_json(&normalized).as_bytes())
    )
}

fn membership_data_hash(layer: &VoxelAnnotationLayer) -> String {
    let mut hash = FNV_OFFSET;
    for region in &layer.regions {
        feed_str(&mut hash, &region.region_id);
        for run in &region.selection.sparse_runs {
            feed_i64(&mut hash, run.start.x);
            feed_i64(&mut hash, run.start.y);
            feed_i64(&mut hash, run.start.z);
            feed_u32(&mut hash, run.length);
        }
    }
    format!("fnv1a64:{hash:016x}")
}

fn canonical_json(layer: &VoxelAnnotationLayer) -> String {
    let value = serde_json::to_value(layer).expect("voxel annotation DTO serializes");
    let canonical = canonical_value(value);
    serde_json::to_string_pretty(&canonical).expect("canonical JSON serializes") + "\n"
}

fn canonical_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(canonical_value).collect())
        }
        serde_json::Value::Object(map) => {
            let ordered: BTreeMap<String, serde_json::Value> = map
                .into_iter()
                .map(|(key, value)| (key, canonical_value(value)))
                .collect();
            serde_json::Value::Object(ordered.into_iter().collect())
        }
        other => other,
    }
}

fn region_readout(region: &VoxelAnnotationRegion) -> VoxelAnnotationRegionReadout {
    VoxelAnnotationRegionReadout {
        region_id: region.region_id.clone(),
        label: region.label.clone(),
        kind: region.kind,
        tags: region.tags.clone(),
        parent_region_id: region.parent_region_id.clone(),
        bounds: region.bounds,
        assigned_cell_count: region
            .selection
            .sparse_runs
            .iter()
            .map(|run| u64::from(run.length))
            .sum(),
    }
}

fn region_contains_cell(region: &VoxelAnnotationRegion, cell: VoxelAnnotationCoord) -> bool {
    region.selection.sparse_runs.iter().any(|run| {
        let end_x = run.start.x + i64::from(run.length) - 1;
        cell.y == run.start.y && cell.z == run.start.z && cell.x >= run.start.x && cell.x <= end_x
    })
}

fn region_intersects_bounds(
    region: &VoxelAnnotationRegion,
    bounds: &VoxelAnnotationBounds,
) -> bool {
    region.selection.sparse_runs.iter().any(|run| {
        let end_x = run.start.x + i64::from(run.length) - 1;
        run.start.z >= bounds.min.z
            && run.start.z <= bounds.max.z
            && run.start.y >= bounds.min.y
            && run.start.y <= bounds.max.y
            && end_x >= bounds.min.x
            && run.start.x <= bounds.max.x
    })
}

fn bounds_contains_bounds(outer: &VoxelAnnotationBounds, inner: &VoxelAnnotationBounds) -> bool {
    bounds_contains_coord(outer, inner.min) && bounds_contains_coord(outer, inner.max)
}

fn bounds_contains_coord(bounds: &VoxelAnnotationBounds, coord: VoxelAnnotationCoord) -> bool {
    coord.x >= bounds.min.x
        && coord.x <= bounds.max.x
        && coord.y >= bounds.min.y
        && coord.y <= bounds.max.y
        && coord.z >= bounds.min.z
        && coord.z <= bounds.max.z
}

fn run_key(run: &VoxelAnnotationSparseRun) -> (i64, i64, i64) {
    (run.start.z, run.start.y, run.start.x)
}

fn diagnostics_are_valid(diagnostics: &[VoxelAnnotationDiagnostic]) -> bool {
    !diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.severity,
            DiagnosticSeverity::Error | DiagnosticSeverity::Fatal
        )
    })
}

fn diagnostic(
    code: VoxelAnnotationDiagnosticCode,
    reference: impl Into<String>,
    message: impl Into<String>,
) -> VoxelAnnotationDiagnostic {
    VoxelAnnotationDiagnostic {
        code,
        severity: DiagnosticSeverity::Error,
        reference: reference.into(),
        message: message.into(),
    }
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for byte in bytes {
        h ^= u64::from(*byte);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

fn feed_str(hash: &mut u64, value: &str) {
    for byte in value.as_bytes() {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
    *hash ^= 0xff;
    *hash = hash.wrapping_mul(FNV_PRIME);
}

fn feed_i64(hash: &mut u64, value: i64) {
    for byte in value.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

fn feed_u32(hash: &mut u64, value: u32) {
    for byte in value.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_voxel_annotation::{
        VoxelAnnotationContentHashes, VoxelAnnotationKind, VoxelAnnotationProvenanceKind,
        VoxelAnnotationProvenanceRef, VoxelAnnotationSelection, VOXEL_ANNOTATION_MEDIA_TYPE,
    };

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

    fn request(layer: VoxelAnnotationLayer) -> VoxelAnnotationLayerValidationRequest {
        VoxelAnnotationLayerValidationRequest {
            input: VoxelAnnotationLayerValidationInput::Finalized { layer },
            expected_target_voxel_volume_asset_id: Some("voxel-volume/test-room".to_string()),
            expected_target_voxel_data_hash: Some("fnv1a64:target".to_string()),
            max_regions: DEFAULT_MAX_REGIONS,
            max_sparse_runs_per_region: DEFAULT_MAX_SPARSE_RUNS_PER_REGION,
            max_total_assigned_cells: DEFAULT_MAX_TOTAL_ASSIGNED_CELLS,
        }
    }

    fn base_layer() -> VoxelAnnotationLayer {
        VoxelAnnotationLayer {
            layer_id: "voxel-annotation/test-room/semantic".to_string(),
            schema_version: VOXEL_ANNOTATION_SCHEMA_VERSION,
            media_type: VOXEL_ANNOTATION_MEDIA_TYPE.to_string(),
            target_voxel_volume_asset_id: "voxel-volume/test-room".to_string(),
            target_voxel_data_hash: "fnv1a64:target".to_string(),
            target_bounds: bounds(0, 0, 0, 9, 3, 3),
            regions: vec![
                VoxelAnnotationRegion {
                    region_id: "region/spawn".to_string(),
                    label: "Spawn".to_string(),
                    kind: VoxelAnnotationKind::SpawnArea,
                    tags: vec!["a".to_string(), "entry".to_string()],
                    parent_region_id: None,
                    bounds: bounds(1, 1, 1, 3, 1, 1),
                    selection: VoxelAnnotationSelection {
                        sparse_runs: vec![VoxelAnnotationSparseRun {
                            start: coord(1, 1, 1),
                            length: 3,
                        }],
                    },
                },
                VoxelAnnotationRegion {
                    region_id: "region/cover".to_string(),
                    label: "Cover".to_string(),
                    kind: VoxelAnnotationKind::Cover,
                    tags: vec!["b".to_string()],
                    parent_region_id: Some("region/spawn".to_string()),
                    bounds: bounds(5, 1, 1, 6, 1, 1),
                    selection: VoxelAnnotationSelection {
                        sparse_runs: vec![VoxelAnnotationSparseRun {
                            start: coord(5, 1, 1),
                            length: 2,
                        }],
                    },
                },
            ],
            provenance: vec![VoxelAnnotationProvenanceRef {
                kind: VoxelAnnotationProvenanceKind::Authored,
                uri: "asha://test/annotation".to_string(),
                content_hash: "fnv1a64:authoring".to_string(),
            }],
            content_hashes: VoxelAnnotationContentHashes {
                canonical_json: String::new(),
                membership_data: String::new(),
            },
            validation_diagnostics: Vec::new(),
        }
    }

    fn draft(layer: &VoxelAnnotationLayer) -> VoxelAnnotationLayerDraft {
        VoxelAnnotationLayerDraft {
            layer_id: layer.layer_id.clone(),
            schema_version: layer.schema_version,
            media_type: layer.media_type.clone(),
            target_voxel_volume_asset_id: layer.target_voxel_volume_asset_id.clone(),
            target_voxel_data_hash: layer.target_voxel_data_hash.clone(),
            target_bounds: layer.target_bounds,
            regions: layer.regions.clone(),
            provenance: layer.provenance.clone(),
        }
    }

    #[test]
    fn normalizes_unhashed_draft_and_rejects_wrong_finalized_hashes() {
        let draft_request = VoxelAnnotationLayerValidationRequest {
            input: VoxelAnnotationLayerValidationInput::Draft {
                draft: draft(&base_layer()),
            },
            expected_target_voxel_volume_asset_id: Some("voxel-volume/test-room".to_string()),
            expected_target_voxel_data_hash: Some("fnv1a64:target".to_string()),
            max_regions: DEFAULT_MAX_REGIONS,
            max_sparse_runs_per_region: DEFAULT_MAX_SPARSE_RUNS_PER_REGION,
            max_total_assigned_cells: DEFAULT_MAX_TOTAL_ASSIGNED_CELLS,
        };
        let report = validate_layer(&draft_request);
        assert!(report.valid, "{:?}", report.diagnostics);
        let normalized = report.normalized_layer.expect("normalized layer");
        assert_eq!(
            report.canonical_json_hash.as_deref(),
            Some(normalized.content_hashes.canonical_json.as_str())
        );
        assert_eq!(
            report.membership_data_hash.as_deref(),
            Some(normalized.content_hashes.membership_data.as_str())
        );

        let mut wrong = normalized;
        wrong.content_hashes.canonical_json = "fnv1a64:wrong".to_string();
        let wrong_report = validate_layer(&request(wrong));
        assert!(!wrong_report.valid);
        assert!(wrong_report.normalized_layer.is_none());
        assert!(wrong_report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == VoxelAnnotationDiagnosticCode::StaleLayerHash
        }));
    }

    #[test]
    fn validates_hashes_and_round_trips_canonical_json() {
        let layer = with_computed_hashes(&base_layer());
        let report = validate_layer(&request(layer.clone()));
        assert!(report.valid, "{:?}", report.diagnostics);
        assert_eq!(
            layer.content_hashes.canonical_json,
            report.canonical_json_hash.unwrap()
        );
        assert_eq!(
            layer.content_hashes.membership_data,
            report.membership_data_hash.unwrap()
        );

        let encoded = encode_layer(&request(layer.clone())).expect("encode");
        let decoded = decode_layer(
            &encoded,
            Some("voxel-volume/test-room".to_string()),
            Some("fnv1a64:target".to_string()),
        )
        .expect("decode");
        assert_eq!(decoded, layer);
    }

    #[test]
    fn stale_target_hash_fails_closed() {
        let layer = with_computed_hashes(&base_layer());
        let mut request = request(layer);
        request.expected_target_voxel_data_hash = Some("fnv1a64:stale".to_string());

        let report = validate_layer(&request);
        assert!(!report.valid);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == VoxelAnnotationDiagnosticCode::TargetVoxelHashMismatch
        }));
    }

    #[test]
    fn out_of_bounds_duplicate_cycles_and_quota_are_classified() {
        let mut layer = base_layer();
        layer.regions[0]
            .selection
            .sparse_runs
            .push(VoxelAnnotationSparseRun {
                start: coord(2, 1, 1),
                length: 2,
            });
        layer.regions[0].parent_region_id = Some("region/cover".to_string());
        layer.regions[1].parent_region_id = Some("region/spawn".to_string());
        layer.regions[1].selection.sparse_runs[0].start = coord(9, 1, 1);
        layer = with_computed_hashes(&layer);
        let mut request = request(layer);
        request.max_regions = 1;

        let report = validate_layer(&request);
        let codes: BTreeSet<_> = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert!(codes.contains(&VoxelAnnotationDiagnosticCode::DuplicateCell));
        assert!(codes.contains(&VoxelAnnotationDiagnosticCode::ParentCycle));
        assert!(codes.contains(&VoxelAnnotationDiagnosticCode::RegionOutOfBounds));
        assert!(codes.contains(&VoxelAnnotationDiagnosticCode::QuotaExceeded));
        assert!(!report.valid);
    }

    #[test]
    fn sparse_runs_must_be_sorted_by_z_y_x() {
        let mut layer = base_layer();
        layer.regions[0].selection.sparse_runs = vec![
            VoxelAnnotationSparseRun {
                start: coord(3, 1, 1),
                length: 1,
            },
            VoxelAnnotationSparseRun {
                start: coord(1, 1, 1),
                length: 1,
            },
        ];
        layer = with_computed_hashes(&layer);

        let report = validate_layer(&request(layer));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == VoxelAnnotationDiagnosticCode::InvalidSparseRun
        }));
    }

    #[test]
    fn unknown_json_fields_are_rejected_before_validation() {
        let layer = with_computed_hashes(&base_layer());
        let mut value = serde_json::to_value(&layer).unwrap();
        value["regions"][0]["surprise"] = serde_json::json!(true);
        let text = serde_json::to_string(&value).unwrap();

        match decode_layer(
            &text,
            Some("voxel-volume/test-room".to_string()),
            Some("fnv1a64:target".to_string()),
        ) {
            Err(VoxelAnnotationDecodeError::Json(message)) => {
                assert!(message.contains("unknown field"));
            }
            other => panic!("expected unknown-field JSON rejection, got {other:?}"),
        }
    }

    #[test]
    fn cell_and_bounds_queries_return_bounded_region_readouts() {
        let layer = with_computed_hashes(&base_layer());
        let layer_hash = canonical_json_hash(&layer);

        let cell_readout = query_layer(
            &layer,
            &VoxelAnnotationQueryRequest {
                runtime_layer_id: None,
                layer_id: layer.layer_id.clone(),
                mode: VoxelAnnotationQueryMode::Cell,
                cell: Some(coord(2, 1, 1)),
                bounds: None,
                region_id: None,
                max_regions: 8,
                expected_layer_hash: Some(layer_hash.clone()),
            },
        );
        assert!(cell_readout.diagnostics.is_empty());
        assert_eq!(cell_readout.matched_regions.len(), 1);
        assert_eq!(cell_readout.matched_regions[0].region_id, "region/spawn");

        let bounds_readout = query_layer(
            &layer,
            &VoxelAnnotationQueryRequest {
                runtime_layer_id: None,
                layer_id: layer.layer_id.clone(),
                mode: VoxelAnnotationQueryMode::Bounds,
                cell: None,
                bounds: Some(bounds(4, 1, 1, 6, 1, 1)),
                region_id: None,
                max_regions: 1,
                expected_layer_hash: Some(layer_hash),
            },
        );
        assert!(bounds_readout.diagnostics.is_empty());
        assert_eq!(bounds_readout.matched_regions.len(), 1);
        assert_eq!(bounds_readout.matched_regions[0].region_id, "region/cover");
    }
}
