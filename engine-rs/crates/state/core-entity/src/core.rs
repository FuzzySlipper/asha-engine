//! The minimal generic entity core: identity, lifecycle, source, labels.
//!
//! Per the design gate (entity-model-design §1), the core is intentionally small
//! and stores **no position, no render handle, no collider, no parent**. Spatial
//! transform, render projection, collision, containment, controller association,
//! and asset binding are all *optional capabilities* (see [`crate::capability`]),
//! never core fields.

use core_ids::{EntityId, ProcessId, SceneNodeId, SubjectId, TagId};

use core_assets::AssetReference;

/// Where an entity came from. A closed enum — each variant proves a different
/// fixture family can exist without product-domain assumptions (design §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntitySource {
    /// Created by atomic scene bootstrap; carries the authored node for the
    /// `scene node → entity` source trace.
    SceneBootstrap { node: SceneNodeId },
    /// Created at runtime by an authority command; no authored provenance.
    RuntimeCreated { by: Option<ProcessId> },
    /// Instantiated from a catalog/asset source.
    Imported { asset: AssetReference },
    /// Created by devtools/diagnostics. Flagged so it is never mistaken for
    /// product/world authority and can be policy-excluded from saves (design §4).
    DiagnosticTooling,
    /// Proposed by a policy process and accepted by authority (authority still
    /// owns the commit; policy never mutates directly).
    PolicyProposed { by: SubjectId },
}

impl EntitySource {
    /// Stable discriminant label for diagnostics and the save identity tuple.
    pub fn label(&self) -> &'static str {
        match self {
            EntitySource::SceneBootstrap { .. } => "sceneBootstrap",
            EntitySource::RuntimeCreated { .. } => "runtimeCreated",
            EntitySource::Imported { .. } => "imported",
            EntitySource::DiagnosticTooling => "diagnosticTooling",
            EntitySource::PolicyProposed { .. } => "policyProposed",
        }
    }

    /// The authored scene node, for scene-sourced entities only.
    pub fn scene_node(&self) -> Option<SceneNodeId> {
        match self {
            EntitySource::SceneBootstrap { node } => Some(*node),
            _ => None,
        }
    }

    /// Whether this source is excluded from durable saves by default policy
    /// (only `DiagnosticTooling`).
    pub fn is_save_excluded_by_default(&self) -> bool {
        matches!(self, EntitySource::DiagnosticTooling)
    }
}

/// An entity's existence state. `Active ↔ Disabled → Tombstoned (terminal)`
/// (design §4). A tombstoned id is retired and never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityLifecycle {
    /// Participates in ticks/queries/projection.
    Active,
    /// Exists in authority and saves but is excluded from projection/movement;
    /// reversible back to `Active`.
    Disabled,
    /// Logically destroyed. The id is retired; a tombstone is retained for
    /// deterministic replay and dangling-reference diagnostics.
    Tombstoned,
}

impl EntityLifecycle {
    pub fn label(self) -> &'static str {
        match self {
            EntityLifecycle::Active => "active",
            EntityLifecycle::Disabled => "disabled",
            EntityLifecycle::Tombstoned => "tombstoned",
        }
    }

    /// Whether the entity still logically exists (not tombstoned).
    pub fn is_alive(self) -> bool {
        !matches!(self, EntityLifecycle::Tombstoned)
    }
}

/// The minimal generic entity core record. Every runtime entity has exactly these
/// fields regardless of capabilities (design §1).
#[derive(Debug, Clone, PartialEq)]
pub struct EntityCore {
    pub id: EntityId,
    pub lifecycle: EntityLifecycle,
    pub source: EntitySource,
    /// Authority-owned classification: an ordered set of typed `TagId`s — **not**
    /// a free-form string→any metadata map (avoids the "weak ECS soup" tripwire).
    pub labels: Vec<TagId>,
}

impl EntityCore {
    /// A new `Active` entity with the given source and no labels.
    pub fn new(id: EntityId, source: EntitySource) -> Self {
        EntityCore {
            id,
            lifecycle: EntityLifecycle::Active,
            source,
            labels: Vec::new(),
        }
    }

    /// Whether `tag` is present in the label set.
    pub fn has_label(&self, tag: TagId) -> bool {
        self.labels.contains(&tag)
    }
}
