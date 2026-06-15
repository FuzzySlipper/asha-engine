//! Offline asset importer: a deterministic, dependency-free conversion of a
//! documented source-mesh format into ASHA-native static-mesh + catalog + texture
//! descriptors (#2384, #2385, #2386).
//!
//! # Lane
//!
//! `rust-tools` — an **offline** tool. It never runs at runtime; the runtime
//! renderer consumes only the ASHA-native descriptors this emits. Catalog
//! validation stays Rust-owned (`core-catalog`); this crate produces border
//! ([`protocol_render`]) + catalog descriptors and classified diagnostics.
//!
//! # Pipeline
//!
//! [`source::parse_source`] reads the documented format → [`import::import`]
//! converts it to [`import::ImportedAssets`] → [`artifacts::render_artifacts`]
//! emits stable JSON. [`manifest`] adds source fingerprints, importer/schema
//! versions, generated-artifact hashes, asset-lock drift, and a reimport plan
//! (#2385). [`import_text`] runs parse + import end to end.

#![forbid(unsafe_code)]

pub mod artifacts;
pub mod cli;
pub mod diagnostic;
pub mod fingerprint;
pub mod fixtures;
pub mod import;
pub mod json;
pub mod manifest;
pub mod sidecar;
pub mod source;

pub use diagnostic::{ImportCode, ImportDiagnostic, ImportSeverity};
pub use import::{import, import_with_context, ImportContext, ImportOutcome, ImportedAssets};
pub use manifest::{ImportManifest, ReimportPlan};
pub use sidecar::{
    detect_duplicate_guids, drift_report, init_metadata, inspect_report, parse_sidecar, reconcile,
    sidecar_path, AssetGuid, ImportSettings, ProjectOverride, SidecarMetadata, SidecarStatus,
    SourceUri, SIDECAR_SCHEMA_VERSION,
};
pub use source::{parse_source, SourceMesh, SourceParse};

/// The importer version. Bumped when import output for unchanged source could
/// change; recorded in the manifest so a drift check can attribute it (#2385).
pub const IMPORTER_VERSION: u32 = 1;

/// Parse and import source text end to end. Parse-stage and import-stage
/// diagnostics are merged; `assets` is present only when no error was raised.
pub fn import_text(text: &str, locus: &str) -> ImportOutcome {
    let parse = parse_source(text, locus);
    let mut diagnostics = parse.diagnostics;
    let Some(mesh) = parse.mesh else {
        return ImportOutcome {
            assets: None,
            diagnostics,
        };
    };
    let mut outcome = import(&mesh);
    diagnostics.append(&mut outcome.diagnostics);
    ImportOutcome {
        assets: outcome.assets,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_a_valid_fixture_into_deterministic_assets() {
        let a = import_text(fixtures::VALID_TRIANGLE, "fixtures/valid.mesh.json");
        let b = import_text(fixtures::VALID_TRIANGLE, "fixtures/valid.mesh.json");
        assert!(!a.has_errors());
        let assets = a.assets.as_ref().unwrap();
        assert_eq!(assets.static_mesh.asset, "mesh/import-fixture-a");
        // Reimport of unchanged source is byte-identical.
        assert_eq!(a, b);
        // The catalog carries the mesh, its material, and the referenced texture.
        let ids: Vec<&str> = assets
            .catalog
            .entries
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec![
                "material/surface-a",
                "mesh/import-fixture-a",
                "texture/surface-a"
            ]
        );
    }

    #[test]
    fn rejects_an_unsupported_feature_with_a_classified_diagnostic() {
        let outcome = import_text(
            fixtures::UNSUPPORTED_FEATURE,
            "fixtures/unsupported.mesh.json",
        );
        assert!(outcome.has_errors());
        assert!(outcome.assets.is_none());
        assert!(outcome
            .diagnostics
            .iter()
            .any(|d| d.code == ImportCode::UnsupportedFeature));
    }

    #[test]
    fn rejects_non_triangle_topology() {
        let outcome = import_text(fixtures::BAD_TOPOLOGY, "fixtures/bad.mesh.json");
        assert!(outcome
            .diagnostics
            .iter()
            .any(|d| d.code == ImportCode::UnsupportedTopology));
        assert!(outcome.assets.is_none());
    }

    #[test]
    fn provenance_is_static_asset_so_runtime_never_imports_gltf() {
        let outcome = import_text(fixtures::VALID_TRIANGLE, "x");
        let mesh = &outcome.assets.unwrap().static_mesh;
        assert_eq!(
            mesh.payload.provenance,
            protocol_render::MeshProvenance::StaticAsset
        );
    }
}
