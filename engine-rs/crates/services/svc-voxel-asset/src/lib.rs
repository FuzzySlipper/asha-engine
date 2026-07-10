//! Rust authority service for Asha-native stored voxel-volume assets.
//!
//! # Lane
//!
//! `rust-service` — validates, canonicalizes, hashes, serializes, and
//! deserializes stored voxel-volume asset DTOs. Studio and TypeScript consume
//! the generated protocol surface; they do not own format acceptance.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use core_assets::{AssetId, AssetKind};
use protocol_diagnostics::DiagnosticSeverity;
use protocol_voxel_asset::{
    VoxelAssetContentHashes, VoxelAssetCoord, VoxelAssetDiagnostic, VoxelAssetDiagnosticCode,
    VoxelAssetRepresentationKind, VoxelVolumeAsset, VOXEL_ASSET_MEDIA_TYPE,
    VOXEL_ASSET_SCHEMA_VERSION,
};

/// Canonical coordinate system tag for the first format version.
pub const VOXEL_ASSET_COORDINATE_SYSTEM: &str = "y_up_right_handed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelAssetValidationReport {
    pub diagnostics: Vec<VoxelAssetDiagnostic>,
    pub canonical_json_hash: String,
    pub voxel_data_hash: String,
}

impl VoxelAssetValidationReport {
    pub fn is_valid(&self) -> bool {
        !self.diagnostics.iter().any(|d| {
            matches!(
                d.severity,
                DiagnosticSeverity::Error | DiagnosticSeverity::Fatal
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoxelAssetDecodeError {
    Json(String),
    Invalid(VoxelAssetValidationReport),
}

impl std::fmt::Display for VoxelAssetDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoxelAssetDecodeError::Json(message) => {
                write!(f, "invalid voxel asset JSON: {message}")
            }
            VoxelAssetDecodeError::Invalid(report) => {
                write!(
                    f,
                    "voxel asset failed validation with {} diagnostic(s)",
                    report.diagnostics.len()
                )
            }
        }
    }
}

impl std::error::Error for VoxelAssetDecodeError {}

/// Validate a stored voxel-volume asset and compute the authority hashes for it.
pub fn validate_asset(asset: &VoxelVolumeAsset) -> VoxelAssetValidationReport {
    let mut diagnostics = Vec::new();

    if asset.schema_version != VOXEL_ASSET_SCHEMA_VERSION {
        diagnostics.push(diagnostic(
            VoxelAssetDiagnosticCode::UnsupportedSchemaVersion,
            "schemaVersion",
            format!(
                "schema version {} is not supported; expected {}",
                asset.schema_version, VOXEL_ASSET_SCHEMA_VERSION
            ),
        ));
    }

    if asset.media_type != VOXEL_ASSET_MEDIA_TYPE {
        diagnostics.push(diagnostic(
            VoxelAssetDiagnosticCode::UnsupportedMediaType,
            "mediaType",
            format!(
                "media type {:?} is not supported; expected {VOXEL_ASSET_MEDIA_TYPE}",
                asset.media_type
            ),
        ));
    }

    validate_asset_id(asset, &mut diagnostics);
    validate_grid(asset, &mut diagnostics);
    validate_bounds(asset, &mut diagnostics);
    let material_palette = validate_material_palette(asset, &mut diagnostics);
    validate_sparse_runs(asset, &material_palette, &mut diagnostics);

    let canonical_json_hash = canonical_json_hash(asset);
    let voxel_data_hash = voxel_data_hash(asset);
    if asset.content_hashes.canonical_json.is_empty() {
        diagnostics.push(diagnostic(
            VoxelAssetDiagnosticCode::ContentHashMismatch,
            "contentHashes.canonicalJson",
            "canonical JSON hash is required",
        ));
    } else if asset.content_hashes.canonical_json != canonical_json_hash {
        diagnostics.push(diagnostic(
            VoxelAssetDiagnosticCode::ContentHashMismatch,
            "contentHashes.canonicalJson",
            "canonical JSON hash does not match authority-computed hash",
        ));
    }
    if asset.content_hashes.voxel_data.is_empty() {
        diagnostics.push(diagnostic(
            VoxelAssetDiagnosticCode::ContentHashMismatch,
            "contentHashes.voxelData",
            "voxel data hash is required",
        ));
    } else if asset.content_hashes.voxel_data != voxel_data_hash {
        diagnostics.push(diagnostic(
            VoxelAssetDiagnosticCode::ContentHashMismatch,
            "contentHashes.voxelData",
            "voxel data hash does not match authority-computed hash",
        ));
    }

    VoxelAssetValidationReport {
        diagnostics,
        canonical_json_hash,
        voxel_data_hash,
    }
}

/// Return a copy with authority-computed hashes populated.
pub fn with_computed_hashes(asset: &VoxelVolumeAsset) -> VoxelVolumeAsset {
    let mut normalized = asset.clone();
    normalized.content_hashes = VoxelAssetContentHashes {
        canonical_json: String::new(),
        voxel_data: String::new(),
    };
    normalized.content_hashes = VoxelAssetContentHashes {
        canonical_json: canonical_json_hash(&normalized),
        voxel_data: voxel_data_hash(&normalized),
    };
    normalized
}

/// Encode canonical JSON after validation.
pub fn encode_asset(asset: &VoxelVolumeAsset) -> Result<String, VoxelAssetValidationReport> {
    let report = validate_asset(asset);
    if !report.is_valid() {
        return Err(report);
    }
    Ok(canonical_json(asset))
}

/// Decode JSON and validate before returning the asset.
pub fn decode_asset(text: &str) -> Result<VoxelVolumeAsset, VoxelAssetDecodeError> {
    let asset: VoxelVolumeAsset =
        serde_json::from_str(text).map_err(|e| VoxelAssetDecodeError::Json(e.to_string()))?;
    let report = validate_asset(&asset);
    if report.is_valid() {
        Ok(asset)
    } else {
        Err(VoxelAssetDecodeError::Invalid(report))
    }
}

fn validate_asset_id(asset: &VoxelVolumeAsset, diagnostics: &mut Vec<VoxelAssetDiagnostic>) {
    match AssetId::parse(&asset.asset_id) {
        Ok(id) if id.kind() == AssetKind::VoxelVolume => {}
        Ok(id) => diagnostics.push(diagnostic(
            VoxelAssetDiagnosticCode::InvalidAssetId,
            "assetId",
            format!(
                "asset id {:?} has kind {}; expected voxel-volume",
                asset.asset_id,
                id.kind()
            ),
        )),
        Err(e) => diagnostics.push(diagnostic(
            VoxelAssetDiagnosticCode::InvalidAssetId,
            "assetId",
            format!("asset id {:?} is invalid: {e}", asset.asset_id),
        )),
    }
}

fn validate_grid(asset: &VoxelVolumeAsset, diagnostics: &mut Vec<VoxelAssetDiagnostic>) {
    if !asset.grid.origin.iter().all(|v| v.is_finite())
        || !asset.grid.cell_size.is_finite()
        || asset.grid.cell_size <= 0.0
        || asset.grid.coordinate_system != VOXEL_ASSET_COORDINATE_SYSTEM
    {
        diagnostics.push(diagnostic(
            VoxelAssetDiagnosticCode::InvalidGrid,
            "grid",
            "grid origin must be finite, cellSize must be finite and > 0, and coordinateSystem must be y_up_right_handed",
        ));
    }
}

fn validate_bounds(asset: &VoxelVolumeAsset, diagnostics: &mut Vec<VoxelAssetDiagnostic>) {
    if asset.bounds.min.x > asset.bounds.max.x
        || asset.bounds.min.y > asset.bounds.max.y
        || asset.bounds.min.z > asset.bounds.max.z
    {
        diagnostics.push(diagnostic(
            VoxelAssetDiagnosticCode::InvalidBounds,
            "bounds",
            "inclusive bounds require min <= max on every axis",
        ));
    }
}

fn validate_material_palette(
    asset: &VoxelVolumeAsset,
    diagnostics: &mut Vec<VoxelAssetDiagnostic>,
) -> BTreeSet<u16> {
    let mut palette = BTreeSet::new();
    let mut palette_entries = BTreeSet::new();
    let mut catalog_bindings = BTreeSet::new();
    for (index, binding) in asset.material_palette.iter().enumerate() {
        if !palette.insert(binding.voxel_material) {
            diagnostics.push(diagnostic(
                VoxelAssetDiagnosticCode::DuplicateMaterialBinding,
                format!("materialPalette[{index}].voxelMaterial"),
                format!(
                    "voxel material {} is bound more than once",
                    binding.voxel_material
                ),
            ));
        }
        if !valid_scoped_binding_id(&binding.palette_entry_id, "voxel-material/") {
            diagnostics.push(diagnostic(
                VoxelAssetDiagnosticCode::InvalidMaterialReference,
                format!("materialPalette[{index}].paletteEntryId"),
                "palette entry id must use voxel-material/ followed by lowercase kebab path segments",
            ));
        } else if !palette_entries.insert(binding.palette_entry_id.as_str()) {
            diagnostics.push(diagnostic(
                VoxelAssetDiagnosticCode::DuplicateMaterialBinding,
                format!("materialPalette[{index}].paletteEntryId"),
                format!(
                    "palette entry id {:?} is bound more than once",
                    binding.palette_entry_id
                ),
            ));
        }
        if let Some(binding_id) = &binding.material_catalog_binding_id {
            if !valid_scoped_binding_id(binding_id, "catalog-binding/") {
                diagnostics.push(diagnostic(
                    VoxelAssetDiagnosticCode::InvalidMaterialReference,
                    format!("materialPalette[{index}].materialCatalogBindingId"),
                    "material catalog binding id must use catalog-binding/ followed by lowercase kebab path segments",
                ));
            } else if !catalog_bindings.insert(binding_id.as_str()) {
                diagnostics.push(diagnostic(
                    VoxelAssetDiagnosticCode::DuplicateMaterialBinding,
                    format!("materialPalette[{index}].materialCatalogBindingId"),
                    format!("material catalog binding id {binding_id:?} is bound more than once"),
                ));
            }
        }
        match AssetId::parse(&binding.material_asset_id) {
            Ok(id) if id.kind() == AssetKind::Material => {}
            Ok(id) => diagnostics.push(diagnostic(
                VoxelAssetDiagnosticCode::InvalidMaterialReference,
                format!("materialPalette[{index}].materialAssetId"),
                format!(
                    "material reference {:?} has kind {}; expected material",
                    binding.material_asset_id,
                    id.kind()
                ),
            )),
            Err(e) => diagnostics.push(diagnostic(
                VoxelAssetDiagnosticCode::InvalidMaterialReference,
                format!("materialPalette[{index}].materialAssetId"),
                format!(
                    "material reference {:?} is invalid: {e}",
                    binding.material_asset_id
                ),
            )),
        }
    }
    palette
}

fn valid_scoped_binding_id(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|tail| {
        !tail.is_empty()
            && tail.split('/').all(|segment| {
                !segment.is_empty()
                    && segment.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                    })
                    && segment
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_alphanumeric)
                    && segment
                        .as_bytes()
                        .last()
                        .is_some_and(u8::is_ascii_alphanumeric)
            })
    })
}

fn validate_sparse_runs(
    asset: &VoxelVolumeAsset,
    material_palette: &BTreeSet<u16>,
    diagnostics: &mut Vec<VoxelAssetDiagnostic>,
) {
    if asset.representation.kind != VoxelAssetRepresentationKind::SparseRuns {
        diagnostics.push(diagnostic(
            VoxelAssetDiagnosticCode::UnsupportedRepresentation,
            "representation.kind",
            "only sparse_runs representation is supported in schema version 1",
        ));
    }

    let mut occupied = BTreeSet::new();
    for (index, run) in asset.representation.sparse_runs.iter().enumerate() {
        if run.length == 0 {
            diagnostics.push(diagnostic(
                VoxelAssetDiagnosticCode::InvalidSparseRun,
                format!("representation.sparseRuns[{index}].length"),
                "sparse run length must be greater than zero",
            ));
            continue;
        }
        if !material_palette.contains(&run.material) {
            diagnostics.push(diagnostic(
                VoxelAssetDiagnosticCode::UnknownVoxelMaterial,
                format!("representation.sparseRuns[{index}].material"),
                format!("voxel material {} is not in materialPalette", run.material),
            ));
        }
        let Some(end_x) = run.start.x.checked_add(i64::from(run.length) - 1) else {
            diagnostics.push(diagnostic(
                VoxelAssetDiagnosticCode::InvalidSparseRun,
                format!("representation.sparseRuns[{index}]"),
                "sparse run end coordinate overflowed",
            ));
            continue;
        };
        if run.start.x < asset.bounds.min.x
            || end_x > asset.bounds.max.x
            || run.start.y < asset.bounds.min.y
            || run.start.y > asset.bounds.max.y
            || run.start.z < asset.bounds.min.z
            || run.start.z > asset.bounds.max.z
        {
            diagnostics.push(diagnostic(
                VoxelAssetDiagnosticCode::InvalidSparseRun,
                format!("representation.sparseRuns[{index}]"),
                "sparse run must stay inside inclusive asset bounds",
            ));
        }
        for x in run.start.x..=end_x {
            let coord = VoxelAssetCoord {
                x,
                y: run.start.y,
                z: run.start.z,
            };
            if !occupied.insert(coord) {
                diagnostics.push(diagnostic(
                    VoxelAssetDiagnosticCode::DuplicateVoxel,
                    format!("representation.sparseRuns[{index}]"),
                    format!(
                        "voxel coordinate ({}, {}, {}) is written more than once",
                        x, run.start.y, run.start.z
                    ),
                ));
            }
        }
    }
}

fn canonical_json_hash(asset: &VoxelVolumeAsset) -> String {
    let mut normalized = asset.clone();
    normalized.content_hashes.canonical_json.clear();
    normalized.content_hashes.voxel_data.clear();
    format!(
        "fnv1a64:{:016x}",
        fnv1a64(canonical_json(&normalized).as_bytes())
    )
}

fn voxel_data_hash(asset: &VoxelVolumeAsset) -> String {
    let mut h = FNV_OFFSET;
    for run in &asset.representation.sparse_runs {
        feed_i64(&mut h, run.start.x);
        feed_i64(&mut h, run.start.y);
        feed_i64(&mut h, run.start.z);
        feed_u32(&mut h, run.length);
        feed_u16(&mut h, run.material);
    }
    format!("fnv1a64:{h:016x}")
}

fn canonical_json(asset: &VoxelVolumeAsset) -> String {
    let value = serde_json::to_value(asset).expect("voxel asset DTO serializes");
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

fn diagnostic(
    code: VoxelAssetDiagnosticCode,
    reference: impl Into<String>,
    message: impl Into<String>,
) -> VoxelAssetDiagnostic {
    VoxelAssetDiagnostic {
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

fn feed_u16(hash: &mut u64, value: u16) {
    for byte in value.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_voxel_asset::{
        VoxelAssetAuthoringMetadata, VoxelAssetBounds, VoxelAssetContentHashes, VoxelAssetGrid,
        VoxelAssetMaterialBinding, VoxelAssetProvenanceKind, VoxelAssetProvenanceRef,
        VoxelAssetRepresentation, VoxelAssetSparseRun, VOXEL_ASSET_MEDIA_TYPE,
    };

    fn coord(x: i64, y: i64, z: i64) -> VoxelAssetCoord {
        VoxelAssetCoord { x, y, z }
    }

    fn hand_authored_asset() -> VoxelVolumeAsset {
        VoxelVolumeAsset {
            asset_id: "voxel-volume/test-room".to_string(),
            schema_version: VOXEL_ASSET_SCHEMA_VERSION,
            media_type: VOXEL_ASSET_MEDIA_TYPE.to_string(),
            grid: VoxelAssetGrid {
                origin: [0.0, 0.0, 0.0],
                cell_size: 0.5,
                coordinate_system: VOXEL_ASSET_COORDINATE_SYSTEM.to_string(),
            },
            bounds: VoxelAssetBounds {
                min: coord(0, 0, 0),
                max: coord(3, 1, 0),
            },
            representation: VoxelAssetRepresentation {
                kind: VoxelAssetRepresentationKind::SparseRuns,
                sparse_runs: vec![
                    VoxelAssetSparseRun {
                        start: coord(0, 0, 0),
                        length: 4,
                        material: 1,
                    },
                    VoxelAssetSparseRun {
                        start: coord(1, 1, 0),
                        length: 2,
                        material: 2,
                    },
                ],
            },
            material_palette: vec![
                VoxelAssetMaterialBinding {
                    voxel_material: 1,
                    palette_entry_id: "voxel-material/concrete".to_string(),
                    display_name: Some("Concrete".to_string()),
                    material_asset_id: "material/concrete".to_string(),
                    material_catalog_binding_id: Some("catalog-binding/concrete".to_string()),
                },
                VoxelAssetMaterialBinding {
                    voxel_material: 2,
                    palette_entry_id: "voxel-material/glass".to_string(),
                    display_name: Some("Glass".to_string()),
                    material_asset_id: "material/glass".to_string(),
                    material_catalog_binding_id: Some("catalog-binding/glass".to_string()),
                },
            ],
            provenance: vec![VoxelAssetProvenanceRef {
                kind: VoxelAssetProvenanceKind::Authored,
                uri: "asha://studio/authoring/session/fixture".to_string(),
                content_hash: "fnv1a64:authoring".to_string(),
            }],
            authoring: VoxelAssetAuthoringMetadata {
                label: Some("Test room".to_string()),
                created_by: Some("svc-voxel-asset-test".to_string()),
                source_tool: Some("asha-studio".to_string()),
            },
            validation_diagnostics: Vec::new(),
            content_hashes: VoxelAssetContentHashes {
                canonical_json: String::new(),
                voxel_data: String::new(),
            },
        }
    }

    #[test]
    fn hand_authored_voxel_asset_roundtrips_and_hashes() {
        let asset = with_computed_hashes(&hand_authored_asset());
        let report = validate_asset(&asset);
        assert!(report.is_valid(), "{:?}", report.diagnostics);
        assert_eq!(
            asset.content_hashes.canonical_json,
            report.canonical_json_hash
        );
        assert_eq!(asset.content_hashes.voxel_data, report.voxel_data_hash);

        let encoded = encode_asset(&asset).expect("encode");
        assert!(
            encoded.contains("\"mediaType\": \"application/vnd.asha.voxel-volume+json;version=1\"")
        );
        let decoded = decode_asset(&encoded).expect("decode");
        assert_eq!(decoded, asset);
    }

    #[test]
    fn converted_volume_asset_records_conversion_provenance() {
        let mut asset = hand_authored_asset();
        asset.asset_id = "voxel-volume/converted-crate".to_string();
        asset.provenance = vec![VoxelAssetProvenanceRef {
            kind: VoxelAssetProvenanceKind::Converted,
            uri: "asha://voxel-conversion/apply/fnv1a64-preview".to_string(),
            content_hash: "fnv1a64:conversion-receipt".to_string(),
        }];
        let asset = with_computed_hashes(&asset);
        let report = validate_asset(&asset);
        assert!(report.is_valid(), "{:?}", report.diagnostics);
        assert_eq!(
            asset.provenance[0].kind,
            VoxelAssetProvenanceKind::Converted
        );
    }

    #[test]
    fn invalid_bounds_and_material_references_are_classified() {
        let mut asset = hand_authored_asset();
        asset.bounds.min.x = 10;
        asset.material_palette[0].palette_entry_id = "Voxel Material/Bad".to_string();
        asset.material_palette[0].material_asset_id = "texture/not-a-material".to_string();
        asset.material_palette[0].material_catalog_binding_id =
            Some("catalog-binding/bad_value".to_string());
        asset.representation.sparse_runs[1].material = 99;

        let report = validate_asset(&asset);
        let codes: BTreeSet<_> = report.diagnostics.iter().map(|d| d.code).collect();
        assert!(codes.contains(&VoxelAssetDiagnosticCode::InvalidBounds));
        assert!(codes.contains(&VoxelAssetDiagnosticCode::InvalidMaterialReference));
        assert!(codes.contains(&VoxelAssetDiagnosticCode::UnknownVoxelMaterial));
        assert!(!report.is_valid());
    }

    #[test]
    fn duplicate_material_palette_binding_ids_fail_closed() {
        let mut asset = hand_authored_asset();
        asset.material_palette[1].palette_entry_id =
            asset.material_palette[0].palette_entry_id.clone();
        asset.material_palette[1].material_catalog_binding_id = asset.material_palette[0]
            .material_catalog_binding_id
            .clone();

        let report = validate_asset(&asset);
        let duplicate_diagnostics = report
            .diagnostics
            .iter()
            .filter(|d| d.code == VoxelAssetDiagnosticCode::DuplicateMaterialBinding)
            .count();
        assert_eq!(duplicate_diagnostics, 2);
        assert!(!report.is_valid());
    }

    #[test]
    fn unsupported_version_is_rejected_on_decode() {
        let mut asset = with_computed_hashes(&hand_authored_asset());
        asset.schema_version = VOXEL_ASSET_SCHEMA_VERSION + 1;
        let encoded = serde_json::to_string(&asset).unwrap();

        match decode_asset(&encoded) {
            Err(VoxelAssetDecodeError::Invalid(report)) => {
                assert!(report
                    .diagnostics
                    .iter()
                    .any(|d| d.code == VoxelAssetDiagnosticCode::UnsupportedSchemaVersion));
            }
            other => panic!("expected unsupported-version rejection, got {other:?}"),
        }
    }
}
