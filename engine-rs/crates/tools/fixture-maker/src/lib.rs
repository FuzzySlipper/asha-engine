//! The canonical abstract voxel fixture (launchable-voxel-03, #2434).
//!
//! One shared, deterministic voxel world that later launchable-voxel tasks
//! (meshing/projection, picking, editing, save/replay, smoke) build on instead of
//! ad-hoc parallel test worlds. This crate is the **generator + validator**: it
//! builds the world in memory, renders it to a committed payload (a small manifest
//! plus per-chunk snapshots in the existing `rule-voxel-edit` persist format), and
//! re-reads/round-trips that payload.
//!
//! # What the fixture is
//!
//! - One [`VoxelGridSpec`] grid (id 1, voxel size 1.0, cubic chunk dims from the
//!   spec — never a hardcoded global chunk assumption).
//! - A 2×2×1 arrangement of chunks (4 chunks) so chunk borders and neighbour
//!   invalidation are exercised: each chunk's bottom layer is solid, so solids meet
//!   across every shared X/Y face.
//! - Multiple abstract materials (ids 1, 2, 3), validated against a
//!   [`MaterialCatalog`]. No product-domain nouns.
//!
//! # Payload
//!
//! - `voxel-world.manifest.json` — grid, materials, and the chunk table (each chunk
//!   with its content hash + snapshot artifact hash) plus a world hash.
//! - `chunk_<x>_<y>_<z>.snapshot` — one RLE chunk snapshot per chunk, in the
//!   `rule-voxel-edit::persist` text format (reused, not duplicated).
//!
//! Regenerate with `cargo run -p fixture-maker -- write`; verify the committed
//! payload with `cargo run -p fixture-maker -- check`. See the fixture README.

#![forbid(unsafe_code)]

use core_space::{ChunkCoord, ChunkDims, GridId, LocalVoxelCoord, VoxelGridSpec};
use core_voxel::{MaterialCatalog, VoxelMaterialId, VoxelValue};
use rule_voxel_edit::persist::encode_chunk_snapshot;
use svc_serialization::BundleHash;
use svc_spatial::VoxelWorld;
use svc_volume::VoxelChunk;

/// Repo-relative directory holding the committed canonical voxel payload.
pub const FIXTURE_DIR: &str = "harness/fixtures/voxel-world";

/// The manifest artifact's file name within [`FIXTURE_DIR`].
pub const MANIFEST_NAME: &str = "voxel-world.manifest.json";

/// The abstract material ids the canonical fixture uses (validated set).
pub const MATERIAL_IDS: [u16; 3] = [1, 2, 3];

/// The 2×2×1 chunk arrangement (z fixed at 0), in canonical ascending order.
const CHUNK_ARRANGEMENT: [(i64, i64, i64); 4] = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (1, 1, 0)];

/// One rendered artifact: a path relative to [`FIXTURE_DIR`] and its full contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedArtifact {
    pub rel_path: String,
    pub contents: String,
}

/// The canonical grid spec. Chunk dims come from the spec; callers must not assume
/// a global chunk size.
pub fn canonical_grid() -> VoxelGridSpec {
    VoxelGridSpec::new(
        GridId::new(1),
        1.0,
        ChunkDims::cubic(2).expect("nonzero dims"),
    )
    .expect("positive voxel size")
}

/// The material catalog the fixture validates its solids against.
pub fn canonical_materials() -> MaterialCatalog {
    MaterialCatalog::new(MATERIAL_IDS.iter().copied().map(VoxelMaterialId::new))
}

/// The material a given chunk is filled with (deterministic, all three ids used).
fn material_for(coord: ChunkCoord) -> u16 {
    let idx = (coord.x * 2 + coord.y).rem_euclid(MATERIAL_IDS.len() as i64) as usize;
    MATERIAL_IDS[idx]
}

/// Build the canonical voxel world in memory. Deterministic: same world every run.
///
/// Each chunk's bottom layer (z = 0) is filled solid with the chunk's material, so
/// solids meet across every shared chunk face (border/neighbour culling matters)
/// while the top stays empty (exposed faces remain).
pub fn build_world() -> VoxelWorld {
    let spec = canonical_grid();
    let catalog = canonical_materials();
    let dims = spec.chunk_dims();
    let mut world = VoxelWorld::new(spec);

    for (x, y, z) in CHUNK_ARRANGEMENT {
        let coord = ChunkCoord::new(x, y, z);
        let value = VoxelValue::solid_raw(material_for(coord));
        catalog
            .validate(value)
            .expect("fixture materials are in the catalog");
        let mut chunk = VoxelChunk::from_spec(&spec);
        // Bottom layer solid: [0,0,0) .. [dx, dy, 1).
        chunk
            .fill_region(
                LocalVoxelCoord::new(0, 0, 0),
                LocalVoxelCoord::new(dims.x(), dims.y(), 1),
                value,
            )
            .expect("fill within chunk bounds");
        world.insert(coord, chunk);
    }
    world
}

/// Render the full committed payload (manifest + chunk snapshots) deterministically,
/// in stable path order (manifest first, then chunks in ascending coord order).
pub fn render_fixture() -> Vec<GeneratedArtifact> {
    let world = build_world();
    let spec = world.grid();

    // Stable, ascending chunk order (VoxelWorld iterates a BTreeMap).
    let mut rows: Vec<ChunkRow> = Vec::new();
    let mut artifacts: Vec<GeneratedArtifact> = Vec::new();
    for (coord, chunk) in world.resident_chunks() {
        let snapshot = encode_chunk_snapshot(chunk);
        let rel_path = chunk_artifact_name(coord);
        rows.push(ChunkRow {
            coord,
            material: material_for(coord),
            artifact: rel_path.clone(),
            chunk_hash: chunk.content_hash().0,
            content_hash: BundleHash::of_str(&snapshot),
        });
        artifacts.push(GeneratedArtifact {
            rel_path,
            contents: snapshot,
        });
    }

    let manifest = render_manifest(spec, &rows);
    // Manifest first for readability; order is otherwise path-stable.
    let mut out = Vec::with_capacity(artifacts.len() + 1);
    out.push(GeneratedArtifact {
        rel_path: MANIFEST_NAME.to_string(),
        contents: manifest,
    });
    out.extend(artifacts);
    out
}

struct ChunkRow {
    coord: ChunkCoord,
    material: u16,
    artifact: String,
    chunk_hash: u64,
    content_hash: BundleHash,
}

fn chunk_artifact_name(coord: ChunkCoord) -> String {
    format!("chunk_{}_{}_{}.snapshot", coord.x, coord.y, coord.z)
}

/// A deterministic world hash folding each chunk's coord + content hash.
fn world_hash(rows: &[ChunkRow]) -> BundleHash {
    let mut buf = String::new();
    for r in rows {
        buf.push_str(&format!(
            "{},{},{}={:016x};",
            r.coord.x, r.coord.y, r.coord.z, r.chunk_hash
        ));
    }
    BundleHash::of_str(&buf)
}

fn render_manifest(spec: VoxelGridSpec, rows: &[ChunkRow]) -> String {
    let dims = spec.chunk_dims();
    let materials = MATERIAL_IDS
        .iter()
        .map(|m| m.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"fixture\": \"canonical-voxel-world\",\n");
    s.push_str(&format!(
        "  \"grid\": {{ \"id\": {}, \"voxelSize\": {}, \"chunkDims\": [{}, {}, {}] }},\n",
        spec.id().raw(),
        spec.voxel_size(),
        dims.x(),
        dims.y(),
        dims.z(),
    ));
    s.push_str(&format!("  \"materials\": [{materials}],\n"));
    s.push_str("  \"chunks\": [\n");
    for (i, r) in rows.iter().enumerate() {
        let last = i + 1 == rows.len();
        s.push_str(&format!(
            "    {{ \"coord\": [{}, {}, {}], \"material\": {}, \"artifact\": {:?}, \
             \"chunkHash\": \"{:016x}\", \"contentHash\": \"{}\" }}{}\n",
            r.coord.x,
            r.coord.y,
            r.coord.z,
            r.material,
            r.artifact,
            r.chunk_hash,
            r.content_hash.to_hex(),
            if last { "" } else { "," },
        ));
    }
    s.push_str("  ],\n");
    s.push_str(&format!(
        "  \"worldHash\": \"{}\"\n",
        world_hash(rows).to_hex()
    ));
    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use rule_voxel_edit::persist::decode_chunk_snapshot;

    #[test]
    fn world_has_four_chunks_with_all_materials_and_solid_borders() {
        let world = build_world();
        let chunks: Vec<_> = world.resident_chunks().collect();
        assert_eq!(chunks.len(), 4, "2x2x1 arrangement");

        let mats: std::collections::BTreeSet<u16> = CHUNK_ARRANGEMENT
            .iter()
            .map(|&(x, y, z)| material_for(ChunkCoord::new(x, y, z)))
            .collect();
        assert_eq!(mats, MATERIAL_IDS.iter().copied().collect());

        // Every chunk's bottom layer is solid (so faces meet across borders).
        for (_, chunk) in &chunks {
            assert!(chunk.get(LocalVoxelCoord::new(0, 0, 0)).unwrap().is_solid());
            // Top layer is empty (exposed faces remain).
            assert!(chunk.get(LocalVoxelCoord::new(0, 0, 1)).unwrap().is_empty());
        }
    }

    #[test]
    fn render_is_deterministic() {
        assert_eq!(render_fixture(), render_fixture());
    }

    #[test]
    fn chunk_snapshots_round_trip_and_match_manifest_hashes() {
        let world = build_world();
        for (coord, chunk) in world.resident_chunks() {
            let text = encode_chunk_snapshot(chunk);
            let decoded = decode_chunk_snapshot(&text).expect("snapshot decodes");
            // Reconstruction preserves the chunk fingerprint...
            assert_eq!(decoded.content_hash(), chunk.content_hash());
            // ...and re-encoding is a fixed point.
            assert_eq!(encode_chunk_snapshot(&decoded), text);
            assert!(coord.z == 0);
        }
    }

    #[test]
    fn manifest_lists_every_chunk_artifact_and_is_lf_terminated() {
        let artifacts = render_fixture();
        let manifest = &artifacts[0];
        assert_eq!(manifest.rel_path, MANIFEST_NAME);
        assert!(manifest.contents.ends_with("}\n"));
        for art in &artifacts[1..] {
            assert!(
                manifest.contents.contains(&format!("{:?}", art.rel_path)),
                "manifest references {}",
                art.rel_path
            );
            assert!(art.contents.starts_with("voxelchunk "));
        }
    }
}
