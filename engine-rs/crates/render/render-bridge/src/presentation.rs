//! Projects authoritative **scene + world + catalog** state into retained render
//! diffs (render-projection super, epic #2352; subtasks #2369/#2370/#2371).
//!
//! # What this is
//!
//! [`RenderProjector`](crate::RenderProjector) projects the abstract entity store
//! (`core-state`) into one cube per entity. This module projects the *authored
//! scene graph*: a [`FlatSceneDocument`] gives each node its kind/asset/transform;
//! [`SpatialSessionState`] gives the **authority-owned runtime transform** (post-bootstrap
//! movement) and the `scene node → entity` source trace; the [`Catalog`] resolves
//! a static mesh's material slots. Static-mesh nodes project to
//! [`RenderDiff::DefineStaticMesh`] (once per asset) + per-node
//! [`RenderDiff::CreateStaticMeshInstance`]; sprite nodes project to
//! [`RenderDiff::CreateSprite`] / [`RenderDiff::UpdateSprite`].
//!
//! # Boundary rules
//!
//! - It **reads** authority and **never** writes it: a render handle is derived,
//!   never durable save truth (boundary 12). The [`RenderRegistry`] keeps the
//!   `handle ⇄ source` read-model so picking/diagnostics can answer
//!   `render handle → scene node / entity / asset`, but nothing here is persisted.
//! - Geometry **import** (glTF, voxel meshing) is out of scope: a static-mesh
//!   asset projects a deterministic placeholder unit quad keyed by its asset id,
//!   so two instances share one uploaded geometry. Real geometry import lands with
//!   the asset pipeline; the *projection contract* (define-once / instance-many /
//!   stable handles / source trace) is the stable part proven here.
//! - Material slots carry catalog material **ids** only; the renderer maps an id
//!   to a `RenderMaterial` (material-wiring super, epic #2353). An unresolved id
//!   is reported as a [`RenderProjectionDiagnostic`], never silently dropped.
//!
//! # Determinism
//!
//! Nodes are visited in ascending [`SceneNodeId`] order, so a given
//! scene/world/catalog always yields the same diffs in the same order: defines and
//! creates first (by node id), then updates, then destroys. An unchanged
//! presentation projects an empty frame.

use std::collections::{BTreeMap, BTreeSet};

use core_assets::AssetKind;
use core_catalog::material::{Rgba, UvStrategy};
use core_catalog::Catalog;
use core_ids::{EntityId, SceneNodeId};
use core_scene::transform::SceneTransform;
use core_scene::{FlatSceneDocument, SceneNodeKind, SceneNodeRecord, SpatialSessionState};
use protocol_render::{
    BillboardMode, MaterialUvStrategy, MeshAttribute, MeshAttributeKind, MeshAttributeName,
    MeshBoundsDescriptor, MeshBufferLayout, MeshCollisionPolicy, MeshGroupDescriptor,
    MeshIndexWidth, MeshMaterialSlot, MeshPayloadDescriptor, MeshPayloadSource, MeshProvenance,
    RenderDiff, RenderFrameDiff, RenderHandle, RenderMaterialDescriptor, RenderMetadata,
    SpriteAtlasDescriptor, SpriteAttachment, SpriteDepthPolicy, SpriteInstanceDescriptor,
    SpriteShading, SpriteSizeMode, StaticMeshAsset, StaticMeshInstanceDescriptor,
    TextureDescriptor, Transform,
};

/// An authority/catalog-owned sprite atlas + its texture, registered into the
/// projector so a sprite node can resolve its frames to UV rects (#2374). The
/// projector emits the matching `DefineTexture` + `DefineSpriteAtlas` before the
/// `CreateSprite` that needs them. File loading stays a renderer concern; this is
/// metadata only.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteAtlasSource {
    pub texture: TextureDescriptor,
    pub atlas: SpriteAtlasDescriptor,
}

// ── Source trace + lifecycle registry (#2371) ──────────────────────────────────

/// Which projection a render handle came from (its diff family).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderSourceKind {
    StaticMesh,
    Sprite,
}

/// The authority identity a render handle was projected from — the read-model that
/// answers `render handle → scene node / entity / asset`. Never persisted: handles
/// are derived projection, not save truth (boundary 12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSource {
    pub scene_node: SceneNodeId,
    /// The bootstrapped runtime entity, when the node has authority provenance.
    pub entity: Option<EntityId>,
    /// The catalog asset id this projection references.
    pub asset: String,
    pub kind: RenderSourceKind,
}

/// Maps `scene node ⇄ render handle` and `handle → source`, allocating stable
/// handles. A node keeps its handle across update/reproject cycles; a destroy
/// frees it so a later recreate gets a *fresh* handle (no silent reuse of a stale
/// mapping). Handles start at 1.
#[derive(Debug, Default)]
pub struct RenderRegistry {
    by_node: BTreeMap<SceneNodeId, RenderHandle>,
    sources: BTreeMap<RenderHandle, RenderSource>,
    next: u64,
}

impl RenderRegistry {
    pub fn new() -> Self {
        Self {
            by_node: BTreeMap::new(),
            sources: BTreeMap::new(),
            next: 1,
        }
    }

    /// Allocate (or return the existing) handle for a node, recording its source.
    fn bind(&mut self, node: SceneNodeId, source: RenderSource) -> RenderHandle {
        if let Some(&h) = self.by_node.get(&node) {
            self.sources.insert(h, source);
            return h;
        }
        let h = RenderHandle::new(self.next.max(1));
        self.next = h.raw() + 1;
        self.by_node.insert(node, h);
        self.sources.insert(h, source);
        h
    }

    /// Release a node's handle (on destroy). Returns the freed handle, if any.
    fn release(&mut self, node: SceneNodeId) -> Option<RenderHandle> {
        let h = self.by_node.remove(&node)?;
        self.sources.remove(&h);
        Some(h)
    }

    /// The handle currently assigned to a scene node, if projected.
    pub fn handle_of_node(&self, node: SceneNodeId) -> Option<RenderHandle> {
        self.by_node.get(&node).copied()
    }

    /// The authority source a handle was projected from (picking/diagnostics).
    pub fn source_of(&self, handle: RenderHandle) -> Option<&RenderSource> {
        self.sources.get(&handle)
    }

    /// Live handles in ascending order.
    pub fn live_handles(&self) -> impl Iterator<Item = RenderHandle> + '_ {
        self.sources.keys().copied()
    }

    /// Number of live (node-backed) handles.
    pub fn len(&self) -> usize {
        self.by_node.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_node.is_empty()
    }

    /// Cross-check the two indices for the stale/dangling conditions a derived
    /// read-model must never hide: a node→handle entry with no source, or a
    /// source whose handle no node points at. A healthy registry yields none.
    pub fn integrity_diagnostics(&self) -> Vec<RenderProjectionDiagnostic> {
        let mut out = Vec::new();
        for (&node, &h) in &self.by_node {
            if !self.sources.contains_key(&h) {
                out.push(RenderProjectionDiagnostic::MissingSourceRef {
                    handle: h.raw(),
                    scene_node: Some(node),
                });
            }
        }
        let pointed: BTreeSet<RenderHandle> = self.by_node.values().copied().collect();
        for &h in self.sources.keys() {
            if !pointed.contains(&h) {
                out.push(RenderProjectionDiagnostic::StaleHandle { handle: h.raw() });
            }
        }
        out
    }
}

// ── Classified diagnostics (#2369/#2371) ───────────────────────────────────────

/// A classified projection problem. Observational only — projection still emits
/// the best deterministic diff it can (a placeholder material, a fallback slot) so
/// a renderer shows *something* visible rather than silently dropping a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderProjectionDiagnostic {
    /// A static-mesh node referenced an asset absent from the catalog.
    MissingMeshAsset { node: SceneNodeId, asset: String },
    /// A static-mesh asset resolved to no material binding (placeholder used).
    UnresolvedMaterial { node: SceneNodeId, asset: String },
    /// A referenced catalog material had no visual definition; a deterministic grey
    /// fallback descriptor was emitted in its place (#2373).
    MissingCosmeticMaterial { node: SceneNodeId, material: String },
    /// A sprite's frame id is absent from its registered atlas; the renderer falls
    /// back to full UVs (#2374).
    InvalidSpriteFrame {
        node: SceneNodeId,
        atlas: String,
        frame: u32,
    },
    /// A render handle exists with no backing source (read-model corruption).
    MissingSourceRef {
        handle: u64,
        scene_node: Option<SceneNodeId>,
    },
    /// A source maps to a handle no node points at (a leaked/stale handle).
    StaleHandle { handle: u64 },
    /// Runtime-authority projection only: a renderable node has no bootstrapped
    /// runtime entity (no authority source trace). The node is **skipped** — never
    /// rendered from stale authored scene truth (#2426).
    RuntimeEntityMissing { node: SceneNodeId, asset: String },
    /// Runtime-authority projection only: a renderable node's runtime entity has no
    /// authority transform. The node is **skipped** rather than rendered from
    /// authored truth (#2426). Unreachable while the spatial-world invariant holds
    /// (#2425), but classified defensively rather than silently falling back.
    RuntimeTransformMissing {
        node: SceneNodeId,
        entity: EntityId,
        asset: String,
    },
}

impl RenderProjectionDiagnostic {
    /// Stable classification code for routing/diagnostics.
    pub fn code(&self) -> &'static str {
        match self {
            RenderProjectionDiagnostic::MissingMeshAsset { .. } => "render-missing-mesh-asset",
            RenderProjectionDiagnostic::UnresolvedMaterial { .. } => "render-unresolved-material",
            RenderProjectionDiagnostic::MissingCosmeticMaterial { .. } => {
                "render-missing-cosmetic-material"
            }
            RenderProjectionDiagnostic::InvalidSpriteFrame { .. } => "render-invalid-sprite-frame",
            RenderProjectionDiagnostic::MissingSourceRef { .. } => "render-missing-source-ref",
            RenderProjectionDiagnostic::StaleHandle { .. } => "render-stale-handle",
            RenderProjectionDiagnostic::RuntimeEntityMissing { .. } => {
                "render-runtime-entity-missing"
            }
            RenderProjectionDiagnostic::RuntimeTransformMissing { .. } => {
                "render-runtime-transform-missing"
            }
        }
    }
}

// ── Projection mode (#2426) ─────────────────────────────────────────────────────

/// Which authority a renderable node's transform must come from.
///
/// The two modes split *authored scene preview* from *runtime-authority*
/// projection so a missing runtime entity/transform can never be silently hidden
/// by rendering authored truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectionMode {
    /// Authored scene preview (editor / pre-bootstrap): a renderable node with no
    /// runtime authority falls back to its **authored** scene transform. This is
    /// the historical behaviour and the default.
    #[default]
    ScenePreview,
    /// Runtime-authority projection: a renderable node MUST have a bootstrapped
    /// runtime entity (authority source trace) and an authority transform. If
    /// either is missing the node is **skipped** and a classified diagnostic is
    /// emitted — authored scene truth is never used as a runtime fallback (#2426).
    RuntimeAuthority,
}

// ── Per-node presentation facets the projector reads from authority ─────────────

/// Authority-owned sprite runtime facets. The projector reflects these into
/// deterministic [`RenderDiff::UpdateSprite`]s; it never advances a frame from
/// wall-clock — the tick that produced these values is the authority's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteRuntime {
    pub frame: u32,
    pub tint: [f32; 4],
    pub render_order: i32,
    pub visible: bool,
}

impl Default for SpriteRuntime {
    fn default() -> Self {
        Self {
            frame: 0,
            tint: [1.0, 1.0, 1.0, 1.0],
            render_order: 0,
            visible: true,
        }
    }
}

/// Optional per-node presentation overrides supplied alongside authority state.
/// Empty by default: a node projects from its scene/world/catalog facts alone.
#[derive(Debug, Clone, Default)]
pub struct NodePresentation {
    /// Per-slot material rebindings for a static-mesh instance (slot → material id).
    pub material_overrides: Vec<(u16, String)>,
    /// Sprite runtime facets, when the node is a sprite.
    pub sprite: Option<SpriteRuntime>,
}

/// The full read-only authority input a single projection frame reads.
pub struct ScenePresentation<'a> {
    pub scene: &'a FlatSceneDocument,
    pub world: &'a SpatialSessionState,
    pub catalog: &'a Catalog,
    /// Per-node overrides (material rebinds, sprite runtime). Keyed by node id.
    pub overrides: &'a BTreeMap<SceneNodeId, NodePresentation>,
}

impl<'a> ScenePresentation<'a> {
    /// A presentation with no per-node overrides (owns an empty override map).
    pub fn without_overrides(
        scene: &'a FlatSceneDocument,
        world: &'a SpatialSessionState,
        catalog: &'a Catalog,
    ) -> ScenePresentationOwned<'a> {
        ScenePresentationOwned {
            scene,
            world,
            catalog,
            overrides: BTreeMap::new(),
        }
    }
}

/// Owns an empty override map so callers without overrides need not allocate one.
pub struct ScenePresentationOwned<'a> {
    scene: &'a FlatSceneDocument,
    world: &'a SpatialSessionState,
    catalog: &'a Catalog,
    overrides: BTreeMap<SceneNodeId, NodePresentation>,
}

impl<'a> ScenePresentationOwned<'a> {
    pub fn as_input(&'a self) -> ScenePresentation<'a> {
        ScenePresentation {
            scene: self.scene,
            world: self.world,
            catalog: self.catalog,
            overrides: &self.overrides,
        }
    }
}

// ── The projected node (change-detection model) ────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Projected {
    StaticMesh {
        asset: String,
        transform: Transform,
        overrides: Vec<MeshMaterialSlot>,
        metadata: RenderMetadata,
    },
    Sprite(SpriteInstanceDescriptor),
}

impl Projected {
    fn source_kind(&self) -> RenderSourceKind {
        match self {
            Projected::StaticMesh { .. } => RenderSourceKind::StaticMesh,
            Projected::Sprite(_) => RenderSourceKind::Sprite,
        }
    }

    fn asset(&self) -> &str {
        match self {
            Projected::StaticMesh { asset, .. } => asset,
            Projected::Sprite(s) => &s.asset,
        }
    }
}

// ── The projector ──────────────────────────────────────────────────────────────

/// Stateful scene-graph projector: define-once assets, stable per-node handles,
/// retained create/update/destroy diffs, and a source-trace read-model.
#[derive(Debug, Default)]
pub struct ScenePresentationProjector {
    registry: RenderRegistry,
    defined_assets: BTreeSet<String>,
    defined_materials: BTreeSet<String>,
    defined_textures: BTreeSet<String>,
    defined_atlases: BTreeSet<String>,
    /// Authority/catalog atlas sources keyed by sprite asset id.
    atlas_sources: BTreeMap<String, SpriteAtlasSource>,
    last: BTreeMap<SceneNodeId, Projected>,
    diagnostics: Vec<RenderProjectionDiagnostic>,
    /// Whether authored transforms may stand in for missing runtime authority.
    mode: ProjectionMode,
}

impl ScenePresentationProjector {
    /// A projector in the default [`ProjectionMode::ScenePreview`] mode.
    pub fn new() -> Self {
        Self::with_mode(ProjectionMode::ScenePreview)
    }

    /// A projector in the given [`ProjectionMode`]. Use
    /// [`ProjectionMode::RuntimeAuthority`] for the runtime render path, where a
    /// renderable node without runtime authority is skipped + classified rather
    /// than rendered from authored scene truth (#2426).
    pub fn with_mode(mode: ProjectionMode) -> Self {
        Self {
            registry: RenderRegistry::new(),
            defined_assets: BTreeSet::new(),
            defined_materials: BTreeSet::new(),
            defined_textures: BTreeSet::new(),
            defined_atlases: BTreeSet::new(),
            atlas_sources: BTreeMap::new(),
            last: BTreeMap::new(),
            diagnostics: Vec::new(),
            mode,
        }
    }

    /// The projection mode this projector enforces.
    pub fn projection_mode(&self) -> ProjectionMode {
        self.mode
    }

    /// Register an authority/catalog sprite atlas source so sprite nodes whose
    /// asset id equals `source.atlas.id` resolve their frames to UV rects (#2374).
    pub fn register_atlas_source(&mut self, source: SpriteAtlasSource) {
        self.atlas_sources.insert(source.atlas.id.clone(), source);
    }

    /// The source-trace registry, for picking / diagnostics readback.
    pub fn registry(&self) -> &RenderRegistry {
        &self.registry
    }

    /// Diagnostics collected during the most recent [`project`](Self::project).
    pub fn diagnostics(&self) -> &[RenderProjectionDiagnostic] {
        &self.diagnostics
    }

    /// Project a presentation, returning the diffs that advance the renderer from
    /// the previous projection to this one. Idempotent on unchanged input.
    pub fn project(&mut self, input: &ScenePresentation<'_>) -> RenderFrameDiff {
        self.diagnostics.clear();

        // Build the current projection, node id ascending (canonical order).
        let mut current: BTreeMap<SceneNodeId, Projected> = BTreeMap::new();
        let mut sorted = input.scene.nodes.clone();
        sorted.sort_by_key(|n| n.id.raw());
        for record in &sorted {
            if let Some(projected) = self.project_node(record, input) {
                current.insert(record.id, projected);
            }
        }

        let mut frame = RenderFrameDiff::new();

        // Creates: nodes present now but not last frame (ascending id).
        for (id, node) in &current {
            if self.last.contains_key(id) {
                continue;
            }
            self.emit_create(&mut frame, *id, node, input);
        }

        // Updates: nodes present both frames whose projection changed.
        for (id, node) in &current {
            if let Some(prev) = self.last.get(id).cloned() {
                if &prev != node {
                    self.emit_update(&mut frame, *id, &prev, node, input);
                }
            }
        }

        // Destroys: nodes gone this frame (ascending id).
        let removed: Vec<SceneNodeId> = self
            .last
            .keys()
            .filter(|id| !current.contains_key(id))
            .copied()
            .collect();
        for id in removed {
            if let Some(h) = self.registry.release(id) {
                frame.push(RenderDiff::Destroy { handle: h });
            }
        }

        self.last = current;
        frame
    }

    /// Build the projected node for one renderable scene record, or `None` for a
    /// non-renderable kind (empty group, voxel volume — handled elsewhere).
    fn project_node(
        &mut self,
        record: &SceneNodeRecord,
        input: &ScenePresentation<'_>,
    ) -> Option<Projected> {
        // Non-renderable kinds carry no geometry (empty groups; voxel volumes
        // upload via the mesh-payload path) — no transform/authority is required.
        let asset = match &record.kind {
            SceneNodeKind::StaticMesh(a) => a.id().as_str().to_string(),
            SceneNodeKind::Sprite(a) => a.id().as_str().to_string(),
            SceneNodeKind::EmptyGroup | SceneNodeKind::VoxelVolume(_) => return None,
        };
        let entity = input.world.entity_for_node(record.id);
        // In runtime-authority mode this returns `None` (and classifies) when the
        // node lacks runtime authority, so it is skipped rather than rendered from
        // authored truth (#2426).
        let transform = self.resolve_transform(record, input, entity, &asset)?;
        let presentation = input.overrides.get(&record.id);

        match &record.kind {
            SceneNodeKind::StaticMesh(asset_ref) => {
                let asset = asset_ref.id().as_str().to_string();
                let overrides = presentation
                    .map(|p| {
                        p.material_overrides
                            .iter()
                            .map(|(slot, material)| MeshMaterialSlot {
                                slot: *slot,
                                material: material.clone(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Some(Projected::StaticMesh {
                    asset,
                    transform,
                    overrides,
                    metadata: node_metadata(record, entity),
                })
            }
            SceneNodeKind::Sprite(asset_ref) => {
                let runtime = presentation.and_then(|p| p.sprite).unwrap_or_default();
                Some(Projected::Sprite(SpriteInstanceDescriptor {
                    asset: asset_ref.id().as_str().to_string(),
                    frame: runtime.frame,
                    pivot: [0.5, 0.5],
                    size: [1.0, 1.0],
                    size_mode: SpriteSizeMode::World,
                    billboard: BillboardMode::Spherical,
                    tint: runtime.tint,
                    render_order: runtime.render_order,
                    depth: SpriteDepthPolicy::Default,
                    shading: SpriteShading::Unlit,
                    transform,
                    attachment: SpriteAttachment {
                        source_entity: entity,
                        source_scene_node: Some(record.id.raw()),
                        attachment_point: None,
                    },
                    metadata: node_metadata(record, entity),
                }))
            }
            // Empty groups carry no geometry; voxel volumes upload through the
            // ADR-0007 mesh-payload path, not this scene projection.
            SceneNodeKind::EmptyGroup | SceneNodeKind::VoxelVolume(_) => None,
        }
    }

    /// Resolve a renderable node's transform under the active [`ProjectionMode`].
    ///
    /// - [`ProjectionMode::ScenePreview`]: authority owns runtime transforms after
    ///   bootstrap; fall back to the scene's authored transform for a node with no
    ///   runtime entity/transform yet.
    /// - [`ProjectionMode::RuntimeAuthority`]: require a runtime entity **and** an
    ///   authority transform. If either is missing, classify the gap and return
    ///   `None` so the node is skipped (never rendered from authored truth, #2426).
    fn resolve_transform(
        &mut self,
        record: &SceneNodeRecord,
        input: &ScenePresentation<'_>,
        entity: Option<EntityId>,
        asset: &str,
    ) -> Option<Transform> {
        match self.mode {
            ProjectionMode::ScenePreview => {
                let scene_transform = entity
                    .and_then(|e| input.world.transform(e))
                    .unwrap_or(record.transform);
                Some(to_render_transform(scene_transform))
            }
            ProjectionMode::RuntimeAuthority => {
                let Some(entity) = entity else {
                    self.diagnostics
                        .push(RenderProjectionDiagnostic::RuntimeEntityMissing {
                            node: record.id,
                            asset: asset.to_string(),
                        });
                    return None;
                };
                match input.world.transform(entity) {
                    Some(t) => Some(to_render_transform(t)),
                    None => {
                        self.diagnostics.push(
                            RenderProjectionDiagnostic::RuntimeTransformMissing {
                                node: record.id,
                                entity,
                                asset: asset.to_string(),
                            },
                        );
                        None
                    }
                }
            }
        }
    }

    fn emit_create(
        &mut self,
        frame: &mut RenderFrameDiff,
        node: SceneNodeId,
        projected: &Projected,
        input: &ScenePresentation<'_>,
    ) {
        let entity = input.world.entity_for_node(node);
        let handle = self.registry.bind(
            node,
            RenderSource {
                scene_node: node,
                entity,
                asset: projected.asset().to_string(),
                kind: projected.source_kind(),
            },
        );

        match projected {
            Projected::StaticMesh {
                asset,
                transform,
                overrides,
                metadata,
            } => {
                if self.defined_assets.insert(asset.clone()) {
                    let mesh = self.build_static_mesh_asset(node, asset, input);
                    // Materials must be defined before the mesh references them.
                    for slot in &mesh.material_slots {
                        self.ensure_material_defined(frame, node, &slot.material, input);
                    }
                    frame.push(RenderDiff::DefineStaticMesh { asset: mesh });
                }
                // Per-instance override materials must be defined before the instance.
                for slot in overrides {
                    self.ensure_material_defined(frame, node, &slot.material, input);
                }
                frame.push(RenderDiff::CreateStaticMeshInstance {
                    handle,
                    parent: None,
                    instance: StaticMeshInstanceDescriptor {
                        asset: asset.clone(),
                        transform: *transform,
                        material_overrides: overrides.clone(),
                        metadata: metadata.clone(),
                    },
                });
            }
            Projected::Sprite(sprite) => {
                self.ensure_atlas_defined(frame, node, sprite.asset.clone(), sprite.frame);
                frame.push(RenderDiff::CreateSprite {
                    handle,
                    parent: None,
                    sprite: sprite.clone(),
                });
            }
        }
    }

    fn emit_update(
        &mut self,
        frame: &mut RenderFrameDiff,
        node: SceneNodeId,
        prev: &Projected,
        cur: &Projected,
        input: &ScenePresentation<'_>,
    ) {
        let handle = self
            .registry
            .handle_of_node(node)
            .expect("an updated node must hold a handle");

        match (prev, cur) {
            (
                Projected::StaticMesh {
                    transform: pt,
                    overrides: po,
                    metadata: pm,
                    ..
                },
                Projected::StaticMesh {
                    asset,
                    transform: ct,
                    overrides: co,
                    metadata: cm,
                },
            ) => {
                // A per-instance material rebind is not expressible as a generic
                // Update (which carries a flat colour, not per-slot bindings), so
                // re-materialize the instance under the *same* handle — the shared
                // asset geometry stays defined (no unnecessary redefine).
                if po != co {
                    frame.push(RenderDiff::Destroy { handle });
                    for slot in co {
                        self.ensure_material_defined(frame, node, &slot.material, input);
                    }
                    frame.push(RenderDiff::CreateStaticMeshInstance {
                        handle,
                        parent: None,
                        instance: StaticMeshInstanceDescriptor {
                            asset: asset.clone(),
                            transform: *ct,
                            material_overrides: co.clone(),
                            metadata: cm.clone(),
                        },
                    });
                } else {
                    frame.push(RenderDiff::Update {
                        handle,
                        transform: (pt != ct).then_some(*ct),
                        material: None,
                        visible: None,
                        metadata: (pm != cm).then(|| cm.clone()),
                    });
                }
            }
            (Projected::Sprite(ps), Projected::Sprite(cs)) => {
                // Transform changes ride the generic Update; the sprite runtime
                // facets ride the deterministic UpdateSprite.
                if ps.transform != cs.transform || ps.metadata != cs.metadata {
                    frame.push(RenderDiff::Update {
                        handle,
                        transform: (ps.transform != cs.transform).then_some(cs.transform),
                        material: None,
                        visible: None,
                        metadata: (ps.metadata != cs.metadata).then(|| cs.metadata.clone()),
                    });
                }
                let frame_changed = ps.frame != cs.frame;
                let tint_changed = ps.tint != cs.tint;
                let order_changed = ps.render_order != cs.render_order;
                if frame_changed {
                    // Validate the new frame against the registered atlas.
                    if let Some(source) = self.atlas_sources.get(&cs.asset).cloned() {
                        self.check_sprite_frame(node, &source.atlas, cs.frame);
                    }
                }
                if frame_changed || tint_changed || order_changed {
                    frame.push(RenderDiff::UpdateSprite {
                        handle,
                        frame: frame_changed.then_some(cs.frame),
                        tint: tint_changed.then_some(cs.tint),
                        render_order: order_changed.then_some(cs.render_order),
                        visible: None,
                    });
                }
            }
            // A kind/asset flip is a destroy + create, not an in-place update.
            _ => {
                if let Some(h) = self.registry.release(node) {
                    frame.push(RenderDiff::Destroy { handle: h });
                }
                self.emit_create(frame, node, cur, input);
            }
        }
    }

    /// Build the shared static-mesh asset for an asset id: a deterministic
    /// placeholder unit quad plus catalog-resolved material slots. Geometry import
    /// is deferred (see module docs); the define-once / shared-geometry contract is
    /// what this proves.
    fn build_static_mesh_asset(
        &mut self,
        node: SceneNodeId,
        asset: &str,
        input: &ScenePresentation<'_>,
    ) -> StaticMeshAsset {
        let material_slots = self.resolve_material_slots(node, asset, input);
        StaticMeshAsset {
            asset: asset.to_string(),
            payload: placeholder_quad_payload(),
            material_slots,
            collision: MeshCollisionPolicy::VisualOnly,
        }
    }

    /// Emit a `DefineMaterial` for `material` once, resolving the catalog
    /// `RenderMaterial` (visual projection only — no collision class crosses). A
    /// material absent from the catalog or with no visual definition resolves to a
    /// deterministic grey fallback descriptor plus a classified diagnostic (#2373).
    fn ensure_material_defined(
        &mut self,
        frame: &mut RenderFrameDiff,
        node: SceneNodeId,
        material: &str,
        input: &ScenePresentation<'_>,
    ) {
        if !self.defined_materials.insert(material.to_string()) {
            return;
        }
        let descriptor = match resolve_render_material(input.catalog, material) {
            Some(d) => d,
            None => {
                self.diagnostics
                    .push(RenderProjectionDiagnostic::MissingCosmeticMaterial {
                        node,
                        material: material.to_string(),
                    });
                fallback_material_descriptor(material)
            }
        };
        frame.push(RenderDiff::DefineMaterial {
            material: descriptor,
        });
    }

    /// Emit `DefineTexture` + `DefineSpriteAtlas` once for a sprite asset that has
    /// a registered atlas source, and flag a frame absent from the atlas (#2374).
    /// A sprite with no registered atlas renders untextured (renderer full-UV
    /// fallback) — a valid solid-tint sprite, so that is not a diagnostic.
    fn ensure_atlas_defined(
        &mut self,
        frame: &mut RenderFrameDiff,
        node: SceneNodeId,
        sprite_asset: String,
        sprite_frame: u32,
    ) {
        let Some(source) = self.atlas_sources.get(&sprite_asset).cloned() else {
            return;
        };
        if self.defined_textures.insert(source.texture.id.clone()) {
            frame.push(RenderDiff::DefineTexture {
                texture: source.texture.clone(),
            });
        }
        if self.defined_atlases.insert(source.atlas.id.clone()) {
            frame.push(RenderDiff::DefineSpriteAtlas {
                atlas: source.atlas.clone(),
            });
        }
        self.check_sprite_frame(node, &source.atlas, sprite_frame);
    }

    /// Flag a sprite frame id absent from its atlas (renderer full-UV fallback).
    fn check_sprite_frame(&mut self, node: SceneNodeId, atlas: &SpriteAtlasDescriptor, frame: u32) {
        if atlas.frame_rect(frame).is_none() {
            self.diagnostics
                .push(RenderProjectionDiagnostic::InvalidSpriteFrame {
                    node,
                    atlas: atlas.id.clone(),
                    frame,
                });
        }
    }

    /// Resolve a mesh asset's material slots from its catalog dependencies. A
    /// mesh's `material` dependencies, in declaration order, become slots 0..n.
    /// A missing catalog entry or no material dependency yields one fallback slot
    /// (slot 0) plus a classified diagnostic — never a silent drop.
    fn resolve_material_slots(
        &mut self,
        node: SceneNodeId,
        asset: &str,
        input: &ScenePresentation<'_>,
    ) -> Vec<MeshMaterialSlot> {
        let entry = input
            .catalog
            .entries
            .iter()
            .find(|e| e.id.as_str() == asset);
        let Some(entry) = entry else {
            self.diagnostics
                .push(RenderProjectionDiagnostic::MissingMeshAsset {
                    node,
                    asset: asset.to_string(),
                });
            return vec![fallback_slot()];
        };
        let slots: Vec<MeshMaterialSlot> = entry
            .dependencies
            .iter()
            .filter(|d| d.kind() == AssetKind::Material)
            .enumerate()
            .map(|(i, dep)| MeshMaterialSlot {
                slot: i as u16,
                material: dep.id().as_str().to_string(),
            })
            .collect();
        if slots.is_empty() {
            self.diagnostics
                .push(RenderProjectionDiagnostic::UnresolvedMaterial {
                    node,
                    asset: asset.to_string(),
                });
            return vec![fallback_slot()];
        }
        slots
    }
}

// ── Free helpers ────────────────────────────────────────────────────────────────

fn fallback_slot() -> MeshMaterialSlot {
    MeshMaterialSlot {
        slot: 0,
        material: "material/fallback".to_string(),
    }
}

/// Resolve a catalog material id to its **visual** render projection descriptor,
/// or `None` if the catalog has no material definition for it.
fn resolve_render_material(catalog: &Catalog, id: &str) -> Option<RenderMaterialDescriptor> {
    let entry = catalog.entries.iter().find(|e| e.id.as_str() == id)?;
    let def = entry.material.as_ref()?;
    let render = def.render_projection();
    Some(RenderMaterialDescriptor {
        id: id.to_string(),
        color: rgba_to_array(render.color),
        texture: render.texture.map(|t| t.id().as_str().to_string()),
        roughness: render.roughness,
        emissive: render.emissive,
        uv_strategy: to_uv_strategy(render.uv_strategy),
    })
}

/// The deterministic missing-cosmetic fallback: neutral debug grey, no texture.
/// Mirrors `core_catalog::material::Rgba::DEBUG_GREY` so the renderer and the
/// authority fallback agree on the placeholder appearance.
fn fallback_material_descriptor(id: &str) -> RenderMaterialDescriptor {
    RenderMaterialDescriptor {
        id: id.to_string(),
        color: rgba_to_array(Rgba::DEBUG_GREY),
        texture: None,
        roughness: 1.0,
        emissive: 0.0,
        uv_strategy: MaterialUvStrategy::Flat,
    }
}

fn rgba_to_array(c: Rgba) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

/// Project the render material descriptors for the voxel material ids a chunk uses
/// (material-wiring super, #2375). Each id resolves through the
/// [`VoxelMaterialTable`] + catalog to its **visual** [`RenderMaterial`] (collision
/// stays on the disjoint authority projection), emitted as a `DefineMaterial` keyed
/// by the catalog material id (or `voxel-material/<id>` for an unresolved fallback).
/// Returns the diffs plus the ids that fell back, so a caller can raise a
/// fallback-used diagnostic (#2376). Ids are visited ascending (deterministic).
pub fn project_voxel_materials(
    table: &core_catalog::VoxelMaterialTable,
    catalog: &Catalog,
    used: &[core_voxel::VoxelMaterialId],
) -> (Vec<RenderDiff>, Vec<core_voxel::VoxelMaterialId>) {
    let mut ids: Vec<core_voxel::VoxelMaterialId> = used.to_vec();
    ids.sort_by_key(|i| i.raw());
    ids.dedup();

    let mut diffs = Vec::new();
    let mut fallbacks = Vec::new();
    for id in ids {
        let resolution = table.render_material(catalog, id);
        let descriptor_id = match table.material_asset(id) {
            Some(asset) if !resolution.used_fallback => asset.as_str().to_string(),
            _ => format!("voxel-material/{}", id.raw()),
        };
        if resolution.used_fallback {
            fallbacks.push(id);
        }
        let m = resolution.material;
        diffs.push(RenderDiff::DefineMaterial {
            material: RenderMaterialDescriptor {
                id: descriptor_id,
                color: rgba_to_array(m.color),
                texture: m.texture.map(|t| t.id().as_str().to_string()),
                roughness: m.roughness,
                emissive: m.emissive,
                uv_strategy: to_uv_strategy(m.uv_strategy),
            },
        });
    }
    (diffs, fallbacks)
}

fn to_uv_strategy(s: UvStrategy) -> MaterialUvStrategy {
    match s {
        UvStrategy::Flat => MaterialUvStrategy::Flat,
        UvStrategy::Planar => MaterialUvStrategy::Planar,
        UvStrategy::Atlas => MaterialUvStrategy::Atlas,
    }
}

fn node_metadata(record: &SceneNodeRecord, entity: Option<EntityId>) -> RenderMetadata {
    RenderMetadata {
        source: entity,
        tags: Vec::new(),
        label: record
            .metadata
            .label
            .clone()
            .or_else(|| Some(format!("node {}", record.id.raw()))),
    }
}

fn to_render_transform(t: SceneTransform) -> Transform {
    Transform {
        translation: [t.translation.x, t.translation.y, t.translation.z],
        rotation: [t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w],
        scale: [t.scale.x, t.scale.y, t.scale.z],
    }
}

/// A deterministic unit-quad placeholder payload (4 verts, 6 indices, one group
/// over slot 0). Shared by every instance of a static-mesh asset until real
/// geometry import lands.
fn placeholder_quad_payload() -> MeshPayloadDescriptor {
    MeshPayloadDescriptor {
        layout: MeshBufferLayout {
            vertex_count: 4,
            index_count: 6,
            index_width: MeshIndexWidth::U32,
            attributes: vec![
                MeshAttribute {
                    name: MeshAttributeName::Position,
                    components: 3,
                    kind: MeshAttributeKind::F32,
                },
                MeshAttribute {
                    name: MeshAttributeName::Normal,
                    components: 3,
                    kind: MeshAttributeKind::F32,
                },
            ],
        },
        groups: vec![MeshGroupDescriptor {
            material_slot: 0,
            start: 0,
            count: 6,
        }],
        bounds: MeshBoundsDescriptor {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 0.0],
        },
        source: MeshPayloadSource::Inline {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0],
            normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            indices: vec![0, 1, 2, 0, 2, 3],
        },
        provenance: MeshProvenance::StaticAsset,
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core_assets::{AssetId, AssetReference, AssetVersionReq};
    use core_catalog::entry::CatalogEntry;
    use core_catalog::material::{
        MaterialAuthority, MaterialDef, MaterialStyle, Rgba, StructuralClass,
    };
    use core_ids::{SceneId, WorldId};
    use core_math::Vec3;
    use core_scene::bootstrap::BootstrapPlan;
    use core_scene::document::{NodeMetadata, SceneMetadata};
    use core_scene::transform::{Quat, SceneTransform};
    use core_scene::{SceneNode, SceneTree};

    fn asset_ref(id: &str) -> AssetReference {
        AssetReference::new(AssetId::parse(id).unwrap(), AssetVersionReq::Any, None)
    }

    /// A mesh node labelled `label` referencing `mesh/...`, with an initial
    /// transform translated along x by `x`.
    fn mesh_node(id: u64, asset: &str, label: &str, x: f32) -> SceneNode {
        let mut node = SceneNode::leaf(
            SceneNodeId::new(id),
            SceneNodeKind::StaticMesh(asset_ref(asset)),
        );
        node.transform = SceneTransform::new(Vec3::new(x, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
        node.metadata = NodeMetadata {
            label: Some(label.to_string()),
            tags: Vec::new(),
        };
        node
    }

    fn sprite_node(id: u64, asset: &str, label: &str) -> SceneNode {
        let mut node = SceneNode::leaf(
            SceneNodeId::new(id),
            SceneNodeKind::Sprite(asset_ref(asset)),
        );
        node.metadata = NodeMetadata {
            label: Some(label.to_string()),
            tags: Vec::new(),
        };
        node
    }

    fn scene(roots: Vec<SceneNode>) -> FlatSceneDocument {
        SceneTree {
            id: SceneId::new(1),
            schema_version: 1,
            metadata: SceneMetadata {
                name: Some("test".into()),
                authoring_format_version: 0,
            },
            dependencies: Vec::new(),
            roots,
        }
        .to_flat()
    }

    fn bootstrap(doc: &FlatSceneDocument) -> SpatialSessionState {
        BootstrapPlan::prepare(doc, WorldId::new(1))
            .expect("valid scene")
            .apply()
            .0
    }

    fn material_entry(id: &str, color: Rgba) -> CatalogEntry {
        CatalogEntry::new(AssetId::parse(id).unwrap(), 1).with_material(MaterialDef {
            authority: MaterialAuthority {
                solid: true,
                collidable: true,
                occludes: true,
                structural_class: StructuralClass::Solid,
            },
            style: MaterialStyle::flat(color),
        })
    }

    /// A catalog where `mesh/crate` depends on `material/wood` (both have a visual
    /// material definition, so the projector resolves real descriptors).
    fn catalog_with_crate() -> Catalog {
        Catalog {
            entries: vec![
                material_entry(
                    "material/wood",
                    Rgba {
                        r: 0.6,
                        g: 0.4,
                        b: 0.2,
                        a: 1.0,
                    },
                ),
                material_entry(
                    "material/wood-painted",
                    Rgba {
                        r: 0.2,
                        g: 0.5,
                        b: 0.7,
                        a: 1.0,
                    },
                ),
                CatalogEntry::new(AssetId::parse("mesh/crate").unwrap(), 1)
                    .with_dependencies(vec![asset_ref("material/wood")]),
            ],
        }
    }

    fn project_once(
        proj: &mut ScenePresentationProjector,
        doc: &FlatSceneDocument,
        world: &SpatialSessionState,
        catalog: &Catalog,
    ) -> RenderFrameDiff {
        let overrides = BTreeMap::new();
        proj.project(&ScenePresentation {
            scene: doc,
            world,
            catalog,
            overrides: &overrides,
        })
    }

    /// An empty world: no scene node has been bootstrapped into a runtime entity.
    fn empty_world() -> SpatialSessionState {
        SpatialSessionState::empty(WorldId::new(1))
    }

    fn count_creates(frame: &RenderFrameDiff) -> usize {
        frame
            .ops
            .iter()
            .filter(|o| {
                matches!(
                    o,
                    RenderDiff::CreateStaticMeshInstance { .. } | RenderDiff::CreateSprite { .. }
                )
            })
            .count()
    }

    #[test]
    fn runtime_authority_skips_and_classifies_a_node_with_no_runtime_entity() {
        let doc = scene(vec![mesh_node(10, "mesh/crate", "crate", 7.0)]);
        let world = empty_world(); // never bootstrapped → no runtime entity
        let catalog = catalog_with_crate();
        let mut proj = ScenePresentationProjector::with_mode(ProjectionMode::RuntimeAuthority);

        let frame = project_once(&mut proj, &doc, &world, &catalog);

        // The renderable node is skipped: no instance is created from authored truth.
        assert_eq!(
            count_creates(&frame),
            0,
            "must not render authored fallback"
        );
        assert!(proj
            .diagnostics()
            .contains(&RenderProjectionDiagnostic::RuntimeEntityMissing {
                node: SceneNodeId::new(10),
                asset: "mesh/crate".into(),
            }));
        assert_eq!(
            proj.diagnostics()[0].code(),
            "render-runtime-entity-missing"
        );
    }

    #[test]
    fn scene_preview_still_renders_the_authored_fallback() {
        // The same setup in scene-preview mode DOES render from the authored
        // transform — the fallback remains available where explicitly intended.
        let doc = scene(vec![mesh_node(10, "mesh/crate", "crate", 7.0)]);
        let world = empty_world();
        let catalog = catalog_with_crate();
        let mut proj = ScenePresentationProjector::with_mode(ProjectionMode::ScenePreview);

        let frame = project_once(&mut proj, &doc, &world, &catalog);

        assert_eq!(
            count_creates(&frame),
            1,
            "preview renders authored fallback"
        );
        let instance = frame.ops.iter().find_map(|o| match o {
            RenderDiff::CreateStaticMeshInstance { instance, .. } => Some(instance),
            _ => None,
        });
        assert_eq!(instance.unwrap().transform.translation, [7.0, 0.0, 0.0]);
        assert!(proj
            .diagnostics()
            .iter()
            .all(|d| !matches!(d, RenderProjectionDiagnostic::RuntimeEntityMissing { .. })));
    }

    #[test]
    fn runtime_authority_renders_a_bootstrapped_node_from_authority_transform() {
        let doc = scene(vec![mesh_node(10, "mesh/crate", "crate", 1.0)]);
        let mut world = bootstrap(&doc);
        let catalog = catalog_with_crate();
        let mut proj = ScenePresentationProjector::with_mode(ProjectionMode::RuntimeAuthority);

        // Move via authority so the rendered transform is provably the runtime one.
        let entity = world.entity_for_node(SceneNodeId::new(10)).unwrap();
        world.set_transform(
            entity,
            SceneTransform::new(Vec3::new(4.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
        );
        let frame = project_once(&mut proj, &doc, &world, &catalog);

        assert_eq!(count_creates(&frame), 1);
        let instance = frame.ops.iter().find_map(|o| match o {
            RenderDiff::CreateStaticMeshInstance { instance, .. } => Some(instance),
            _ => None,
        });
        assert_eq!(instance.unwrap().transform.translation, [4.0, 0.0, 0.0]);
        assert!(proj
            .diagnostics()
            .iter()
            .all(|d| !matches!(d, RenderProjectionDiagnostic::RuntimeEntityMissing { .. })));
    }

    #[test]
    fn runtime_authority_destroys_a_node_that_loses_its_runtime_entity() {
        let doc = scene(vec![mesh_node(10, "mesh/crate", "crate", 0.0)]);
        let bootstrapped = bootstrap(&doc);
        let catalog = catalog_with_crate();
        let mut proj = ScenePresentationProjector::with_mode(ProjectionMode::RuntimeAuthority);

        // Frame 1: a bootstrapped world renders the node.
        let frame1 = project_once(&mut proj, &doc, &bootstrapped, &catalog);
        assert_eq!(count_creates(&frame1), 1);

        // Frame 2: the runtime authority no longer has the entity → the node is
        // skipped, so its previously-created handle is deterministically destroyed
        // (not left rendering stale authored truth).
        let gone = empty_world();
        let frame2 = project_once(&mut proj, &doc, &gone, &catalog);
        assert!(
            frame2
                .ops
                .iter()
                .any(|o| matches!(o, RenderDiff::Destroy { .. })),
            "a node that loses runtime authority must be destroyed"
        );
        assert!(proj
            .diagnostics()
            .contains(&RenderProjectionDiagnostic::RuntimeEntityMissing {
                node: SceneNodeId::new(10),
                asset: "mesh/crate".into(),
            }));
    }

    #[test]
    fn static_mesh_nodes_define_once_and_instance_per_node() {
        let doc = scene(vec![
            mesh_node(10, "mesh/crate", "crate-a", 0.0),
            mesh_node(20, "mesh/crate", "crate-b", 3.0),
        ]);
        let world = bootstrap(&doc);
        let catalog = catalog_with_crate();
        let mut proj = ScenePresentationProjector::new();
        let frame = project_once(&mut proj, &doc, &world, &catalog);

        // The material is defined before the mesh that references it.
        let mat_pos = frame
            .ops
            .iter()
            .position(|o| matches!(o, RenderDiff::DefineMaterial { .. }))
            .expect("material define");
        let mesh_pos = frame
            .ops
            .iter()
            .position(|o| matches!(o, RenderDiff::DefineStaticMesh { .. }))
            .expect("mesh define");
        assert!(mat_pos < mesh_pos, "material must define before the mesh");

        // One mesh define (shared geometry) + two instances, in node-id order.
        let defines = frame
            .ops
            .iter()
            .filter(|o| matches!(o, RenderDiff::DefineStaticMesh { .. }))
            .count();
        assert_eq!(defines, 1, "shared asset defines exactly once");
        let instances: Vec<_> = frame
            .ops
            .iter()
            .filter_map(|o| match o {
                RenderDiff::CreateStaticMeshInstance {
                    handle, instance, ..
                } => Some((handle.raw(), instance.metadata.label.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(
            instances,
            vec![(1, Some("crate-a".into())), (2, Some("crate-b".into()))]
        );

        // The define carries the catalog-resolved material slot, and the material
        // descriptor carries the real catalog colour (not a placeholder).
        if let RenderDiff::DefineStaticMesh { asset } = &frame.ops[mesh_pos] {
            assert_eq!(
                asset.material_slots,
                vec![MeshMaterialSlot {
                    slot: 0,
                    material: "material/wood".into()
                }]
            );
            assert!(asset.validate().is_ok());
        }
        if let RenderDiff::DefineMaterial { material } = &frame.ops[mat_pos] {
            assert_eq!(material.id, "material/wood");
            assert_eq!(material.color, [0.6, 0.4, 0.2, 1.0]);
        }
        assert!(proj.diagnostics().is_empty());
    }

    #[test]
    fn unchanged_presentation_projects_empty_frame() {
        let doc = scene(vec![mesh_node(10, "mesh/crate", "crate", 0.0)]);
        let world = bootstrap(&doc);
        let catalog = catalog_with_crate();
        let mut proj = ScenePresentationProjector::new();
        let _ = project_once(&mut proj, &doc, &world, &catalog);
        let second = project_once(&mut proj, &doc, &world, &catalog);
        assert!(second.is_empty());
    }

    #[test]
    fn authority_transform_move_projects_a_transform_update_only() {
        let doc = scene(vec![mesh_node(10, "mesh/crate", "crate", 0.0)]);
        let mut world = bootstrap(&doc);
        let catalog = catalog_with_crate();
        let mut proj = ScenePresentationProjector::new();
        let _ = project_once(&mut proj, &doc, &world, &catalog);

        // Authority moves the entity (runtime transform, not the scene doc).
        let entity = world.entity_for_node(SceneNodeId::new(10)).unwrap();
        world.set_transform(
            entity,
            SceneTransform::new(Vec3::new(5.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE),
        );
        let frame = project_once(&mut proj, &doc, &world, &catalog);

        assert_eq!(frame.len(), 1);
        match &frame.ops[0] {
            RenderDiff::Update {
                transform,
                metadata,
                ..
            } => {
                assert_eq!(transform.unwrap().translation, [5.0, 0.0, 0.0]);
                assert!(metadata.is_none());
            }
            other => panic!("expected transform update, got {other:?}"),
        }
    }

    #[test]
    fn material_override_change_recreates_instance_without_redefining_geometry() {
        let doc = scene(vec![mesh_node(10, "mesh/crate", "crate", 0.0)]);
        let world = bootstrap(&doc);
        let catalog = catalog_with_crate();
        let mut proj = ScenePresentationProjector::new();
        let _ = project_once(&mut proj, &doc, &world, &catalog);

        let mut overrides = BTreeMap::new();
        overrides.insert(
            SceneNodeId::new(10),
            NodePresentation {
                material_overrides: vec![(0, "material/wood-painted".into())],
                sprite: None,
            },
        );
        let frame = proj.project(&ScenePresentation {
            scene: &doc,
            world: &world,
            catalog: &catalog,
            overrides: &overrides,
        });

        // Destroy + define the new override material + recreate under the same
        // handle; the shared mesh geometry is never redefined.
        assert!(matches!(frame.ops[0], RenderDiff::Destroy { handle } if handle.raw() == 1));
        assert!(frame.ops.iter().any(|o| matches!(
            o,
            RenderDiff::DefineMaterial { material } if material.id == "material/wood-painted"
        )));
        assert!(frame.ops.iter().any(|o| matches!(
            o,
            RenderDiff::CreateStaticMeshInstance { handle, instance, .. }
                if handle.raw() == 1
                    && instance.material_overrides[0].material == "material/wood-painted"
        )));
        assert!(!frame
            .ops
            .iter()
            .any(|o| matches!(o, RenderDiff::DefineStaticMesh { .. })));
    }

    #[test]
    fn sprite_node_projects_create_then_deterministic_frame_update() {
        let doc = scene(vec![sprite_node(10, "sprite/spark-sheet", "spark")]);
        let world = bootstrap(&doc);
        let catalog = Catalog::default();
        let mut proj = ScenePresentationProjector::new();
        let first = project_once(&mut proj, &doc, &world, &catalog);
        assert!(matches!(first.ops[0], RenderDiff::CreateSprite { .. }));

        // Authority advances the sprite frame deterministically (tick-owned).
        let mut overrides = BTreeMap::new();
        overrides.insert(
            SceneNodeId::new(10),
            NodePresentation {
                material_overrides: Vec::new(),
                sprite: Some(SpriteRuntime {
                    frame: 3,
                    ..SpriteRuntime::default()
                }),
            },
        );
        let frame = proj.project(&ScenePresentation {
            scene: &doc,
            world: &world,
            catalog: &catalog,
            overrides: &overrides,
        });
        assert_eq!(frame.len(), 1);
        assert!(matches!(
            frame.ops[0],
            RenderDiff::UpdateSprite { handle, frame: Some(3), .. } if handle.raw() == 1
        ));
    }

    #[test]
    fn removing_a_node_destroys_its_handle_and_frees_it() {
        let doc1 = scene(vec![
            mesh_node(10, "mesh/crate", "a", 0.0),
            mesh_node(20, "mesh/crate", "b", 3.0),
        ]);
        let world = bootstrap(&doc1);
        let catalog = catalog_with_crate();
        let mut proj = ScenePresentationProjector::new();
        let _ = project_once(&mut proj, &doc1, &world, &catalog);
        let h2 = proj
            .registry()
            .handle_of_node(SceneNodeId::new(20))
            .unwrap();

        let doc2 = scene(vec![mesh_node(10, "mesh/crate", "a", 0.0)]);
        let world2 = bootstrap(&doc2);
        let frame = project_once(&mut proj, &doc2, &world2, &catalog);

        assert_eq!(frame.len(), 1);
        assert!(matches!(frame.ops[0], RenderDiff::Destroy { handle } if handle == h2));
        assert_eq!(proj.registry().handle_of_node(SceneNodeId::new(20)), None);
        assert!(proj.registry().integrity_diagnostics().is_empty());
    }

    #[test]
    fn source_trace_answers_handle_to_scene_node_entity_and_asset() {
        let doc = scene(vec![mesh_node(42, "mesh/crate", "crate", 0.0)]);
        let world = bootstrap(&doc);
        let catalog = catalog_with_crate();
        let mut proj = ScenePresentationProjector::new();
        let _ = project_once(&mut proj, &doc, &world, &catalog);

        let handle = proj
            .registry()
            .handle_of_node(SceneNodeId::new(42))
            .unwrap();
        let source = proj.registry().source_of(handle).unwrap();
        assert_eq!(source.scene_node, SceneNodeId::new(42));
        assert_eq!(source.entity, world.entity_for_node(SceneNodeId::new(42)));
        assert_eq!(source.asset, "mesh/crate");
        assert_eq!(source.kind, RenderSourceKind::StaticMesh);
    }

    #[test]
    fn missing_material_dependency_uses_fallback_slot_and_reports() {
        let doc = scene(vec![mesh_node(10, "mesh/lonely", "lonely", 0.0)]);
        let world = bootstrap(&doc);
        // Catalog has the mesh entry but no material dependency.
        let catalog = Catalog {
            entries: vec![CatalogEntry::new(AssetId::parse("mesh/lonely").unwrap(), 1)],
        };
        let mut proj = ScenePresentationProjector::new();
        let frame = project_once(&mut proj, &doc, &world, &catalog);

        let mesh = frame
            .ops
            .iter()
            .find_map(|o| match o {
                RenderDiff::DefineStaticMesh { asset } => Some(asset),
                _ => None,
            })
            .expect("mesh define");
        assert_eq!(mesh.material_slots, vec![fallback_slot()]);
        assert!(proj
            .diagnostics()
            .contains(&RenderProjectionDiagnostic::UnresolvedMaterial {
                node: SceneNodeId::new(10),
                asset: "mesh/lonely".into()
            }));
    }

    #[test]
    fn missing_mesh_asset_in_catalog_is_classified() {
        let doc = scene(vec![mesh_node(10, "mesh/ghost", "ghost", 0.0)]);
        let world = bootstrap(&doc);
        let catalog = Catalog::default();
        let mut proj = ScenePresentationProjector::new();
        let _ = project_once(&mut proj, &doc, &world, &catalog);
        assert!(proj
            .diagnostics()
            .contains(&RenderProjectionDiagnostic::MissingMeshAsset {
                node: SceneNodeId::new(10),
                asset: "mesh/ghost".into()
            }));
        assert_eq!(proj.diagnostics()[0].code(), "render-missing-mesh-asset");
    }

    #[test]
    fn projection_is_deterministic_across_repeated_runs() {
        let doc = scene(vec![
            mesh_node(10, "mesh/crate", "a", 0.0),
            sprite_node(20, "sprite/spark-sheet", "b"),
        ]);
        let world = bootstrap(&doc);
        let catalog = catalog_with_crate();

        let mut a = ScenePresentationProjector::new();
        let mut b = ScenePresentationProjector::new();
        let fa = project_once(&mut a, &doc, &world, &catalog);
        let fb = project_once(&mut b, &doc, &world, &catalog);
        assert_eq!(fa, fb, "same input must project identical diffs");
    }
}
