//! Live runtime world authority produced by scene bootstrap.
//!
//! A [`WorldState`] is the live authority that scene-capability-01 distinguishes
//! from the authored `SceneDocument`: it owns **runtime** transforms (seeded from
//! the scene's initial transforms at bootstrap, then authority-owned and free to
//! diverge) and the source trace `scene node → runtime entity`. The authored
//! document is never mutated by runtime movement.
//!
//! # Composition over a generic entity substrate (#2388, design §0/§7)
//!
//! `WorldState` no longer embeds a mandatory `transform` in every entity record —
//! that was the "entity means thing-with-a-position" anti-pattern the entity design
//! gate forbids. Instead it *composes* [`core_entity::EntityStore`]: identity +
//! lifecycle + source live in the entity core, and the runtime transform is an
//! **optional `TransformCapability`** that bootstrap attaches to scene-sourced
//! entities. The world's public transform/provenance API is unchanged, and the
//! [`WorldState::hash`] byte sequence is preserved (every world entity carries a
//! transform today, so the fingerprint is identical).
//!
//! # Spatial-world invariant (#2425, decision: option 1)
//!
//! The generic entity model treats transform as an *optional* capability, but
//! `WorldState` is the **spatial scene-runtime world**: every live entity it holds
//! has the transform capability. This is enforced by construction — the only ways
//! to add an entity are [`WorldState::insert_scene_entity`] and
//! [`WorldState::create_runtime_entity`], both of which attach a transform, and
//! there is no public destroy/disable/detach path that could strip one. Non-spatial
//! / logical entities therefore do **not** live in a `WorldState`; they belong in a
//! separate [`core_entity::EntityStore`] scope.
//!
//! Consequences this module guarantees:
//! * [`WorldState::entities`] and [`WorldState::hash`] iterate **live** entities
//!   only, so a (currently unreachable) tombstone — whose capabilities are cleared
//!   — can never feed a transform-less record into the fingerprint.
//! * [`WorldState::hash`] therefore cannot panic from any normal public API path.
//!   The `expect` it contains documents the spatial-world invariant for future
//!   maintainers, and `tests/world_invariant.rs` proves it holds across the public
//!   surface. A worker who wants non-spatial entities in the world authority must
//!   revisit this decision (option 2: hash `Option<Transform>` deterministically),
//!   not silently insert transform-less entities here.

use std::collections::BTreeMap;

use core_entity::{EntityLifecycleCommand, EntitySource, EntityStore, EntityTransform, Quat};
use core_ids::{EntityId, SceneNodeId, WorldId};

use crate::transform::SceneTransform;

/// A compact, deterministic fingerprint of a [`WorldState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldHash(pub u64);

/// A read view of one entity's runtime world state: its (optional) runtime
/// transform and scene-node provenance. Transform is `Option` because, in the
/// composed model, transform is a capability — not every entity has one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityRuntime {
    /// Authority-owned runtime transform, if the entity has the transform
    /// capability. Seeded from the scene initial transform at bootstrap.
    pub transform: Option<SceneTransform>,
    /// The scene node this entity was bootstrapped from, or `None` for an entity
    /// created at runtime (no authored provenance).
    pub source_node: Option<SceneNodeId>,
}

/// Live world authority: a composed entity store plus scene-node provenance index.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldState {
    id: WorldId,
    entities: EntityStore,
    /// Reverse trace: scene node (raw) → runtime entity.
    node_to_entity: BTreeMap<u64, EntityId>,
}

impl WorldState {
    /// An empty world with no entities.
    pub fn empty(id: WorldId) -> Self {
        Self {
            id,
            entities: EntityStore::new(),
            node_to_entity: BTreeMap::new(),
        }
    }

    pub fn id(&self) -> WorldId {
        self.id
    }

    pub fn entity_count(&self) -> usize {
        self.entities.total_count()
    }

    /// Insert a scene-sourced entity and attach its initial transform capability.
    /// Returns `false` (no-op) if the entity id or the source node is already
    /// present, so bootstrap stays one-to-one.
    pub(crate) fn insert_scene_entity(
        &mut self,
        entity: EntityId,
        node: SceneNodeId,
        transform: SceneTransform,
    ) -> bool {
        if self.entities.contains(entity) || self.node_to_entity.contains_key(&node.raw()) {
            return false;
        }
        self.entities
            .apply(EntityLifecycleCommand::Create {
                id: entity,
                source: EntitySource::SceneBootstrap { node },
                labels: Vec::new(),
            })
            .expect("fresh scene entity id is unique");
        self.entities
            .attach_transform(entity, to_entity_transform(transform));
        self.node_to_entity.insert(node.raw(), entity);
        true
    }

    /// Create an entity at runtime with no scene provenance, attaching a runtime
    /// transform capability. Returns `false` if the id is already present.
    pub fn create_runtime_entity(&mut self, entity: EntityId, transform: SceneTransform) -> bool {
        if self.entities.contains(entity) {
            return false;
        }
        self.entities
            .apply(EntityLifecycleCommand::Create {
                id: entity,
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .expect("fresh runtime entity id is unique");
        self.entities
            .attach_transform(entity, to_entity_transform(transform));
        true
    }

    /// The runtime view for `entity`, if present.
    pub fn entity(&self, entity: EntityId) -> Option<EntityRuntime> {
        let core = self.entities.core(entity)?;
        Some(EntityRuntime {
            transform: self
                .entities
                .transform(entity)
                .map(|c| to_scene_transform(c.transform)),
            source_node: core.source.scene_node(),
        })
    }

    /// The runtime transform for `entity`, if it has one.
    pub fn transform(&self, entity: EntityId) -> Option<SceneTransform> {
        self.entities
            .transform(entity)
            .map(|c| to_scene_transform(c.transform))
    }

    /// The scene node `entity` was bootstrapped from, if any.
    pub fn source_node(&self, entity: EntityId) -> Option<SceneNodeId> {
        self.entities
            .core(entity)
            .and_then(|c| c.source.scene_node())
    }

    /// The runtime entity a scene node bootstrapped into, if any.
    pub fn entity_for_node(&self, node: SceneNodeId) -> Option<EntityId> {
        self.node_to_entity.get(&node.raw()).copied()
    }

    /// Overwrite an entity's runtime transform (authority-owned movement).
    /// Returns `false` if the entity is unknown or tombstoned. Never touches scene
    /// documents.
    pub fn set_transform(&mut self, entity: EntityId, transform: SceneTransform) -> bool {
        self.entities
            .attach_transform(entity, to_entity_transform(transform))
    }

    /// Live entities (with their runtime views) in ascending id order. Tombstoned
    /// entities are excluded: in the spatial-world model (see module docs) a live
    /// world entity always has a transform, while a tombstone has none.
    pub fn entities(&self) -> impl Iterator<Item = (EntityId, EntityRuntime)> + '_ {
        self.entities
            .entities()
            .filter(|core| core.lifecycle.is_alive())
            .map(|core| {
                (
                    core.id,
                    EntityRuntime {
                        transform: self
                            .entities
                            .transform(core.id)
                            .map(|c| to_scene_transform(c.transform)),
                        source_node: core.source.scene_node(),
                    },
                )
            })
    }

    /// Deterministic FNV-1a fingerprint of the world: id, then each **live** entity
    /// (in ascending id order) with its transform bits and source node. The byte
    /// sequence is preserved from the pre-composition layout so world hashes (and
    /// the bootstrap-summary golden) stay stable.
    ///
    /// Cannot panic from a normal public API path: by the spatial-world invariant
    /// (module docs) every live world entity has a transform, and only live
    /// entities are hashed. The `expect` documents that invariant for maintainers.
    pub fn hash(&self) -> WorldHash {
        let mut h = Fnv1a::new();
        h.write_u64(self.id.raw());
        h.write_u8(0x01); // entities section
        for (id, rec) in self.entities() {
            h.write_u64(id.raw());
            let transform = rec.transform.expect(
                "spatial-world invariant: every live WorldState entity has a transform \
                 (see module docs, #2425)",
            );
            hash_transform(&mut h, &transform);
            match rec.source_node {
                Some(n) => {
                    h.write_u8(1);
                    h.write_u64(n.raw());
                }
                None => h.write_u8(0),
            }
        }
        WorldHash(h.finish())
    }
}

// ── SceneTransform ⇄ EntityTransform ─────────────────────────────────────────--
// A straight field copy: the two shapes are intentionally identical (design §value)
// so this is a representation bridge, not a reinterpretation.

fn to_entity_transform(t: SceneTransform) -> EntityTransform {
    EntityTransform {
        translation: t.translation,
        rotation: Quat {
            x: t.rotation.x,
            y: t.rotation.y,
            z: t.rotation.z,
            w: t.rotation.w,
        },
        scale: t.scale,
    }
}

fn to_scene_transform(t: EntityTransform) -> SceneTransform {
    SceneTransform {
        translation: t.translation,
        rotation: crate::transform::Quat {
            x: t.rotation.x,
            y: t.rotation.y,
            z: t.rotation.z,
            w: t.rotation.w,
        },
        scale: t.scale,
    }
}

fn hash_transform(h: &mut Fnv1a, t: &SceneTransform) {
    for f in [
        t.translation.x,
        t.translation.y,
        t.translation.z,
        t.rotation.x,
        t.rotation.y,
        t.rotation.z,
        t.rotation.w,
        t.scale.x,
        t.scale.y,
        t.scale.z,
    ] {
        h.write_u64(f.to_bits() as u64);
    }
}

// ── FNV-1a hasher (mirrors core-snapshot's deterministic fingerprint) ─────────

const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
const FNV_PRIME: u64 = 1_099_511_628_211;

struct Fnv1a(u64);

impl Fnv1a {
    fn new() -> Self {
        Fnv1a(FNV_OFFSET)
    }

    fn write_u8(&mut self, b: u8) {
        self.0 ^= b as u64;
        self.0 = self.0.wrapping_mul(FNV_PRIME);
    }

    fn write_u64(&mut self, v: u64) {
        for byte in v.to_le_bytes() {
            self.write_u8(byte);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
