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
//! - newly visible chunk → [`RenderDiff::Create`] parented beneath a retained
//!   voxel-instance root + [`RenderDiff::ReplaceMeshPayload`];
//! - existing chunk re-meshed → [`RenderDiff::ReplaceMeshPayload`] (stable handle);
//! - chunk emptied / absent → [`RenderDiff::Destroy`].
//!
//! A chunk keeps its handle across edits while it still has visible geometry.

use std::collections::BTreeMap;

use core_scene::transform::{SceneTransform, TransformInvalid};
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

/// Renderer-neutral binding for one projected use of a voxel asset.
///
/// Voxel cell and chunk coordinates remain asset-local. The scene transform is
/// applied only to this retained instance root, so many scene nodes can project
/// the same authoritative voxel asset without copying or rebasing its cells.
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelProjectionInstance {
    pub instance_id: String,
    pub asset_id: String,
    pub transform: SceneTransform,
}

/// Why an instance set was rejected before it could change retained projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoxelProjectionInstanceError {
    EmptyInstanceId,
    EmptyAssetId {
        instance_id: String,
    },
    DuplicateInstanceId {
        instance_id: String,
    },
    InvalidTransform {
        instance_id: String,
        reason: TransformInvalid,
    },
}

const DEFAULT_INSTANCE_ID: &str = "voxel-instance/default";
const DEFAULT_ASSET_ID: &str = "voxel-asset/default";

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
    instance_bindings_configured: bool,
    instances: BTreeMap<String, VoxelProjectionInstance>,
    root_handles: BTreeMap<String, RenderHandle>,
    handles: BTreeMap<(String, ChunkCoord), RenderHandle>,
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
            instance_bindings_configured: false,
            instances: BTreeMap::new(),
            root_handles: BTreeMap::new(),
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
        self.instance_chunk_handle(DEFAULT_INSTANCE_ID, coord)
    }

    /// The retained root handle for `instance_id`.
    pub fn instance_root_handle(&self, instance_id: &str) -> Option<RenderHandle> {
        self.root_handles.get(instance_id).copied()
    }

    /// The retained child handle for one instance/chunk pair.
    pub fn instance_chunk_handle(
        &self,
        instance_id: &str,
        coord: ChunkCoord,
    ) -> Option<RenderHandle> {
        self.handles.get(&(instance_id.to_owned(), coord)).copied()
    }

    /// Replace the complete instance binding set atomically.
    ///
    /// New instances receive a root plus every currently resident visible chunk.
    /// Transform-only changes update only the corresponding root. Asset rebinding
    /// tears down and recreates the root so no retained child can silently change
    /// asset identity. Removed roots rely on the render protocol's recursive
    /// destroy convention and release every child handle from this registry.
    pub fn set_instances(
        &mut self,
        world: &VoxelWorld,
        instances: Vec<VoxelProjectionInstance>,
    ) -> Result<RenderFrameDiff, VoxelProjectionInstanceError> {
        let next = validate_instances(instances)?;
        let mut frame = RenderFrameDiff::new();

        let removed_or_rebound: Vec<String> = self
            .instances
            .iter()
            .filter(|(id, previous)| {
                next.get(*id)
                    .is_none_or(|candidate| candidate.asset_id != previous.asset_id)
            })
            .map(|(id, _)| id.clone())
            .collect();
        for instance_id in removed_or_rebound {
            self.destroy_instance(&instance_id, &mut frame);
        }

        for (instance_id, instance) in &next {
            match self.instances.get(instance_id) {
                Some(previous) if previous.asset_id == instance.asset_id => {
                    if previous.transform != instance.transform {
                        let handle = self.root_handles[instance_id];
                        frame.push(RenderDiff::Update {
                            handle,
                            transform: Some(to_render_transform(instance.transform)),
                            material: None,
                            visible: None,
                            metadata: None,
                        });
                    }
                }
                _ => {
                    self.create_instance(instance, &mut frame);
                    let coords: Vec<_> = world.resident_chunks().map(|(coord, _)| coord).collect();
                    for coord in coords {
                        self.project_one_for_instance(world, instance_id, coord, &mut frame);
                    }
                }
            }
        }

        self.instances = next;
        self.instance_bindings_configured = true;
        Ok(frame)
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
        for (_, handle) in std::mem::take(&mut self.root_handles) {
            frame.push(RenderDiff::Destroy { handle });
        }
        self.handles.clear();
        self.instances.clear();
        self.instance_bindings_configured = false;
        frame
    }

    /// Drain the world's authoritative dirty chunk set and project each dirty chunk
    /// into render diffs. Deterministic: the dirty set drains in ascending coord
    /// order, so diffs are ordered and reproducible.
    pub fn project_dirty(&mut self, world: &mut VoxelWorld) -> RenderFrameDiff {
        let mut frame = self.ensure_default_instance();
        let dirty = world.drain_dirty();
        frame.ops.extend(self.project_coords(world, &dirty).ops);
        frame
    }

    /// Project a specific set of chunk coords (e.g. an initial full projection)
    /// without consuming the dirty set. Coords are de-duplicated and visited in
    /// ascending order.
    pub fn project_coords(&mut self, world: &VoxelWorld, coords: &[ChunkCoord]) -> RenderFrameDiff {
        self.diagnostics.clear();
        let mut ordered: Vec<ChunkCoord> = coords.to_vec();
        ordered.sort();
        ordered.dedup();

        let mut frame = self.ensure_default_instance();
        for coord in ordered {
            let instance_ids: Vec<String> = self.instances.keys().cloned().collect();
            for instance_id in instance_ids {
                self.project_one_for_instance(world, &instance_id, coord, &mut frame);
            }
        }
        frame
    }

    fn project_one_for_instance(
        &mut self,
        world: &VoxelWorld,
        instance_id: &str,
        coord: ChunkCoord,
        frame: &mut RenderFrameDiff,
    ) {
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
                let key = (instance_id.to_owned(), coord);
                if let Some(&handle) = self.handles.get(&key) {
                    frame.push(RenderDiff::ReplaceMeshPayload { handle, payload });
                } else {
                    let handle = self.allocate_chunk(instance_id, coord);
                    frame.push(RenderDiff::Create {
                        handle,
                        parent: self.root_handles.get(instance_id).copied(),
                        node: chunk_node(world, coord),
                    });
                    frame.push(RenderDiff::ReplaceMeshPayload { handle, payload });
                }
            }
            None => {
                if let Some(handle) = self.handles.remove(&(instance_id.to_owned(), coord)) {
                    frame.push(RenderDiff::Destroy { handle });
                }
            }
        }
    }

    fn allocate_handle(&mut self) -> RenderHandle {
        let handle = RenderHandle::new(self.next.max(1));
        self.next = handle.raw() + 1;
        handle
    }

    fn allocate_chunk(&mut self, instance_id: &str, coord: ChunkCoord) -> RenderHandle {
        let handle = self.allocate_handle();
        self.handles.insert((instance_id.to_owned(), coord), handle);
        handle
    }

    fn ensure_default_instance(&mut self) -> RenderFrameDiff {
        if self.instance_bindings_configured || !self.instances.is_empty() {
            return RenderFrameDiff::new();
        }
        let instance = VoxelProjectionInstance {
            instance_id: DEFAULT_INSTANCE_ID.to_owned(),
            asset_id: DEFAULT_ASSET_ID.to_owned(),
            transform: SceneTransform::IDENTITY,
        };
        let mut frame = RenderFrameDiff::new();
        self.create_instance(&instance, &mut frame);
        self.instances
            .insert(instance.instance_id.clone(), instance);
        frame
    }

    fn create_instance(&mut self, instance: &VoxelProjectionInstance, frame: &mut RenderFrameDiff) {
        let handle = self.allocate_handle();
        self.root_handles
            .insert(instance.instance_id.clone(), handle);
        frame.push(RenderDiff::Create {
            handle,
            parent: None,
            node: instance_node(instance),
        });
    }

    fn destroy_instance(&mut self, instance_id: &str, frame: &mut RenderFrameDiff) {
        if let Some(handle) = self.root_handles.remove(instance_id) {
            frame.push(RenderDiff::Destroy { handle });
        }
        self.handles.retain(|(id, _), _| id != instance_id);
        self.instances.remove(instance_id);
    }
}

/// The placeholder render node for a chunk: positioned at the chunk's asset-local
/// origin (its min corner), scene layer, labelled by coord. Its geometry is immediately
/// replaced by the chunk mesh payload; transform/material/identity then persist
/// across remeshes.
fn chunk_node(world: &VoxelWorld, coord: ChunkCoord) -> RenderNode {
    let spec = world.grid();
    let voxel = spec.chunk_origin_voxel(coord);
    let size = spec.voxel_size() as f32;
    let origin = [
        voxel.x as f32 * size,
        voxel.y as f32 * size,
        voxel.z as f32 * size,
    ];
    RenderNode {
        geometry: Geometry::Cube, // placeholder; replaced by ReplaceMeshPayload
        material: Material::DEFAULT,
        transform: Transform {
            translation: origin,
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

fn instance_node(instance: &VoxelProjectionInstance) -> RenderNode {
    RenderNode {
        geometry: Geometry::Point,
        material: Material {
            color: [0.0, 0.0, 0.0, 0.0],
            wireframe: false,
        },
        transform: to_render_transform(instance.transform),
        visible: true,
        layer: RenderLayer::Scene,
        metadata: RenderMetadata {
            source: None,
            tags: Vec::new(),
            label: Some(format!(
                "voxel instance {} asset {}",
                instance.instance_id, instance.asset_id
            )),
        },
    }
}

fn to_render_transform(transform: SceneTransform) -> Transform {
    Transform {
        translation: transform.translation.to_array(),
        rotation: [
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
            transform.rotation.w,
        ],
        scale: transform.scale.to_array(),
    }
}

fn validate_instances(
    instances: Vec<VoxelProjectionInstance>,
) -> Result<BTreeMap<String, VoxelProjectionInstance>, VoxelProjectionInstanceError> {
    let mut validated = BTreeMap::new();
    for instance in instances {
        if instance.instance_id.trim().is_empty() {
            return Err(VoxelProjectionInstanceError::EmptyInstanceId);
        }
        if instance.asset_id.trim().is_empty() {
            return Err(VoxelProjectionInstanceError::EmptyAssetId {
                instance_id: instance.instance_id,
            });
        }
        if let Err(reason) = instance.transform.validate() {
            return Err(VoxelProjectionInstanceError::InvalidTransform {
                instance_id: instance.instance_id,
                reason,
            });
        }
        let id = instance.instance_id.clone();
        if validated.insert(id.clone(), instance).is_some() {
            return Err(VoxelProjectionInstanceError::DuplicateInstanceId { instance_id: id });
        }
    }
    Ok(validated)
}
