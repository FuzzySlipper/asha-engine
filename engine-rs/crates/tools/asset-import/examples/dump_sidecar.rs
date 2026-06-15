//! Deterministic dump of a sidecar metadata record + its inspect/drift reports
//! and a shared-source project-override resolution (#2486). Backs the committed
//! golden `harness/fixtures/asset-import/sidecar.golden`.
//!
//!   cargo run -p asset-import --example dump_sidecar > \
//!     harness/fixtures/asset-import/sidecar.golden

use asset_import::{
    drift_report, init_metadata, inspect_report, manifest::ArtifactFingerprint, reconcile,
    ImportSettings, ProjectOverride, SidecarMetadata, SourceUri, IMPORTER_VERSION,
};

fn sample() -> SidecarMetadata {
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
    m
}

fn main() {
    let m = sample();
    print!("=== sidecar.json ===\n{}", m.render());
    print!("\n=== inspect ===\n{}", inspect_report(&m));

    // Drift cases (deterministic): unchanged, moved, content-changed.
    let unchanged = reconcile(
        Some(&m),
        &SourceUri::RelativePath("assets/crate.mesh.json".into()),
        b"source-bytes-v1",
    );
    let moved = reconcile(
        Some(&m),
        &SourceUri::RelativePath("moved/crate.mesh.json".into()),
        b"source-bytes-v1",
    );
    let changed = reconcile(
        Some(&m),
        &SourceUri::RelativePath("assets/crate.mesh.json".into()),
        b"source-bytes-v2",
    );
    print!("\n=== drift ===\n");
    println!("{}", drift_report(&unchanged));
    println!("{}", drift_report(&moved));
    println!("{}", drift_report(&changed));

    // The same shared source serves two projects with distinct effective settings,
    // without mutating the sidecar (project-agnostic source identity).
    let project_a = ProjectOverride {
        scale: Some(0.5),
        ..Default::default()
    };
    let project_b = ProjectOverride {
        scale: Some(4.0),
        generate_collision: Some(false),
        ..Default::default()
    };
    let a = project_a.apply(&m.import_settings);
    let b = project_b.apply(&m.import_settings);
    print!(
        "\n=== shared-source overrides (guid {}) ===\n",
        m.guid.as_str()
    );
    println!(
        "projectA scale={} generateCollision={}",
        a.scale, a.generate_collision
    );
    println!(
        "projectB scale={} generateCollision={}",
        b.scale, b.generate_collision
    );
    println!(
        "shared scale={} generateCollision={} (unmutated)",
        m.import_settings.scale, m.import_settings.generate_collision
    );
}
