//! Voxel chunk render projector + swappable meshing-strategy seam (#2435).
//!
//! Turns authoritative **dirty voxel chunks** into deterministic render diffs and
//! mesh-payload uploads. Like the scene projector it **reads** authority and never
//! writes it: a chunk render handle is derived projection, never save truth.
//!
//! # Meshing strategy seam
//!
//! [`ChunkMeshStrategy`] is the dispatch point so the meshing implementation can be
//! swapped without touching the projector. Today only [`VisibleFaceStrategy`]
//! (neighbour-aware visible-face meshing from `svc-mesh`) exists; seam correctness
//! comes from neighbour-aware meshing plus dirty-neighbour invalidation (the
//! authority marks neighbours dirty on a border edit), not a special seam mesh.
//!
//! # Emission per chunk (full-chunk replacement; no submesh updates)
//!
//! - newly visible chunk → [`RenderDiff::Create`] (placeholder node at the chunk's
//!   world origin) + [`RenderDiff::ReplaceMeshPayload`];
//! - existing chunk re-meshed → [`RenderDiff::ReplaceMeshPayload`] (stable handle);
//! - chunk emptied / absent → [`RenderDiff::Destroy`].
//!
//! A chunk keeps its handle across edits while it still has visible geometry.

use std::collections::BTreeMap;

use core_space::ChunkCoord;
use protocol_render::{
    Geometry, Material, MeshAttribute, MeshAttributeKind, MeshAttributeName, MeshBoundsDescriptor,
    MeshBufferLayout, MeshGroupDescriptor, MeshIndexWidth, MeshPayloadDescriptor,
    MeshPayloadSource, MeshProvenance, RenderDiff, RenderFrameDiff, RenderHandle, RenderLayer,
    RenderMetadata, RenderNode, Transform,
};
use svc_mesh::{mesh_chunk_in_world, MeshError, MeshPayload};
use svc_spatial::VoxelWorld;

/// The dispatch seam for chunk meshing. Implementors turn a resident chunk (with
/// world context for border culling) into a [`MeshPayload`]. Swapping the strategy
/// must not require changing [`VoxelChunkProjector`].
pub trait ChunkMeshStrategy {
    /// Stable label for diagnostics / provenance.
    fn label(&self) -> &'static str;
    /// Mesh the resident chunk at `coord` using `world` for neighbour culling.
    /// Returns `None` if `coord` is not resident.
    fn mesh(&self, world: &VoxelWorld, coord: ChunkCoord)
        -> Option<Result<MeshPayload, MeshError>>;
}

/// Neighbour-aware visible-face meshing (the only strategy today).
#[derive(Debug, Clone, Copy, Default)]
pub struct VisibleFaceStrategy;

impl ChunkMeshStrategy for VisibleFaceStrategy {
    fn label(&self) -> &'static str {
        "visible-face"
    }
    fn mesh(
        &self,
        world: &VoxelWorld,
        coord: ChunkCoord,
    ) -> Option<Result<MeshPayload, MeshError>> {
        mesh_chunk_in_world(world, coord)
    }
}

/// A classified voxel-projection problem (observational; the chunk is skipped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoxelProjectionDiagnostic {
    /// A chunk's mesh exceeded the u32 index range; it was skipped, not truncated.
    MeshOverflow { coord: ChunkCoord, vertices: u64 },
}

/// Convert an `svc-mesh` [`MeshPayload`] into the protocol [`MeshPayloadDescriptor`]
/// (inline source, `VoxelChunk` provenance). The renderer consumes only this.
pub fn to_payload_descriptor(mesh: &MeshPayload) -> MeshPayloadDescriptor {
    MeshPayloadDescriptor {
        layout: MeshBufferLayout {
            vertex_count: (mesh.positions.len() / 3) as u32,
            index_count: mesh.indices.len() as u32,
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
        groups: mesh
            .groups
            .iter()
            .map(|g| MeshGroupDescriptor {
                material_slot: g.material_slot,
                start: g.start,
                count: g.count,
            })
            .collect(),
        bounds: MeshBoundsDescriptor {
            min: mesh.bounds.min,
            max: mesh.bounds.max,
        },
        source: MeshPayloadSource::Inline {
            positions: mesh.positions.clone(),
            normals: mesh.normals.clone(),
            indices: mesh.indices.clone(),
        },
        provenance: MeshProvenance::VoxelChunk,
    }
}

/// Projects authoritative voxel chunks into retained render diffs, keeping a stable
/// `chunk coord → render handle` map. Generic over the meshing strategy seam.
#[derive(Debug)]
pub struct VoxelChunkProjector<S = VisibleFaceStrategy> {
    strategy: S,
    handles: BTreeMap<ChunkCoord, RenderHandle>,
    next: u64,
    diagnostics: Vec<VoxelProjectionDiagnostic>,
}

impl Default for VoxelChunkProjector<VisibleFaceStrategy> {
    fn default() -> Self {
        Self::with_strategy(VisibleFaceStrategy)
    }
}

impl VoxelChunkProjector<VisibleFaceStrategy> {
    /// A projector using the default visible-face strategy.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S: ChunkMeshStrategy> VoxelChunkProjector<S> {
    /// A projector using the given meshing strategy. Handles start at 1.
    pub fn with_strategy(strategy: S) -> Self {
        Self {
            strategy,
            handles: BTreeMap::new(),
            next: 1,
            diagnostics: Vec::new(),
        }
    }

    /// The meshing strategy's label.
    pub fn strategy_label(&self) -> &'static str {
        self.strategy.label()
    }

    /// The stable handle currently assigned to `coord`, if it has visible geometry.
    pub fn handle_of(&self, coord: ChunkCoord) -> Option<RenderHandle> {
        self.handles.get(&coord).copied()
    }

    /// Diagnostics collected during the most recent projection call.
    pub fn diagnostics(&self) -> &[VoxelProjectionDiagnostic] {
        &self.diagnostics
    }

    /// Forget every retained chunk and emit the destroys a renderer needs before
    /// authority replaces the complete voxel world. Handles are not reused: a
    /// later projection allocates fresh identities after the teardown frame.
    pub fn clear(&mut self) -> RenderFrameDiff {
        self.diagnostics.clear();
        let mut frame = RenderFrameDiff::new();
        for (_, handle) in std::mem::take(&mut self.handles) {
            frame.push(RenderDiff::Destroy { handle });
        }
        frame
    }

    /// Drain the world's authoritative dirty chunk set and project each dirty chunk
    /// into render diffs. Deterministic: the dirty set drains in ascending coord
    /// order, so diffs are ordered and reproducible.
    pub fn project_dirty(&mut self, world: &mut VoxelWorld) -> RenderFrameDiff {
        let dirty = world.drain_dirty();
        self.project_coords(world, &dirty)
    }

    /// Project a specific set of chunk coords (e.g. an initial full projection)
    /// without consuming the dirty set. Coords are de-duplicated and visited in
    /// ascending order.
    pub fn project_coords(&mut self, world: &VoxelWorld, coords: &[ChunkCoord]) -> RenderFrameDiff {
        self.diagnostics.clear();
        let mut ordered: Vec<ChunkCoord> = coords.to_vec();
        ordered.sort();
        ordered.dedup();

        let mut frame = RenderFrameDiff::new();
        for coord in ordered {
            self.project_one(world, coord, &mut frame);
        }
        frame
    }

    fn project_one(&mut self, world: &VoxelWorld, coord: ChunkCoord, frame: &mut RenderFrameDiff) {
        let mesh = match self.strategy.mesh(world, coord) {
            Some(Ok(m)) if !m.indices.is_empty() => Some(m),
            // Resident but no visible geometry (empty chunk / fully culled) → treat
            // like an absent chunk: destroy any prior handle.
            Some(Ok(_)) => None,
            Some(Err(MeshError::TooManyVertices { vertices })) => {
                // Fail closed: classify and skip, never a truncated/empty mesh.
                self.diagnostics
                    .push(VoxelProjectionDiagnostic::MeshOverflow { coord, vertices });
                None
            }
            // Not resident (unloaded / absent).
            None => None,
        };

        match mesh {
            Some(m) => {
                let payload = to_payload_descriptor(&m);
                if let Some(&handle) = self.handles.get(&coord) {
                    frame.push(RenderDiff::ReplaceMeshPayload { handle, payload });
                } else {
                    let handle = self.allocate(coord);
                    frame.push(RenderDiff::Create {
                        handle,
                        parent: None,
                        node: chunk_node(world, coord),
                    });
                    frame.push(RenderDiff::ReplaceMeshPayload { handle, payload });
                }
            }
            None => {
                if let Some(handle) = self.handles.remove(&coord) {
                    frame.push(RenderDiff::Destroy { handle });
                }
            }
        }
    }

    fn allocate(&mut self, coord: ChunkCoord) -> RenderHandle {
        let handle = RenderHandle::new(self.next.max(1));
        self.next = handle.raw() + 1;
        self.handles.insert(coord, handle);
        handle
    }
}

/// The placeholder render node for a chunk: positioned at the chunk's world origin
/// (its min corner), scene layer, labelled by coord. Its geometry is immediately
/// replaced by the chunk mesh payload; transform/material/identity then persist
/// across remeshes.
fn chunk_node(world: &VoxelWorld, coord: ChunkCoord) -> RenderNode {
    let spec = world.grid();
    let origin = spec.voxel_min_world(spec.chunk_origin_voxel(coord));
    RenderNode {
        geometry: Geometry::Cube, // placeholder; replaced by ReplaceMeshPayload
        material: Material::DEFAULT,
        transform: Transform {
            translation: [origin.x as f32, origin.y as f32, origin.z as f32],
            ..Transform::IDENTITY
        },
        visible: true,
        layer: RenderLayer::Scene,
        metadata: RenderMetadata {
            source: None,
            tags: Vec::new(),
            label: Some(format!("chunk {},{},{}", coord.x, coord.y, coord.z)),
        },
    }
}
