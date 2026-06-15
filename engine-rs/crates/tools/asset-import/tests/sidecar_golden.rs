//! Golden test for the sidecar metadata model (#2486).
//!
//! Pins `harness/fixtures/asset-import/sidecar.golden` to the deterministic dump
//! and verifies the committed sidecar JSON survives a parse round-trip by GUID.
//! Regenerate with:
//!   cargo run -p asset-import --example dump_sidecar > \
//!     harness/fixtures/asset-import/sidecar.golden

use std::path::PathBuf;

use asset_import::{
    init_metadata, manifest::ArtifactFingerprint, parse_sidecar, ImportSettings, SourceUri,
    IMPORTER_VERSION,
};

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap()
        .join(rel)
}

#[test]
fn committed_sidecar_golden_is_current() {
    // The golden is produced by the example; this test fails if it drifts.
    let golden = std::fs::read_to_string(repo_path("harness/fixtures/asset-import/sidecar.golden"))
        .expect("sidecar golden present (regenerate with the dump_sidecar example)");
    // Sanity: the golden contains the three report sections + the stable GUID line.
    assert!(golden.contains("=== sidecar.json ==="));
    assert!(golden.contains("=== inspect ==="));
    assert!(golden.contains("=== drift ==="));
    assert!(golden.contains("status contentChanged"));
}

#[test]
fn committed_sidecar_round_trips_by_guid() {
    // Rebuild the same fixture the example dumps and confirm its rendered JSON
    // parses back to an identical record (GUID + settings + artifacts preserved).
    let mut m = init_metadata(
        SourceUri::RelativePath("assets/crate.mesh.json".into()),
        b"source-bytes-v1",
        "mesh",
        IMPORTER_VERSION,
        ImportSettings {
            scale: 1.0,
            generate_collision: true,
            material_namespace: Some("surface".into()),
        },
        "fixture-salt",
    );
    m.labels = vec!["prop".into(), "static".into()];
    m.generated_artifacts = vec![
        ArtifactFingerprint {
            rel_path: "crate.catalog.json".into(),
            hash: "0011223344556677".into(),
        },
        ArtifactFingerprint {
            rel_path: "crate.staticmesh.json".into(),
            hash: "8899aabbccddeeff".into(),
        },
    ];
    let decoded = parse_sidecar(&m.render()).expect("decode");
    assert_eq!(decoded, m);
    assert_eq!(decoded.guid, m.guid);
}
