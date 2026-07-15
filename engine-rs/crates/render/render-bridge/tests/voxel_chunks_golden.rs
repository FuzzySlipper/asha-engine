//! Voxel chunk projector golden + reproject tests (#2435).
//!
//! Projects the canonical abstract voxel world (the same shape `fixture-maker`
//! commits in #2434: a 2×2×1 arrangement, solid bottom layers, materials by chunk)
//! into render diffs through the Rust [`VoxelChunkProjector`], pins the multi-chunk
//! seam/material-slot frame to a committed golden, and proves a single dirty-chunk
//! edit reprojects only the expected chunk + resident neighbours.
//!
//! Regenerate the golden with:
//!   BLESS=1 cargo test -p render-bridge --test voxel_chunks_golden

use std::path::PathBuf;

use core_math::Vec3;
use core_scene::transform::{Quat, SceneTransform};
use core_space::{ChunkCoord, ChunkDims, GridId, LocalVoxelCoord, VoxelGridSpec};
use core_voxel::VoxelValue;
use protocol_render::RenderDiff;
use render_bridge::json;
use render_bridge::voxel::{VoxelChunkProjector, VoxelProjectionInstance};
use svc_spatial::VoxelWorld;
use svc_volume::VoxelChunk;

/// The canonical grid (matches `fixture_maker::canonical_grid`).
fn grid() -> VoxelGridSpec {
    VoxelGridSpec::new(GridId::new(1), 1.0, ChunkDims::cubic(2).unwrap()).unwrap()
}

const ARRANGEMENT: [(i64, i64, i64); 4] = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (1, 1, 0)];

fn material_for(coord: ChunkCoord) -> u16 {
    [1u16, 2, 3][(coord.x * 2 + coord.y).rem_euclid(3) as usize]
}

/// Build the canonical voxel world (bottom layer of each chunk solid).
fn canonical_world() -> VoxelWorld {
    let spec = grid();
    let dims = spec.chunk_dims();
    let mut world = VoxelWorld::new(spec);
    for (x, y, z) in ARRANGEMENT {
        let coord = ChunkCoord::new(x, y, z);
        let mut chunk = VoxelChunk::from_spec(&spec);
        chunk
            .fill_region(
                LocalVoxelCoord::new(0, 0, 0),
                LocalVoxelCoord::new(dims.x(), dims.y(), 1),
                VoxelValue::solid_raw(material_for(coord)),
            )
            .unwrap();
        world.insert(coord, chunk);
    }
    world
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../harness/fixtures/render-diffs/voxel-chunks.json")
}

#[test]
fn projects_canonical_world_to_committed_golden() {
    let mut world = canonical_world();
    let mut projector = VoxelChunkProjector::new();
    // Drain the insert-dirty set: a full projection of all four chunks.
    let frame = projector.project_dirty(&mut world);
    assert!(projector.diagnostics().is_empty());

    let actual = json::encode_frame(&frame);
    let path = golden_path();
    if std::env::var_os("BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let golden = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {} ({e}); run with BLESS=1 to create", path.display()));
    assert_eq!(
        actual,
        golden,
        "voxel chunk projection drifted from {} — regenerate with BLESS=1 if intended",
        path.display()
    );
}

#[test]
fn full_projection_creates_a_mesh_per_chunk() {
    let mut world = canonical_world();
    let mut projector = VoxelChunkProjector::new();
    let frame = projector.project_dirty(&mut world);

    let creates = frame
        .ops
        .iter()
        .filter(|o| matches!(o, RenderDiff::Create { .. }))
        .count();
    let payloads = frame
        .ops
        .iter()
        .filter(|o| matches!(o, RenderDiff::ReplaceMeshPayload { .. }))
        .count();
    assert_eq!(creates, 5, "one retained root plus one create per chunk");
    assert_eq!(payloads, 4, "one mesh payload per chunk");
    // Each chunk has a stable handle.
    for (x, y, z) in ARRANGEMENT {
        assert!(projector.handle_of(ChunkCoord::new(x, y, z)).is_some());
    }
}

fn instance(id: &str, translation: [f32; 3]) -> VoxelProjectionInstance {
    VoxelProjectionInstance {
        instance_id: id.to_owned(),
        asset_id: "voxel/house".to_owned(),
        transform: SceneTransform::new(
            Vec3::new(translation[0], translation[1], translation[2]),
            Quat::IDENTITY,
            Vec3::ONE,
        ),
    }
}

#[test]
fn two_instances_share_asset_meshes_but_keep_independent_roots() {
    let mut world = canonical_world();
    world.drain_dirty();
    let mut projector = VoxelChunkProjector::new();

    let initial = projector
        .set_instances(
            &world,
            vec![
                instance("scene-node/10", [3.0, 0.0, 0.0]),
                instance("scene-node/20", [-4.0, 2.0, 1.0]),
            ],
        )
        .unwrap();
    let root_a = projector.instance_root_handle("scene-node/10").unwrap();
    let root_b = projector.instance_root_handle("scene-node/20").unwrap();
    assert_ne!(root_a, root_b);
    assert_eq!(
        initial
            .ops
            .iter()
            .filter(|op| matches!(op, RenderDiff::Create { parent: None, .. }))
            .count(),
        2,
    );
    assert_eq!(
        initial
            .ops
            .iter()
            .filter(|op| matches!(op, RenderDiff::ReplaceMeshPayload { .. }))
            .count(),
        ARRANGEMENT.len() * 2,
    );
    for (x, y, z) in ARRANGEMENT {
        let coord = ChunkCoord::new(x, y, z);
        assert_ne!(
            projector.instance_chunk_handle("scene-node/10", coord),
            projector.instance_chunk_handle("scene-node/20", coord),
        );
    }

    let moved = projector
        .set_instances(
            &world,
            vec![
                instance("scene-node/10", [9.0, 0.0, 0.0]),
                instance("scene-node/20", [-4.0, 2.0, 1.0]),
            ],
        )
        .unwrap();
    assert_eq!(moved.len(), 1, "moving A does not recreate or touch B");
    assert!(matches!(
        moved.ops[0],
        RenderDiff::Update { handle, transform: Some(_), .. } if handle == root_a
    ));
    assert_eq!(
        projector.instance_root_handle("scene-node/20"),
        Some(root_b)
    );

    world
        .get_mut(ChunkCoord::new(0, 0, 0))
        .unwrap()
        .set(LocalVoxelCoord::new(0, 0, 1), VoxelValue::solid_raw(2))
        .unwrap();
    let remesh = projector.project_dirty(&mut world);
    let remeshed_handles: std::collections::BTreeSet<_> = remesh
        .ops
        .iter()
        .filter_map(|op| match op {
            RenderDiff::ReplaceMeshPayload { handle, .. } => Some(*handle),
            _ => None,
        })
        .collect();
    assert!(remeshed_handles.contains(
        &projector
            .instance_chunk_handle("scene-node/10", ChunkCoord::new(0, 0, 0))
            .unwrap()
    ));
    assert!(remeshed_handles.contains(
        &projector
            .instance_chunk_handle("scene-node/20", ChunkCoord::new(0, 0, 0))
            .unwrap()
    ));
}

#[test]
fn chunk_children_ignore_grid_world_origin_and_remain_asset_local() {
    let spec = grid().with_origin(core_space::WorldPos::new(100.0, 200.0, 300.0));
    let mut world = VoxelWorld::new(spec);
    let mut chunk = VoxelChunk::from_spec(&spec);
    chunk
        .set(LocalVoxelCoord::new(0, 0, 0), VoxelValue::solid_raw(1))
        .unwrap();
    world.insert(ChunkCoord::new(1, 0, -1), chunk);
    let mut projector = VoxelChunkProjector::new();
    let frame = projector.project_dirty(&mut world);
    let child = frame
        .ops
        .iter()
        .find_map(|op| match op {
            RenderDiff::Create {
                parent: Some(_),
                node,
                ..
            } => Some(node),
            _ => None,
        })
        .expect("chunk child");
    assert_eq!(child.transform.translation, [2.0, 0.0, -2.0]);
}

#[test]
fn single_dirty_chunk_reprojects_only_that_chunk_and_resident_neighbours() {
    let mut world = canonical_world();
    let mut projector = VoxelChunkProjector::new();
    let _ = projector.project_dirty(&mut world); // initial full projection, drains dirty

    let edited = ChunkCoord::new(0, 0, 0);
    let handles_before: Vec<_> = ARRANGEMENT
        .iter()
        .map(|&(x, y, z)| projector.handle_of(ChunkCoord::new(x, y, z)).unwrap())
        .collect();

    // Edit one voxel and invalidate the chunk + its neighbours (authority's job).
    world
        .get_mut(edited)
        .unwrap()
        .set(LocalVoxelCoord::new(0, 0, 1), VoxelValue::solid_raw(2))
        .unwrap();
    world.mark_dirty_with_neighbors(edited);

    let frame = projector.project_dirty(&mut world);

    // Only the edited chunk and its RESIDENT neighbours ((1,0,0) and (0,1,0)) are
    // reprojected; the diagonal chunk (1,1,0) is untouched.
    let touched: std::collections::BTreeSet<u64> = frame
        .ops
        .iter()
        .map(|o| match o {
            RenderDiff::ReplaceMeshPayload { handle, .. } => handle.raw(),
            RenderDiff::Create { handle, .. } => handle.raw(),
            RenderDiff::Destroy { handle } => handle.raw(),
            _ => panic!("unexpected diff in voxel reprojection"),
        })
        .collect();
    let h = |c: ChunkCoord| projector.handle_of(c).unwrap().raw();
    let expected: std::collections::BTreeSet<u64> = [
        h(ChunkCoord::new(0, 0, 0)),
        h(ChunkCoord::new(1, 0, 0)),
        h(ChunkCoord::new(0, 1, 0)),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        touched, expected,
        "only chunk + resident neighbours reproject"
    );
    assert!(
        !touched.contains(&h(ChunkCoord::new(1, 1, 0))),
        "the diagonal chunk must not reproject"
    );

    // Reprojection is ReplaceMeshPayload (no Create/Destroy) — handles are stable.
    assert!(frame
        .ops
        .iter()
        .all(|o| matches!(o, RenderDiff::ReplaceMeshPayload { .. })));
    let handles_after: Vec<_> = ARRANGEMENT
        .iter()
        .map(|&(x, y, z)| projector.handle_of(ChunkCoord::new(x, y, z)).unwrap())
        .collect();
    assert_eq!(
        handles_before, handles_after,
        "chunk handles are stable across edits"
    );
}

#[test]
fn emptying_a_chunk_destroys_its_handle() {
    let mut world = canonical_world();
    let mut projector = VoxelChunkProjector::new();
    let _ = projector.project_dirty(&mut world);

    let target = ChunkCoord::new(1, 1, 0);
    let handle = projector.handle_of(target).unwrap();
    // Clear the whole chunk → no visible geometry.
    let dims = world.grid().chunk_dims();
    world
        .get_mut(target)
        .unwrap()
        .fill_region(
            LocalVoxelCoord::new(0, 0, 0),
            LocalVoxelCoord::new(dims.x(), dims.y(), dims.z()),
            VoxelValue::EMPTY,
        )
        .unwrap();
    world.mark_dirty_with_neighbors(target);

    let frame = projector.project_dirty(&mut world);
    assert!(
        frame
            .ops
            .iter()
            .any(|o| matches!(o, RenderDiff::Destroy { handle: h } if *h == handle)),
        "an emptied chunk's handle is destroyed"
    );
    assert!(projector.handle_of(target).is_none(), "handle is freed");
}

#[test]
fn strategy_label_is_exposed() {
    let projector = VoxelChunkProjector::new();
    assert_eq!(projector.strategy_label(), "visible-face");
}
