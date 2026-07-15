//! Projects authoritative state into retained render diffs.
//!
//! # Lane
//!
//! `rust-render` — may depend on `core-ids`, `core-state`, `core-error`, and
//! `protocol-render`. It **projects** state into [`RenderDiff`]s; it never
//! renders, and it never feeds render concepts back into authority state.
//!
//! # Design
//!
//! A [`RenderProjector`] reads a read-only [`StateStore`] and emits the
//! retained-mode create / update / destroy [`RenderDiff`]s needed to bring a
//! renderer's scene from the *previously projected* state to the *current*
//! state. It owns a stable [`EntityId`] → [`RenderHandle`] registry so the
//! renderer never sees authority IDs and a node keeps its handle for its
//! lifetime.
//!
//! What a node *looks like* is decided by a [`NodeProjection`]; the diff
//! machinery (handle allocation, create/update/destroy, change detection) is
//! shared. [`SceneProjection`] is the default: one abstract cube per entity.
//!
//! # Determinism
//!
//! Entities iterate in sorted (`BTreeMap`) order, so a given sequence of stores
//! always yields the same diffs in the same order: creates first, then updates,
//! then destroys — each sorted by entity id. An unchanged store projects to an
//! empty frame.
//!
//! # Forbidden convenience logic
//!
//! No drawing, no GPU/scene-graph concepts, no interpolation, no TypeScript.
//! Geometry is treated as fixed per node — a geometry change means destroy +
//! create, not an update.

#![forbid(unsafe_code)]

pub mod json;
pub mod presentation;
pub mod voxel;

pub use presentation::{
    NodePresentation, ProjectionMode, RenderProjectionDiagnostic, RenderRegistry, RenderSource,
    RenderSourceKind, ScenePresentation, ScenePresentationProjector, SpriteRuntime,
};
pub use voxel::{
    to_payload_descriptor, ChunkMeshStrategy, VisibleFaceStrategy, VoxelChunkProjector,
    VoxelProjectionDiagnostic, VoxelProjectionInstance, VoxelProjectionInstanceError,
};

use std::collections::BTreeMap;

use core_ids::EntityId;
use core_state::{EntityRecord, StateStore};
use protocol_render::{
    Geometry, Material, RenderDiff, RenderFrameDiff, RenderHandle, RenderLayer, RenderMetadata,
    RenderNode, Transform,
};

/// Decides what render node an entity projects to. The diff machinery in
/// [`RenderProjector`] is shared; only the per-entity appearance varies.
pub trait NodeProjection {
    fn project_entity(&self, record: &EntityRecord) -> RenderNode;
}

/// Stateful projector: maps entities to stable handles and emits retained diffs.
pub struct RenderProjector<P> {
    projection: P,
    handles: BTreeMap<EntityId, RenderHandle>,
    last: BTreeMap<EntityId, RenderNode>,
    next_handle: u64,
}

impl<P: NodeProjection> RenderProjector<P> {
    /// A projector using the given per-entity projection. Handles start at 1.
    pub fn new(projection: P) -> Self {
        Self {
            projection,
            handles: BTreeMap::new(),
            last: BTreeMap::new(),
            next_handle: 1,
        }
    }

    /// Project `store` and return the diffs that advance the renderer's scene
    /// from the last projection to this one. Calling it again on an unchanged
    /// store returns an empty frame.
    pub fn project(&mut self, store: &StateStore) -> RenderFrameDiff {
        let current: BTreeMap<EntityId, RenderNode> = store
            .entities()
            .map(|r| (r.id, self.projection.project_entity(r)))
            .collect();

        let mut frame = RenderFrameDiff::new();

        // Creates: entities present now but not last frame (sorted by id).
        for (id, node) in &current {
            if !self.last.contains_key(id) {
                let handle = self.allocate(*id);
                frame.push(RenderDiff::Create {
                    handle,
                    parent: None,
                    node: node.clone(),
                });
            }
        }

        // Updates: entities present both frames whose node changed.
        for (id, node) in &current {
            if let Some(prev) = self.last.get(id) {
                if prev != node {
                    let handle = self.handles[id];
                    frame.push(update_diff(handle, prev, node));
                }
            }
        }

        // Destroys: entities gone this frame (sorted by id).
        let removed: Vec<EntityId> = self
            .last
            .keys()
            .filter(|id| !current.contains_key(id))
            .copied()
            .collect();
        for id in removed {
            let handle = self
                .handles
                .remove(&id)
                .expect("a projected entity must have a handle");
            frame.push(RenderDiff::Destroy { handle });
        }

        self.last = current;
        frame
    }

    /// The stable handle currently assigned to `entity`, if it is projected.
    pub fn handle_of(&self, entity: EntityId) -> Option<RenderHandle> {
        self.handles.get(&entity).copied()
    }

    fn allocate(&mut self, id: EntityId) -> RenderHandle {
        if let Some(h) = self.handles.get(&id) {
            return *h;
        }
        let handle = RenderHandle::new(self.next_handle);
        self.next_handle += 1;
        self.handles.insert(id, handle);
        handle
    }
}

/// Build an `Update` carrying only the facets that actually changed.
fn update_diff(handle: RenderHandle, prev: &RenderNode, node: &RenderNode) -> RenderDiff {
    RenderDiff::Update {
        handle,
        transform: (prev.transform != node.transform).then_some(node.transform),
        material: (prev.material != node.material).then_some(node.material),
        visible: (prev.visible != node.visible).then_some(node.visible),
        metadata: (prev.metadata != node.metadata).then(|| node.metadata.clone()),
    }
}

/// The default scene projection: one abstract cube per entity, placed along the
/// x-axis by its id, labelled and tagged from its record.
pub struct SceneProjection;

impl NodeProjection for SceneProjection {
    fn project_entity(&self, record: &EntityRecord) -> RenderNode {
        let id = record.id.raw();
        RenderNode {
            geometry: Geometry::Cube,
            material: Material::DEFAULT,
            transform: Transform {
                translation: [id as f32, 0.0, 0.0],
                ..Transform::IDENTITY
            },
            visible: true,
            layer: RenderLayer::Scene,
            metadata: RenderMetadata {
                source: Some(record.id),
                tags: record.tags.iter().copied().collect(),
                label: Some(format!("entity {id}")),
            },
        }
    }
}

/// A projector using the default [`SceneProjection`].
pub fn scene_projector() -> RenderProjector<SceneProjection> {
    RenderProjector::new(SceneProjection)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core_ids::{EntityId, TagId};

    fn store_with(entities: &[u64]) -> StateStore {
        let mut s = StateStore::new();
        for &e in entities {
            s.insert_entity(EntityId::new(e));
        }
        s
    }

    #[test]
    fn projects_create_diffs_for_new_entities_in_sorted_order() {
        let store = store_with(&[2, 1]);
        let mut p = scene_projector();
        let frame = p.project(&store);

        assert_eq!(frame.len(), 2);
        // Sorted by entity id: entity 1 first, then 2.
        match &frame.ops[0] {
            RenderDiff::Create { handle, node, .. } => {
                assert_eq!(*handle, RenderHandle::new(1));
                assert_eq!(node.metadata.source, Some(EntityId::new(1)));
                assert!(matches!(node.geometry, Geometry::Cube));
            }
            other => panic!("expected create, got {other:?}"),
        }
        assert_eq!(p.handle_of(EntityId::new(2)), Some(RenderHandle::new(2)));
    }

    #[test]
    fn unchanged_store_projects_empty_frame() {
        let store = store_with(&[1, 2]);
        let mut p = scene_projector();
        let _ = p.project(&store);
        let second = p.project(&store);
        assert!(second.is_empty(), "no change must project no diffs");
    }

    #[test]
    fn tag_change_projects_a_metadata_update_only() {
        let mut store = store_with(&[1]);
        store.insert_tag(TagId::new(7));
        let mut p = scene_projector();
        let _ = p.project(&store);

        // Add a tag to entity 1.
        store
            .entity_mut(EntityId::new(1))
            .unwrap()
            .tags
            .insert(TagId::new(7));
        let frame = p.project(&store);

        assert_eq!(frame.len(), 1);
        match &frame.ops[0] {
            RenderDiff::Update {
                handle,
                transform,
                material,
                visible,
                metadata,
            } => {
                assert_eq!(*handle, RenderHandle::new(1));
                assert!(transform.is_none());
                assert!(material.is_none());
                assert!(visible.is_none());
                assert_eq!(metadata.as_ref().unwrap().tags, vec![TagId::new(7)]);
            }
            other => panic!("expected update, got {other:?}"),
        }
    }

    #[test]
    fn removed_entity_projects_a_destroy_with_its_handle() {
        let mut store = store_with(&[1, 2]);
        let mut p = scene_projector();
        let _ = p.project(&store);
        let h2 = p.handle_of(EntityId::new(2)).unwrap();

        store.remove_entity(EntityId::new(2));
        let frame = p.project(&store);

        assert_eq!(frame.len(), 1);
        assert!(matches!(
            frame.ops[0],
            RenderDiff::Destroy { handle } if handle == h2
        ));
        assert_eq!(p.handle_of(EntityId::new(2)), None);
    }

    #[test]
    fn mixed_frame_orders_creates_then_updates_then_destroys() {
        let mut store = store_with(&[1, 2]);
        store.insert_tag(TagId::new(5));
        let mut p = scene_projector();
        let _ = p.project(&store);

        // Frame 2: add entity 3 (create), tag entity 1 (update), remove entity 2 (destroy).
        store.insert_entity(EntityId::new(3));
        store
            .entity_mut(EntityId::new(1))
            .unwrap()
            .tags
            .insert(TagId::new(5));
        store.remove_entity(EntityId::new(2));
        let frame = p.project(&store);

        assert_eq!(frame.len(), 3);
        assert!(matches!(frame.ops[0], RenderDiff::Create { .. }));
        assert!(matches!(frame.ops[1], RenderDiff::Update { .. }));
        assert!(matches!(frame.ops[2], RenderDiff::Destroy { .. }));
    }

    /// Build the two-frame integration scenario used by the cross-language
    /// fixture: frame 1 creates entities 1 & 2; frame 2 adds 3, tags 1, removes 2.
    fn bridge_sequence() -> Vec<protocol_render::RenderFrameDiff> {
        let mut store = StateStore::new();
        store.insert_entity(EntityId::new(1));
        store.insert_entity(EntityId::new(2));
        store.insert_tag(TagId::new(5));
        let mut p = scene_projector();

        let frame1 = p.project(&store);

        store.insert_entity(EntityId::new(3));
        store
            .entity_mut(EntityId::new(1))
            .unwrap()
            .tags
            .insert(TagId::new(5));
        store.remove_entity(EntityId::new(2));
        let frame2 = p.project(&store);

        vec![frame1, frame2]
    }

    /// The Rust render bridge still emits exactly the committed cross-language
    /// fixture that `runtime-bridge` (decode) and `renderer-three` consume. If this drifts,
    /// the render *protocol or Rust bridge* changed — fix Rust, regenerate the
    /// fixture, and re-run the TS decode/renderer tests.
    #[test]
    fn bridge_emits_the_committed_render_fixture() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .expect("repo root")
            .join("harness/fixtures/render-diffs/bridge-sequence.json");
        let golden = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert_eq!(
            json::encode_sequence(&bridge_sequence()),
            golden,
            "render bridge output drifted from harness/fixtures/render-diffs/bridge-sequence.json"
        );
    }

    #[test]
    fn handles_are_stable_across_frames() {
        let mut store = store_with(&[1]);
        let mut p = scene_projector();
        let _ = p.project(&store);
        let h1 = p.handle_of(EntityId::new(1)).unwrap();

        store.insert_entity(EntityId::new(2));
        let _ = p.project(&store);
        assert_eq!(
            p.handle_of(EntityId::new(1)),
            Some(h1),
            "handle must not change"
        );
    }
}
