//! Atomic scene → authority bootstrap (scene-capability-01, "Bootstrap posture").
//!
//! Bootstrap is **one atomic authority initialization**, not N ordinary
//! create-entity commands. The flow is two-phase so authority is never partially
//! mutated:
//!
//! 1. [`BootstrapPlan::prepare`] validates the whole document and the schema/asset
//!    context and builds a deterministic plan. Any failure returns here, before a
//!    [`SpatialSessionState`] exists.
//! 2. [`BootstrapPlan::apply`] turns the plan into a populated [`SpatialSessionState`] plus
//!    a single [`BootstrapRecord`] — the one replay/audit unit for the whole
//!    initialization, with a deterministic world hash.
//!
//! Entity ids are allocated deterministically (ascending scene-node id from a
//! base), and scene initial transforms are copied into authority runtime
//! transforms. After bootstrap the world is authority-owned and free to diverge
//! from the scene document (see [`SpatialSessionState::set_transform`]).

use core_ids::{EntityId, RuntimeSessionId, SceneId, SceneNodeId};

use crate::document::FlatSceneDocument;
use crate::spatial_session::{SpatialSessionHash, SpatialSessionState};
use crate::validate::{validate, SceneValidationReport};

/// The scene schema version this bootstrap understands. A real migration policy
/// (scene-capability-01, "Decisions to make") is future work; for now an
/// unsupported version fails closed rather than guessing.
pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

/// The default first entity id allocated to scene-sourced entities.
pub const DEFAULT_BASE_ENTITY_ID: EntityId = EntityId::new(1);

/// Why a scene could not be prepared for bootstrap. Returned *before* any
/// authority state is created, so a rejected scene never partially mutates a
/// world.
#[derive(Debug, Clone, PartialEq)]
pub enum BootstrapError {
    /// The scene failed structural/semantic validation.
    Invalid(SceneValidationReport),
    /// The document's schema version is not supported by this engine build.
    UnsupportedSchemaVersion { found: u32, supported: u32 },
}

/// One node's place in the deterministic bootstrap plan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlannedEntity {
    /// The authored scene node this entity comes from.
    pub node: SceneNodeId,
    /// The runtime entity id allocated for it.
    pub entity: EntityId,
}

/// A validated, deterministic bootstrap plan. Holding one is proof the scene
/// passed validation; [`BootstrapPlan::apply`] is therefore infallible.
#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapPlan {
    scene_id: SceneId,
    runtime_session_id: RuntimeSessionId,
    schema_version: u32,
    /// Node→entity allocations in ascending scene-node id order.
    allocations: Vec<PlannedEntity>,
    /// The canonicalized document the plan was built from (carries transforms).
    doc: FlatSceneDocument,
}

impl BootstrapPlan {
    /// Validate `doc` and build a deterministic plan that bootstraps into
    /// `runtime_session_id`, allocating entity ids from [`DEFAULT_BASE_ENTITY_ID`].
    pub fn prepare(
        doc: &FlatSceneDocument,
        runtime_session_id: RuntimeSessionId,
    ) -> Result<BootstrapPlan, BootstrapError> {
        Self::prepare_with_base(doc, runtime_session_id, DEFAULT_BASE_ENTITY_ID)
    }

    /// Like [`BootstrapPlan::prepare`] but with an explicit base entity id, for
    /// callers threading their own allocator.
    pub fn prepare_with_base(
        doc: &FlatSceneDocument,
        runtime_session_id: RuntimeSessionId,
        base_entity: EntityId,
    ) -> Result<BootstrapPlan, BootstrapError> {
        if doc.schema_version != SUPPORTED_SCHEMA_VERSION {
            return Err(BootstrapError::UnsupportedSchemaVersion {
                found: doc.schema_version,
                supported: SUPPORTED_SCHEMA_VERSION,
            });
        }
        let report = validate(doc);
        if !report.is_ok() {
            return Err(BootstrapError::Invalid(report));
        }

        // Canonicalize so allocation order (ascending node id) is deterministic
        // regardless of the authoring order the document arrived in.
        let doc = doc.canonical();
        let allocations = doc
            .nodes
            .iter()
            .enumerate()
            .map(|(i, rec)| PlannedEntity {
                node: rec.id,
                entity: EntityId::new(base_entity.raw() + i as u64),
            })
            .collect();

        Ok(BootstrapPlan {
            scene_id: doc.id,
            runtime_session_id,
            schema_version: doc.schema_version,
            allocations,
            doc,
        })
    }

    /// The node→entity allocations, in deterministic order.
    pub fn allocations(&self) -> &[PlannedEntity] {
        &self.allocations
    }

    /// Apply the plan as one atomic initialization: populate a fresh world with
    /// every scene-sourced entity (initial transforms copied in) and return it
    /// alongside the single [`BootstrapRecord`] for replay/audit.
    pub fn apply(&self) -> (SpatialSessionState, BootstrapRecord) {
        let mut world = SpatialSessionState::empty(self.runtime_session_id);
        // `allocations` is parallel to `doc.nodes` (both canonical order).
        for (alloc, rec) in self.allocations.iter().zip(self.doc.nodes.iter()) {
            debug_assert_eq!(alloc.node, rec.id);
            let inserted = world.insert_scene_entity(alloc.entity, alloc.node, rec.transform);
            debug_assert!(
                inserted,
                "validated plan must allocate unique entities/nodes"
            );
        }
        let record = BootstrapRecord {
            scene_id: self.scene_id,
            runtime_session_id: self.runtime_session_id,
            schema_version: self.schema_version,
            node_count: self.doc.nodes.len(),
            entity_count: world.entity_count(),
            spatial_session_hash: world.hash(),
            source_trace: self.allocations.clone(),
        };
        (world, record)
    }
}

/// Convenience: prepare and apply in one call. Errors if the scene is invalid.
pub fn bootstrap_scene(
    doc: &FlatSceneDocument,
    runtime_session_id: RuntimeSessionId,
) -> Result<(SpatialSessionState, BootstrapRecord), BootstrapError> {
    Ok(BootstrapPlan::prepare(doc, runtime_session_id)?.apply())
}

/// The single replay/audit unit recorded for one scene bootstrap. Replay sees
/// this one initialization unit, **not** N ordinary create events.
#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapRecord {
    pub scene_id: SceneId,
    pub runtime_session_id: RuntimeSessionId,
    pub schema_version: u32,
    pub node_count: usize,
    pub entity_count: usize,
    /// Deterministic fingerprint of the world produced by this bootstrap.
    pub spatial_session_hash: SpatialSessionHash,
    /// The source trace `scene node → runtime entity`. Render-handle/projection
    /// metadata is appended later, at projection time (scene-capability-01).
    pub source_trace: Vec<PlannedEntity>,
}

impl BootstrapRecord {
    /// Stable label identifying this as one bootstrap semantic unit in audit logs.
    pub fn replay_unit_label(&self) -> &'static str {
        "scene.bootstrap"
    }
}
