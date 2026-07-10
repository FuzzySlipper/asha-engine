//! Deterministic generated-level substrate.
//!
//! # Lane
//!
//! `rust-service` — validates reusable generation configs and produces
//! authoritative voxel data plus stable replay/hash metadata. It does not render
//! and does not own collision queries; those lanes consume the generated
//! [`VoxelWorld`](svc_spatial::VoxelWorld).

#![forbid(unsafe_code)]

use core_events::VoxelEditEvent;
use core_space::{
    ChunkCoord, ChunkDims, GridId, LocalVoxelCoord, VoxelCoord, VoxelGridSpec, WorldPos, WorldVec,
};
use core_voxel::{VoxelMaterialId, VoxelValue};
use svc_rng::{RngSeed, ScopedRng};
use svc_spatial::VoxelWorld;
use svc_volume::VoxelChunk;

pub const TUNNEL_GENERATOR_ID: &str = "asha.tunnel.enclosed.v1";
pub const TUNNEL_GENERATOR_VERSION: u32 = 1;

/// Minimal generic preset vocabulary for enclosed voxel tunnel spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelPreset {
    TinyEnclosed,
}

impl TunnelPreset {
    pub const fn label(self) -> &'static str {
        match self {
            TunnelPreset::TinyEnclosed => "tiny-enclosed",
        }
    }
}

/// Validated input to the deterministic tunnel generator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TunnelGeneratorConfig {
    pub seed: RngSeed,
    pub preset: TunnelPreset,
    pub grid_id: GridId,
    pub voxel_size: f64,
    pub chunk_dims: ChunkDims,
    pub width: u32,
    pub height: u32,
    pub length: u32,
    pub wall_material: VoxelMaterialId,
    pub floor_material: VoxelMaterialId,
    pub accent_material: VoxelMaterialId,
}

impl TunnelGeneratorConfig {
    /// A tiny single-chunk fixture that is large enough for movement/collision
    /// proofs while staying reviewable in goldens.
    pub fn tiny_enclosed(seed: u64) -> Self {
        Self {
            seed: RngSeed::new(seed),
            preset: TunnelPreset::TinyEnclosed,
            grid_id: GridId::new(0),
            voxel_size: 1.0,
            chunk_dims: ChunkDims::new(8, 6, 12).expect("non-zero fixture dims"),
            width: 5,
            height: 4,
            length: 9,
            wall_material: VoxelMaterialId::new(1),
            floor_material: VoxelMaterialId::new(2),
            accent_material: VoxelMaterialId::new(3),
        }
    }
}

/// Classified validation failures for generator configs.
#[derive(Debug, Clone, PartialEq)]
pub enum TunnelGenerationError {
    InvalidVoxelSize {
        value: f64,
    },
    TooSmall {
        width: u32,
        height: u32,
        length: u32,
    },
    ExceedsChunkDims {
        dims: [u32; 3],
        width: u32,
        height: u32,
        length: u32,
    },
    DuplicateMaterials {
        material: VoxelMaterialId,
    },
}

impl core::fmt::Display for TunnelGenerationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TunnelGenerationError::InvalidVoxelSize { value } => {
                write!(f, "invalid voxel size {value}")
            }
            TunnelGenerationError::TooSmall {
                width,
                height,
                length,
            } => write!(
                f,
                "tunnel dimensions {width}x{height}x{length} are too small"
            ),
            TunnelGenerationError::ExceedsChunkDims {
                dims,
                width,
                height,
                length,
            } => write!(
                f,
                "tunnel dimensions {width}x{height}x{length} exceed chunk dims {dims:?}"
            ),
            TunnelGenerationError::DuplicateMaterials { material } => {
                write!(
                    f,
                    "material {} is used for more than one tunnel role",
                    material.raw()
                )
            }
        }
    }
}

impl std::error::Error for TunnelGenerationError {}

/// A generated marker consumers can map to game-specific spawn/catalog concepts.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedSpawnMarker {
    pub id: &'static str,
    pub kind: &'static str,
    pub voxel: VoxelCoord,
    pub world: WorldPos,
    pub yaw_degrees: i32,
}

/// Axis-aligned solid geometry derived from generated voxel authority.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneratedCollisionAabb {
    pub voxel: VoxelCoord,
    pub material: VoxelMaterialId,
    pub min: WorldPos,
    pub max: WorldPos,
}

/// Per-chunk render projection input. The render lane turns this world/chunk into
/// `RenderFrameDiff`s; this service only supplies stable source metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratedRenderChunk {
    pub chunk: ChunkCoord,
    pub content_hash: u64,
    pub solid_voxels: u32,
}

/// Stable replay/hash record for a generated tunnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelGenerationRecord {
    pub generator_id: &'static str,
    pub generator_version: u32,
    pub preset: &'static str,
    pub seed: u64,
    pub config_hash: u64,
    pub output_hash: u64,
    pub chunk_hashes: Vec<(ChunkCoord, u64)>,
}

/// Full generated-level output.
#[derive(Debug, Clone)]
pub struct GeneratedTunnel {
    pub config: TunnelGeneratorConfig,
    pub grid: VoxelGridSpec,
    pub world: VoxelWorld,
    pub events: Vec<VoxelEditEvent>,
    pub spawn_markers: Vec<GeneratedSpawnMarker>,
    pub collision_aabbs: Vec<GeneratedCollisionAabb>,
    pub render_chunks: Vec<GeneratedRenderChunk>,
    pub record: TunnelGenerationRecord,
}

impl GeneratedTunnel {
    /// Translation from canonical positive voxel coordinates into the centered
    /// runtime room frame used by first-person cameras and combat.
    pub fn centered_runtime_world_offset(&self) -> WorldVec {
        WorldVec::new(
            -(f64::from(self.config.width) * self.config.voxel_size * 0.5),
            -self.config.voxel_size,
            -(f64::from(self.config.length) * self.config.voxel_size * 0.5),
        )
    }
}

/// Generate the validated tunnel fixture.
pub fn generate_tunnel(
    config: TunnelGeneratorConfig,
) -> Result<GeneratedTunnel, TunnelGenerationError> {
    validate_config(config)?;
    let grid = VoxelGridSpec::new(config.grid_id, config.voxel_size, config.chunk_dims).ok_or(
        TunnelGenerationError::InvalidVoxelSize {
            value: config.voxel_size,
        },
    )?;

    let mut rng = ScopedRng::new(config.seed, TUNNEL_GENERATOR_ID);
    let accent_side_is_positive_x = rng.next_bool();
    let accent_span = (config.length - 2).max(1);
    let accent_z = 1 + rng.next_bounded_u32(accent_span).expect("positive span");
    let player_yaw = if rng.next_bool() { 0 } else { 90 };

    let chunk_coord = ChunkCoord::ORIGIN;
    let mut chunk = VoxelChunk::from_spec(&grid);
    for z in 0..config.length {
        for y in 0..config.height {
            for x in 0..config.width {
                let material =
                    material_for_cell(config, x, y, z, accent_side_is_positive_x, accent_z);
                if let Some(material) = material {
                    chunk
                        .set(LocalVoxelCoord::new(x, y, z), VoxelValue::solid(material))
                        .expect("generator writes inside chunk bounds");
                }
            }
        }
    }
    chunk.mark_clean();
    let chunk_hash = chunk.content_hash().0;

    let mut world = VoxelWorld::new(grid);
    world.insert(chunk_coord, chunk);

    let collision_aabbs = collect_collision_aabbs(&grid, &world);
    let render_chunks = collect_render_chunks(&world);
    let spawn_markers = vec![
        spawn_marker(
            &grid,
            "player_start",
            "player",
            VoxelCoord::new(1, 1, 1),
            player_yaw,
        ),
        spawn_marker(
            &grid,
            "exit_hint",
            "navigation",
            VoxelCoord::new(config.width as i64 - 2, 1, config.length as i64 - 2),
            180,
        ),
    ];
    let config_hash = hash_config(config);
    let output_hash = hash_output(
        config_hash,
        &render_chunks,
        &spawn_markers,
        &collision_aabbs,
    );
    let events = vec![VoxelEditEvent::ChunkGenerated {
        grid: config.grid_id,
        chunk: chunk_coord,
        seed: config.seed.raw(),
        generator_version: TUNNEL_GENERATOR_VERSION,
        hash: chunk_hash,
    }];
    let record = TunnelGenerationRecord {
        generator_id: TUNNEL_GENERATOR_ID,
        generator_version: TUNNEL_GENERATOR_VERSION,
        preset: config.preset.label(),
        seed: config.seed.raw(),
        config_hash,
        output_hash,
        chunk_hashes: vec![(chunk_coord, chunk_hash)],
    };

    Ok(GeneratedTunnel {
        config,
        grid,
        world,
        events,
        spawn_markers,
        collision_aabbs,
        render_chunks,
        record,
    })
}

fn validate_config(config: TunnelGeneratorConfig) -> Result<(), TunnelGenerationError> {
    if !config.voxel_size.is_finite() || config.voxel_size <= 0.0 {
        return Err(TunnelGenerationError::InvalidVoxelSize {
            value: config.voxel_size,
        });
    }
    if config.width < 3 || config.height < 3 || config.length < 4 {
        return Err(TunnelGenerationError::TooSmall {
            width: config.width,
            height: config.height,
            length: config.length,
        });
    }
    let dims = config.chunk_dims.to_array();
    if config.width > dims[0] || config.height > dims[1] || config.length > dims[2] {
        return Err(TunnelGenerationError::ExceedsChunkDims {
            dims,
            width: config.width,
            height: config.height,
            length: config.length,
        });
    }
    for (a, b) in [
        (config.wall_material, config.floor_material),
        (config.wall_material, config.accent_material),
        (config.floor_material, config.accent_material),
    ] {
        if a == b {
            return Err(TunnelGenerationError::DuplicateMaterials { material: a });
        }
    }
    Ok(())
}

fn material_for_cell(
    config: TunnelGeneratorConfig,
    x: u32,
    y: u32,
    z: u32,
    accent_side_is_positive_x: bool,
    accent_z: u32,
) -> Option<VoxelMaterialId> {
    let on_shell = x == 0
        || x + 1 == config.width
        || y == 0
        || y + 1 == config.height
        || z == 0
        || z + 1 == config.length;
    if !on_shell {
        return None;
    }
    let accent_x = if accent_side_is_positive_x {
        config.width - 1
    } else {
        0
    };
    if x == accent_x && y == 1 && z == accent_z {
        return Some(config.accent_material);
    }
    if y == 0 {
        Some(config.floor_material)
    } else {
        Some(config.wall_material)
    }
}

fn spawn_marker(
    grid: &VoxelGridSpec,
    id: &'static str,
    kind: &'static str,
    voxel: VoxelCoord,
    yaw_degrees: i32,
) -> GeneratedSpawnMarker {
    GeneratedSpawnMarker {
        id,
        kind,
        voxel,
        world: grid.voxel_center_world(voxel),
        yaw_degrees,
    }
}

fn collect_collision_aabbs(
    grid: &VoxelGridSpec,
    world: &VoxelWorld,
) -> Vec<GeneratedCollisionAabb> {
    let mut out = Vec::new();
    for (chunk_coord, chunk) in world.resident_chunks() {
        for (local, value) in chunk.iter() {
            let Some(material) = value.material() else {
                continue;
            };
            let voxel = grid.chunk_local_to_voxel(chunk_coord, local);
            let (min, max) = grid.voxel_bounds_world(voxel);
            out.push(GeneratedCollisionAabb {
                voxel,
                material,
                min,
                max,
            });
        }
    }
    out
}

fn collect_render_chunks(world: &VoxelWorld) -> Vec<GeneratedRenderChunk> {
    let mut out = Vec::new();
    for (chunk, data) in world.resident_chunks() {
        let solid_voxels = data.iter().filter(|(_, value)| value.is_solid()).count() as u32;
        out.push(GeneratedRenderChunk {
            chunk,
            content_hash: data.content_hash().0,
            solid_voxels,
        });
    }
    out
}

fn hash_config(config: TunnelGeneratorConfig) -> u64 {
    let mut h = fnv_offset();
    feed_str(&mut h, config.preset.label());
    feed_u64(&mut h, config.seed.raw());
    feed_u32(&mut h, config.grid_id.raw());
    feed_u64(&mut h, config.voxel_size.to_bits());
    for d in config.chunk_dims.to_array() {
        feed_u32(&mut h, d);
    }
    for d in [config.width, config.height, config.length] {
        feed_u32(&mut h, d);
    }
    for m in [
        config.wall_material,
        config.floor_material,
        config.accent_material,
    ] {
        feed_u32(&mut h, m.raw() as u32);
    }
    h
}

fn hash_output(
    config_hash: u64,
    render_chunks: &[GeneratedRenderChunk],
    spawn_markers: &[GeneratedSpawnMarker],
    collision_aabbs: &[GeneratedCollisionAabb],
) -> u64 {
    let mut h = fnv_offset();
    feed_u64(&mut h, config_hash);
    for chunk in render_chunks {
        for v in chunk.chunk.to_array() {
            feed_i64(&mut h, v);
        }
        feed_u64(&mut h, chunk.content_hash);
        feed_u32(&mut h, chunk.solid_voxels);
    }
    for marker in spawn_markers {
        feed_str(&mut h, marker.id);
        feed_str(&mut h, marker.kind);
        for v in marker.voxel.to_array() {
            feed_i64(&mut h, v);
        }
        feed_i64(&mut h, marker.yaw_degrees as i64);
    }
    feed_u64(&mut h, collision_aabbs.len() as u64);
    h
}

fn fnv_offset() -> u64 {
    0xcbf2_9ce4_8422_2325
}

fn feed_byte(h: &mut u64, b: u8) {
    *h ^= b as u64;
    *h = h.wrapping_mul(0x0000_0100_0000_01b3);
}

fn feed_u32(h: &mut u64, value: u32) {
    for b in value.to_le_bytes() {
        feed_byte(h, b);
    }
}

fn feed_i64(h: &mut u64, value: i64) {
    for b in value.to_le_bytes() {
        feed_byte(h, b);
    }
}

fn feed_u64(h: &mut u64, value: u64) {
    for b in value.to_le_bytes() {
        feed_byte(h, b);
    }
}

fn feed_str(h: &mut u64, value: &str) {
    for b in value.as_bytes() {
        feed_byte(h, *b);
    }
}

/// Human-reviewable deterministic summary used by committed fixtures.
pub fn describe_generated_tunnel(tunnel: &GeneratedTunnel) -> String {
    let mut out = String::new();
    out.push_str("generated-tunnel 1\n");
    out.push_str(&format!("generator={}\n", tunnel.record.generator_id));
    out.push_str(&format!("version={}\n", tunnel.record.generator_version));
    out.push_str(&format!("preset={}\n", tunnel.record.preset));
    out.push_str(&format!("seed={}\n", tunnel.record.seed));
    out.push_str(&format!("config_hash={:016x}\n", tunnel.record.config_hash));
    out.push_str(&format!("output_hash={:016x}\n", tunnel.record.output_hash));
    out.push_str(&format!(
        "dims={}x{}x{}\n",
        tunnel.config.width, tunnel.config.height, tunnel.config.length
    ));
    out.push_str(&format!("events={}\n", tunnel.events.len()));
    for event in &tunnel.events {
        if let VoxelEditEvent::ChunkGenerated {
            chunk,
            seed,
            generator_version,
            hash,
            ..
        } = event
        {
            out.push_str(&format!(
                "event=chunk_generated chunk={},{},{} seed={} version={} hash={:016x}\n",
                chunk.x, chunk.y, chunk.z, seed, generator_version, hash
            ));
        }
    }
    out.push_str(&format!("render_chunks={}\n", tunnel.render_chunks.len()));
    for chunk in &tunnel.render_chunks {
        out.push_str(&format!(
            "render_chunk={},{},{} solids={} hash={:016x}\n",
            chunk.chunk.x, chunk.chunk.y, chunk.chunk.z, chunk.solid_voxels, chunk.content_hash
        ));
    }
    out.push_str(&format!(
        "collision_aabbs={}\n",
        tunnel.collision_aabbs.len()
    ));
    out.push_str(&format!("spawn_markers={}\n", tunnel.spawn_markers.len()));
    for marker in &tunnel.spawn_markers {
        out.push_str(&format!(
            "spawn={} kind={} voxel={},{},{} world={:.1},{:.1},{:.1} yaw={}\n",
            marker.id,
            marker.kind,
            marker.voxel.x,
            marker.voxel.y,
            marker.voxel.z,
            marker.world.x,
            marker.world.y,
            marker.world.z,
            marker.yaw_degrees
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_space::WorldPos;
    use svc_collision::CollisionProjection;

    #[test]
    fn same_seed_produces_same_hash_and_fixture() {
        let config = TunnelGeneratorConfig::tiny_enclosed(17);
        let a = generate_tunnel(config).expect("generate a");
        let b = generate_tunnel(config).expect("generate b");
        assert_eq!(a.record, b.record);
        assert_eq!(describe_generated_tunnel(&a), describe_generated_tunnel(&b));
    }

    #[test]
    fn different_seed_changes_output_metadata() {
        let a = generate_tunnel(TunnelGeneratorConfig::tiny_enclosed(17)).expect("generate a");
        let b = generate_tunnel(TunnelGeneratorConfig::tiny_enclosed(18)).expect("generate b");
        assert_ne!(a.record.output_hash, b.record.output_hash);
        assert_ne!(describe_generated_tunnel(&a), describe_generated_tunnel(&b));
    }

    #[test]
    fn invalid_config_is_rejected() {
        let mut config = TunnelGeneratorConfig::tiny_enclosed(17);
        config.width = 2;
        assert!(matches!(
            generate_tunnel(config),
            Err(TunnelGenerationError::TooSmall { .. })
        ));

        let mut config = TunnelGeneratorConfig::tiny_enclosed(17);
        config.accent_material = config.wall_material;
        assert!(matches!(
            generate_tunnel(config),
            Err(TunnelGenerationError::DuplicateMaterials { .. })
        ));
    }

    #[test]
    fn collision_projection_blocks_shell_but_not_spawn() {
        let tunnel = generate_tunnel(TunnelGeneratorConfig::tiny_enclosed(17)).expect("generate");
        let projection = CollisionProjection::build(&tunnel.world);
        assert_eq!(projection.collider_count(), 1);
        assert!(projection.contains_point(WorldPos::new(0.5, 1.5, 1.5)));
        assert!(!projection.contains_point(tunnel.spawn_markers[0].world));
        let identity = projection.identity(&tunnel.world);
        assert_eq!(identity.source_hash_hex(), "47e4c52bb98a5f36");
        assert_eq!(identity.projection_hash_label(), "fnv1a64:5499053dc60a873b");
    }

    #[test]
    fn centered_runtime_collision_frame_matches_first_person_room_coordinates() {
        let tunnel = generate_tunnel(TunnelGeneratorConfig::tiny_enclosed(17)).expect("generate");
        assert_eq!(
            tunnel.centered_runtime_world_offset(),
            WorldVec::new(-2.5, -1.0, -4.5)
        );
        let projection = CollisionProjection::build_with_offset(
            &tunnel.world,
            tunnel.centered_runtime_world_offset(),
        );

        assert!(projection.contains_point(WorldPos::new(0.0, -0.5, 0.0)));
        assert!(!projection.contains_point(WorldPos::new(0.0, 0.5, 0.0)));
        assert_eq!(
            projection.identity(&tunnel.world).projection_hash_label(),
            "fnv1a64:b2312fbcfb060db3"
        );
    }

    #[test]
    fn tiny_tunnel_fixture_matches_committed_golden() {
        let tunnel = generate_tunnel(TunnelGeneratorConfig::tiny_enclosed(17)).expect("generate");
        assert_eq!(
            describe_generated_tunnel(&tunnel),
            include_str!(
                "../../../../../harness/fixtures/generated-levels/tiny-tunnel.snapshot.txt"
            )
        );
    }
}
