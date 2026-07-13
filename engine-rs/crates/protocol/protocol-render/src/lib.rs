//! Retained-mode render diff shapes for the ASHA generated-contract boundary.
//!
//! # Lane
//!
//! `contract-steward` — owns the border shape the authority core uses to drive a
//! retained-mode renderer. May depend on `core-ids` and `core-error`; it must
//! **not** depend on `core-state` or `sim-kernel`, because the border describes
//! *what changed on screen*, never *why the world changed*.
//!
//! # Border ownership
//!
//! A retained-mode renderer keeps a long-lived scene of nodes addressed by
//! stable [`RenderHandle`]s. Each tick the authority core emits a
//! [`RenderFrameDiff`]: a list of create / update / destroy operations against
//! those handles. The renderer applies the diff; it never reconstructs the
//! scene from scratch.
//!
//! These are the shapes Phase 2 codegen turns into TypeScript so a renderer
//! bridge can consume diffs in a type-safe way.
//!
//! # Abstract renderables
//!
//! Phase 5 fixes the vocabulary to *abstract* renderables only: a node is a
//! [`Geometry`] primitive (cube, sphere, quad, point, line) with a placeholder
//! [`Material`], a [`Transform`], a visibility flag, a [`RenderLayer`]
//! (scene vs. debug overlay), and [`RenderMetadata`] (source entity, tags,
//! label). [`Material`] is deliberately a placeholder (flat colour + wireframe);
//! there is no texture/shader system here, and no product-domain geometry.
//!
//! # Forbidden convenience logic
//!
//! No renderer behavior: no scene application, no interpolation, no transform
//! math, no diffing of two scenes. This crate defines the wire shape of a diff
//! and nothing that acts on one.

#![forbid(unsafe_code)]

use core_ids::{EntityId, TagId};
use protocol_assets::{CatalogEntry, MaterialProjection};
use serde::{Deserialize, Serialize};

mod material_feedback;
mod pick;
pub use material_feedback::{
    MaterialInstanceParameters, MaterialUvStrategy, RenderMaterialDescriptor,
};
pub use pick::{MeshPickHit, SpritePickHit};

// ── Handles ───────────────────────────────────────────────────────────────────

/// Stable identifier for a node in the retained render scene.
///
/// A handle is allocated when a node is created and stays valid until the node
/// is destroyed. It is distinct from an [`EntityId`]: many render nodes may
/// project a single sim entity, and some nodes (overlays, gizmos) project none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RenderHandle(pub u64);

impl RenderHandle {
    #[inline]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

// ── Transform ─────────────────────────────────────────────────────────────────

/// Minimal affine transform for a render node.
///
/// Translation, a quaternion rotation, and a non-uniform scale. Enough to place
/// a node; deliberately not a full transform hierarchy or matrix type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: [f32; 3],
    /// Rotation quaternion in `[x, y, z, w]` order.
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl Transform {
    /// The identity transform: origin, no rotation, unit scale.
    pub const IDENTITY: Transform = Transform {
        translation: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    };
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

// ── Geometry ──────────────────────────────────────────────────────────────────

/// An abstract primitive shape. Concrete extents come from the node's
/// [`Transform`] scale; primitives are unit-sized in local space.
///
/// This is intentionally a tiny, product-agnostic vocabulary — enough to draw
/// boxes, markers, and debug lines, not a mesh/asset system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Geometry {
    /// A unit cube.
    Cube,
    /// A unit sphere.
    Sphere,
    /// A flat unit quad (e.g. a ground tile or billboard backing).
    Quad,
    /// A single point marker.
    Point,
    /// A line segment between two local-space endpoints (debug overlays).
    Line { a: [f32; 3], b: [f32; 3] },
}

// ── Material ──────────────────────────────────────────────────────────────────

/// Placeholder visual appearance for a node: a flat linear-RGBA colour and an
/// optional wireframe flag. No textures, shaders, or PBR — that is out of scope
/// for the abstract border.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    /// Linear RGBA, each component in `0.0..=1.0`.
    pub color: [f32; 4],
    /// Draw as wireframe (common for debug overlays).
    pub wireframe: bool,
}

impl Material {
    /// Opaque white, filled.
    pub const DEFAULT: Material = Material {
        color: [1.0, 1.0, 1.0, 1.0],
        wireframe: false,
    };
}

impl Default for Material {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// ── Layer ─────────────────────────────────────────────────────────────────────

/// Which retained layer a node belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderLayer {
    /// The main projected scene.
    #[default]
    Scene,
    /// A debug overlay drawn on top of the scene (gizmos, labels, lines).
    Debug,
}

// ── Metadata ──────────────────────────────────────────────────────────────────

/// Descriptive metadata carried on a render node.
///
/// Links a node back to the abstract sim vocabulary (an optional source entity
/// and any descriptive tags) plus a human label for inspection/overlay text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderMetadata {
    /// The sim entity this node visualizes, if any.
    pub source: Option<EntityId>,
    /// Descriptive tags, in ascending order.
    pub tags: Vec<TagId>,
    /// Optional human-readable label (also used as overlay text).
    pub label: Option<String>,
}

// ── Node ──────────────────────────────────────────────────────────────────────

/// The full description of a node at creation time.
///
/// Geometry is fixed for a node's lifetime — changing the primitive means
/// destroy + create. Everything else (transform, material, visibility,
/// metadata) is independently mutable via [`RenderDiff::Update`].
#[derive(Debug, Clone, PartialEq)]
pub struct RenderNode {
    pub geometry: Geometry,
    pub material: Material,
    pub transform: Transform,
    pub visible: bool,
    pub layer: RenderLayer,
    pub metadata: RenderMetadata,
}

impl RenderNode {
    /// A visible scene node with the given geometry and otherwise default
    /// transform/material/metadata.
    pub fn new(geometry: Geometry) -> Self {
        Self {
            geometry,
            material: Material::DEFAULT,
            transform: Transform::IDENTITY,
            visible: true,
            layer: RenderLayer::Scene,
            metadata: RenderMetadata::default(),
        }
    }
}

// ── Diff operations ───────────────────────────────────────────────────────────

/// A single retained-mode change against the render scene.
///
/// `Update` carries optional fields so a tick can change only a transform, only
/// visibility, only material, or only metadata, without re-sending the node.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderDiff {
    /// Introduce a new node, optionally parented under an existing one.
    Create {
        handle: RenderHandle,
        parent: Option<RenderHandle>,
        node: RenderNode,
    },
    /// Mutate an existing node's mutable facets.
    Update {
        handle: RenderHandle,
        transform: Option<Transform>,
        material: Option<Material>,
        visible: Option<bool>,
        metadata: Option<RenderMetadata>,
    },
    /// Remove a node (and, by renderer convention, its descendants).
    Destroy { handle: RenderHandle },
    /// Replace a node's geometry with an uploaded voxel mesh payload (ADR 0007).
    /// Identity/material/transform stay on the node, so a chunk remesh is just
    /// another `ReplaceMeshPayload` rather than a destroy+create.
    ReplaceMeshPayload {
        handle: RenderHandle,
        payload: MeshPayloadDescriptor,
    },
    /// Define (or redefine) a catalog material descriptor under its asset id, so a
    /// static-mesh slot or sprite ref resolves to a real visual material instead of
    /// a placeholder colour (render-material-01, #2373). Idempotent: define once,
    /// reference by id from many instances.
    DefineMaterial { material: RenderMaterialDescriptor },
    /// Replace the visual feedback parameters for one material slot on one live
    /// static-mesh instance. Geometry, asset identity, and render-handle identity
    /// remain unchanged; a missing handle/slot is rejected by the renderer.
    SetMaterialInstanceParameters {
        handle: RenderHandle,
        slot: u16,
        /// Complete override, or `None` to return this slot to descriptor defaults.
        parameters: Option<MaterialInstanceParameters>,
    },
    /// Define (or redefine) a texture asset descriptor under its id (dimensions +
    /// sampling policy + content metadata; pixel bytes load via the renderer's
    /// texture provider). Idempotent (render-material-02, #2374).
    DefineTexture { texture: TextureDescriptor },
    /// Define (or redefine) a sprite atlas descriptor under its id, so a sprite's
    /// frame resolves to a deterministic UV sub-rectangle (#2374). Idempotent.
    DefineSpriteAtlas { atlas: SpriteAtlasDescriptor },
    /// Define (or redefine) a static mesh asset's shared geometry + material
    /// slots + collision policy under its asset id. Idempotent: many instances
    /// reference the asset and share one uploaded geometry (render-asset-04).
    DefineStaticMesh { asset: StaticMeshAsset },
    /// Define (or redefine) an animated mesh asset under its asset id. The
    /// descriptor carries stable clip ids and metadata only; renderer asset
    /// loading resolves the id through an explicit provider, never an arbitrary
    /// URL in the diff stream.
    DefineAnimatedMesh { asset: AnimatedMeshAsset },
    /// Create one placed instance of a previously defined static mesh asset.
    /// Instances share the asset geometry and own their transform, per-slot
    /// material overrides, and metadata.
    CreateStaticMeshInstance {
        handle: RenderHandle,
        parent: Option<RenderHandle>,
        instance: StaticMeshInstanceDescriptor,
    },
    /// Create one placed instance of a previously defined animated mesh asset.
    CreateAnimatedMeshInstance {
        handle: RenderHandle,
        parent: Option<RenderHandle>,
        instance: AnimatedMeshInstanceDescriptor,
    },
    /// Projection-only animation playback command for an animated mesh instance.
    /// Runtime/gameplay authority may choose this intent; renderer mixer progress
    /// never feeds back into authority state.
    SetAnimatedMeshPlayback {
        handle: RenderHandle,
        playback: AnimatedMeshPlaybackCommand,
    },
    /// Create one plane-geometry sprite/billboard instance (render-asset-05).
    CreateSprite {
        handle: RenderHandle,
        parent: Option<RenderHandle>,
        sprite: SpriteInstanceDescriptor,
    },
    /// Deterministic, projection-driven sprite update (frame/tint/order/
    /// visibility). Driven by an authority tick, never renderer wall-clock.
    UpdateSprite {
        handle: RenderHandle,
        frame: Option<u32>,
        tint: Option<[f32; 4]>,
        render_order: Option<i32>,
        visible: Option<bool>,
    },
}

// ── Mesh payload descriptors (voxel-capability-07 / ADR 0007) ──────────────────

/// A vertex attribute stream's element type. Only `f32` today; the enum leaves
/// room for future attribute encodings without a shape break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshAttributeKind {
    F32,
}

/// Which vertex attribute a stream carries. `Uv`/`Color` are reserved for the
/// terrain-atlas and per-vertex-colour material strategies (unused initially).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshAttributeName {
    Position,
    Normal,
    Uv,
    Color,
}

/// One declared vertex attribute stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshAttribute {
    pub name: MeshAttributeName,
    /// Components per vertex (e.g. 3 for position/normal).
    pub components: u8,
    pub kind: MeshAttributeKind,
}

/// Index buffer element width. `u32` everywhere today (u16 optimisation deferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshIndexWidth {
    U32,
}

impl MeshIndexWidth {
    /// The stable wire label for this index width — the single source of truth for
    /// JSON/artifact serialization, so emitters derive it instead of hardcoding a
    /// literal that could drift from a future variant (#2429).
    pub fn label(self) -> &'static str {
        match self {
            MeshIndexWidth::U32 => "u32",
        }
    }
}

/// The buffer layout a renderer needs to wrap bytes as typed arrays without
/// transcoding (separate attribute streams; `BufferGeometry`-compatible).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshBufferLayout {
    pub vertex_count: u32,
    pub index_count: u32,
    pub index_width: MeshIndexWidth,
    pub attributes: Vec<MeshAttribute>,
}

/// One material-slot draw group over a contiguous index range (→ `addGroup`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshGroupDescriptor {
    pub material_slot: u16,
    pub start: u32,
    pub count: u32,
}

/// Axis-aligned mesh bounds (chunk-local).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshBoundsDescriptor {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Which authoring/generation source produced a mesh payload.
///
/// A voxel chunk remesh and an authored static-mesh asset share **one**
/// [`MeshPayloadDescriptor`] and one upload path; they differ only by this
/// provenance tag, so a renderer / source-trace can attribute an uploaded mesh
/// without duplicating the upload protocol per source (render-asset-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeshProvenance {
    /// A voxel-generated chunk remesh.
    VoxelChunk,
    /// An authored static mesh asset (offline-imported, e.g. from glTF).
    StaticAsset,
    /// A procedurally generated mesh that is neither a voxel chunk nor an asset.
    #[default]
    Generated,
    /// A debug/overlay mesh (gizmos, visualizers).
    Debug,
}

impl MeshProvenance {
    /// Stable border label for diagnostics and JSON encoding.
    pub fn label(self) -> &'static str {
        match self {
            MeshProvenance::VoxelChunk => "voxelChunk",
            MeshProvenance::StaticAsset => "staticAsset",
            MeshProvenance::Generated => "generated",
            MeshProvenance::Debug => "debug",
        }
    }
}

/// Where the bulk vertex/index bytes live: `Inline` for small golden fixtures,
/// `Handle` for runtime (bridge-owned buffer referenced by handle + byte offsets,
/// per ADR 0006 — the renderer wraps the bytes as typed-array views).
#[derive(Debug, Clone, PartialEq)]
pub enum MeshPayloadSource {
    Inline {
        positions: Vec<f32>,
        normals: Vec<f32>,
        indices: Vec<u32>,
    },
    Handle {
        buffer: u64,
        positions_byte_offset: u32,
        normals_byte_offset: u32,
        indices_byte_offset: u32,
    },
}

/// The full mesh-payload border: layout, material groups, bounds, data source,
/// and provenance. Source-agnostic: voxel chunks and authored static meshes
/// share this one shape and differ only by [`MeshProvenance`].
#[derive(Debug, Clone, PartialEq)]
pub struct MeshPayloadDescriptor {
    pub layout: MeshBufferLayout,
    pub groups: Vec<MeshGroupDescriptor>,
    pub bounds: MeshBoundsDescriptor,
    pub source: MeshPayloadSource,
    /// Which source produced this mesh (provenance / source trace).
    pub provenance: MeshProvenance,
}

/// A malformed mesh payload descriptor, classified for agent routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshDescriptorError {
    /// An inline attribute stream's length disagrees with the layout.
    AttributeLengthMismatch {
        name: MeshAttributeName,
        expected: usize,
        actual: usize,
    },
    /// An inline index references a vertex outside `vertex_count`.
    IndexOutOfRange { index: u32, vertex_count: u32 },
    /// Material group ranges do not exactly tile `index_count`.
    GroupsDoNotTile { covered: u64, index_count: u32 },
    /// A group's `[start, start+count)` range falls outside the index buffer.
    GroupOutOfRange {
        start: u32,
        count: u32,
        index_count: u32,
    },
}

impl core::fmt::Display for MeshDescriptorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MeshDescriptorError::AttributeLengthMismatch {
                name,
                expected,
                actual,
            } => {
                write!(f, "attribute {name:?} length {actual}, expected {expected}")
            }
            MeshDescriptorError::IndexOutOfRange {
                index,
                vertex_count,
            } => {
                write!(f, "index {index} out of range for {vertex_count} vertices")
            }
            MeshDescriptorError::GroupsDoNotTile {
                covered,
                index_count,
            } => {
                write!(f, "groups cover {covered} indices, expected {index_count}")
            }
            MeshDescriptorError::GroupOutOfRange {
                start,
                count,
                index_count,
            } => write!(
                f,
                "group [{start}, {}) outside {index_count} indices",
                *start as u64 + *count as u64
            ),
        }
    }
}

impl std::error::Error for MeshDescriptorError {}

impl MeshPayloadDescriptor {
    /// Validate self-consistency: inline stream lengths, index ranges, and that
    /// the material groups exactly tile the index buffer.
    pub fn validate(&self) -> Result<(), MeshDescriptorError> {
        let vc = self.layout.vertex_count;
        let ic = self.layout.index_count;

        if let MeshPayloadSource::Inline {
            positions,
            normals,
            indices,
        } = &self.source
        {
            let expect_v = vc as usize * 3;
            if positions.len() != expect_v {
                return Err(MeshDescriptorError::AttributeLengthMismatch {
                    name: MeshAttributeName::Position,
                    expected: expect_v,
                    actual: positions.len(),
                });
            }
            if normals.len() != expect_v {
                return Err(MeshDescriptorError::AttributeLengthMismatch {
                    name: MeshAttributeName::Normal,
                    expected: expect_v,
                    actual: normals.len(),
                });
            }
            if indices.len() != ic as usize {
                return Err(MeshDescriptorError::GroupsDoNotTile {
                    covered: indices.len() as u64,
                    index_count: ic,
                });
            }
            for &i in indices {
                if i >= vc {
                    return Err(MeshDescriptorError::IndexOutOfRange {
                        index: i,
                        vertex_count: vc,
                    });
                }
            }
        }

        let mut covered: u64 = 0;
        for g in &self.groups {
            let end = g.start as u64 + g.count as u64;
            if end > ic as u64 {
                return Err(MeshDescriptorError::GroupOutOfRange {
                    start: g.start,
                    count: g.count,
                    index_count: ic,
                });
            }
            covered += g.count as u64;
        }
        if covered != ic as u64 {
            return Err(MeshDescriptorError::GroupsDoNotTile {
                covered,
                index_count: ic,
            });
        }
        Ok(())
    }
}

// ── Static mesh assets + instances (render-asset-04 / scene-capability-04) ─────

/// One material slot of a static mesh: the slot index that mesh groups reference,
/// bound to a catalog material asset id.
///
/// Asset ids are border **strings** (the renderer maps them to a `RenderMaterial`
/// via its registry). The render border never carries collision authority — a
/// material's solid/collidable flags stay on the collision side (boundary 18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshMaterialSlot {
    pub slot: u16,
    /// Catalog material asset id, e.g. `material/concrete-wet`.
    pub material: String,
}

/// Collision policy for a static mesh. A *visual-only* mesh skips collision; a
/// *physical* mesh must either carry an explicit collision proxy or opt into the
/// payload-AABB fallback. A physical mesh with neither is a classified error.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MeshCollisionPolicy {
    /// Visual-only: no collision proxy is produced.
    #[default]
    VisualOnly,
    /// Physical with an explicit collision proxy asset (a simplified shape).
    Proxy { proxy_asset: String },
    /// Physical, no authored proxy: derive a box collider from the payload AABB.
    AabbFallback,
}

/// An authored static mesh asset: one shared geometry payload (one initial LOD),
/// its material slots, and its collision policy. Uploaded once per asset id; many
/// [`StaticMeshInstanceDescriptor`]s reference it and share its geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticMeshAsset {
    /// Catalog asset id, e.g. `mesh/factory-belt-straight`.
    pub asset: String,
    pub payload: MeshPayloadDescriptor,
    pub material_slots: Vec<MeshMaterialSlot>,
    pub collision: MeshCollisionPolicy,
}

/// The concrete collision a static mesh resolves to once its policy is applied.
#[derive(Debug, Clone, PartialEq)]
pub enum CollisionResolution {
    /// No collider (visual-only).
    None,
    /// An explicit proxy asset is responsible for collision.
    Proxy { proxy_asset: String },
    /// A box collider derived from the payload AABB bounds (fallback).
    Aabb { min: [f32; 3], max: [f32; 3] },
}

/// A malformed static mesh asset, classified for agent routing.
#[derive(Debug, Clone, PartialEq)]
pub enum StaticMeshError {
    /// The asset id is empty.
    EmptyAssetId,
    /// Two material slots share a slot index.
    DuplicateMaterialSlot { slot: u16 },
    /// A mesh group references a material slot with no slot binding.
    GroupSlotUnbound { slot: u16 },
    /// A `Proxy` collision policy carries an empty proxy asset id.
    EmptyCollisionProxy,
    /// The underlying mesh payload is itself malformed.
    Payload(MeshDescriptorError),
}

impl StaticMeshAsset {
    /// Validate the asset: payload self-consistency, unique + bound material
    /// slots, and a well-formed collision policy. Collects the first failure.
    pub fn validate(&self) -> Result<(), StaticMeshError> {
        if self.asset.is_empty() {
            return Err(StaticMeshError::EmptyAssetId);
        }
        self.payload.validate().map_err(StaticMeshError::Payload)?;

        // Unique slot indices.
        let mut seen: Vec<u16> = Vec::with_capacity(self.material_slots.len());
        for s in &self.material_slots {
            if seen.contains(&s.slot) {
                return Err(StaticMeshError::DuplicateMaterialSlot { slot: s.slot });
            }
            seen.push(s.slot);
        }
        // Every group's material slot must be bound to a material.
        for g in &self.payload.groups {
            if !seen.contains(&g.material_slot) {
                return Err(StaticMeshError::GroupSlotUnbound {
                    slot: g.material_slot,
                });
            }
        }
        if let MeshCollisionPolicy::Proxy { proxy_asset } = &self.collision {
            if proxy_asset.is_empty() {
                return Err(StaticMeshError::EmptyCollisionProxy);
            }
        }
        Ok(())
    }

    /// Resolve the collision policy to a concrete [`CollisionResolution`]. The
    /// `AabbFallback` policy yields a box collider from the payload bounds, so a
    /// physical mesh without an authored proxy still gets *some* collider rather
    /// than silently rendering non-physical.
    pub fn resolve_collision(&self) -> CollisionResolution {
        match &self.collision {
            MeshCollisionPolicy::VisualOnly => CollisionResolution::None,
            MeshCollisionPolicy::Proxy { proxy_asset } => CollisionResolution::Proxy {
                proxy_asset: proxy_asset.clone(),
            },
            MeshCollisionPolicy::AabbFallback => CollisionResolution::Aabb {
                min: self.payload.bounds.min,
                max: self.payload.bounds.max,
            },
        }
    }
}

/// One placed instance of a static mesh asset. Instances share the asset's
/// geometry and own their transform, optional per-slot material overrides, and
/// metadata / source trace.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticMeshInstanceDescriptor {
    /// The static mesh asset id this instance references.
    pub asset: String,
    pub transform: Transform,
    /// Per-slot material rebindings for just this instance (empty = use asset's).
    pub material_overrides: Vec<MeshMaterialSlot>,
    pub metadata: RenderMetadata,
}

// ── Animated mesh assets + projection-only playback (#5288) ───────────────────

/// Runtime container format for an animated mesh asset. The first supported
/// format is GLB; FBX stays an import/source format, not a runtime render diff
/// payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimatedMeshRuntimeFormat {
    #[default]
    Glb,
}

impl AnimatedMeshRuntimeFormat {
    pub fn label(self) -> &'static str {
        match self {
            AnimatedMeshRuntimeFormat::Glb => "glb",
        }
    }
}

/// Looping policy for visual animation playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationLoopMode {
    /// Play once and clamp/stop according to renderer backend behaviour.
    Once,
    /// Repeat continuously.
    #[default]
    Repeat,
    /// Alternate forward and backward playback.
    PingPong,
}

impl AnimationLoopMode {
    pub fn label(self) -> &'static str {
        match self {
            AnimationLoopMode::Once => "once",
            AnimationLoopMode::Repeat => "repeat",
            AnimationLoopMode::PingPong => "pingPong",
        }
    }
}

/// One named animation clip available on an animated mesh asset.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationClipDescriptor {
    /// Stable ASHA clip id, e.g. `run`.
    pub id: String,
    /// Optional source/display name from the imported asset.
    pub name: Option<String>,
    /// Optional duration discovered by import validation.
    pub duration_seconds: Option<f32>,
}

/// An authored animated mesh asset descriptor. This registers identity and clip
/// vocabulary only; binary GLB data is resolved by the renderer asset provider.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatedMeshAsset {
    /// Catalog asset id, e.g. `mesh-animation/kenney-retro-character-medium`.
    pub asset: String,
    pub runtime_format: AnimatedMeshRuntimeFormat,
    /// Optional content hash for the resolved runtime artifact.
    pub content_hash: Option<String>,
    pub clips: Vec<AnimationClipDescriptor>,
    /// Optional rest/default visual clip. This does not authorize renderer
    /// autoplay proofs; explicit commands still identify the proof clip.
    pub default_clip: Option<String>,
    /// Per-slot material defaults for the imported mesh.
    pub material_slots: Vec<MeshMaterialSlot>,
    pub bounds: MeshBoundsDescriptor,
}

/// A malformed animated mesh asset descriptor, classified for agent routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimatedMeshAssetError {
    EmptyAssetId,
    EmptyClipId,
    DuplicateClipId { clip: String },
    DefaultClipMissing { clip: String },
    DuplicateMaterialSlot { slot: u16 },
}

impl AnimatedMeshAsset {
    /// Validate asset identity, unique clip ids, default-clip membership, and
    /// unique material slots.
    pub fn validate(&self) -> Result<(), AnimatedMeshAssetError> {
        if self.asset.is_empty() {
            return Err(AnimatedMeshAssetError::EmptyAssetId);
        }
        let mut clips: Vec<&str> = Vec::with_capacity(self.clips.len());
        for clip in &self.clips {
            if clip.id.is_empty() {
                return Err(AnimatedMeshAssetError::EmptyClipId);
            }
            if clips.contains(&clip.id.as_str()) {
                return Err(AnimatedMeshAssetError::DuplicateClipId {
                    clip: clip.id.clone(),
                });
            }
            clips.push(clip.id.as_str());
        }
        if let Some(default_clip) = &self.default_clip {
            if !clips.contains(&default_clip.as_str()) {
                return Err(AnimatedMeshAssetError::DefaultClipMissing {
                    clip: default_clip.clone(),
                });
            }
        }
        let mut slots: Vec<u16> = Vec::with_capacity(self.material_slots.len());
        for slot in &self.material_slots {
            if slots.contains(&slot.slot) {
                return Err(AnimatedMeshAssetError::DuplicateMaterialSlot { slot: slot.slot });
            }
            slots.push(slot.slot);
        }
        Ok(())
    }
}

/// One placed instance of an animated mesh asset.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatedMeshInstanceDescriptor {
    /// The animated mesh asset id this instance references.
    pub asset: String,
    pub transform: Transform,
    /// Per-slot material rebindings for just this instance (empty = use asset's).
    pub material_overrides: Vec<MeshMaterialSlot>,
    /// Optional initial playback intent. `None` means no clip starts implicitly.
    pub playback: Option<AnimatedMeshPlaybackCommand>,
    pub metadata: RenderMetadata,
}

/// Projection-only animation playback command. These are renderer mixer inputs,
/// never gameplay authority.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimatedMeshPlaybackCommand {
    Play {
        clip: String,
        r#loop: AnimationLoopMode,
        speed: f32,
        weight: f32,
        restart: bool,
        fade_seconds: Option<f32>,
    },
    Stop {
        fade_seconds: Option<f32>,
    },
    Pause,
    Resume,
}

// ── Sprites / billboards (render-asset-05 / scene-capability-05) ───────────────

/// How a sprite's [`SpriteInstanceDescriptor::size`] is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpriteSizeMode {
    /// Size is in world units.
    #[default]
    World,
    /// Size is in screen pixels (constant on-screen size).
    Pixel,
}

impl SpriteSizeMode {
    pub fn label(self) -> &'static str {
        match self {
            SpriteSizeMode::World => "world",
            SpriteSizeMode::Pixel => "pixel",
        }
    }
}

/// Billboarding behaviour for a sprite plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BillboardMode {
    /// No billboarding: the plane keeps its authored orientation.
    None,
    /// Always face the camera fully (spherical billboard).
    #[default]
    Spherical,
    /// Face the camera but keep the world up-axis (cylindrical billboard).
    Cylindrical,
}

impl BillboardMode {
    pub fn label(self) -> &'static str {
        match self {
            BillboardMode::None => "none",
            BillboardMode::Spherical => "spherical",
            BillboardMode::Cylindrical => "cylindrical",
        }
    }
}

/// Depth handling for a sprite. Reserves room for overlay sprites that must not
/// write/test depth without forcing that on the common case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpriteDepthPolicy {
    /// Normal depth test + write.
    #[default]
    Default,
    /// Draw without depth testing (overlay).
    DepthTestOff,
    /// Depth-test but do not write depth (soft particles / decals).
    DepthWriteOff,
}

impl SpriteDepthPolicy {
    pub fn label(self) -> &'static str {
        match self {
            SpriteDepthPolicy::Default => "default",
            SpriteDepthPolicy::DepthTestOff => "depthTestOff",
            SpriteDepthPolicy::DepthWriteOff => "depthWriteOff",
        }
    }
}

/// Reserved shading mode for a sprite material. The initial renderer implements
/// `Unlit`; the other modes are validated/reserved so the descriptor does not
/// bake in an unlit-only assumption (lighting/shadow/custom-shader headroom,
/// render-asset-06). Full shader systems are deliberately deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpriteShading {
    /// Flat, unlit textured quad (implemented today).
    #[default]
    Unlit,
    /// Receives scene lighting (reserved).
    Lit,
    /// Receives lighting and casts/receives shadows (reserved).
    Shadowed,
    /// A named custom shader strategy (reserved).
    Custom,
}

impl SpriteShading {
    pub fn label(self) -> &'static str {
        match self {
            SpriteShading::Unlit => "unlit",
            SpriteShading::Lit => "lit",
            SpriteShading::Shadowed => "shadowed",
            SpriteShading::Custom => "custom",
        }
    }

    /// Whether the initial renderer can render this mode today. Reserved modes
    /// are *accepted* by the protocol (validation never forces unlit), but a
    /// renderer may report that it falls back to unlit for now.
    pub fn is_implemented(self) -> bool {
        matches!(self, SpriteShading::Unlit)
    }
}

/// Where a sprite is attached in **authority** terms (render-asset-06).
///
/// References source scene/entity IDs and a named attachment point — never a
/// durable [`RenderHandle`], because handles are derived projection, not save
/// authority (boundary rule 12).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpriteAttachment {
    /// The sim entity this sprite is attached to, if any.
    pub source_entity: Option<EntityId>,
    /// The authored scene node id this sprite projects from, if any (raw id).
    pub source_scene_node: Option<u64>,
    /// A named attachment point on the source (e.g. `muzzle`, `hand-left`).
    pub attachment_point: Option<String>,
}

/// One placed plane-geometry sprite/billboard instance.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteInstanceDescriptor {
    /// Sprite/atlas asset id, e.g. `sprite/spark-sheet`.
    pub asset: String,
    /// Frame id within the atlas. Updated deterministically from authority ticks.
    pub frame: u32,
    /// Pivot/anchor in `0..=1` of the quad (`[0,0]` = bottom-left).
    pub pivot: [f32; 2],
    /// Quad size, interpreted per [`SpriteSizeMode`].
    pub size: [f32; 2],
    pub size_mode: SpriteSizeMode,
    pub billboard: BillboardMode,
    /// Linear-RGBA tint multiplied into the sprite texture.
    pub tint: [f32; 4],
    /// Explicit draw order among sprites (higher draws later).
    pub render_order: i32,
    pub depth: SpriteDepthPolicy,
    pub shading: SpriteShading,
    pub transform: Transform,
    pub attachment: SpriteAttachment,
    pub metadata: RenderMetadata,
}

/// A malformed sprite descriptor, classified for agent routing.
#[derive(Debug, Clone, PartialEq)]
pub enum SpriteError {
    /// The sprite asset id is empty.
    EmptyAssetId,
    /// A pivot component is outside `0..=1`.
    PivotOutOfRange { pivot: [f32; 2] },
    /// A size component is non-positive.
    NonPositiveSize { size: [f32; 2] },
}

impl SpriteInstanceDescriptor {
    /// Validate the sprite: non-empty asset, pivot in range, positive size. The
    /// shading mode is **not** validated against "unlit only" — reserved modes
    /// are accepted so the border keeps lighting/shadow headroom.
    pub fn validate(&self) -> Result<(), SpriteError> {
        if self.asset.is_empty() {
            return Err(SpriteError::EmptyAssetId);
        }
        if !(0.0..=1.0).contains(&self.pivot[0]) || !(0.0..=1.0).contains(&self.pivot[1]) {
            return Err(SpriteError::PivotOutOfRange { pivot: self.pivot });
        }
        if self.size[0] <= 0.0 || self.size[1] <= 0.0 {
            return Err(SpriteError::NonPositiveSize { size: self.size });
        }
        Ok(())
    }
}

// ── Textures + sprite atlases (material-wiring super, epic #2353; #2374) ────────

/// Texture sampling filter. The border carries the *policy*; the renderer maps it
/// to its GPU equivalent. Pixel bytes are not in the descriptor — they load
/// through a renderer-side texture provider, never ambiently from policy code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextureFilter {
    /// Nearest-neighbour (crisp pixel art).
    #[default]
    Nearest,
    /// Bilinear smoothing.
    Linear,
}

impl TextureFilter {
    pub fn label(self) -> &'static str {
        match self {
            TextureFilter::Nearest => "nearest",
            TextureFilter::Linear => "linear",
        }
    }
}

/// Texture wrap/addressing policy outside `[0,1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextureWrap {
    /// Clamp to edge.
    #[default]
    Clamp,
    /// Repeat (tile).
    Repeat,
}

impl TextureWrap {
    pub fn label(self) -> &'static str {
        match self {
            TextureWrap::Clamp => "clamp",
            TextureWrap::Repeat => "repeat",
        }
    }
}

/// A texture asset descriptor: identity, pixel dimensions, sampling policy, and
/// content metadata. Carries **no pixel bytes** — the renderer loads those through
/// an explicit texture provider (the file-loading seam), so authority/policy code
/// never touches the filesystem (boundary: file loading is a renderer concern).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureDescriptor {
    /// Catalog texture asset id, e.g. `texture/spark-sheet`.
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub filter: TextureFilter,
    pub wrap: TextureWrap,
    /// Optional content fingerprint (cache/version key); identity stays `id`.
    pub content_hash: Option<String>,
    /// Monotonic content version (bumped on a content change).
    pub version: u32,
}

/// A malformed texture descriptor, classified for agent routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureError {
    EmptyTextureId,
    ZeroDimension { width: u32, height: u32 },
}

impl TextureDescriptor {
    /// Validate identity + non-zero dimensions.
    pub fn validate(&self) -> Result<(), TextureError> {
        if self.id.is_empty() {
            return Err(TextureError::EmptyTextureId);
        }
        if self.width == 0 || self.height == 0 {
            return Err(TextureError::ZeroDimension {
                width: self.width,
                height: self.height,
            });
        }
        Ok(())
    }
}

/// One atlas frame: its sprite frame id and its **normalized** UV sub-rectangle in
/// `[0,1]` (origin bottom-left). Normalized UVs keep the rect texture-resolution
/// independent and let the renderer map a sprite frame straight onto plane UVs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteFrameRect {
    pub frame: u32,
    /// Lower-left UV corner.
    pub uv_min: [f32; 2],
    /// Upper-right UV corner.
    pub uv_max: [f32; 2],
}

/// A sprite atlas/sheet descriptor: the texture it samples and its frame rects.
/// The renderer resolves a `SpriteInstanceDescriptor::frame` to one of these rects
/// deterministically; a frame id absent here is a classified miss, not a guess.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteAtlasDescriptor {
    /// Sprite-sheet asset id, e.g. `sprite/spark-sheet`.
    pub id: String,
    /// The texture asset id this atlas samples.
    pub texture: String,
    /// Frame rects, expected in ascending `frame` order with unique ids.
    pub frames: Vec<SpriteFrameRect>,
}

/// A malformed sprite atlas descriptor, classified for agent routing.
#[derive(Debug, Clone, PartialEq)]
pub enum SpriteAtlasError {
    EmptyAtlasId,
    EmptyTextureRef,
    NoFrames,
    DuplicateFrame { frame: u32 },
    UvOutOfRange { frame: u32 },
    DegenerateRect { frame: u32 },
}

impl SpriteAtlasDescriptor {
    /// Validate identity, a bound texture, and every frame rect (in range,
    /// non-degenerate, unique frame id).
    pub fn validate(&self) -> Result<(), SpriteAtlasError> {
        if self.id.is_empty() {
            return Err(SpriteAtlasError::EmptyAtlasId);
        }
        if self.texture.is_empty() {
            return Err(SpriteAtlasError::EmptyTextureRef);
        }
        if self.frames.is_empty() {
            return Err(SpriteAtlasError::NoFrames);
        }
        let mut seen: Vec<u32> = Vec::with_capacity(self.frames.len());
        for rect in &self.frames {
            if seen.contains(&rect.frame) {
                return Err(SpriteAtlasError::DuplicateFrame { frame: rect.frame });
            }
            seen.push(rect.frame);
            for c in [
                rect.uv_min[0],
                rect.uv_min[1],
                rect.uv_max[0],
                rect.uv_max[1],
            ] {
                if !(0.0..=1.0).contains(&c) {
                    return Err(SpriteAtlasError::UvOutOfRange { frame: rect.frame });
                }
            }
            if rect.uv_max[0] <= rect.uv_min[0] || rect.uv_max[1] <= rect.uv_min[1] {
                return Err(SpriteAtlasError::DegenerateRect { frame: rect.frame });
            }
        }
        Ok(())
    }

    /// The UV rect for a sprite frame id, or `None` if the atlas has no such frame.
    pub fn frame_rect(&self, frame: u32) -> Option<&SpriteFrameRect> {
        self.frames.iter().find(|r| r.frame == frame)
    }
}

/// All retained-mode changes emitted for a single tick, in apply order.
///
/// Order is significant: a `Create` of a parent must precede a `Create` of its
/// child, and a `Destroy` is the last word on a handle within the frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderFrameDiff {
    pub ops: Vec<RenderDiff>,
}

// ── Model/material preview bridge DTOs (#2895) ────────────────────────────────

/// Request to derive/read a model/material preview using public catalog/material
/// and static-mesh DTOs. This is a protocol-owned envelope for the stable
/// `read_model_material_preview` bridge operation; transports must not replace it
/// with local facade-only wrapper types.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelMaterialPreviewRequest {
    pub catalog_entry: CatalogEntry,
    pub mesh_asset: StaticMeshAsset,
    pub instance_handle: RenderHandle,
}

/// Snapshot returned by `read_model_material_preview`: public material/model DTOs
/// plus retained-mode render-diff evidence and classified diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelMaterialPreviewSnapshot {
    pub catalog_entry: CatalogEntry,
    pub material: MaterialProjection,
    pub mesh_asset: StaticMeshAsset,
    pub preview_diff: RenderFrameDiff,
    pub renderer_classification: String,
    pub diagnostics: Vec<String>,
}

impl RenderFrameDiff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, op: RenderDiff) {
        self.ops.push(op);
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn animated_asset() -> AnimatedMeshAsset {
        AnimatedMeshAsset {
            asset: "mesh-animation/kenney-retro-character-medium".to_string(),
            runtime_format: AnimatedMeshRuntimeFormat::Glb,
            content_hash: Some("sha256-fixture-pending".to_string()),
            clips: vec![
                AnimationClipDescriptor {
                    id: "idle".to_string(),
                    name: Some("Idle".to_string()),
                    duration_seconds: Some(1.2),
                },
                AnimationClipDescriptor {
                    id: "run".to_string(),
                    name: Some("Run".to_string()),
                    duration_seconds: Some(0.8),
                },
            ],
            default_clip: Some("idle".to_string()),
            material_slots: vec![MeshMaterialSlot {
                slot: 0,
                material: "material/kenney-human-male-a".to_string(),
            }],
            bounds: MeshBoundsDescriptor {
                min: [-0.5, 0.0, -0.5],
                max: [0.5, 1.8, 0.5],
            },
        }
    }

    #[test]
    fn handle_roundtrip_and_distinct_from_entity() {
        let h = RenderHandle::new(7);
        assert_eq!(h.raw(), 7);
        let meta = RenderMetadata {
            source: Some(EntityId::new(7)),
            ..RenderMetadata::default()
        };
        assert_eq!(meta.source, Some(EntityId::new(7)));
    }

    #[test]
    fn mesh_index_width_label_is_the_contract_vocabulary() {
        // The single source of truth for the wire label; emitters derive from this
        // (#2429). A new variant forces the exhaustive match above to be updated.
        assert_eq!(MeshIndexWidth::U32.label(), "u32");
    }

    #[test]
    fn animated_mesh_asset_validates_clip_vocabulary() {
        assert!(animated_asset().validate().is_ok());

        let mut duplicate = animated_asset();
        duplicate.clips.push(AnimationClipDescriptor {
            id: "run".to_string(),
            name: Some("Run Duplicate".to_string()),
            duration_seconds: None,
        });
        assert_eq!(
            duplicate.validate(),
            Err(AnimatedMeshAssetError::DuplicateClipId {
                clip: "run".to_string(),
            })
        );

        let mut missing_default = animated_asset();
        missing_default.default_clip = Some("jump".to_string());
        assert_eq!(
            missing_default.validate(),
            Err(AnimatedMeshAssetError::DefaultClipMissing {
                clip: "jump".to_string(),
            })
        );
    }

    #[test]
    fn defaults_are_sensible() {
        assert_eq!(Transform::default(), Transform::IDENTITY);
        assert_eq!(Material::default(), Material::DEFAULT);
        assert_eq!(RenderLayer::default(), RenderLayer::Scene);

        let node = RenderNode::new(Geometry::Cube);
        assert!(node.visible);
        assert_eq!(node.layer, RenderLayer::Scene);
        assert_eq!(node.material, Material::DEFAULT);
        assert_eq!(node.geometry, Geometry::Cube);
    }

    #[test]
    fn create_update_destroy_frame_in_order() {
        let mut frame = RenderFrameDiff::new();
        assert!(frame.is_empty());

        frame.push(RenderDiff::Create {
            handle: RenderHandle::new(1),
            parent: None,
            node: RenderNode {
                metadata: RenderMetadata {
                    source: Some(EntityId::new(42)),
                    tags: vec![TagId::new(1)],
                    label: Some("root".to_string()),
                },
                ..RenderNode::new(Geometry::Cube)
            },
        });
        frame.push(RenderDiff::Create {
            handle: RenderHandle::new(2),
            parent: Some(RenderHandle::new(1)),
            node: RenderNode::new(Geometry::Sphere),
        });
        frame.push(RenderDiff::Update {
            handle: RenderHandle::new(2),
            transform: Some(Transform {
                translation: [1.0, 0.0, 0.0],
                ..Transform::IDENTITY
            }),
            material: None,
            visible: Some(false),
            metadata: None,
        });
        frame.push(RenderDiff::Destroy {
            handle: RenderHandle::new(2),
        });

        assert_eq!(frame.len(), 4);
        assert!(matches!(
            frame.ops[1],
            RenderDiff::Create {
                parent: Some(RenderHandle(1)),
                ..
            }
        ));
        assert!(matches!(
            frame.ops.last(),
            Some(RenderDiff::Destroy {
                handle: RenderHandle(2)
            })
        ));
    }

    #[test]
    fn update_can_change_only_one_facet() {
        let only_visibility = RenderDiff::Update {
            handle: RenderHandle::new(3),
            transform: None,
            material: None,
            visible: Some(false),
            metadata: None,
        };
        if let RenderDiff::Update {
            transform,
            material,
            visible,
            metadata,
            ..
        } = only_visibility
        {
            assert!(transform.is_none());
            assert!(material.is_none());
            assert!(metadata.is_none());
            assert_eq!(visible, Some(false));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn debug_overlay_line_node() {
        let node = RenderNode {
            geometry: Geometry::Line {
                a: [0.0, 0.0, 0.0],
                b: [1.0, 1.0, 0.0],
            },
            layer: RenderLayer::Debug,
            material: Material {
                color: [1.0, 0.0, 0.0, 1.0],
                wireframe: true,
            },
            ..RenderNode::new(Geometry::Point)
        };
        assert_eq!(node.layer, RenderLayer::Debug);
        assert!(matches!(node.geometry, Geometry::Line { .. }));
        assert!(node.material.wireframe);
    }
}

#[cfg(test)]
mod mesh_tests {
    use super::*;

    /// A minimal valid inline descriptor: one triangle (3 verts, 3 indices), one group.
    fn one_triangle() -> MeshPayloadDescriptor {
        MeshPayloadDescriptor {
            layout: MeshBufferLayout {
                vertex_count: 3,
                index_count: 3,
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
                material_slot: 1,
                start: 0,
                count: 3,
            }],
            bounds: MeshBoundsDescriptor {
                min: [0.0; 3],
                max: [1.0, 1.0, 0.0],
            },
            source: MeshPayloadSource::Inline {
                positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0],
                normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
                indices: vec![0, 1, 2],
            },
            provenance: MeshProvenance::VoxelChunk,
        }
    }

    #[test]
    fn valid_inline_descriptor_passes() {
        assert_eq!(one_triangle().validate(), Ok(()));
    }

    #[test]
    fn handle_source_skips_inline_length_checks_but_groups_must_tile() {
        let mut d = one_triangle();
        d.source = MeshPayloadSource::Handle {
            buffer: 7,
            positions_byte_offset: 0,
            normals_byte_offset: 36,
            indices_byte_offset: 72,
        };
        assert_eq!(d.validate(), Ok(()));
    }

    #[test]
    fn wrong_position_length_is_classified() {
        let mut d = one_triangle();
        if let MeshPayloadSource::Inline { positions, .. } = &mut d.source {
            positions.pop();
        }
        assert!(matches!(
            d.validate(),
            Err(MeshDescriptorError::AttributeLengthMismatch {
                name: MeshAttributeName::Position,
                ..
            })
        ));
    }

    #[test]
    fn index_out_of_range_is_classified() {
        let mut d = one_triangle();
        if let MeshPayloadSource::Inline { indices, .. } = &mut d.source {
            indices[2] = 9;
        }
        assert_eq!(
            d.validate(),
            Err(MeshDescriptorError::IndexOutOfRange {
                index: 9,
                vertex_count: 3
            }),
        );
    }

    #[test]
    fn groups_must_tile_the_index_buffer() {
        let mut d = one_triangle();
        d.groups = vec![MeshGroupDescriptor {
            material_slot: 1,
            start: 0,
            count: 2,
        }];
        assert!(matches!(
            d.validate(),
            Err(MeshDescriptorError::GroupsDoNotTile { .. })
        ));
    }

    #[test]
    fn group_range_beyond_indices_is_classified() {
        let mut d = one_triangle();
        d.groups = vec![MeshGroupDescriptor {
            material_slot: 1,
            start: 2,
            count: 5,
        }];
        assert!(matches!(
            d.validate(),
            Err(MeshDescriptorError::GroupOutOfRange { .. })
        ));
    }

    #[test]
    fn replace_mesh_payload_diff_constructs() {
        let diff = RenderDiff::ReplaceMeshPayload {
            handle: RenderHandle::new(3),
            payload: one_triangle(),
        };
        assert!(matches!(diff, RenderDiff::ReplaceMeshPayload { .. }));
    }

    #[test]
    fn provenance_labels_are_stable() {
        assert_eq!(MeshProvenance::VoxelChunk.label(), "voxelChunk");
        assert_eq!(MeshProvenance::StaticAsset.label(), "staticAsset");
        assert_eq!(MeshProvenance::default(), MeshProvenance::Generated);
    }
}

#[cfg(test)]
mod static_mesh_tests {
    use super::*;

    fn quad_payload(provenance: MeshProvenance) -> MeshPayloadDescriptor {
        MeshPayloadDescriptor {
            layout: MeshBufferLayout {
                vertex_count: 4,
                index_count: 6,
                index_width: MeshIndexWidth::U32,
                attributes: vec![MeshAttribute {
                    name: MeshAttributeName::Position,
                    components: 3,
                    kind: MeshAttributeKind::F32,
                }],
            },
            groups: vec![
                MeshGroupDescriptor {
                    material_slot: 1,
                    start: 0,
                    count: 3,
                },
                MeshGroupDescriptor {
                    material_slot: 2,
                    start: 3,
                    count: 3,
                },
            ],
            bounds: MeshBoundsDescriptor {
                min: [0.0, 0.0, 0.0],
                max: [2.0, 1.0, 0.0],
            },
            source: MeshPayloadSource::Handle {
                buffer: 1,
                positions_byte_offset: 0,
                normals_byte_offset: 48,
                indices_byte_offset: 96,
            },
            provenance,
        }
    }

    fn asset(collision: MeshCollisionPolicy) -> StaticMeshAsset {
        StaticMeshAsset {
            asset: "mesh/factory-belt-straight".to_string(),
            payload: quad_payload(MeshProvenance::StaticAsset),
            material_slots: vec![
                MeshMaterialSlot {
                    slot: 1,
                    material: "material/belt-rubber".to_string(),
                },
                MeshMaterialSlot {
                    slot: 2,
                    material: "material/belt-frame".to_string(),
                },
            ],
            collision,
        }
    }

    #[test]
    fn valid_static_mesh_passes() {
        assert_eq!(asset(MeshCollisionPolicy::VisualOnly).validate(), Ok(()));
    }

    #[test]
    fn duplicate_material_slot_is_classified() {
        let mut a = asset(MeshCollisionPolicy::VisualOnly);
        a.material_slots[1].slot = 1;
        assert_eq!(
            a.validate(),
            Err(StaticMeshError::DuplicateMaterialSlot { slot: 1 })
        );
    }

    #[test]
    fn group_referencing_unbound_slot_is_classified() {
        let mut a = asset(MeshCollisionPolicy::VisualOnly);
        a.material_slots.pop(); // drop slot 2's binding; group 2 still references it
        assert_eq!(
            a.validate(),
            Err(StaticMeshError::GroupSlotUnbound { slot: 2 })
        );
    }

    #[test]
    fn proxy_policy_requires_a_proxy_asset() {
        let a = asset(MeshCollisionPolicy::Proxy {
            proxy_asset: String::new(),
        });
        assert_eq!(a.validate(), Err(StaticMeshError::EmptyCollisionProxy));
    }

    #[test]
    fn visual_only_resolves_to_no_collider() {
        assert_eq!(
            asset(MeshCollisionPolicy::VisualOnly).resolve_collision(),
            CollisionResolution::None
        );
    }

    #[test]
    fn physical_without_proxy_falls_back_to_aabb() {
        // The acceptance case: a physical mesh with no authored proxy still gets
        // a box collider from its payload bounds rather than silently dropping.
        assert_eq!(
            asset(MeshCollisionPolicy::AabbFallback).resolve_collision(),
            CollisionResolution::Aabb {
                min: [0.0, 0.0, 0.0],
                max: [2.0, 1.0, 0.0],
            }
        );
    }

    #[test]
    fn explicit_proxy_resolves_to_proxy() {
        let a = asset(MeshCollisionPolicy::Proxy {
            proxy_asset: "mesh/belt-collider".to_string(),
        });
        assert_eq!(
            a.resolve_collision(),
            CollisionResolution::Proxy {
                proxy_asset: "mesh/belt-collider".to_string()
            }
        );
    }

    #[test]
    fn instance_carries_overrides_and_shares_asset_id() {
        let inst = StaticMeshInstanceDescriptor {
            asset: "mesh/factory-belt-straight".to_string(),
            transform: Transform::IDENTITY,
            material_overrides: vec![MeshMaterialSlot {
                slot: 2,
                material: "material/belt-frame-rusty".to_string(),
            }],
            metadata: RenderMetadata::default(),
        };
        assert_eq!(inst.material_overrides.len(), 1);
        assert_eq!(inst.asset, "mesh/factory-belt-straight");
    }
}

#[cfg(test)]
mod sprite_tests {
    use super::*;

    fn sprite() -> SpriteInstanceDescriptor {
        SpriteInstanceDescriptor {
            asset: "sprite/spark-sheet".to_string(),
            frame: 0,
            pivot: [0.5, 0.5],
            size: [1.0, 1.0],
            size_mode: SpriteSizeMode::World,
            billboard: BillboardMode::Spherical,
            tint: [1.0, 1.0, 1.0, 1.0],
            render_order: 0,
            depth: SpriteDepthPolicy::Default,
            shading: SpriteShading::Unlit,
            transform: Transform::IDENTITY,
            attachment: SpriteAttachment::default(),
            metadata: RenderMetadata::default(),
        }
    }

    #[test]
    fn valid_sprite_passes() {
        assert_eq!(sprite().validate(), Ok(()));
    }

    #[test]
    fn pivot_out_of_range_is_classified() {
        let mut s = sprite();
        s.pivot = [1.5, 0.0];
        assert!(matches!(
            s.validate(),
            Err(SpriteError::PivotOutOfRange { .. })
        ));
    }

    #[test]
    fn non_positive_size_is_classified() {
        let mut s = sprite();
        s.size = [0.0, 1.0];
        assert!(matches!(
            s.validate(),
            Err(SpriteError::NonPositiveSize { .. })
        ));
    }

    #[test]
    fn reserved_shading_modes_are_accepted_not_forced_unlit() {
        // Lighting/shadow headroom: a Lit sprite is a valid descriptor even
        // though the initial renderer only implements Unlit.
        let mut s = sprite();
        s.shading = SpriteShading::Lit;
        assert_eq!(s.validate(), Ok(()));
        assert!(!SpriteShading::Lit.is_implemented());
        assert!(SpriteShading::Unlit.is_implemented());
    }

    #[test]
    fn attachment_references_authority_ids_not_handles() {
        let s = SpriteInstanceDescriptor {
            attachment: SpriteAttachment {
                source_entity: Some(EntityId::new(42)),
                source_scene_node: Some(7),
                attachment_point: Some("muzzle".to_string()),
            },
            ..sprite()
        };
        assert_eq!(s.attachment.source_entity, Some(EntityId::new(42)));
        assert_eq!(s.attachment.attachment_point.as_deref(), Some("muzzle"));
    }

    #[test]
    fn pick_hit_traces_to_source_identity() {
        let hit = SpritePickHit {
            handle: RenderHandle::new(9),
            source_entity: Some(EntityId::new(42)),
            source_scene_node: None,
            asset: "sprite/spark-sheet".to_string(),
            attachment_point: Some("muzzle".to_string()),
        };
        assert_eq!(hit.source_entity, Some(EntityId::new(42)));
        assert_eq!(hit.handle, RenderHandle::new(9));
    }

    #[test]
    fn deterministic_sprite_update_diff_constructs() {
        let diff = RenderDiff::UpdateSprite {
            handle: RenderHandle::new(9),
            frame: Some(3),
            tint: None,
            render_order: Some(5),
            visible: None,
        };
        assert!(matches!(
            diff,
            RenderDiff::UpdateSprite { frame: Some(3), .. }
        ));
    }

    fn atlas(frames: Vec<SpriteFrameRect>) -> SpriteAtlasDescriptor {
        SpriteAtlasDescriptor {
            id: "sprite/spark-sheet".into(),
            texture: "texture/spark".into(),
            frames,
        }
    }

    #[test]
    fn texture_descriptor_validates_id_and_dimensions() {
        let ok = TextureDescriptor {
            id: "texture/spark".into(),
            width: 64,
            height: 64,
            filter: TextureFilter::Nearest,
            wrap: TextureWrap::Clamp,
            content_hash: None,
            version: 1,
        };
        assert!(ok.validate().is_ok());

        let zero = TextureDescriptor {
            width: 0,
            ..ok.clone()
        };
        assert_eq!(
            zero.validate(),
            Err(TextureError::ZeroDimension {
                width: 0,
                height: 64
            })
        );
        let empty = TextureDescriptor {
            id: "".into(),
            ..ok
        };
        assert_eq!(empty.validate(), Err(TextureError::EmptyTextureId));
    }

    #[test]
    fn atlas_resolves_frames_and_rejects_bad_rects() {
        let a = atlas(vec![
            SpriteFrameRect {
                frame: 0,
                uv_min: [0.0, 0.0],
                uv_max: [0.5, 1.0],
            },
            SpriteFrameRect {
                frame: 3,
                uv_min: [0.5, 0.0],
                uv_max: [1.0, 1.0],
            },
        ]);
        assert!(a.validate().is_ok());
        assert_eq!(a.frame_rect(3).unwrap().uv_min, [0.5, 0.0]);
        assert!(a.frame_rect(7).is_none(), "unknown frame resolves to None");

        // Duplicate frame id.
        let dup = atlas(vec![
            SpriteFrameRect {
                frame: 0,
                uv_min: [0.0, 0.0],
                uv_max: [0.5, 1.0],
            },
            SpriteFrameRect {
                frame: 0,
                uv_min: [0.5, 0.0],
                uv_max: [1.0, 1.0],
            },
        ]);
        assert_eq!(
            dup.validate(),
            Err(SpriteAtlasError::DuplicateFrame { frame: 0 })
        );

        // Degenerate (zero-area) rect.
        let degenerate = atlas(vec![SpriteFrameRect {
            frame: 1,
            uv_min: [0.5, 0.5],
            uv_max: [0.5, 1.0],
        }]);
        assert_eq!(
            degenerate.validate(),
            Err(SpriteAtlasError::DegenerateRect { frame: 1 })
        );

        // UV outside [0,1].
        let oob = atlas(vec![SpriteFrameRect {
            frame: 2,
            uv_min: [0.0, 0.0],
            uv_max: [1.5, 1.0],
        }]);
        assert_eq!(
            oob.validate(),
            Err(SpriteAtlasError::UvOutOfRange { frame: 2 })
        );

        assert_eq!(atlas(vec![]).validate(), Err(SpriteAtlasError::NoFrames));
    }
}
