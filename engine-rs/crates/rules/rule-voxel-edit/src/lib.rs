//! Authoritative voxel edit/generation: validation, application, deterministic
//! generation, and non-mutating preview (voxel-capability-05).
//!
//! # Lane
//!
//! `rust-rule` — consumes the **canonical** command/event types
//! (`core_commands::VoxelCommand`, `core_events::VoxelEditEvent`); it does not
//! define parallel local command/event types (that would be the ownership leak
//! the capability doc warns about). It validates proposed edits against voxel
//! storage + a material catalog, turns them into accepted events, and applies
//! those events to a [`VoxelWorld`].
//!
//! # Authority flow
//!
//! ```text
//! VoxelCommand --validate(world, materials)--> [VoxelEditEvent]   (or VoxelEditRejection)
//! VoxelEditEvent --apply(world)--> mutated VoxelWorld
//! ```
//!
//! `preview` runs validation **without** applying, so UI/tooling can show a brush
//! result without mutating authority. Generation is deterministic from
//! `seed + chunk coord + generator_version` with no noise-library commitment.

#![forbid(unsafe_code)]

pub mod persist;
pub mod picking;

use core_commands::VoxelCommand;
use core_events::VoxelEditEvent;
use core_space::{ChunkCoord, ChunkRegion, VoxelCoord, VoxelGridSpec, VoxelRegion};
use core_voxel::{MaterialCatalog, VoxelMaterialId, VoxelValue};
use svc_spatial::{ChunkState, VoxelWorld};
use svc_volume::VoxelChunk;

/// Why a proposed voxel edit was refused. The authoritative rejection surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelEditRejection {
    /// A value referenced a material the catalog does not contain.
    UnknownMaterial(VoxelMaterialId),
    /// A fill region was empty (`max <= min` on some axis).
    EmptyRegion { min: VoxelCoord, max: VoxelCoord },
    /// The edit touched a chunk that is not resident.
    ChunkNotResident { chunk: ChunkCoord },
    /// On replay, regenerating a chunk produced a different hash than recorded —
    /// a generator drift (e.g. a `generator_version` change), surfaced rather than
    /// silently reconstructing different terrain (decision 6).
    GenerationDivergence {
        chunk: ChunkCoord,
        expected: u64,
        actual: u64,
    },
}

impl core::fmt::Display for VoxelEditRejection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VoxelEditRejection::UnknownMaterial(id) => {
                write!(f, "unknown material id {}", id.raw())
            }
            VoxelEditRejection::EmptyRegion { min, max } => {
                write!(
                    f,
                    "empty fill region {:?}..{:?}",
                    min.to_array(),
                    max.to_array()
                )
            }
            VoxelEditRejection::ChunkNotResident { chunk } => {
                write!(f, "chunk {:?} is not resident", chunk.to_array())
            }
            VoxelEditRejection::GenerationDivergence {
                chunk,
                expected,
                actual,
            } => write!(
                f,
                "generation divergence at chunk {:?}: expected hash {expected:#x}, got {actual:#x}",
                chunk.to_array()
            ),
        }
    }
}

impl std::error::Error for VoxelEditRejection {}

// ── Deterministic generation hook ──────────────────────────────────────────────

/// FNV-1a over the generation inputs — a deterministic, library-free stand-in for
/// a future noise generator. Same inputs always yield the same value.
fn gen_hash(seed: u64, version: u32, words: &[i64]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h = OFFSET;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(PRIME);
        }
    };
    feed(&seed.to_le_bytes());
    feed(&version.to_le_bytes());
    for w in words {
        feed(&w.to_le_bytes());
    }
    h
}

/// Deterministically generate a chunk's voxels from `seed` + chunk coord +
/// `generator_version`. A placeholder "blocky heightfield": each column `(x,z)`
/// is solid up to a per-column height derived from the hash. Abstract on purpose —
/// no terrain/material taxonomy, single fixture material id 1.
pub fn generate_chunk(
    spec: &VoxelGridSpec,
    chunk: ChunkCoord,
    seed: u64,
    generator_version: u32,
) -> VoxelChunk {
    use core_space::LocalVoxelCoord;
    let dims = spec.chunk_dims();
    let mut out = VoxelChunk::new(spec.id(), dims);
    let material = VoxelValue::solid_raw(1);
    for z in 0..dims.z() {
        for x in 0..dims.x() {
            // World voxel coord of this column's base, so generation is continuous
            // across chunk borders (uses absolute coords, not chunk-local).
            let base = spec.chunk_local_to_voxel(chunk, LocalVoxelCoord::new(x, 0, z));
            let h = gen_hash(seed, generator_version, &[base.x, base.z]);
            let height = (h % (dims.y() as u64 + 1)) as u32;
            for y in 0..height {
                out.set(LocalVoxelCoord::new(x, y, z), material)
                    .expect("generation local in bounds");
            }
        }
    }
    out.mark_clean();
    out
}

// ── Validation (no mutation) ───────────────────────────────────────────────────

/// Validate a proposed [`VoxelCommand`] against the world + material catalog,
/// producing the accepted [`VoxelEditEvent`]s — **without mutating** anything.
/// This is exactly what [`preview`] returns.
pub fn validate(
    cmd: &VoxelCommand,
    world: &VoxelWorld,
    materials: &MaterialCatalog,
) -> Result<Vec<VoxelEditEvent>, VoxelEditRejection> {
    let spec = world.grid();
    match *cmd {
        VoxelCommand::SetVoxel { grid, coord, value } => {
            check_material(materials, value)?;
            require_resident(world, spec.voxel_to_chunk(coord))?;
            Ok(vec![VoxelEditEvent::VoxelSet { grid, coord, value }])
        }
        VoxelCommand::FillRegion {
            grid,
            min,
            max,
            value,
        } => {
            check_material(materials, value)?;
            let region = VoxelRegion::new(min, max);
            if region.is_empty() {
                return Err(VoxelEditRejection::EmptyRegion { min, max });
            }
            // Every chunk the region overlaps must be resident.
            for chunk in chunk_span(&spec, region).iter() {
                require_resident(world, chunk)?;
            }
            Ok(vec![VoxelEditEvent::VoxelRegionFilled {
                grid,
                min,
                max,
                value,
            }])
        }
        VoxelCommand::GenerateChunk {
            grid,
            chunk,
            seed,
            generator_version,
        } => {
            let generated = generate_chunk(&spec, chunk, seed, generator_version);
            Ok(vec![VoxelEditEvent::ChunkGenerated {
                grid,
                chunk,
                seed,
                generator_version,
                hash: generated.content_hash().0,
            }])
        }
    }
}

/// Validate without applying — UI/tooling brush preview. Identical to [`validate`]
/// but named to make the non-mutating intent explicit at call sites.
pub fn preview(
    cmd: &VoxelCommand,
    world: &VoxelWorld,
    materials: &MaterialCatalog,
) -> Result<Vec<VoxelEditEvent>, VoxelEditRejection> {
    validate(cmd, world, materials)
}

// ── Bulk transactions (bounded, atomic authority surface) ──────────────────────

/// Whether a bulk voxel edit transaction mutates authority or only computes its
/// accepted event log and projected state hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelEditTransactionMode {
    /// Validate, quota-check, and project the result without mutating `world`.
    PreviewOnly,
    /// Validate and apply atomically. Any rejection leaves `world` unchanged.
    Apply,
}

/// Broad enough for authored model-building operations, bounded enough to catch
/// runaway tools before they become load-bearing runtime behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelEditTransactionLimits {
    pub max_commands: u32,
    pub max_events: u32,
    pub max_touched_voxels: u64,
}

impl VoxelEditTransactionLimits {
    pub const fn new(max_commands: u32, max_events: u32, max_touched_voxels: u64) -> Self {
        Self {
            max_commands,
            max_events,
            max_touched_voxels,
        }
    }
}

impl Default for VoxelEditTransactionLimits {
    fn default() -> Self {
        Self::new(10_000, 20_000, 1_000_000)
    }
}

/// A bulk edit request over the canonical generated voxel command union.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelEditTransaction<'a> {
    pub mode: VoxelEditTransactionMode,
    pub commands: &'a [VoxelCommand],
    pub limits: VoxelEditTransactionLimits,
}

impl<'a> VoxelEditTransaction<'a> {
    pub fn preview(commands: &'a [VoxelCommand]) -> Self {
        Self {
            mode: VoxelEditTransactionMode::PreviewOnly,
            commands,
            limits: VoxelEditTransactionLimits::default(),
        }
    }

    pub fn apply(commands: &'a [VoxelCommand]) -> Self {
        Self {
            mode: VoxelEditTransactionMode::Apply,
            commands,
            limits: VoxelEditTransactionLimits::default(),
        }
    }

    pub fn with_limits(mut self, limits: VoxelEditTransactionLimits) -> Self {
        self.limits = limits;
        self
    }
}

/// Why a bulk transaction was refused. Command-level failures retain the existing
/// authoritative voxel edit rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelEditTransactionRejection {
    CommandQuotaExceeded {
        limit: u32,
        actual: u32,
    },
    EventQuotaExceeded {
        limit: u32,
        actual: u32,
    },
    TouchedVoxelQuotaExceeded {
        limit: u64,
        actual: u64,
    },
    InvalidCommand {
        index: u32,
        rejection: VoxelEditRejection,
    },
    ApplyFailed {
        index: u32,
        rejection: VoxelEditRejection,
    },
}

/// The deterministic receipt for a bulk transaction attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelEditTransactionReceipt {
    pub mode: VoxelEditTransactionMode,
    pub applied: bool,
    pub accepted: u32,
    pub rejected: u32,
    pub event_count: u32,
    pub touched_voxels: u64,
    pub before_hash: u64,
    pub projected_hash: u64,
    pub after_hash: u64,
    pub transaction_hash: u64,
    pub events: Vec<VoxelEditEvent>,
    pub rejections: Vec<VoxelEditTransactionRejection>,
}

/// Validate and optionally apply a bulk voxel transaction atomically.
///
/// Validation runs sequentially on a scratch world, so a transaction may generate
/// a chunk and then edit that chunk. The real `world` is replaced only when the
/// full transaction is accepted and `mode == Apply`.
pub fn execute_transaction(
    world: &mut VoxelWorld,
    materials: &MaterialCatalog,
    tx: &VoxelEditTransaction<'_>,
) -> VoxelEditTransactionReceipt {
    let before_hash = voxel_world_hash(world);
    let command_count = tx.commands.len().min(u32::MAX as usize) as u32;
    let mut events = Vec::<VoxelEditEvent>::new();
    let mut rejections = Vec::<VoxelEditTransactionRejection>::new();

    if command_count > tx.limits.max_commands {
        rejections.push(VoxelEditTransactionRejection::CommandQuotaExceeded {
            limit: tx.limits.max_commands,
            actual: command_count,
        });
        return transaction_receipt(PendingTransactionReceipt {
            mode: tx.mode,
            applied: false,
            accepted: 0,
            touched_voxels: 0,
            before_hash,
            projected_hash: before_hash,
            after_hash: before_hash,
            events,
            rejections,
        });
    }

    let mut scratch = world.clone();
    let mut accepted = 0u32;
    let mut touched_voxels = 0u64;

    for (index, command) in tx.commands.iter().enumerate() {
        let index = index.min(u32::MAX as usize) as u32;
        match validate(command, &scratch, materials) {
            Ok(command_events) => {
                for event in &command_events {
                    if let Err(rejection) = apply(&mut scratch, event) {
                        rejections
                            .push(VoxelEditTransactionRejection::ApplyFailed { index, rejection });
                    }
                }
                accepted = accepted.saturating_add(1);
                touched_voxels =
                    touched_voxels.saturating_add(command_touched_voxels(command, scratch.grid()));
                events.extend(command_events);
            }
            Err(rejection) => {
                rejections.push(VoxelEditTransactionRejection::InvalidCommand { index, rejection });
            }
        }
    }

    let event_count = events.len().min(u32::MAX as usize) as u32;
    if event_count > tx.limits.max_events {
        rejections.push(VoxelEditTransactionRejection::EventQuotaExceeded {
            limit: tx.limits.max_events,
            actual: event_count,
        });
    }
    if touched_voxels > tx.limits.max_touched_voxels {
        rejections.push(VoxelEditTransactionRejection::TouchedVoxelQuotaExceeded {
            limit: tx.limits.max_touched_voxels,
            actual: touched_voxels,
        });
    }

    let projected_hash = if rejections.is_empty() {
        voxel_world_hash(&scratch)
    } else {
        before_hash
    };
    let applied = rejections.is_empty() && tx.mode == VoxelEditTransactionMode::Apply;
    if applied {
        *world = scratch;
    }
    let after_hash = voxel_world_hash(world);

    transaction_receipt(PendingTransactionReceipt {
        mode: tx.mode,
        applied,
        accepted,
        touched_voxels,
        before_hash,
        projected_hash,
        after_hash,
        events,
        rejections,
    })
}

struct PendingTransactionReceipt {
    mode: VoxelEditTransactionMode,
    applied: bool,
    accepted: u32,
    touched_voxels: u64,
    before_hash: u64,
    projected_hash: u64,
    after_hash: u64,
    events: Vec<VoxelEditEvent>,
    rejections: Vec<VoxelEditTransactionRejection>,
}

fn transaction_receipt(parts: PendingTransactionReceipt) -> VoxelEditTransactionReceipt {
    let event_count = parts.events.len().min(u32::MAX as usize) as u32;
    let rejected = parts.rejections.len().min(u32::MAX as usize) as u32;
    let mut receipt = VoxelEditTransactionReceipt {
        mode: parts.mode,
        applied: parts.applied,
        accepted: parts.accepted,
        rejected,
        event_count,
        touched_voxels: parts.touched_voxels,
        before_hash: parts.before_hash,
        projected_hash: parts.projected_hash,
        after_hash: parts.after_hash,
        transaction_hash: 0,
        events: parts.events,
        rejections: parts.rejections,
    };
    receipt.transaction_hash = transaction_hash(&receipt);
    receipt
}

fn command_touched_voxels(command: &VoxelCommand, spec: VoxelGridSpec) -> u64 {
    match *command {
        VoxelCommand::SetVoxel { .. } => 1,
        VoxelCommand::FillRegion { min, max, .. } => region_volume(min, max).unwrap_or(u64::MAX),
        VoxelCommand::GenerateChunk { .. } => spec.chunk_dims().volume(),
    }
}

fn region_volume(min: VoxelCoord, max: VoxelCoord) -> Option<u64> {
    let dx = u64::try_from(max.x.checked_sub(min.x)?).ok()?;
    let dy = u64::try_from(max.y.checked_sub(min.y)?).ok()?;
    let dz = u64::try_from(max.z.checked_sub(min.z)?).ok()?;
    dx.checked_mul(dy)?.checked_mul(dz)
}

fn voxel_world_hash(world: &VoxelWorld) -> u64 {
    let mut hasher = Fnv1a::new();
    for (coord, chunk) in world.resident_chunks() {
        hasher.feed_i64(coord.x);
        hasher.feed_i64(coord.y);
        hasher.feed_i64(coord.z);
        hasher.feed_u64(chunk.content_hash().0);
    }
    hasher.finish()
}

fn transaction_hash(receipt: &VoxelEditTransactionReceipt) -> u64 {
    let mut hasher = Fnv1a::new();
    hasher.feed_u8(match receipt.mode {
        VoxelEditTransactionMode::PreviewOnly => 0,
        VoxelEditTransactionMode::Apply => 1,
    });
    hasher.feed_u8(u8::from(receipt.applied));
    hasher.feed_u32(receipt.accepted);
    hasher.feed_u32(receipt.rejected);
    hasher.feed_u64(receipt.touched_voxels);
    hasher.feed_u64(receipt.before_hash);
    hasher.feed_u64(receipt.projected_hash);
    hasher.feed_u64(receipt.after_hash);
    for event in &receipt.events {
        feed_event_hash(&mut hasher, event);
    }
    for rejection in &receipt.rejections {
        feed_transaction_rejection_hash(&mut hasher, rejection);
    }
    hasher.finish()
}

fn feed_event_hash(hasher: &mut Fnv1a, event: &VoxelEditEvent) {
    match *event {
        VoxelEditEvent::VoxelSet { grid, coord, value } => {
            hasher.feed_u8(0);
            hasher.feed_u32(grid.raw());
            feed_coord_hash(hasher, coord);
            hasher.feed_u32(value.to_encoded());
        }
        VoxelEditEvent::VoxelRegionFilled {
            grid,
            min,
            max,
            value,
        } => {
            hasher.feed_u8(1);
            hasher.feed_u32(grid.raw());
            feed_coord_hash(hasher, min);
            feed_coord_hash(hasher, max);
            hasher.feed_u32(value.to_encoded());
        }
        VoxelEditEvent::ChunkGenerated {
            grid,
            chunk,
            seed,
            generator_version,
            hash,
        } => {
            hasher.feed_u8(2);
            hasher.feed_u32(grid.raw());
            hasher.feed_i64(chunk.x);
            hasher.feed_i64(chunk.y);
            hasher.feed_i64(chunk.z);
            hasher.feed_u64(seed);
            hasher.feed_u32(generator_version);
            hasher.feed_u64(hash);
        }
    }
}

fn feed_transaction_rejection_hash(hasher: &mut Fnv1a, rejection: &VoxelEditTransactionRejection) {
    match *rejection {
        VoxelEditTransactionRejection::CommandQuotaExceeded { limit, actual } => {
            hasher.feed_u8(0);
            hasher.feed_u32(limit);
            hasher.feed_u32(actual);
        }
        VoxelEditTransactionRejection::EventQuotaExceeded { limit, actual } => {
            hasher.feed_u8(1);
            hasher.feed_u32(limit);
            hasher.feed_u32(actual);
        }
        VoxelEditTransactionRejection::TouchedVoxelQuotaExceeded { limit, actual } => {
            hasher.feed_u8(2);
            hasher.feed_u64(limit);
            hasher.feed_u64(actual);
        }
        VoxelEditTransactionRejection::InvalidCommand { index, rejection } => {
            hasher.feed_u8(3);
            hasher.feed_u32(index);
            feed_edit_rejection_hash(hasher, rejection);
        }
        VoxelEditTransactionRejection::ApplyFailed { index, rejection } => {
            hasher.feed_u8(4);
            hasher.feed_u32(index);
            feed_edit_rejection_hash(hasher, rejection);
        }
    }
}

fn feed_edit_rejection_hash(hasher: &mut Fnv1a, rejection: VoxelEditRejection) {
    match rejection {
        VoxelEditRejection::UnknownMaterial(id) => {
            hasher.feed_u8(0);
            hasher.feed_u16(id.raw());
        }
        VoxelEditRejection::EmptyRegion { min, max } => {
            hasher.feed_u8(1);
            feed_coord_hash(hasher, min);
            feed_coord_hash(hasher, max);
        }
        VoxelEditRejection::ChunkNotResident { chunk } => {
            hasher.feed_u8(2);
            hasher.feed_i64(chunk.x);
            hasher.feed_i64(chunk.y);
            hasher.feed_i64(chunk.z);
        }
        VoxelEditRejection::GenerationDivergence {
            chunk,
            expected,
            actual,
        } => {
            hasher.feed_u8(3);
            hasher.feed_i64(chunk.x);
            hasher.feed_i64(chunk.y);
            hasher.feed_i64(chunk.z);
            hasher.feed_u64(expected);
            hasher.feed_u64(actual);
        }
    }
}

fn feed_coord_hash(hasher: &mut Fnv1a, coord: VoxelCoord) {
    hasher.feed_i64(coord.x);
    hasher.feed_i64(coord.y);
    hasher.feed_i64(coord.z);
}

struct Fnv1a {
    value: u64,
}

impl Fnv1a {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self {
            value: Self::OFFSET,
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.value ^= u64::from(*byte);
            self.value = self.value.wrapping_mul(Self::PRIME);
        }
    }

    fn feed_u8(&mut self, value: u8) {
        self.feed(&value.to_le_bytes());
    }

    fn feed_u16(&mut self, value: u16) {
        self.feed(&value.to_le_bytes());
    }

    fn feed_u32(&mut self, value: u32) {
        self.feed(&value.to_le_bytes());
    }

    fn feed_u64(&mut self, value: u64) {
        self.feed(&value.to_le_bytes());
    }

    fn feed_i64(&mut self, value: i64) {
        self.feed(&value.to_le_bytes());
    }

    fn finish(self) -> u64 {
        self.value
    }
}

fn check_material(
    materials: &MaterialCatalog,
    value: VoxelValue,
) -> Result<(), VoxelEditRejection> {
    materials.validate(value).map_err(|e| {
        let core_voxel::MaterialError::UnknownMaterial(id) = e;
        VoxelEditRejection::UnknownMaterial(id)
    })
}

fn require_resident(world: &VoxelWorld, chunk: ChunkCoord) -> Result<(), VoxelEditRejection> {
    if world.state(chunk) == ChunkState::Resident {
        Ok(())
    } else {
        Err(VoxelEditRejection::ChunkNotResident { chunk })
    }
}

/// The inclusive chunk span a voxel region overlaps, as a half-open [`ChunkRegion`].
fn chunk_span(spec: &VoxelGridSpec, region: VoxelRegion) -> ChunkRegion {
    let min_chunk = spec.voxel_to_chunk(region.min);
    // `region.max` is exclusive; the last contained voxel is `max - 1`.
    let last = VoxelCoord::new(region.max.x - 1, region.max.y - 1, region.max.z - 1);
    let max_chunk = spec.voxel_to_chunk(last);
    ChunkRegion::new(
        min_chunk,
        ChunkCoord::new(max_chunk.x + 1, max_chunk.y + 1, max_chunk.z + 1),
    )
}

// ── Application (mutation) ──────────────────────────────────────────────────────

/// Apply an accepted [`VoxelEditEvent`] to the world. Marks touched chunks (and
/// their resident neighbours, for border edits) dirty.
pub fn apply(world: &mut VoxelWorld, event: &VoxelEditEvent) -> Result<(), VoxelEditRejection> {
    let spec = world.grid();
    match *event {
        VoxelEditEvent::VoxelSet { coord, value, .. } => {
            let (chunk, local) = spec.voxel_to_chunk_local(coord);
            let c = world
                .get_mut(chunk)
                .ok_or(VoxelEditRejection::ChunkNotResident { chunk })?;
            c.set(local, value).expect("local in bounds");
            if is_border_local(&spec, local) {
                world.mark_dirty_with_neighbors(chunk);
            }
            Ok(())
        }
        VoxelEditEvent::VoxelRegionFilled {
            min, max, value, ..
        } => {
            let region = VoxelRegion::new(min, max);
            // Per chunk: fill the local intersection in one shot.
            for chunk in chunk_span(&spec, region).iter() {
                if world.state(chunk) != ChunkState::Resident {
                    return Err(VoxelEditRejection::ChunkNotResident { chunk });
                }
                let (lmin, lmax) = local_intersection(&spec, chunk, region);
                let c = world.get_mut(chunk).expect("resident");
                c.fill_region(lmin, lmax, value)
                    .expect("intersection in bounds");
                world.mark_dirty_with_neighbors(chunk);
            }
            Ok(())
        }
        VoxelEditEvent::ChunkGenerated {
            chunk,
            seed,
            generator_version,
            hash,
            ..
        } => {
            let generated = generate_chunk(&spec, chunk, seed, generator_version);
            let actual = generated.content_hash().0;
            if actual != hash {
                return Err(VoxelEditRejection::GenerationDivergence {
                    chunk,
                    expected: hash,
                    actual,
                });
            }
            world.insert(chunk, generated);
            Ok(())
        }
    }
}

/// Apply a whole sequence of events in order (replay).
pub fn apply_all(
    world: &mut VoxelWorld,
    events: &[VoxelEditEvent],
) -> Result<(), VoxelEditRejection> {
    for e in events {
        apply(world, e)?;
    }
    Ok(())
}

fn is_border_local(spec: &VoxelGridSpec, local: core_space::LocalVoxelCoord) -> bool {
    let d = spec.chunk_dims();
    local.x == 0
        || local.y == 0
        || local.z == 0
        || local.x == d.x() - 1
        || local.y == d.y() - 1
        || local.z == d.z() - 1
}

/// The local `[min, max)` box where `region` intersects `chunk`.
fn local_intersection(
    spec: &VoxelGridSpec,
    chunk: ChunkCoord,
    region: VoxelRegion,
) -> (core_space::LocalVoxelCoord, core_space::LocalVoxelCoord) {
    use core_space::LocalVoxelCoord;
    let origin = spec.chunk_origin_voxel(chunk);
    let d = spec.chunk_dims();
    let axis = |rmin: i64, rmax: i64, o: i64, dim: u32| -> (u32, u32) {
        let lo = (rmin - o).max(0) as u32;
        let hi = (rmax - o).min(dim as i64) as u32;
        (lo, hi)
    };
    let (xmin, xmax) = axis(region.min.x, region.max.x, origin.x, d.x());
    let (ymin, ymax) = axis(region.min.y, region.max.y, origin.y, d.y());
    let (zmin, zmax) = axis(region.min.z, region.max.z, origin.z, d.z());
    (
        LocalVoxelCoord::new(xmin, ymin, zmin),
        LocalVoxelCoord::new(xmax, ymax, zmax),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_space::{ChunkDims, GridId, LocalVoxelCoord};

    fn spec() -> VoxelGridSpec {
        VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(8).unwrap()).unwrap()
    }

    fn materials() -> MaterialCatalog {
        MaterialCatalog::new([VoxelMaterialId::new(1), VoxelMaterialId::new(2)])
    }

    fn resident_world() -> VoxelWorld {
        let mut w = VoxelWorld::new(spec());
        w.insert(ChunkCoord::new(0, 0, 0), VoxelChunk::from_spec(&spec()));
        w.drain_dirty();
        w
    }

    #[test]
    fn generation_is_deterministic_for_same_inputs() {
        let a = generate_chunk(&spec(), ChunkCoord::new(1, 0, -2), 42, 1);
        let b = generate_chunk(&spec(), ChunkCoord::new(1, 0, -2), 42, 1);
        assert_eq!(a, b);
        assert_eq!(a.content_hash(), b.content_hash());
        // Different seed or version diverges.
        let c = generate_chunk(&spec(), ChunkCoord::new(1, 0, -2), 43, 1);
        assert_ne!(a.content_hash(), c.content_hash());
        let d = generate_chunk(&spec(), ChunkCoord::new(1, 0, -2), 42, 2);
        assert_ne!(a.content_hash(), d.content_hash());
    }

    #[test]
    fn invalid_material_is_rejected() {
        let world = resident_world();
        let cmd = VoxelCommand::SetVoxel {
            grid: GridId::new(0),
            coord: VoxelCoord::new(0, 0, 0),
            value: VoxelValue::solid_raw(9),
        };
        assert_eq!(
            validate(&cmd, &world, &materials()),
            Err(VoxelEditRejection::UnknownMaterial(VoxelMaterialId::new(9))),
        );
    }

    #[test]
    fn set_into_non_resident_chunk_is_rejected() {
        let world = VoxelWorld::new(spec()); // nothing resident
        let cmd = VoxelCommand::SetVoxel {
            grid: GridId::new(0),
            coord: VoxelCoord::new(0, 0, 0),
            value: VoxelValue::solid_raw(1),
        };
        assert_eq!(
            validate(&cmd, &world, &materials()),
            Err(VoxelEditRejection::ChunkNotResident {
                chunk: ChunkCoord::new(0, 0, 0)
            }),
        );
    }

    #[test]
    fn empty_fill_region_is_rejected() {
        let world = resident_world();
        let cmd = VoxelCommand::FillRegion {
            grid: GridId::new(0),
            min: VoxelCoord::new(2, 2, 2),
            max: VoxelCoord::new(2, 5, 5),
            value: VoxelValue::solid_raw(1),
        };
        assert!(matches!(
            validate(&cmd, &world, &materials()),
            Err(VoxelEditRejection::EmptyRegion { .. })
        ));
    }

    #[test]
    fn set_event_applies_and_produces_expected_hash() {
        let mut world = resident_world();
        let cmd = VoxelCommand::SetVoxel {
            grid: GridId::new(0),
            coord: VoxelCoord::new(3, 4, 5),
            value: VoxelValue::solid_raw(2),
        };
        let events = validate(&cmd, &world, &materials()).unwrap();
        apply_all(&mut world, &events).unwrap();
        let chunk = world.get(ChunkCoord::new(0, 0, 0)).unwrap();
        assert_eq!(
            chunk.get(LocalVoxelCoord::new(3, 4, 5)),
            Some(VoxelValue::solid_raw(2))
        );

        // Independent reconstruction reaches the same hash.
        let mut other = resident_world();
        apply_all(&mut other, &events).unwrap();
        assert_eq!(
            world.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash(),
            other.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash(),
        );
    }

    #[test]
    fn edit_overrides_generated_base() {
        let mut world = VoxelWorld::new(spec());
        let chunk = ChunkCoord::new(0, 0, 0);
        // Generate the base, then overwrite one cell to Empty.
        let gen = VoxelCommand::GenerateChunk {
            grid: GridId::new(0),
            chunk,
            seed: 7,
            generator_version: 1,
        };
        let gen_events = validate(&gen, &world, &materials()).unwrap();
        apply_all(&mut world, &gen_events).unwrap();
        // Pick a cell the generator filled solid (column height > 0 somewhere).
        let base = generate_chunk(&spec(), chunk, 7, 1);
        let solid = base
            .iter()
            .find(|(_, v)| v.is_solid())
            .map(|(l, _)| l)
            .expect("generation produced some solids for this seed");
        let world_coord = spec().chunk_local_to_voxel(chunk, solid);
        let edit = VoxelCommand::SetVoxel {
            grid: GridId::new(0),
            coord: world_coord,
            value: VoxelValue::EMPTY,
        };
        let edit_events = validate(&edit, &world, &materials()).unwrap();
        apply_all(&mut world, &edit_events).unwrap();
        assert_eq!(
            world.get(chunk).unwrap().get(solid),
            Some(VoxelValue::EMPTY)
        );
    }

    #[test]
    fn preview_does_not_mutate_authority() {
        let world = resident_world();
        let before = world.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash();
        let cmd = VoxelCommand::SetVoxel {
            grid: GridId::new(0),
            coord: VoxelCoord::new(1, 1, 1),
            value: VoxelValue::solid_raw(1),
        };
        let events = preview(&cmd, &world, &materials()).unwrap();
        assert_eq!(events.len(), 1);
        // World is untouched by preview.
        assert_eq!(
            world.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash(),
            before
        );
    }

    #[test]
    fn fill_spanning_two_chunks_applies_to_both() {
        let mut world = VoxelWorld::new(spec());
        world.insert(ChunkCoord::new(0, 0, 0), VoxelChunk::from_spec(&spec()));
        world.insert(ChunkCoord::new(1, 0, 0), VoxelChunk::from_spec(&spec()));
        world.drain_dirty();
        // 8-wide chunks: x in 6..10 spans chunk 0 (x 6,7) and chunk 1 (x 8,9).
        let cmd = VoxelCommand::FillRegion {
            grid: GridId::new(0),
            min: VoxelCoord::new(6, 0, 0),
            max: VoxelCoord::new(10, 1, 1),
            value: VoxelValue::solid_raw(1),
        };
        let fill_events = validate(&cmd, &world, &materials()).unwrap();
        apply_all(&mut world, &fill_events).unwrap();
        assert_eq!(
            world
                .get(ChunkCoord::new(0, 0, 0))
                .unwrap()
                .get(LocalVoxelCoord::new(7, 0, 0)),
            Some(VoxelValue::solid_raw(1))
        );
        assert_eq!(
            world
                .get(ChunkCoord::new(1, 0, 0))
                .unwrap()
                .get(LocalVoxelCoord::new(0, 0, 0)),
            Some(VoxelValue::solid_raw(1))
        );
    }

    #[test]
    fn bulk_preview_reports_projected_hash_without_mutating() {
        let mut world = resident_world();
        let before = world.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash();
        let commands = vec![
            VoxelCommand::SetVoxel {
                grid: GridId::new(0),
                coord: VoxelCoord::new(1, 1, 1),
                value: VoxelValue::solid_raw(1),
            },
            VoxelCommand::FillRegion {
                grid: GridId::new(0),
                min: VoxelCoord::new(2, 0, 0),
                max: VoxelCoord::new(4, 1, 1),
                value: VoxelValue::solid_raw(2),
            },
        ];

        let receipt = execute_transaction(
            &mut world,
            &materials(),
            &VoxelEditTransaction::preview(&commands),
        );

        assert!(!receipt.applied);
        assert_eq!(receipt.accepted, 2);
        assert_eq!(receipt.rejected, 0);
        assert_eq!(receipt.event_count, 2);
        assert_eq!(receipt.touched_voxels, 3);
        assert_eq!(receipt.after_hash, receipt.before_hash);
        assert_ne!(receipt.projected_hash, receipt.before_hash);
        assert_eq!(
            world.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash(),
            before
        );
        assert_eq!(receipt.transaction_hash, receipt.transaction_hash);
    }

    #[test]
    fn bulk_apply_is_atomic_and_supports_generate_then_edit() {
        let mut world = VoxelWorld::new(spec());
        let chunk = ChunkCoord::new(0, 0, 0);
        let commands = vec![
            VoxelCommand::GenerateChunk {
                grid: GridId::new(0),
                chunk,
                seed: 77,
                generator_version: 1,
            },
            VoxelCommand::SetVoxel {
                grid: GridId::new(0),
                coord: VoxelCoord::new(0, 0, 0),
                value: VoxelValue::solid_raw(2),
            },
        ];

        let receipt = execute_transaction(
            &mut world,
            &materials(),
            &VoxelEditTransaction::apply(&commands),
        );

        assert!(receipt.applied);
        assert_eq!(receipt.accepted, 2);
        assert_eq!(receipt.rejected, 0);
        assert_eq!(receipt.projected_hash, receipt.after_hash);
        assert_ne!(receipt.before_hash, receipt.after_hash);
        assert_eq!(
            world.get(chunk).unwrap().get(LocalVoxelCoord::new(0, 0, 0)),
            Some(VoxelValue::solid_raw(2))
        );
    }

    #[test]
    fn bulk_transaction_rejects_material_and_leaves_world_unchanged() {
        let mut world = resident_world();
        let before = world.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash();
        let commands = vec![
            VoxelCommand::SetVoxel {
                grid: GridId::new(0),
                coord: VoxelCoord::new(1, 1, 1),
                value: VoxelValue::solid_raw(1),
            },
            VoxelCommand::SetVoxel {
                grid: GridId::new(0),
                coord: VoxelCoord::new(2, 2, 2),
                value: VoxelValue::solid_raw(9),
            },
        ];

        let receipt = execute_transaction(
            &mut world,
            &materials(),
            &VoxelEditTransaction::apply(&commands),
        );

        assert!(!receipt.applied);
        assert_eq!(receipt.accepted, 1);
        assert_eq!(receipt.rejected, 1);
        assert!(matches!(
            receipt.rejections.as_slice(),
            [VoxelEditTransactionRejection::InvalidCommand {
                index: 1,
                rejection: VoxelEditRejection::UnknownMaterial(id)
            }] if *id == VoxelMaterialId::new(9)
        ));
        assert_eq!(receipt.after_hash, receipt.before_hash);
        assert_eq!(
            world.get(ChunkCoord::new(0, 0, 0)).unwrap().content_hash(),
            before
        );
    }

    #[test]
    fn bulk_transaction_rejects_bounds_and_quotas() {
        let mut world = resident_world();
        let empty_region = vec![VoxelCommand::FillRegion {
            grid: GridId::new(0),
            min: VoxelCoord::new(2, 2, 2),
            max: VoxelCoord::new(2, 3, 3),
            value: VoxelValue::solid_raw(1),
        }];

        let empty_receipt = execute_transaction(
            &mut world,
            &materials(),
            &VoxelEditTransaction::apply(&empty_region),
        );
        assert!(matches!(
            empty_receipt.rejections.as_slice(),
            [VoxelEditTransactionRejection::InvalidCommand {
                rejection: VoxelEditRejection::EmptyRegion { .. },
                ..
            }]
        ));

        let too_many_commands = vec![
            VoxelCommand::SetVoxel {
                grid: GridId::new(0),
                coord: VoxelCoord::new(0, 0, 0),
                value: VoxelValue::solid_raw(1),
            },
            VoxelCommand::SetVoxel {
                grid: GridId::new(0),
                coord: VoxelCoord::new(1, 0, 0),
                value: VoxelValue::solid_raw(1),
            },
        ];
        let command_quota = execute_transaction(
            &mut world,
            &materials(),
            &VoxelEditTransaction::apply(&too_many_commands)
                .with_limits(VoxelEditTransactionLimits::new(1, 10, 10)),
        );
        assert!(matches!(
            command_quota.rejections.as_slice(),
            [VoxelEditTransactionRejection::CommandQuotaExceeded {
                limit: 1,
                actual: 2
            }]
        ));

        let too_many_voxels = vec![VoxelCommand::FillRegion {
            grid: GridId::new(0),
            min: VoxelCoord::new(0, 0, 0),
            max: VoxelCoord::new(4, 4, 4),
            value: VoxelValue::solid_raw(1),
        }];
        let voxel_quota = execute_transaction(
            &mut world,
            &materials(),
            &VoxelEditTransaction::apply(&too_many_voxels)
                .with_limits(VoxelEditTransactionLimits::new(10, 10, 10)),
        );
        assert!(matches!(
            voxel_quota.rejections.as_slice(),
            [VoxelEditTransactionRejection::TouchedVoxelQuotaExceeded {
                limit: 10,
                actual: 64
            }]
        ));
        assert!(!voxel_quota.applied);
    }

    #[test]
    fn bulk_transaction_events_persist_and_replay() {
        let mut world = VoxelWorld::new(spec());
        let chunk = ChunkCoord::new(0, 0, 0);
        let commands = vec![
            VoxelCommand::GenerateChunk {
                grid: GridId::new(0),
                chunk,
                seed: 11,
                generator_version: 1,
            },
            VoxelCommand::FillRegion {
                grid: GridId::new(0),
                min: VoxelCoord::new(0, 0, 0),
                max: VoxelCoord::new(3, 2, 1),
                value: VoxelValue::solid_raw(2),
            },
        ];

        let receipt = execute_transaction(
            &mut world,
            &materials(),
            &VoxelEditTransaction::apply(&commands),
        );
        assert!(receipt.applied);

        let log = persist::encode_edit_log(&receipt.events);
        let decoded = persist::decode_edit_log(&log).unwrap();
        let replayed = persist::replay_edit_log(spec(), &decoded).unwrap();
        assert_eq!(
            replayed.get(chunk).unwrap().content_hash(),
            world.get(chunk).unwrap().content_hash()
        );
    }

    #[test]
    fn replay_reconstructs_chunk_and_detects_generation_divergence() {
        // A canonical edit sequence (the replay fixture, mirrored in
        // harness/fixtures/voxel-edits/basic-edits.md).
        let g = GridId::new(0);
        let chunk = ChunkCoord::new(0, 0, 0);
        let mut world = VoxelWorld::new(spec());
        let gen = generate_chunk(&spec(), chunk, 100, 1);
        let events = vec![
            VoxelEditEvent::ChunkGenerated {
                grid: g,
                chunk,
                seed: 100,
                generator_version: 1,
                hash: gen.content_hash().0,
            },
            VoxelEditEvent::VoxelSet {
                grid: g,
                coord: VoxelCoord::new(0, 7, 0),
                value: VoxelValue::solid_raw(2),
            },
            VoxelEditEvent::VoxelRegionFilled {
                grid: g,
                min: VoxelCoord::new(1, 0, 1),
                max: VoxelCoord::new(3, 2, 3),
                value: VoxelValue::solid_raw(1),
            },
        ];
        apply_all(&mut world, &events).unwrap();
        let reconstructed = world.get(chunk).unwrap().content_hash();

        // Replaying the same events on a fresh world reproduces the chunk exactly.
        let mut world2 = VoxelWorld::new(spec());
        apply_all(&mut world2, &events).unwrap();
        assert_eq!(world2.get(chunk).unwrap().content_hash(), reconstructed);

        // A generator drift (wrong recorded hash) is surfaced, not silently reconstructed.
        let mut world3 = VoxelWorld::new(spec());
        let drifted = vec![VoxelEditEvent::ChunkGenerated {
            grid: g,
            chunk,
            seed: 100,
            generator_version: 1,
            hash: 0xdead_beef,
        }];
        assert!(matches!(
            apply_all(&mut world3, &drifted),
            Err(VoxelEditRejection::GenerationDivergence { .. })
        ));
    }
}
