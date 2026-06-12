//! Voxel persistence: chunk snapshots, edit-log replay, and generator-version
//! migration (voxel-capability-12).
//!
//! Std-only, hand-written text encoders/decoders (the Phase-4 posture — no serde),
//! building on the canonical edit events and the deterministic generation hook.
//!
//! # Model
//!
//! - Generated terrain is reproducible from `seed + generator_version` (a
//!   `ChunkGenerated` event). User edits are durable deltas (`VoxelSet`/
//!   `VoxelRegionFilled`). A world save is an **edit log**; replaying it
//!   reconstructs the world. **Chunk snapshots** are an optional compaction that
//!   captures a chunk's voxels directly (skipping edit replay).
//! - **Generator version migration** is a first-class concern, not just a
//!   diagnostic: when the generator changes, a recorded `ChunkGenerated` hash no
//!   longer matches regeneration ([`crate::VoxelEditRejection::GenerationDivergence`]),
//!   and [`generator_migration_report`] prescribes the recovery options.

use core_events::VoxelEditEvent;
use core_space::{ChunkCoord, ChunkDims, GridId, VoxelCoord, VoxelGridSpec};
use core_voxel::VoxelValue;
use svc_spatial::VoxelWorld;
use svc_volume::VoxelChunk;

use crate::{apply_all, VoxelEditRejection};

/// The text format version for snapshots and edit logs. Bumped on a layout change.
pub const PERSIST_FORMAT_VERSION: u32 = 1;

/// A malformed persistence artifact, classified for agent-legible diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    /// A required header line was missing or malformed.
    BadHeader { line: usize, detail: String },
    /// A token could not be parsed.
    BadToken { line: usize, detail: String },
    /// A line's leading keyword was not recognized.
    UnexpectedLine { line: usize, content: String },
    /// The decoded cell count did not match the chunk dimensions.
    LengthMismatch { expected: u64, actual: u64 },
    /// An encoded voxel value bit pattern was not recognized.
    UnknownValue { line: usize, bits: u32 },
}

impl core::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SnapshotError::BadHeader { line, detail } => {
                write!(f, "line {line}: bad header: {detail}")
            }
            SnapshotError::BadToken { line, detail } => {
                write!(f, "line {line}: bad token: {detail}")
            }
            SnapshotError::UnexpectedLine { line, content } => {
                write!(f, "line {line}: unexpected line {content:?}")
            }
            SnapshotError::LengthMismatch { expected, actual } => {
                write!(f, "decoded {actual} cells, expected {expected}")
            }
            SnapshotError::UnknownValue { line, bits } => {
                write!(f, "line {line}: unknown encoded voxel value {bits:#010x}")
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

// ── small parse helpers ────────────────────────────────────────────────────────

fn parse<T: std::str::FromStr>(
    tok: Option<&str>,
    line: usize,
    what: &str,
) -> Result<T, SnapshotError> {
    tok.and_then(|t| t.parse().ok())
        .ok_or_else(|| SnapshotError::BadToken {
            line,
            detail: format!("expected {what}"),
        })
}

fn value_from_bits(bits: u32, line: usize) -> Result<VoxelValue, SnapshotError> {
    VoxelValue::from_encoded(bits).ok_or(SnapshotError::UnknownValue { line, bits })
}

// ── chunk snapshot ─────────────────────────────────────────────────────────────

/// Encode a chunk as a run-length-compressed text snapshot (compact for the common
/// large-mostly-empty case). Reconstruction preserves the chunk's `content_hash`.
pub fn encode_chunk_snapshot(chunk: &VoxelChunk) -> String {
    use core::fmt::Write;
    let dims = chunk.dims();
    let mut s = String::new();
    let _ = writeln!(s, "voxelchunk {PERSIST_FORMAT_VERSION}");
    let _ = writeln!(s, "grid {}", chunk.grid_id().raw());
    let _ = writeln!(s, "dims {} {} {}", dims.x(), dims.y(), dims.z());
    // RLE over storage order (X-fastest, the same order `iter` yields).
    let mut run_value: Option<VoxelValue> = None;
    let mut run_len: u32 = 0;
    let flush = |s: &mut String, v: VoxelValue, n: u32| {
        let _ = writeln!(s, "rle {n} {}", v.to_encoded());
    };
    for (_, v) in chunk.iter() {
        match run_value {
            Some(rv) if rv == v => run_len += 1,
            Some(rv) => {
                flush(&mut s, rv, run_len);
                run_value = Some(v);
                run_len = 1;
            }
            None => {
                run_value = Some(v);
                run_len = 1;
            }
        }
    }
    if let Some(rv) = run_value {
        flush(&mut s, rv, run_len);
    }
    s
}

/// Decode a chunk snapshot. The reconstructed chunk hashes identically to the one
/// that was encoded.
pub fn decode_chunk_snapshot(text: &str) -> Result<VoxelChunk, SnapshotError> {
    let mut lines = text.lines().enumerate();

    let (ln, header) = lines.next().ok_or(SnapshotError::BadHeader {
        line: 0,
        detail: "empty".into(),
    })?;
    let mut h = header.split_whitespace();
    if h.next() != Some("voxelchunk") {
        return Err(SnapshotError::BadHeader {
            line: ln + 1,
            detail: "expected `voxelchunk`".into(),
        });
    }
    let _version: u32 = parse(h.next(), ln + 1, "format version")?;

    let (gl, grid_line) = lines.next().ok_or(SnapshotError::BadHeader {
        line: ln + 2,
        detail: "missing grid".into(),
    })?;
    let mut g = grid_line.split_whitespace();
    if g.next() != Some("grid") {
        return Err(SnapshotError::BadHeader {
            line: gl + 1,
            detail: "expected `grid`".into(),
        });
    }
    let grid = GridId::new(parse(g.next(), gl + 1, "grid id")?);

    let (dl, dims_line) = lines.next().ok_or(SnapshotError::BadHeader {
        line: gl + 2,
        detail: "missing dims".into(),
    })?;
    let mut d = dims_line.split_whitespace();
    if d.next() != Some("dims") {
        return Err(SnapshotError::BadHeader {
            line: dl + 1,
            detail: "expected `dims`".into(),
        });
    }
    let dx: u32 = parse(d.next(), dl + 1, "dim x")?;
    let dy: u32 = parse(d.next(), dl + 1, "dim y")?;
    let dz: u32 = parse(d.next(), dl + 1, "dim z")?;
    let dims = ChunkDims::new(dx, dy, dz).ok_or(SnapshotError::BadToken {
        line: dl + 1,
        detail: "chunk dims must be >= 1".into(),
    })?;

    let mut values: Vec<VoxelValue> = Vec::with_capacity(dims.volume() as usize);
    for (ln, line) in lines {
        if line.trim().is_empty() {
            continue;
        }
        let mut t = line.split_whitespace();
        match t.next() {
            Some("rle") => {
                let count: u32 = parse(t.next(), ln + 1, "run length")?;
                let bits: u32 = parse(t.next(), ln + 1, "encoded value")?;
                let value = value_from_bits(bits, ln + 1)?;
                for _ in 0..count {
                    values.push(value);
                }
            }
            other => {
                return Err(SnapshotError::UnexpectedLine {
                    line: ln + 1,
                    content: other.unwrap_or("").into(),
                })
            }
        }
    }

    if values.len() as u64 != dims.volume() {
        return Err(SnapshotError::LengthMismatch {
            expected: dims.volume(),
            actual: values.len() as u64,
        });
    }
    VoxelChunk::from_values(grid, dims, &values).map_err(|_| SnapshotError::LengthMismatch {
        expected: dims.volume(),
        actual: values.len() as u64,
    })
}

// ── edit log ───────────────────────────────────────────────────────────────────

/// Encode an edit-event log (the durable record of a world's edits + generation).
/// Generator metadata (`seed`/`generator_version`) is carried by `gen` lines.
pub fn encode_edit_log(events: &[VoxelEditEvent]) -> String {
    use core::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "voxeledits {PERSIST_FORMAT_VERSION}");
    for e in events {
        match *e {
            VoxelEditEvent::VoxelSet { grid, coord, value } => {
                let _ = writeln!(
                    s,
                    "set {} {} {} {} {}",
                    grid.raw(),
                    coord.x,
                    coord.y,
                    coord.z,
                    value.to_encoded()
                );
            }
            VoxelEditEvent::VoxelRegionFilled {
                grid,
                min,
                max,
                value,
            } => {
                let _ = writeln!(
                    s,
                    "fill {} {} {} {} {} {} {} {}",
                    grid.raw(),
                    min.x,
                    min.y,
                    min.z,
                    max.x,
                    max.y,
                    max.z,
                    value.to_encoded()
                );
            }
            VoxelEditEvent::ChunkGenerated {
                grid,
                chunk,
                seed,
                generator_version,
                hash,
            } => {
                let _ = writeln!(
                    s,
                    "gen {} {} {} {} {} {} {}",
                    grid.raw(),
                    chunk.x,
                    chunk.y,
                    chunk.z,
                    seed,
                    generator_version,
                    hash
                );
            }
        }
    }
    s
}

/// Decode an edit-event log produced by [`encode_edit_log`].
pub fn decode_edit_log(text: &str) -> Result<Vec<VoxelEditEvent>, SnapshotError> {
    let mut events = Vec::new();
    let mut lines = text.lines().enumerate();

    let (ln, header) = lines.next().ok_or(SnapshotError::BadHeader {
        line: 0,
        detail: "empty".into(),
    })?;
    let mut h = header.split_whitespace();
    if h.next() != Some("voxeledits") {
        return Err(SnapshotError::BadHeader {
            line: ln + 1,
            detail: "expected `voxeledits`".into(),
        });
    }
    let _version: u32 = parse(h.next(), ln + 1, "format version")?;

    for (ln, line) in lines {
        if line.trim().is_empty() {
            continue;
        }
        let mut t = line.split_whitespace();
        let kw = t.next();
        let l = ln + 1;
        match kw {
            Some("set") => {
                let grid = GridId::new(parse(t.next(), l, "grid")?);
                let coord = VoxelCoord::new(
                    parse(t.next(), l, "x")?,
                    parse(t.next(), l, "y")?,
                    parse(t.next(), l, "z")?,
                );
                let value = value_from_bits(parse(t.next(), l, "value")?, l)?;
                events.push(VoxelEditEvent::VoxelSet { grid, coord, value });
            }
            Some("fill") => {
                let grid = GridId::new(parse(t.next(), l, "grid")?);
                let min = VoxelCoord::new(
                    parse(t.next(), l, "minx")?,
                    parse(t.next(), l, "miny")?,
                    parse(t.next(), l, "minz")?,
                );
                let max = VoxelCoord::new(
                    parse(t.next(), l, "maxx")?,
                    parse(t.next(), l, "maxy")?,
                    parse(t.next(), l, "maxz")?,
                );
                let value = value_from_bits(parse(t.next(), l, "value")?, l)?;
                events.push(VoxelEditEvent::VoxelRegionFilled {
                    grid,
                    min,
                    max,
                    value,
                });
            }
            Some("gen") => {
                let grid = GridId::new(parse(t.next(), l, "grid")?);
                let chunk = ChunkCoord::new(
                    parse(t.next(), l, "cx")?,
                    parse(t.next(), l, "cy")?,
                    parse(t.next(), l, "cz")?,
                );
                let seed: u64 = parse(t.next(), l, "seed")?;
                let generator_version: u32 = parse(t.next(), l, "generator version")?;
                let hash: u64 = parse(t.next(), l, "hash")?;
                events.push(VoxelEditEvent::ChunkGenerated {
                    grid,
                    chunk,
                    seed,
                    generator_version,
                    hash,
                });
            }
            other => {
                return Err(SnapshotError::UnexpectedLine {
                    line: l,
                    content: other.unwrap_or("").into(),
                })
            }
        }
    }
    Ok(events)
}

/// Reconstruct a world by replaying an edit log onto a fresh [`VoxelWorld`] for
/// `spec`. A `ChunkGenerated` event whose recorded hash no longer matches
/// regeneration surfaces [`VoxelEditRejection::GenerationDivergence`] — the runtime
/// generator-version-mismatch detector (pair with [`generator_migration_report`]).
pub fn replay_edit_log(
    spec: VoxelGridSpec,
    events: &[VoxelEditEvent],
) -> Result<VoxelWorld, VoxelEditRejection> {
    let mut world = VoxelWorld::new(spec);
    apply_all(&mut world, events)?;
    Ok(world)
}

// ── generator version migration ────────────────────────────────────────────────

/// A strategy for surviving a terrain generator version change (doc 12 §"Generator
/// version strategies").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationStrategy {
    /// Keep the world on the generator version it was created with.
    Pin,
    /// Regenerate base terrain from seed at the new version, then reapply edit deltas.
    RegenerateAndReplay,
    /// Snapshot generated terrain at the version boundary; it becomes the new base.
    SnapshotAtBoundary,
    /// Treat the version bump as a deliberate fixture/golden update.
    FixtureVersioning,
}

/// A prescribed-migration report for a generator version mismatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub saved_version: u32,
    pub current_version: u32,
    /// The strategies an operator/agent may choose, in recommended-first order.
    pub strategies: Vec<MigrationStrategy>,
}

impl MigrationReport {
    /// The default strategy for development saves.
    pub fn recommended(&self) -> MigrationStrategy {
        self.strategies[0]
    }
}

/// Compare a saved generator version against the current one. Returns `None` when
/// they match (no migration needed), else a [`MigrationReport`] listing the
/// prescribed options (regenerate+replay recommended for dev saves).
pub fn generator_migration_report(
    saved_version: u32,
    current_version: u32,
) -> Option<MigrationReport> {
    if saved_version == current_version {
        return None;
    }
    Some(MigrationReport {
        saved_version,
        current_version,
        strategies: vec![
            MigrationStrategy::RegenerateAndReplay,
            MigrationStrategy::SnapshotAtBoundary,
            MigrationStrategy::Pin,
            MigrationStrategy::FixtureVersioning,
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate_chunk;
    use core_space::LocalVoxelCoord;

    fn spec() -> VoxelGridSpec {
        VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap()
    }

    #[test]
    fn chunk_snapshot_round_trips_and_preserves_hash() {
        let mut chunk = VoxelChunk::from_spec(&spec());
        chunk
            .set(LocalVoxelCoord::new(0, 0, 0), VoxelValue::solid_raw(1))
            .unwrap();
        chunk
            .set(LocalVoxelCoord::new(1, 0, 0), VoxelValue::solid_raw(1))
            .unwrap();
        chunk
            .set(LocalVoxelCoord::new(3, 3, 3), VoxelValue::solid_raw(2))
            .unwrap();

        let text = encode_chunk_snapshot(&chunk);
        let decoded = decode_chunk_snapshot(&text).unwrap();
        assert_eq!(decoded.content_hash(), chunk.content_hash());
        assert_eq!(decoded.dims().to_array(), [4, 4, 4]);
        assert_eq!(
            decoded.get(LocalVoxelCoord::new(3, 3, 3)),
            Some(VoxelValue::solid_raw(2))
        );
    }

    #[test]
    fn corrupt_snapshot_is_classified() {
        assert!(matches!(
            decode_chunk_snapshot("nope 1\n"),
            Err(SnapshotError::BadHeader { .. })
        ));
        let truncated = "voxelchunk 1\ngrid 0\ndims 4 4 4\nrle 1 0\n"; // only 1 cell, need 64
        assert!(matches!(
            decode_chunk_snapshot(truncated),
            Err(SnapshotError::LengthMismatch { .. })
        ));
        let bad_val = "voxelchunk 1\ngrid 0\ndims 1 1 1\nrle 1 5\n"; // 5 is not a valid encoding
        assert!(matches!(
            decode_chunk_snapshot(bad_val),
            Err(SnapshotError::UnknownValue { .. })
        ));
    }

    #[test]
    fn edit_log_round_trips() {
        let g = GridId::new(0);
        let events = vec![
            VoxelEditEvent::ChunkGenerated {
                grid: g,
                chunk: ChunkCoord::new(0, 0, 0),
                seed: 7,
                generator_version: 1,
                hash: 0xabcd,
            },
            VoxelEditEvent::VoxelSet {
                grid: g,
                coord: VoxelCoord::new(1, 2, 3),
                value: VoxelValue::solid_raw(2),
            },
            VoxelEditEvent::VoxelRegionFilled {
                grid: g,
                min: VoxelCoord::new(0, 0, 0),
                max: VoxelCoord::new(2, 2, 2),
                value: VoxelValue::EMPTY,
            },
        ];
        let text = encode_edit_log(&events);
        assert_eq!(decode_edit_log(&text).unwrap(), events);
    }

    #[test]
    fn replay_edit_log_reconstructs_the_chunk() {
        let g = GridId::new(0);
        let chunk = ChunkCoord::new(0, 0, 0);
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
                coord: VoxelCoord::new(0, 0, 0),
                value: VoxelValue::solid_raw(2),
            },
        ];
        // Encode → decode → replay reproduces the chunk exactly.
        let log = encode_edit_log(&events);
        let decoded = decode_edit_log(&log).unwrap();
        let world = replay_edit_log(spec(), &decoded).unwrap();
        let direct = replay_edit_log(spec(), &events).unwrap();
        assert_eq!(
            world.get(chunk).unwrap().content_hash(),
            direct.get(chunk).unwrap().content_hash(),
        );
    }

    #[test]
    fn generator_version_mismatch_reports_migration_choices() {
        assert_eq!(generator_migration_report(1, 1), None);
        let report = generator_migration_report(1, 2).unwrap();
        assert_eq!(report.saved_version, 1);
        assert_eq!(report.current_version, 2);
        assert_eq!(report.recommended(), MigrationStrategy::RegenerateAndReplay);
        assert!(report
            .strategies
            .contains(&MigrationStrategy::SnapshotAtBoundary));

        // The runtime detector: a stale recorded generation hash (old generator)
        // surfaces a GenerationDivergence on replay.
        let g = GridId::new(0);
        let chunk = ChunkCoord::new(0, 0, 0);
        let stale = vec![VoxelEditEvent::ChunkGenerated {
            grid: g,
            chunk,
            seed: 100,
            generator_version: 1,
            hash: 0xdead_beef,
        }];
        assert!(matches!(
            replay_edit_log(spec(), &stale),
            Err(VoxelEditRejection::GenerationDivergence { .. })
        ));
    }

    #[test]
    fn snapshot_and_log_match_committed_goldens() {
        let mut chunk = VoxelChunk::from_spec(&spec());
        chunk
            .fill_region(
                LocalVoxelCoord::new(0, 0, 0),
                LocalVoxelCoord::new(2, 1, 1),
                VoxelValue::solid_raw(1),
            )
            .unwrap();
        assert_eq!(
            encode_chunk_snapshot(&chunk),
            include_str!("../../../../../harness/fixtures/voxel-persist/sample-chunk.snapshot.txt"),
        );

        let g = GridId::new(0);
        let events = vec![
            VoxelEditEvent::ChunkGenerated {
                grid: g,
                chunk: ChunkCoord::new(0, 0, 0),
                seed: 42,
                generator_version: 1,
                hash: 12345,
            },
            VoxelEditEvent::VoxelSet {
                grid: g,
                coord: VoxelCoord::new(1, 0, 0),
                value: VoxelValue::solid_raw(1),
            },
        ];
        assert_eq!(
            encode_edit_log(&events),
            include_str!("../../../../../harness/fixtures/voxel-persist/sample-edits.log.txt"),
        );
    }
}
