//! The composed entity store: core records + optional capability tables, with the
//! lifecycle applier, deterministic hashing, and save/restore (design §1, §2, §4).
//!
//! `SpatialSessionState` (in `core-scene`) will *compose* this store rather than embedding a
//! mandatory transform in every entity (design §0 finding). The store itself is
//! authority-only: it knows nothing of render, scene documents, or TypeScript.

use std::collections::BTreeMap;

use core_assets::{AssetReference, AssetVersionReq};
use core_ids::{EntityId, TagId};

use crate::capability::{
    AssetBindingCapability, BoundsCapability, CollisionCapability, ContainmentCapability,
    ControllerCapability, RenderProjectionCapability, TransformCapability,
};
use crate::command::{EntityLifecycleCommand, EntityLifecycleError, EntityLifecycleEvent};
use crate::core::{EntityCore, EntityLifecycle, EntitySource};
use crate::value::{Aabb, EntityTransform, Quat};

/// A compact, deterministic fingerprint of an [`EntityStore`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityHash(pub u64);

/// Authority store: entity cores keyed by id (deterministic `BTreeMap` order) plus
/// one table per optional capability. Capability tables only hold entries for
/// entities that have that capability.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntityStore {
    cores: BTreeMap<EntityId, EntityCore>,
    transforms: BTreeMap<EntityId, TransformCapability>,
    bounds: BTreeMap<EntityId, BoundsCapability>,
    render: BTreeMap<EntityId, RenderProjectionCapability>,
    collision: BTreeMap<EntityId, CollisionCapability>,
    containment: BTreeMap<EntityId, ContainmentCapability>,
    controller: BTreeMap<EntityId, ControllerCapability>,
    asset_binding: BTreeMap<EntityId, AssetBindingCapability>,
    /// Relation 1 (design §5): spatial transform attachment, child → parent. Only
    /// this relation propagates transforms; cycle-checked.
    transform_parent: BTreeMap<EntityId, EntityId>,
    /// Relation 3 (design §5): source ancestry, derived entity → origin. A
    /// read-only provenance trace; not transform propagation, not cycle-walked.
    derived_from: BTreeMap<EntityId, EntityId>,
}

impl EntityStore {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Lifecycle applier ─────────────────────────────────────────────────────

    /// Validate and apply one lifecycle command. On success the store is mutated
    /// and the corresponding event returned; on failure **nothing is mutated** and
    /// a classified error is returned (atomic, fail-closed).
    pub fn apply(
        &mut self,
        command: EntityLifecycleCommand,
    ) -> Result<EntityLifecycleEvent, EntityLifecycleError> {
        match command {
            EntityLifecycleCommand::Create { id, source, labels } => {
                self.create(id, source, labels)
            }
            EntityLifecycleCommand::Destroy { id } => self.destroy(id),
            EntityLifecycleCommand::Disable { id } => self.disable(id),
            EntityLifecycleCommand::Enable { id } => self.enable(id),
            EntityLifecycleCommand::AddLabel { id, tag } => self.add_label(id, tag),
            EntityLifecycleCommand::RemoveLabel { id, tag } => self.remove_label(id, tag),
        }
    }

    fn create(
        &mut self,
        id: EntityId,
        source: EntitySource,
        labels: Vec<TagId>,
    ) -> Result<EntityLifecycleEvent, EntityLifecycleError> {
        if let Some(existing) = self.cores.get(&id) {
            return Err(match existing.lifecycle {
                EntityLifecycle::Tombstoned => EntityLifecycleError::IdRetired { id },
                _ => EntityLifecycleError::AlreadyExists { id },
            });
        }
        // Normalise labels to an ordered set (first occurrence wins).
        let mut normalised: Vec<TagId> = Vec::with_capacity(labels.len());
        for tag in labels {
            if !normalised.contains(&tag) {
                normalised.push(tag);
            }
        }
        let mut core = EntityCore::new(id, source.clone());
        core.labels = normalised.clone();
        self.cores.insert(id, core);
        Ok(EntityLifecycleEvent::Created {
            id,
            source,
            labels: normalised,
        })
    }

    fn destroy(&mut self, id: EntityId) -> Result<EntityLifecycleEvent, EntityLifecycleError> {
        let core = self.alive_core_mut(id)?;
        core.lifecycle = EntityLifecycle::Tombstoned;
        // A destroyed entity has no live capabilities; the tombstone core is
        // retained for replay/dangling-reference diagnostics.
        self.clear_capabilities(id);
        Ok(EntityLifecycleEvent::Destroyed { id })
    }

    fn disable(&mut self, id: EntityId) -> Result<EntityLifecycleEvent, EntityLifecycleError> {
        let core = self.alive_core_mut(id)?;
        match core.lifecycle {
            EntityLifecycle::Active => {
                core.lifecycle = EntityLifecycle::Disabled;
                Ok(EntityLifecycleEvent::Disabled { id })
            }
            from => Err(EntityLifecycleError::InvalidTransition {
                id,
                from,
                op: "disable",
            }),
        }
    }

    fn enable(&mut self, id: EntityId) -> Result<EntityLifecycleEvent, EntityLifecycleError> {
        let core = self.alive_core_mut(id)?;
        match core.lifecycle {
            EntityLifecycle::Disabled => {
                core.lifecycle = EntityLifecycle::Active;
                Ok(EntityLifecycleEvent::Enabled { id })
            }
            from => Err(EntityLifecycleError::InvalidTransition {
                id,
                from,
                op: "enable",
            }),
        }
    }

    fn add_label(
        &mut self,
        id: EntityId,
        tag: TagId,
    ) -> Result<EntityLifecycleEvent, EntityLifecycleError> {
        let core = self.alive_core_mut(id)?;
        if core.labels.contains(&tag) {
            return Err(EntityLifecycleError::LabelAlreadyPresent { id, tag });
        }
        core.labels.push(tag);
        Ok(EntityLifecycleEvent::LabelAdded { id, tag })
    }

    fn remove_label(
        &mut self,
        id: EntityId,
        tag: TagId,
    ) -> Result<EntityLifecycleEvent, EntityLifecycleError> {
        let core = self.alive_core_mut(id)?;
        match core.labels.iter().position(|t| *t == tag) {
            Some(pos) => {
                core.labels.remove(pos);
                Ok(EntityLifecycleEvent::LabelRemoved { id, tag })
            }
            None => Err(EntityLifecycleError::LabelAbsent { id, tag }),
        }
    }

    /// Resolve a mutable core that exists and is not tombstoned, with classified
    /// errors otherwise. Borrows the map immutably first so no partial mutation
    /// can occur before the error is decided.
    fn alive_core_mut(&mut self, id: EntityId) -> Result<&mut EntityCore, EntityLifecycleError> {
        match self.cores.get(&id).map(|c| c.lifecycle) {
            None => Err(EntityLifecycleError::UnknownEntity { id }),
            Some(EntityLifecycle::Tombstoned) => Err(EntityLifecycleError::Tombstoned { id }),
            Some(_) => Ok(self.cores.get_mut(&id).expect("checked present")),
        }
    }

    fn clear_capabilities(&mut self, id: EntityId) {
        self.transforms.remove(&id);
        self.bounds.remove(&id);
        self.render.remove(&id);
        self.collision.remove(&id);
        self.containment.remove(&id);
        self.controller.remove(&id);
        self.asset_binding.remove(&id);
        // Relations the destroyed entity owns.
        self.transform_parent.remove(&id);
        self.derived_from.remove(&id);
        // Per-relation destroy policy for entities that referenced this one:
        // - transform children re-root to world space (mapping removed);
        // - contained members are orphaned (containment removed).
        // Source ancestry (`derived_from`) pointing here is intentionally retained
        // as a dangling provenance trace (design §5 relation 3).
        let detached_children: Vec<EntityId> = self
            .transform_parent
            .iter()
            .filter(|(_, parent)| **parent == id)
            .map(|(child, _)| *child)
            .collect();
        for child in detached_children {
            self.transform_parent.remove(&child);
        }
        let orphaned_members: Vec<EntityId> = self
            .containment
            .iter()
            .filter(|(_, c)| c.container == id)
            .map(|(member, _)| *member)
            .collect();
        for member in orphaned_members {
            self.containment.remove(&member);
        }
    }

    // ── Capability attach/query ───────────────────────────────────────────────
    //
    // Attaching a capability = inserting into its table. Attach is only valid for a
    // live entity; attaching to an unknown/tombstoned entity is a no-op returning
    // `false` (the caller — bootstrap/import — should create the entity first).

    /// Attach (or replace) a capability on a live entity. Returns `false` if the
    /// entity is unknown or tombstoned.
    pub fn attach_transform(&mut self, id: EntityId, transform: EntityTransform) -> bool {
        self.attach(id, |s| {
            s.transforms.insert(id, TransformCapability { transform });
        })
    }

    pub fn attach_bounds(&mut self, id: EntityId, bounds: Aabb) -> bool {
        self.attach(id, |s| {
            s.bounds.insert(id, BoundsCapability { bounds });
        })
    }

    pub fn attach_render_projection(&mut self, id: EntityId, visible: bool) -> bool {
        self.attach(id, |s| {
            s.render.insert(id, RenderProjectionCapability { visible });
        })
    }

    pub fn attach_collision(&mut self, id: EntityId, static_collider: bool) -> bool {
        self.attach(id, |s| {
            s.collision
                .insert(id, CollisionCapability { static_collider });
        })
    }

    pub fn attach_containment(&mut self, id: EntityId, container: EntityId) -> bool {
        self.attach(id, |s| {
            s.containment
                .insert(id, ContainmentCapability { container });
        })
    }

    pub fn attach_controller(&mut self, id: EntityId, controller: ControllerCapability) -> bool {
        self.attach(id, |s| {
            s.controller.insert(id, controller);
        })
    }

    pub fn attach_asset_binding(&mut self, id: EntityId, asset: AssetReference) -> bool {
        self.attach(id, |s| {
            s.asset_binding.insert(id, AssetBindingCapability { asset });
        })
    }

    fn attach(&mut self, id: EntityId, insert: impl FnOnce(&mut Self)) -> bool {
        let alive = self
            .cores
            .get(&id)
            .map(|c| c.lifecycle.is_alive())
            .unwrap_or(false);
        if alive {
            insert(self);
        }
        alive
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    pub fn core(&self, id: EntityId) -> Option<&EntityCore> {
        self.cores.get(&id)
    }

    pub fn lifecycle(&self, id: EntityId) -> Option<EntityLifecycle> {
        self.cores.get(&id).map(|c| c.lifecycle)
    }

    pub fn contains(&self, id: EntityId) -> bool {
        self.cores.contains_key(&id)
    }

    pub fn is_alive(&self, id: EntityId) -> bool {
        self.cores
            .get(&id)
            .map(|c| c.lifecycle.is_alive())
            .unwrap_or(false)
    }

    pub fn transform(&self, id: EntityId) -> Option<&TransformCapability> {
        self.transforms.get(&id)
    }

    pub fn bounds(&self, id: EntityId) -> Option<&BoundsCapability> {
        self.bounds.get(&id)
    }

    pub fn render_projection(&self, id: EntityId) -> Option<&RenderProjectionCapability> {
        self.render.get(&id)
    }

    pub fn collision(&self, id: EntityId) -> Option<&CollisionCapability> {
        self.collision.get(&id)
    }

    pub fn containment(&self, id: EntityId) -> Option<&ContainmentCapability> {
        self.containment.get(&id)
    }

    pub fn controller(&self, id: EntityId) -> Option<&ControllerCapability> {
        self.controller.get(&id)
    }

    pub fn asset_binding(&self, id: EntityId) -> Option<&AssetBindingCapability> {
        self.asset_binding.get(&id)
    }

    // ── Relation table access (used by the relation module) ───────────────────

    /// The transform-attachment parent of `id`, if attached (relation 1).
    pub fn transform_parent_of(&self, id: EntityId) -> Option<EntityId> {
        self.transform_parent.get(&id).copied()
    }

    /// The source-ancestry origin of `id`, if recorded (relation 3, read-only).
    pub fn derived_from(&self, id: EntityId) -> Option<EntityId> {
        self.derived_from.get(&id).copied()
    }

    pub(crate) fn set_transform_parent(&mut self, child: EntityId, parent: EntityId) {
        self.transform_parent.insert(child, parent);
    }

    pub(crate) fn remove_transform_parent(&mut self, child: EntityId) {
        self.transform_parent.remove(&child);
    }

    pub(crate) fn remove_containment(&mut self, member: EntityId) {
        self.containment.remove(&member);
    }

    pub(crate) fn set_derived_from_raw(&mut self, derived: EntityId, origin: EntityId) {
        self.derived_from.insert(derived, origin);
    }

    /// Total number of entity cores, including tombstones.
    pub fn total_count(&self) -> usize {
        self.cores.len()
    }

    /// Number of entities that still logically exist (not tombstoned).
    pub fn alive_count(&self) -> usize {
        self.cores
            .values()
            .filter(|c| c.lifecycle.is_alive())
            .count()
    }

    /// Entity cores in ascending id order (tombstones included).
    pub fn entities(&self) -> impl Iterator<Item = &EntityCore> {
        self.cores.values()
    }

    // ── Deterministic hash (replay stability) ─────────────────────────────────

    /// FNV-1a fingerprint over every entity core and capability table in ascending
    /// id order. Stable across runs; a save→reload must reproduce this exactly.
    pub fn hash(&self) -> EntityHash {
        let mut h = Fnv1a::new();
        h.write_u64(self.cores.len() as u64);
        for (id, core) in &self.cores {
            h.write_u64(id.raw());
            h.write_u8(lifecycle_tag(core.lifecycle));
            hash_source(&mut h, &core.source);
            h.write_u64(core.labels.len() as u64);
            for tag in &core.labels {
                h.write_u64(tag.raw());
            }
            self.hash_capabilities(&mut h, *id);
        }
        EntityHash(h.finish())
    }

    fn hash_capabilities(&self, h: &mut Fnv1a, id: EntityId) {
        match self.transforms.get(&id) {
            Some(t) => {
                h.write_u8(1);
                hash_transform(h, &t.transform);
            }
            None => h.write_u8(0),
        }
        match self.bounds.get(&id) {
            Some(b) => {
                h.write_u8(1);
                hash_vec3(h, b.bounds.min);
                hash_vec3(h, b.bounds.max);
            }
            None => h.write_u8(0),
        }
        match self.render.get(&id) {
            Some(r) => {
                h.write_u8(1);
                h.write_u8(r.visible as u8);
            }
            None => h.write_u8(0),
        }
        match self.collision.get(&id) {
            Some(c) => {
                h.write_u8(1);
                h.write_u8(c.static_collider as u8);
            }
            None => h.write_u8(0),
        }
        match self.containment.get(&id) {
            Some(c) => {
                h.write_u8(1);
                h.write_u64(c.container.raw());
            }
            None => h.write_u8(0),
        }
        match self.controller.get(&id) {
            Some(ControllerCapability::Process(p)) => {
                h.write_u8(1);
                h.write_u64(p.raw());
            }
            Some(ControllerCapability::Subject(s)) => {
                h.write_u8(2);
                h.write_u64(s.raw());
            }
            None => h.write_u8(0),
        }
        match self.asset_binding.get(&id) {
            Some(a) => {
                h.write_u8(1);
                hash_asset_reference(h, &a.asset);
            }
            None => h.write_u8(0),
        }
        match self.transform_parent.get(&id) {
            Some(parent) => {
                h.write_u8(1);
                h.write_u64(parent.raw());
            }
            None => h.write_u8(0),
        }
        match self.derived_from.get(&id) {
            Some(origin) => {
                h.write_u8(1);
                h.write_u64(origin.raw());
            }
            None => h.write_u8(0),
        }
    }

    // ── Save / restore ────────────────────────────────────────────────────────

    /// A full snapshot (including tombstones and `DiagnosticTooling` entities).
    pub fn snapshot(&self) -> EntitySnapshot {
        self.snapshot_filtered(false)
    }

    /// A durable snapshot for world saves: excludes `DiagnosticTooling`-sourced
    /// entities by default policy (design §4).
    pub fn snapshot_durable(&self) -> EntitySnapshot {
        self.snapshot_filtered(true)
    }

    fn snapshot_filtered(&self, exclude_diagnostic: bool) -> EntitySnapshot {
        let mut records = Vec::new();
        for (id, core) in &self.cores {
            if exclude_diagnostic && core.source.is_save_excluded_by_default() {
                continue;
            }
            records.push(EntityRecord {
                core: core.clone(),
                transform: self.transforms.get(id).copied(),
                bounds: self.bounds.get(id).copied(),
                render: self.render.get(id).copied(),
                collision: self.collision.get(id).copied(),
                containment: self.containment.get(id).copied(),
                controller: self.controller.get(id).copied(),
                asset_binding: self.asset_binding.get(id).cloned(),
                transform_parent: self.transform_parent.get(id).copied(),
                derived_from: self.derived_from.get(id).copied(),
            });
        }
        EntitySnapshot { records }
    }

    /// Rebuild a store from a snapshot. Reproduces ids, lifecycle, sources, labels,
    /// capability tables, and therefore the [`EntityStore::hash`] exactly.
    pub fn from_snapshot(snapshot: EntitySnapshot) -> Self {
        let mut store = EntityStore::new();
        for record in snapshot.records {
            let id = record.core.id;
            store.cores.insert(id, record.core);
            if let Some(t) = record.transform {
                store.transforms.insert(id, t);
            }
            if let Some(b) = record.bounds {
                store.bounds.insert(id, b);
            }
            if let Some(r) = record.render {
                store.render.insert(id, r);
            }
            if let Some(c) = record.collision {
                store.collision.insert(id, c);
            }
            if let Some(c) = record.containment {
                store.containment.insert(id, c);
            }
            if let Some(c) = record.controller {
                store.controller.insert(id, c);
            }
            if let Some(a) = record.asset_binding {
                store.asset_binding.insert(id, a);
            }
            if let Some(p) = record.transform_parent {
                store.transform_parent.insert(id, p);
            }
            if let Some(o) = record.derived_from {
                store.derived_from.insert(id, o);
            }
        }
        store
    }

    // ── Deterministic textual dump (golden artifact) ──────────────────────────

    /// A deterministic, human/agent-legible dump of every entity core, its source,
    /// lifecycle, labels, and present capabilities — one entity per line group.
    /// Backs the golden fixtures under `harness/fixtures/entities/`.
    pub fn dump(&self) -> String {
        let mut out = String::new();
        for (id, core) in &self.cores {
            out.push_str(&format!(
                "entity {}  lifecycle {}  source {}",
                id.raw(),
                core.lifecycle.label(),
                source_dump(&core.source),
            ));
            let labels: Vec<String> = core.labels.iter().map(|t| t.raw().to_string()).collect();
            out.push_str(&format!("  labels [{}]", labels.join(",")));
            out.push_str(&format!("  caps [{}]", self.capability_dump(*id)));
            let rels = self.relation_dump(*id);
            if !rels.is_empty() {
                out.push_str(&format!("  rels [{rels}]"));
            }
            out.push('\n');
        }
        out
    }

    fn capability_dump(&self, id: EntityId) -> String {
        let mut caps: Vec<String> = Vec::new();
        if let Some(t) = self.transforms.get(&id) {
            let tr = t.transform.translation;
            caps.push(format!(
                "transform({},{},{})",
                fmt(tr.x),
                fmt(tr.y),
                fmt(tr.z)
            ));
        }
        if let Some(b) = self.bounds.get(&id) {
            caps.push(format!(
                "bounds({},{},{}..{},{},{})",
                fmt(b.bounds.min.x),
                fmt(b.bounds.min.y),
                fmt(b.bounds.min.z),
                fmt(b.bounds.max.x),
                fmt(b.bounds.max.y),
                fmt(b.bounds.max.z),
            ));
        }
        if let Some(r) = self.render.get(&id) {
            caps.push(format!("render(visible={})", r.visible));
        }
        if let Some(c) = self.collision.get(&id) {
            caps.push(format!("collision(static={})", c.static_collider));
        }
        if let Some(c) = self.containment.get(&id) {
            caps.push(format!("contained_in({})", c.container.raw()));
        }
        if let Some(c) = self.controller.get(&id) {
            caps.push(match c {
                ControllerCapability::Process(p) => format!("controller(process={})", p.raw()),
                ControllerCapability::Subject(s) => format!("controller(subject={})", s.raw()),
            });
        }
        if let Some(a) = self.asset_binding.get(&id) {
            caps.push(format!("asset({})", a.asset.id().as_str()));
        }
        caps.join(",")
    }

    fn relation_dump(&self, id: EntityId) -> String {
        let mut rels: Vec<String> = Vec::new();
        if let Some(parent) = self.transform_parent.get(&id) {
            rels.push(format!("transform_parent({})", parent.raw()));
        }
        if let Some(origin) = self.derived_from.get(&id) {
            rels.push(format!("derived_from({})", origin.raw()));
        }
        rels.join(",")
    }
}

// ── Snapshot records ────────────────────────────────────────────────────────--

/// One entity's full saved state: its core plus any attached capabilities.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityRecord {
    pub core: EntityCore,
    pub transform: Option<TransformCapability>,
    pub bounds: Option<BoundsCapability>,
    pub render: Option<RenderProjectionCapability>,
    pub collision: Option<CollisionCapability>,
    pub containment: Option<ContainmentCapability>,
    pub controller: Option<ControllerCapability>,
    pub asset_binding: Option<AssetBindingCapability>,
    pub transform_parent: Option<EntityId>,
    pub derived_from: Option<EntityId>,
}

/// A deterministic save snapshot of an [`EntityStore`] in ascending id order.
#[derive(Debug, Clone, PartialEq)]
pub struct EntitySnapshot {
    pub records: Vec<EntityRecord>,
}

// ── Dump helpers ──────────────────────────────────────────────────────────────

fn source_dump(source: &EntitySource) -> String {
    match source {
        EntitySource::SceneBootstrap { node } => format!("sceneBootstrap(node={})", node.raw()),
        EntitySource::RuntimeCreated { by } => match by {
            Some(p) => format!("runtimeCreated(by={})", p.raw()),
            None => "runtimeCreated".to_string(),
        },
        EntitySource::Imported { asset } => format!("imported({})", asset.id().as_str()),
        EntitySource::DiagnosticTooling => "diagnosticTooling".to_string(),
        EntitySource::PolicyProposed { by } => format!("policyProposed(by={})", by.raw()),
    }
}

fn fmt(x: f32) -> String {
    // Tame -0.0 and float noise so dumps are stable.
    let r = (x * 10_000.0).round() / 10_000.0;
    let r = if r == 0.0 { 0.0 } else { r };
    r.to_string()
}

// ── Hashing helpers ───────────────────────────────────────────────────────────

fn lifecycle_tag(l: EntityLifecycle) -> u8 {
    match l {
        EntityLifecycle::Active => 1,
        EntityLifecycle::Disabled => 2,
        EntityLifecycle::Tombstoned => 3,
    }
}

fn hash_source(h: &mut Fnv1a, source: &EntitySource) {
    match source {
        EntitySource::SceneBootstrap { node } => {
            h.write_u8(1);
            h.write_u64(node.raw());
        }
        EntitySource::RuntimeCreated { by } => {
            h.write_u8(2);
            match by {
                Some(p) => {
                    h.write_u8(1);
                    h.write_u64(p.raw());
                }
                None => h.write_u8(0),
            }
        }
        EntitySource::Imported { asset } => {
            h.write_u8(3);
            hash_asset_reference(h, asset);
        }
        EntitySource::DiagnosticTooling => h.write_u8(4),
        EntitySource::PolicyProposed { by } => {
            h.write_u8(5);
            h.write_u64(by.raw());
        }
    }
}

fn hash_asset_reference(h: &mut Fnv1a, asset: &AssetReference) {
    for b in asset.id().as_str().bytes() {
        h.write_u8(b);
    }
    match asset.version() {
        AssetVersionReq::Any => h.write_u8(0),
        AssetVersionReq::Exact(v) => {
            h.write_u8(1);
            h.write_u64(v as u64);
        }
        AssetVersionReq::AtLeast(v) => {
            h.write_u8(2);
            h.write_u64(v as u64);
        }
    }
    match asset.hash() {
        Some(hash) => {
            h.write_u8(1);
            for b in hash.as_str().bytes() {
                h.write_u8(b);
            }
        }
        None => h.write_u8(0),
    }
}

fn hash_transform(h: &mut Fnv1a, t: &EntityTransform) {
    hash_vec3(h, t.translation);
    let Quat { x, y, z, w } = t.rotation;
    for f in [x, y, z, w] {
        h.write_u64(f.to_bits() as u64);
    }
    hash_vec3(h, t.scale);
}

fn hash_vec3(h: &mut Fnv1a, v: core_math::Vec3) {
    for f in [v.x, v.y, v.z] {
        h.write_u64(f.to_bits() as u64);
    }
}

// ── FNV-1a hasher (mirrors core-scene's deterministic fingerprint) ─────────────

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
