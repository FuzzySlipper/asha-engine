//! Authored scene documents and their canonical flat form for the ASHA
//! scene/world foundation.
//!
//! # Lane
//!
//! `rust-state` — authority-relevant validation/serialization. Depends only on
//! the foundation crates `core-ids`, `core-assets`, `core-math`, and
//! `core-error`; it must not reach into protocol, render, or wasm layers.
//!
//! # Scope (subtask #2315)
//!
//! Implements scene-capability-01's document model:
//!
//! * [`SceneTree`] — the ergonomic authoring/visualization tree.
//! * [`FlatSceneDocument`] / [`SceneNodeRecord`] — the **canonical** flat form
//!   (`parent_id` + `child_order`) that serialization and validation operate on.
//! * Deterministic [`SceneTree::to_flat`] / [`FlatSceneDocument::to_tree`]
//!   round-trip that preserves authoring order and source ids.
//! * [`validate`] — classified checks for duplicate ids, unknown parents,
//!   cycles, invalid transforms, and wrong-kind asset references (the latter via
//!   the `core-assets` `AssetRef` vocabulary from subtask #2314).
//! * [`json`] — std-only canonical encode/decode so authored JSON crosses to
//!   Rust authority and re-serializes byte-deterministically.
//! * [`spatial_session`] / [`bootstrap`] — the live [`SpatialSessionState`]
//!   authority and the atomic [`bootstrap::bootstrap_scene`] initialization
//!   (subtask #2316): validate →
//!   deterministic plan → one [`BootstrapRecord`] replay unit, copying initial
//!   transforms into authority-owned runtime transforms with a `scene node →
//!   runtime entity` source trace.
//!
//! The tree is for authoring/visualization only; the flat form is the canonical
//! truth. After bootstrap, runtime transforms are authority-owned and may diverge
//! from the authored document, which is never mutated by runtime movement.
//!
//! # Not in scope here
//!
//! The full asset *catalog* (resolution, DAG, locks, fallback) is task #2311;
//! this crate validates reference *shape/kind* only and never resolves a
//! reference against a catalog. Render-handle/projection metadata on the source
//! trace is appended at projection time. No `protocol-*`/codegen border surface
//! is added yet — the TS-facing scene/bootstrap contract lands when bootstrap
//! plans actually cross to TS tools.

#![forbid(unsafe_code)]

pub mod bootstrap;
pub mod document;
pub mod json;
pub mod scene_object;
pub mod spatial_session;
pub mod transform;
pub mod validate;

pub use bootstrap::{
    bootstrap_scene, BootstrapError, BootstrapPlan, BootstrapRecord, PlannedEntity,
};
pub use document::{
    FlatSceneDocument, NodeMetadata, SceneMetadata, SceneNode, SceneNodeKind, SceneNodeRecord,
    SceneTree,
};
pub use json::{decode, encode, SceneDecodeError};
pub use scene_object::{
    apply_scene_object_command, scene_object_snapshot, SceneObjectCommand,
    SceneObjectCommandOutcome, SceneObjectCommandRejection, SceneObjectRecord, SceneObjectSnapshot,
    SceneObjectSnapshotHash,
};
pub use spatial_session::{EntityRuntime, SpatialSessionHash, SpatialSessionState};
pub use transform::{Quat, SceneTransform, TransformInvalid};
pub use validate::{validate, SceneValidationError, SceneValidationReport};

#[cfg(test)]
mod tests {
    use super::*;
    use core_assets::{markers, AssetId, AssetRef, AssetReference, AssetVersionReq};
    use core_ids::{SceneId, SceneNodeId};
    use core_math::Vec3;

    fn mesh_ref(id: &str) -> AssetReference {
        AssetRef::<markers::StaticMesh>::parse(id, AssetVersionReq::Any, None)
            .unwrap()
            .erase()
    }

    /// A small abstract scene: a root group with two ordered children, one a
    /// static mesh, one an empty group with its own child. Abstract fixture
    /// nouns only (scene-capability-01, recommendation 8).
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

    #[test]
    fn tree_to_flat_assigns_parents_and_order() {
        let flat = sample_tree().to_flat();
        assert_eq!(flat.nodes.len(), 4);
        let by_id = |id: u64| flat.nodes.iter().find(|n| n.id.raw() == id).unwrap();
        assert_eq!(by_id(1).parent, None);
        assert_eq!(by_id(2).parent, Some(SceneNodeId::new(1)));
        assert_eq!(by_id(2).child_order, 0);
        assert_eq!(by_id(3).child_order, 1);
        assert_eq!(by_id(4).parent, Some(SceneNodeId::new(3)));
    }

    #[test]
    fn tree_flat_tree_round_trip_preserves_order_and_ids() {
        let tree = sample_tree();
        let back = tree.to_flat().to_tree().expect("valid forest");
        assert_eq!(back, tree);
    }

    #[test]
    fn flat_to_tree_orders_siblings_by_child_order() {
        // Author children out of id order to prove `child_order` drives the view.
        let first = SceneNode {
            id: SceneNodeId::new(9),
            ..SceneNode::leaf(SceneNodeId::new(9), SceneNodeKind::EmptyGroup)
        };
        let second = SceneNode::leaf(SceneNodeId::new(2), SceneNodeKind::EmptyGroup);
        let root = SceneNode::leaf(SceneNodeId::new(1), SceneNodeKind::EmptyGroup)
            .with_children(vec![first, second]);
        let tree = SceneTree {
            id: SceneId::new(1),
            schema_version: 1,
            metadata: SceneMetadata::default(),
            dependencies: vec![],
            roots: vec![root],
        };
        let rebuilt = tree.to_flat().to_tree().unwrap();
        let kids = &rebuilt.roots[0].children;
        assert_eq!(kids[0].id, SceneNodeId::new(9)); // child_order 0 wins over lower id
        assert_eq!(kids[1].id, SceneNodeId::new(2));
    }

    #[test]
    fn valid_document_passes() {
        let flat = sample_tree().to_flat();
        assert!(validate(&flat).is_ok());
    }

    #[test]
    fn detects_duplicate_ids() {
        let mut flat = sample_tree().to_flat();
        flat.nodes[1].id = SceneNodeId::new(1); // collide with root
        let report = validate(&flat);
        assert!(report
            .errors
            .iter()
            .any(|e| matches!(e, SceneValidationError::DuplicateNodeId { id } if id.raw() == 1)));
    }

    #[test]
    fn detects_unknown_parent() {
        let mut flat = sample_tree().to_flat();
        flat.nodes[1].parent = Some(SceneNodeId::new(999));
        let report = validate(&flat);
        assert!(report.errors.iter().any(|e| matches!(
            e,
            SceneValidationError::UnknownParent { parent, .. } if parent.raw() == 999
        )));
    }

    #[test]
    fn detects_cycle_with_path() {
        // Two-node cycle: 1 -> 2 -> 1.
        let mut flat = sample_tree().to_flat();
        flat.nodes.truncate(2);
        flat.nodes[0].parent = Some(SceneNodeId::new(2));
        flat.nodes[1].parent = Some(SceneNodeId::new(1));
        flat.nodes[1].id = SceneNodeId::new(2);
        let report = validate(&flat);
        let cycle = report
            .errors
            .iter()
            .find_map(|e| match e {
                SceneValidationError::Cycle { path } => Some(path.clone()),
                _ => None,
            })
            .expect("cycle reported");
        assert_eq!(cycle.len(), 2);
        // to_tree refuses to build a cyclic forest.
        assert!(flat.to_tree().is_none());
    }

    #[test]
    fn detects_invalid_transform() {
        let mut flat = sample_tree().to_flat();
        flat.nodes[0].transform.scale = Vec3::new(0.0, 1.0, 1.0);
        let report = validate(&flat);
        assert!(report.errors.iter().any(|e| matches!(
            e,
            SceneValidationError::InvalidTransform {
                reason: TransformInvalid::ZeroScaleAxis,
                ..
            }
        )));
    }

    #[test]
    fn detects_wrong_kind_asset_ref() {
        // A StaticMesh node pointing at a `material/...` id.
        let bad = AssetReference::new(
            AssetId::parse("material/concrete-wet").unwrap(),
            AssetVersionReq::Any,
            None,
        );
        let mut flat = sample_tree().to_flat();
        flat.nodes[1].kind = SceneNodeKind::StaticMesh(bad);
        let report = validate(&flat);
        assert!(report.errors.iter().any(|e| matches!(
            e,
            SceneValidationError::AssetKindMismatch {
                expected: core_assets::AssetKind::StaticMesh,
                actual: core_assets::AssetKind::Material,
                ..
            }
        )));
    }

    #[test]
    fn json_round_trips_through_decode_encode() {
        let flat = sample_tree().to_flat();
        let encoded = encode(&flat);
        let decoded = decode(&encoded).expect("decode");
        // Encode is canonical; re-encoding the decoded doc is a fixed point.
        assert_eq!(encode(&decoded), encoded);
        // And the decoded doc validates and rebuilds the same tree.
        assert!(validate(&decoded).is_ok());
        assert_eq!(decoded.canonical(), flat.canonical());
    }

    #[test]
    fn decode_rejects_unknown_kind() {
        let flat = sample_tree().to_flat();
        let encoded = encode(&flat).replace("\"emptyGroup\"", "\"forceField\"");
        assert!(matches!(
            decode(&encoded),
            Err(SceneDecodeError::UnknownKind(k)) if k == "forceField"
        ));
    }
}
