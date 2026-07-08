//! Golden-fixture drift + readback for the canonical flat scene document.
//!
//! Pins `harness/fixtures/scenes/sample-flat.json` against the in-crate builder
//! so a serialization change fails loudly, and proves the committed bytes decode,
//! validate, and re-encode to themselves (encode/decode fixture readback).
//!
//! Regenerate the fixture with:
//!   cargo run -p core-scene --example dump_canonical_scene > \
//!     harness/fixtures/scenes/sample-flat.json

use std::path::PathBuf;

use core_assets::{markers, AssetRef, AssetReference, AssetVersionReq};
use core_ids::{RuntimeSessionId, SceneId, SceneNodeId};
use core_math::Vec3;
use core_scene::{
    bootstrap_scene, decode, encode, validate, NodeMetadata, SceneMetadata, SceneNode,
    SceneNodeKind, SceneTransform, SceneTree, SceneValidationError,
};

#[path = "support/bootstrap_summary_fmt.rs"]
mod bootstrap_summary_fmt;

fn mesh_ref(id: &str) -> AssetReference {
    AssetRef::<markers::StaticMesh>::parse(id, AssetVersionReq::Any, None)
        .unwrap()
        .erase()
}

/// Must match `examples/dump_canonical_scene.rs`.
fn sample_tree() -> SceneTree {
    let child_a = SceneNode {
        id: SceneNodeId::new(2),
        transform: SceneTransform {
            translation: Vec3::new(1.0, 0.0, 0.0),
            ..SceneTransform::IDENTITY
        },
        kind: SceneNodeKind::StaticMesh(mesh_ref("mesh/static-mesh-fixture-a")),
        metadata: NodeMetadata {
            label: Some("mesh-a".into()),
            tags: vec!["b-tag".into(), "a-tag".into()],
        },
        children: vec![],
    };
    let grandchild = SceneNode::leaf(SceneNodeId::new(4), SceneNodeKind::EmptyGroup);
    let child_b = SceneNode::leaf(SceneNodeId::new(3), SceneNodeKind::EmptyGroup)
        .with_children(vec![grandchild]);
    let root = SceneNode::leaf(SceneNodeId::new(1), SceneNodeKind::EmptyGroup)
        .with_children(vec![child_a, child_b]);

    SceneTree {
        id: SceneId::new(100),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("sample".into()),
            authoring_format_version: 1,
        },
        dependencies: vec![mesh_ref("mesh/static-mesh-fixture-a")],
        roots: vec![root],
    }
}

fn scenes_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .find(|ancestor| ancestor.join("engine-rs").is_dir() && ancestor.join("harness").is_dir())
        .expect("repo root")
        .join("harness/fixtures/scenes")
}

fn fixture_path() -> PathBuf {
    scenes_dir().join("sample-flat.json")
}

#[test]
fn canonical_encoding_matches_committed_golden() {
    let committed = std::fs::read_to_string(fixture_path()).expect("read sample-flat.json");
    let encoded = encode(&sample_tree().to_flat());
    assert_eq!(
        encoded, committed,
        "canonical scene encoding drifted from harness/fixtures/scenes/sample-flat.json; \
         regenerate with `cargo run -p core-scene --example dump_canonical_scene`"
    );
}

#[test]
fn committed_golden_decodes_validates_and_round_trips() {
    let committed = std::fs::read_to_string(fixture_path()).expect("read sample-flat.json");
    let doc = decode(&committed).expect("golden decodes");
    assert!(validate(&doc).is_ok(), "golden must validate clean");
    assert_eq!(encode(&doc), committed, "decode∘encode is a fixed point");
    // The decoded doc is the canonical form of the in-crate builder, and rebuilds
    // to a tree that re-flattens to the same canonical document.
    assert_eq!(doc.canonical(), sample_tree().to_flat().canonical());
    let rebuilt = doc.to_tree().expect("forest");
    assert_eq!(rebuilt.to_flat().canonical(), doc.canonical());
}

#[test]
fn invalid_cycle_fixture_is_classified_and_unbuildable() {
    let raw = std::fs::read_to_string(scenes_dir().join("invalid-cycle.json"))
        .expect("read invalid-cycle.json");
    // Structurally valid JSON decodes; the cycle is a *semantic* failure.
    let doc = decode(&raw).expect("decodes");
    let report = validate(&doc);
    assert!(
        report
            .errors
            .iter()
            .any(|e| matches!(e, SceneValidationError::Cycle { .. })),
        "expected a classified Cycle error, got {:?}",
        report.errors
    );
    // A cyclic document cannot be rebuilt into an authoring forest...
    assert!(doc.to_tree().is_none());
    // ...and bootstrap refuses it before producing any world.
    assert!(bootstrap_scene(&doc, RuntimeSessionId::new(1)).is_err());
}

#[test]
fn bootstrap_summary_matches_committed_golden() {
    let committed = std::fs::read_to_string(scenes_dir().join("bootstrap-summary.json"))
        .expect("read bootstrap-summary.json");
    let doc = decode(&std::fs::read_to_string(fixture_path()).unwrap()).unwrap();
    let (_world, record) = bootstrap_scene(&doc, RuntimeSessionId::new(7)).expect("bootstrap");
    assert_eq!(
        bootstrap_summary_fmt::render(&record),
        committed,
        "bootstrap summary drifted from harness/fixtures/scenes/bootstrap-summary.json; \
         regenerate with `cargo run -p core-scene --example dump_bootstrap_summary`"
    );
}
