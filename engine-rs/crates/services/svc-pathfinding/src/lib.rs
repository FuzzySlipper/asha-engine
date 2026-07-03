//! Deterministic navigation/pathfinding projection over voxel authority.
//!
//! # Lane
//!
//! `rust-service` — builds read-only navigation projections from authoritative
//! voxel worlds and answers deterministic path queries. It does not own AI,
//! policy mutation, movement application, render state, or demo behavior.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use core_space::{VoxelCoord, VoxelGridSpec};
use svc_spatial::VoxelWorld;

/// Configuration for deriving walkable nav cells from voxel authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavProjectionConfig {
    /// Number of empty vertical cells required for an agent to stand.
    pub agent_height_voxels: u32,
    /// Whether the cell immediately below the agent must be solid.
    pub require_solid_floor: bool,
}

impl Default for NavProjectionConfig {
    fn default() -> Self {
        Self {
            agent_height_voxels: 2,
            require_solid_floor: true,
        }
    }
}

/// Read-only navigation projection suitable for policy/devtools inspection.
#[derive(Debug, Clone, PartialEq)]
pub struct NavProjection {
    grid: VoxelGridSpec,
    walkable: BTreeSet<VoxelCoord>,
    projection_hash: u64,
}

impl NavProjection {
    pub fn grid(&self) -> VoxelGridSpec {
        self.grid
    }

    pub fn walkable_len(&self) -> usize {
        self.walkable.len()
    }

    pub fn projection_hash(&self) -> u64 {
        self.projection_hash
    }

    pub fn is_walkable(&self, coord: VoxelCoord) -> bool {
        self.walkable.contains(&coord)
    }

    #[cfg(test)]
    fn without_walkable(mut self, coord: VoxelCoord) -> Self {
        self.walkable.remove(&coord);
        self.projection_hash = hash_walkable(&self.walkable);
        self
    }

    pub fn walkable_cells(&self) -> impl Iterator<Item = VoxelCoord> + '_ {
        self.walkable.iter().copied()
    }
}

/// Path query over an existing nav projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavPathQuery {
    pub start: VoxelCoord,
    pub goal: VoxelCoord,
    pub max_visited: usize,
}

/// Deterministic path readout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavPathReadout {
    pub outcome: NavPathOutcome,
    pub visited: usize,
    pub path: Vec<VoxelCoord>,
    pub path_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavPathOutcome {
    Reached,
    NoPath,
}

/// Why nav projection/query failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavError {
    InvalidAgentHeight,
    InvalidQueryBudget,
    StartNotWalkable { start: VoxelCoord },
    GoalNotWalkable { goal: VoxelCoord },
}

impl NavError {
    pub const fn label(self) -> &'static str {
        match self {
            NavError::InvalidAgentHeight => "invalidAgentHeight",
            NavError::InvalidQueryBudget => "invalidQueryBudget",
            NavError::StartNotWalkable { .. } => "startNotWalkable",
            NavError::GoalNotWalkable { .. } => "goalNotWalkable",
        }
    }
}

/// Build a read-only nav projection from authoritative voxel data.
pub fn build_nav_projection(
    world: &VoxelWorld,
    config: NavProjectionConfig,
) -> Result<NavProjection, NavError> {
    if config.agent_height_voxels == 0 {
        return Err(NavError::InvalidAgentHeight);
    }
    let grid = world.grid();
    let mut walkable = BTreeSet::new();
    for (chunk_coord, chunk) in world.resident_chunks() {
        for (local, value) in chunk.iter() {
            if value.is_solid() {
                continue;
            }
            let coord = grid.chunk_local_to_voxel(chunk_coord, local);
            if is_walkable_cell(world, coord, config) {
                walkable.insert(coord);
            }
        }
    }
    let projection_hash = hash_walkable(&walkable);
    Ok(NavProjection {
        grid,
        walkable,
        projection_hash,
    })
}

fn is_walkable_cell(world: &VoxelWorld, coord: VoxelCoord, config: NavProjectionConfig) -> bool {
    if config.require_solid_floor
        && !voxel_is_solid(world, VoxelCoord::new(coord.x, coord.y - 1, coord.z))
    {
        return false;
    }
    for dy in 0..config.agent_height_voxels {
        let check = VoxelCoord::new(coord.x, coord.y + dy as i64, coord.z);
        if voxel_is_solid(world, check) {
            return false;
        }
    }
    true
}

fn voxel_is_solid(world: &VoxelWorld, coord: VoxelCoord) -> bool {
    let grid = world.grid();
    let (chunk, local) = grid.voxel_to_chunk_local(coord);
    world
        .get(chunk)
        .and_then(|data| data.get(local))
        .is_some_and(|value| value.is_solid())
}

/// Query a deterministic shortest path through the nav projection.
pub fn find_path(
    projection: &NavProjection,
    query: NavPathQuery,
) -> Result<NavPathReadout, NavError> {
    if query.max_visited == 0 {
        return Err(NavError::InvalidQueryBudget);
    }
    if !projection.is_walkable(query.start) {
        return Err(NavError::StartNotWalkable { start: query.start });
    }
    if !projection.is_walkable(query.goal) {
        return Err(NavError::GoalNotWalkable { goal: query.goal });
    }
    if query.start == query.goal {
        let path = vec![query.start];
        return Ok(NavPathReadout {
            outcome: NavPathOutcome::Reached,
            visited: 1,
            path_hash: hash_path(&path),
            path,
        });
    }

    let mut queue = VecDeque::new();
    let mut visited = BTreeSet::new();
    let mut came_from = BTreeMap::new();
    queue.push_back(query.start);
    visited.insert(query.start);

    while let Some(current) = queue.pop_front() {
        if visited.len() > query.max_visited {
            break;
        }
        for next in nav_neighbors(current) {
            if !projection.is_walkable(next) || visited.contains(&next) {
                continue;
            }
            came_from.insert(next, current);
            if next == query.goal {
                let path = reconstruct_path(query.start, query.goal, &came_from);
                return Ok(NavPathReadout {
                    outcome: NavPathOutcome::Reached,
                    visited: visited.len() + 1,
                    path_hash: hash_path(&path),
                    path,
                });
            }
            visited.insert(next);
            queue.push_back(next);
        }
    }

    Ok(NavPathReadout {
        outcome: NavPathOutcome::NoPath,
        visited: visited.len(),
        path: Vec::new(),
        path_hash: hash_path(&[]),
    })
}

fn nav_neighbors(coord: VoxelCoord) -> [VoxelCoord; 4] {
    [
        VoxelCoord::new(coord.x + 1, coord.y, coord.z),
        VoxelCoord::new(coord.x, coord.y, coord.z + 1),
        VoxelCoord::new(coord.x - 1, coord.y, coord.z),
        VoxelCoord::new(coord.x, coord.y, coord.z - 1),
    ]
}

fn reconstruct_path(
    start: VoxelCoord,
    goal: VoxelCoord,
    came_from: &BTreeMap<VoxelCoord, VoxelCoord>,
) -> Vec<VoxelCoord> {
    let mut path = vec![goal];
    let mut current = goal;
    while current != start {
        current = came_from[&current];
        path.push(current);
    }
    path.reverse();
    path
}

/// Human-reviewable deterministic summary used by committed fixtures.
pub fn describe_nav_path(projection: &NavProjection, readout: &NavPathReadout) -> String {
    let mut out = String::new();
    out.push_str("nav-path 1\n");
    out.push_str(&format!("walkable={}\n", projection.walkable_len()));
    out.push_str(&format!(
        "projection_hash={:016x}\n",
        projection.projection_hash()
    ));
    out.push_str(&format!("outcome={:?}\n", readout.outcome));
    out.push_str(&format!("visited={}\n", readout.visited));
    out.push_str(&format!("path_len={}\n", readout.path.len()));
    out.push_str("path=");
    for (index, coord) in readout.path.iter().enumerate() {
        if index > 0 {
            out.push_str(" -> ");
        }
        out.push_str(&format!("{},{},{}", coord.x, coord.y, coord.z));
    }
    out.push('\n');
    out.push_str(&format!("path_hash={:016x}\n", readout.path_hash));
    out
}

fn hash_walkable(walkable: &BTreeSet<VoxelCoord>) -> u64 {
    let mut h = fnv_offset();
    feed_u64(&mut h, walkable.len() as u64);
    for coord in walkable {
        feed_coord(&mut h, *coord);
    }
    h
}

fn hash_path(path: &[VoxelCoord]) -> u64 {
    let mut h = fnv_offset();
    feed_u64(&mut h, path.len() as u64);
    for coord in path {
        feed_coord(&mut h, *coord);
    }
    h
}

fn feed_coord(h: &mut u64, coord: VoxelCoord) {
    for value in coord.to_array() {
        feed_i64(h, value);
    }
}

fn fnv_offset() -> u64 {
    0xcbf2_9ce4_8422_2325
}

fn feed_byte(h: &mut u64, b: u8) {
    *h ^= b as u64;
    *h = h.wrapping_mul(0x0000_0100_0000_01b3);
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

#[cfg(test)]
mod tests {
    use super::*;
    use svc_levelgen::{generate_tunnel, TunnelGeneratorConfig};

    fn projection() -> NavProjection {
        let tunnel = generate_tunnel(TunnelGeneratorConfig::tiny_enclosed(17)).expect("tunnel");
        build_nav_projection(&tunnel.world, NavProjectionConfig::default()).expect("nav")
    }

    #[test]
    fn generated_tunnel_has_reachable_player_path() {
        let projection = projection();
        let readout = find_path(
            &projection,
            NavPathQuery {
                start: VoxelCoord::new(3, 1, 7),
                goal: VoxelCoord::new(1, 1, 1),
                max_visited: 128,
            },
        )
        .expect("path");
        assert_eq!(readout.outcome, NavPathOutcome::Reached);
        assert_eq!(readout.path.first(), Some(&VoxelCoord::new(3, 1, 7)));
        assert_eq!(readout.path.last(), Some(&VoxelCoord::new(1, 1, 1)));
    }

    #[test]
    fn blocked_tunnel_reports_no_path() {
        let mut projection = projection();
        for x in 1..=3 {
            projection = projection.without_walkable(VoxelCoord::new(x, 1, 4));
        }
        let readout = find_path(
            &projection,
            NavPathQuery {
                start: VoxelCoord::new(3, 1, 7),
                goal: VoxelCoord::new(1, 1, 1),
                max_visited: 128,
            },
        )
        .expect("no path readout");
        assert_eq!(readout.outcome, NavPathOutcome::NoPath);
        assert!(readout.path.is_empty());
    }

    #[test]
    fn invalid_query_rejects_unwalkable_start() {
        let projection = projection();
        assert_eq!(
            find_path(
                &projection,
                NavPathQuery {
                    start: VoxelCoord::new(0, 1, 0),
                    goal: VoxelCoord::new(1, 1, 1),
                    max_visited: 128,
                },
            ),
            Err(NavError::StartNotWalkable {
                start: VoxelCoord::new(0, 1, 0)
            })
        );
    }

    #[test]
    fn path_readout_matches_committed_golden() {
        let projection = projection();
        let readout = find_path(
            &projection,
            NavPathQuery {
                start: VoxelCoord::new(3, 1, 7),
                goal: VoxelCoord::new(1, 1, 1),
                max_visited: 128,
            },
        )
        .expect("path");
        assert_eq!(
            describe_nav_path(&projection, &readout),
            include_str!("../../../../../harness/fixtures/nav/generated-tunnel-path.snapshot.txt")
        );
    }
}
