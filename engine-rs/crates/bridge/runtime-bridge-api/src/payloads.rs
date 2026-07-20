use crate::*;

// ── Prototype operation payloads ────────────────────────────────────────────--
//
// PROTOTYPE NOTE: these minimal structs stand in for generated `protocol_runtime`
// types (`EngineConfig`/`StepInputEnvelope`/`StepResult`). The codegen emitter
// (#2250 follow-up) replaces them with protocol-crate types; the *shape* of the
// trait below is the stable part.

/// Engine construction config. A deterministic seed is the only required input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    pub seed: u64,
}

/// Deterministic per-tick input envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepInputEnvelope {
    pub tick: u64,
}

/// Result of advancing one tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepResult {
    pub tick: u64,
    /// Number of accepted authority events applied across the fixed-tick batch.
    /// A future generated runtime descriptor may replace this bounded summary.
    pub diff_count: u32,
}

/// Bounded request to apply an enemy direct-nav movement transaction.
///
/// `seed_position` is used only when this bridge session has not yet seen the
/// entity. Once seeded, Rust authority reads the current transform from its
/// [`EntityStore`] and ignores any stale caller-side position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnemyDirectNavMovementRequest {
    pub entity: u64,
    pub seed_position: Vec3,
    pub target: Vec3,
    pub max_step_units: f32,
}

/// Where the movement transaction read the starting transform from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyDirectNavAuthoritySource {
    SeededFromRequest,
    RustEntityStore,
}

impl EnemyDirectNavAuthoritySource {
    pub fn label(self) -> &'static str {
        match self {
            EnemyDirectNavAuthoritySource::SeededFromRequest => "seeded_from_request",
            EnemyDirectNavAuthoritySource::RustEntityStore => "rust_entity_store",
        }
    }
}

/// Result of a Rust-owned enemy direct-nav movement application.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnemyDirectNavMovementResult {
    pub entity: u64,
    pub authority_source: EnemyDirectNavAuthoritySource,
    pub from: Vec3,
    pub target: Vec3,
    pub next_waypoint: Vec3,
    pub distance_units: f32,
    pub reached: bool,
    pub path_hash: u64,
    pub transform_hash: u64,
    pub projection_changed: bool,
}

/// Why an enemy direct-nav movement transaction was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyDirectNavMovementError {
    InvalidEntity,
    Navigation(DirectNavMovementError),
    Transform(TransformError),
}

impl EnemyDirectNavMovementError {
    pub fn label(self) -> &'static str {
        match self {
            EnemyDirectNavMovementError::InvalidEntity => "invalidEntity",
            EnemyDirectNavMovementError::Navigation(error) => error.label(),
            EnemyDirectNavMovementError::Transform(error) => error.label(),
        }
    }
}

pub use protocol_project_bundle::{
    ActiveRuntimeProjectIdentity, ProjectArtifactRelocation, ProjectResourceBeginRequest,
    ProjectResourceStageRequest, ProjectResourceTransactionReceipt, ProjectSourceBatchDiagnostic,
    ProjectSourceBatchErrorCode, ProjectSourceBatchValidationReceipt, ProjectSourceBody,
    ProjectStoreIdentity, ProjectWriteCandidate, ProjectWriteConfirmReceipt,
    ProjectWriteConfirmRequest, ProjectWriteDiagnostic, ProjectWritePrepareReceipt,
    ProjectWritePrepareRequest, ProjectWritePublication, RuntimeProjectCloseReceipt,
    RuntimeProjectCloseRequest, RuntimeProjectDiagnostic, RuntimeProjectDiagnosticPhase,
    RuntimeProjectLoadReceipt, RuntimeProjectLoadRequest, RuntimeProjectSourceAdapterInput,
    RuntimeProjectSourceAdapterKind, RuntimeProjectSourceBatch, StagedProjectResourceRef,
    WorkspaceAuthoringCloseReceipt, WorkspaceAuthoringCloseRequest,
    WorkspaceAuthoringCompositionStatus, WorkspaceAuthoringIdentity, WorkspaceAuthoringOpenRequest,
    WorkspaceAuthoringProjectBundleRef, WorkspaceAuthoringProjectIdentity,
    WorkspaceAuthoringProjectionReceipt, WorkspaceAuthoringProjectionRequest,
    WorkspaceAuthoringStateSummary, WorkspaceAuthoringStoredConfirmationReceipt,
    WorkspaceAuthoringStoredConfirmationRequest,
};

// ── Command submission payloads (launchable-voxel, #2436) ─────────────────────
//
// The launch/edit loop submits **generated** voxel commands (the authority-owned
// `core_commands::VoxelCommand`, mirrored into the TS `voxel.ts` contract) for
// Rust-side validation + apply via `rule-voxel-edit`. No placeholder `{ kind }`
// command tunnel: the batch carries the real typed command union.

/// A batch of proposed voxel commands awaiting Rust-side validation + apply.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandBatch {
    pub commands: Vec<VoxelCommand>,
}

/// Native JSON guardrails. These are deliberately roomy for desktop authoring,
/// but finite so compact commands cannot amplify into unbounded parser or edit work.
pub const VOXEL_COMMAND_BATCH_MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;
pub const VOXEL_COMMAND_BATCH_MAX_COMMANDS: usize =
    rule_voxel_edit::DEFAULT_VOXEL_EDIT_MAX_COMMANDS as usize;
pub const VOXEL_COMMAND_BATCH_MAX_TOUCHED_VOXELS: u64 =
    rule_voxel_edit::DEFAULT_VOXEL_EDIT_MAX_TOUCHED_VOXELS;

/// The classified outcome of a [`RuntimeBridge::submit_commands`] batch: how many
/// commands authority committed plus the classified rejection for each invalid
/// command (never a silent drop). The batch is atomic: if any command is invalid,
/// `accepted` is zero and otherwise-valid peer commands are withheld without
/// mutation. In that case `accepted + rejected` need not equal the batch length.
/// Accepted commands have already mutated authority voxel state and marked their
/// chunks dirty.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandResult {
    pub accepted: u32,
    pub rejected: u32,
    /// One classified rejection per refused command, in submission order.
    pub rejections: Vec<VoxelEditRejection>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "op", deny_unknown_fields)]
enum VoxelCommandJson {
    SetVoxel {
        grid: u32,
        coord: VoxelCoordJson,
        value: VoxelValueJson,
    },
    FillRegion {
        grid: u32,
        min: VoxelCoordJson,
        max: VoxelCoordJson,
        value: VoxelValueJson,
    },
    GenerateChunk {
        grid: u32,
        chunk: ChunkCoordJson,
        seed: u64,
        #[serde(rename = "generatorVersion")]
        generator_version: u32,
    },
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct VoxelCoordJson {
    x: i64,
    y: i64,
    z: i64,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ChunkCoordJson {
    x: i64,
    y: i64,
    z: i64,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", deny_unknown_fields)]
enum VoxelValueJson {
    Empty,
    Solid { material: u16 },
}

struct BoundedVoxelCommandBatchJson(Vec<VoxelCommandJson>);

impl<'de> serde::Deserialize<'de> for BoundedVoxelCommandBatchJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BatchVisitor;

        impl<'de> serde::de::Visitor<'de> for BatchVisitor {
            type Value = BoundedVoxelCommandBatchJson;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(formatter, "a bounded array of generated voxel commands")
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                if let Some(size_hint) = sequence.size_hint() {
                    if size_hint > VOXEL_COMMAND_BATCH_MAX_COMMANDS {
                        return Err(<A::Error as serde::de::Error>::custom(format!(
                            "voxel command batch exceeds command limit {} (actual {size_hint})",
                            VOXEL_COMMAND_BATCH_MAX_COMMANDS
                        )));
                    }
                }
                let capacity = sequence
                    .size_hint()
                    .unwrap_or(0)
                    .min(VOXEL_COMMAND_BATCH_MAX_COMMANDS);
                let mut commands = Vec::with_capacity(capacity);
                while let Some(command) = sequence.next_element::<VoxelCommandJson>()? {
                    if commands.len() == VOXEL_COMMAND_BATCH_MAX_COMMANDS {
                        return Err(<A::Error as serde::de::Error>::custom(format!(
                            "voxel command batch exceeds command limit {} (actual at least {})",
                            VOXEL_COMMAND_BATCH_MAX_COMMANDS,
                            VOXEL_COMMAND_BATCH_MAX_COMMANDS + 1
                        )));
                    }
                    commands.push(command);
                }
                Ok(BoundedVoxelCommandBatchJson(commands))
            }
        }

        deserializer.deserialize_seq(BatchVisitor)
    }
}

impl VoxelValueJson {
    fn into_voxel_value(self) -> VoxelValue {
        match self {
            Self::Empty => VoxelValue::EMPTY,
            Self::Solid { material } => VoxelValue::solid_raw(material),
        }
    }
}

/// Parse the generated TypeScript `VoxelCommand` JSON shape into the canonical
/// Rust command union. Transport adapters use this bounded parser rather than
/// maintaining their own command model.
pub fn parse_voxel_command_batch_json(commands_json: &str) -> BridgeResult<CommandBatch> {
    if commands_json.len() > VOXEL_COMMAND_BATCH_MAX_REQUEST_BYTES {
        return Err(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!(
                "voxel command batch exceeds request byte limit {} (actual {})",
                VOXEL_COMMAND_BATCH_MAX_REQUEST_BYTES,
                commands_json.len()
            ),
        ));
    }
    let BoundedVoxelCommandBatchJson(inputs) =
        serde_json::from_str(commands_json).map_err(|error| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("invalid command batch JSON: {error}"),
            )
        })?;
    let commands = inputs
        .into_iter()
        .map(|input| match input {
            VoxelCommandJson::SetVoxel { grid, coord, value } => VoxelCommand::SetVoxel {
                grid: GridId::new(grid),
                coord: VoxelCoord::new(coord.x, coord.y, coord.z),
                value: value.into_voxel_value(),
            },
            VoxelCommandJson::FillRegion {
                grid,
                min,
                max,
                value,
            } => VoxelCommand::FillRegion {
                grid: GridId::new(grid),
                min: VoxelCoord::new(min.x, min.y, min.z),
                max: VoxelCoord::new(max.x, max.y, max.z),
                value: value.into_voxel_value(),
            },
            VoxelCommandJson::GenerateChunk {
                grid,
                chunk,
                seed,
                generator_version,
            } => VoxelCommand::GenerateChunk {
                grid: GridId::new(grid),
                chunk: ChunkCoord::new(chunk.x, chunk.y, chunk.z),
                seed,
                generator_version,
            },
        })
        .collect::<Vec<_>>();
    let known_touched_voxels = commands.iter().fold(0u64, |total, command| {
        total.saturating_add(
            rule_voxel_edit::command_touched_voxels_without_grid(command).unwrap_or(0),
        )
    });
    if known_touched_voxels > VOXEL_COMMAND_BATCH_MAX_TOUCHED_VOXELS {
        return Err(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!(
                "voxel command batch exceeds expanded touched-voxel limit {} (actual {})",
                VOXEL_COMMAND_BATCH_MAX_TOUCHED_VOXELS, known_touched_voxels
            ),
        ));
    }
    Ok(CommandBatch { commands })
}

/// Build the public set-voxel command used by transport glue that must stay
/// outside the state/rule crates. This keeps native/wasm adapters from depending
/// directly on authority internals while still carrying the real command union.
pub fn set_voxel_command(grid: u32, x: i64, y: i64, z: i64, material: u16) -> VoxelCommand {
    VoxelCommand::SetVoxel {
        grid: GridId::new(grid),
        coord: VoxelCoord::new(x, y, z),
        value: VoxelValue::solid_raw(material),
    }
}

#[cfg(test)]
mod voxel_command_json_tests {
    use super::*;

    #[test]
    fn generated_voxel_command_json_maps_to_canonical_rust_union() {
        let batch = parse_voxel_command_batch_json(
            r#"[
                {"op":"generateChunk","grid":1,"chunk":{"x":0,"y":0,"z":0},"seed":77,"generatorVersion":1},
                {"op":"fillRegion","grid":1,"min":{"x":0,"y":0,"z":0},"max":{"x":2,"y":2,"z":2},"value":{"kind":"empty"}},
                {"op":"setVoxel","grid":1,"coord":{"x":1,"y":1,"z":1},"value":{"kind":"solid","material":3}}
            ]"#,
        )
        .expect("every generated command variant parses");

        assert!(matches!(
            batch.commands.as_slice(),
            [
                VoxelCommand::GenerateChunk {
                    seed: 77,
                    generator_version: 1,
                    ..
                },
                VoxelCommand::FillRegion {
                    value: VoxelValue::Empty,
                    ..
                },
                VoxelCommand::SetVoxel {
                    value: VoxelValue::Solid { .. },
                    ..
                }
            ]
        ));
    }

    #[test]
    fn voxel_command_json_rejects_unknown_or_malformed_variants() {
        for invalid in [
            r#"[{"op":"deleteEverything"}]"#,
            r#"[{"op":"setVoxel","grid":1,"coord":{"x":0,"y":0,"z":0},"value":{"kind":"water"}}]"#,
            r#"[{"op":"generateChunk","grid":1,"chunk":{"x":0,"y":0,"z":0},"seed":77}]"#,
            r#"[{"op":"fillRegion","grid":1,"min":{"x":0,"y":0,"z":0},"max":{"x":1,"y":1,"z":1},"value":{"kind":"empty"},"authorityBypass":true}]"#,
        ] {
            let error =
                parse_voxel_command_batch_json(invalid).expect_err("input must fail closed");
            assert_eq!(error.kind, RuntimeBridgeErrorKind::InvalidInput);
            assert!(error.message.starts_with("invalid command batch JSON:"));
        }
    }

    #[test]
    fn voxel_command_json_byte_limit_is_exact_and_fails_before_parsing() {
        let at_limit = format!(
            "[]{}",
            " ".repeat(VOXEL_COMMAND_BATCH_MAX_REQUEST_BYTES - 2)
        );
        assert!(parse_voxel_command_batch_json(&at_limit).is_ok());

        let over_limit = format!("{at_limit} ");
        let error = parse_voxel_command_batch_json(&over_limit).expect_err("limit + 1 rejects");
        assert_eq!(error.kind, RuntimeBridgeErrorKind::InvalidInput);
        assert!(error.message.contains("exceeds request byte limit"));
    }

    #[test]
    fn voxel_command_json_command_limit_is_exact() {
        let command =
            r#"{"op":"setVoxel","grid":1,"coord":{"x":0,"y":0,"z":0},"value":{"kind":"empty"}}"#;
        let at_limit = format!(
            "[{}]",
            vec![command; VOXEL_COMMAND_BATCH_MAX_COMMANDS].join(",")
        );
        assert!(at_limit.len() <= VOXEL_COMMAND_BATCH_MAX_REQUEST_BYTES);
        assert_eq!(
            parse_voxel_command_batch_json(&at_limit)
                .expect("exact command limit parses")
                .commands
                .len(),
            VOXEL_COMMAND_BATCH_MAX_COMMANDS
        );

        let over_limit = format!("{},{command}]", &at_limit[..at_limit.len() - 1]);
        let error = parse_voxel_command_batch_json(&over_limit).expect_err("limit + 1 rejects");
        assert_eq!(error.kind, RuntimeBridgeErrorKind::InvalidInput);
        assert!(error.message.contains("exceeds command limit"));
    }

    #[test]
    fn voxel_command_json_expanded_touched_limit_is_exact() {
        let at_limit = r#"[{"op":"fillRegion","grid":1,"min":{"x":0,"y":0,"z":0},"max":{"x":1000000,"y":1,"z":1},"value":{"kind":"empty"}}]"#;
        assert!(parse_voxel_command_batch_json(at_limit).is_ok());

        let over_limit = r#"[{"op":"fillRegion","grid":1,"min":{"x":0,"y":0,"z":0},"max":{"x":1000001,"y":1,"z":1},"value":{"kind":"empty"}}]"#;
        let error = parse_voxel_command_batch_json(over_limit).expect_err("limit + 1 rejects");
        assert_eq!(error.kind, RuntimeBridgeErrorKind::InvalidInput);
        assert!(error.message.contains("expanded touched-voxel limit"));
    }
}

// ── Pick (voxel raycast) payloads (launchable-voxel picking, #2437) ───────────
//
// The renderer/UI builds a world-space ray from camera + pointer and hands it to
// `pick_voxel`. Rust authority owns the voxel-grid raycast (`svc-collision`); the
// renderer never owns authoritative voxel coordinates. Mirrors the generated
// `voxel.ts` `PickRay` / `VoxelHit` / `PickResult` border shapes.

/// A world-space pick ray. `grid` selects which authority grid to cast against;
/// `origin`/`direction` are world-space `[x, y, z]`; `max_distance` bounds the cast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickRay {
    pub grid: u64,
    pub origin: [f64; 3],
    pub direction: [f64; 3],
    pub max_distance: f64,
}

/// An authoritative voxel ray hit (the border mirror of `svc_collision::VoxelHit`,
/// carrying the grid id so the border is self-describing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelHit {
    pub grid: u64,
    pub voxel: VoxelCoord,
    pub chunk: ChunkCoord,
    pub face: Face,
    pub point: [f64; 3],
    pub distance: f64,
}

/// Why a pick produced no authoritative hit. Mirrors the `noHit` arm of the
/// generated `PickRejection`; `hitMismatch` is reserved for the renderer-hint
/// revalidation path (a later picking slice), so the raw-ray pick only ever
/// reports `NoHit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickRejection {
    #[default]
    NoHit,
}

/// The classified outcome of an authority voxel pick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PickResult {
    Hit(VoxelHit),
    Miss(PickRejection),
}

// ── Workspace voxel instances (asset-local projection and picking, #5832) ────

/// One scene-node use of the currently loaded voxel asset. The transform is a
/// renderer-neutral validated scene TRS; voxel coordinates remain asset-local.
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelProjectionInstanceBinding {
    pub instance_id: String,
    pub scene_node_id: u64,
    pub asset_id: String,
    pub transform: SceneTransformDto,
}

/// Complete replacement request for workspace voxel projection bindings.
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelProjectionBindingRequest {
    pub workspace_id: String,
    pub workspace_generation: u64,
    pub working_revision: u64,
    pub registry_digest: String,
    pub instances: Vec<VoxelProjectionInstanceBinding>,
}

/// Hash-bound receipt for the accepted retained instance graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelProjectionBindingReceipt {
    pub workspace_id: String,
    pub workspace_generation: u64,
    pub working_revision: u64,
    pub registry_digest: String,
    pub binding_hash: String,
    pub instance_count: u32,
    pub projection_op_count: u32,
}

/// Optional renderer observation. Rust independently re-casts the world ray and
/// compares this local cell/face before returning an edit anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelInstancePickHint {
    pub local_voxel: VoxelCoord,
    pub local_face: Face,
}

/// World-ray pick request bound to an accepted workspace instance registry.
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelInstancePickRequest {
    pub workspace_id: String,
    pub workspace_generation: u64,
    pub working_revision: u64,
    pub registry_digest: String,
    pub binding_hash: String,
    pub instance_id: String,
    pub origin: [f64; 3],
    pub direction: [f64; 3],
    pub max_distance: f64,
    pub renderer_hint: VoxelInstancePickHint,
}

/// Authority-confirmed asset-local selection plus world-space observation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelInstancePickHit {
    pub local_voxel: VoxelCoord,
    pub local_chunk: ChunkCoord,
    pub local_face: Face,
    pub local_place_anchor: VoxelCoord,
    pub world_point: [f64; 3],
    pub world_distance: f64,
}

/// Why an instance-bound workspace pick was rejected before any edit proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelInstancePickRejection {
    StaleWorkspaceGeneration,
    StaleWorkingRevision,
    RegistryDigestChanged,
    BindingHashMismatch,
    UnknownInstance,
    InvalidRay,
    NoHit,
    RendererHintMismatch,
}

/// Classified instance pick outcome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VoxelInstancePickOutcome {
    Hit(VoxelInstancePickHit),
    Rejected(VoxelInstancePickRejection),
}

/// Pick result repeats its workspace binding so a later edit cannot detach the
/// asset-local anchor from the projection revision that authorized it.
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelInstancePickResult {
    pub workspace_id: String,
    pub workspace_generation: u64,
    pub working_revision: u64,
    pub binding_hash: String,
    pub instance_id: String,
    pub outcome: VoxelInstancePickOutcome,
}

// ── Voxel mesh/remesh evidence (basic graphical voxel proof, #2646) ───────────

/// Compact request for deterministic voxel mesh evidence. If `chunks` is empty,
/// the bridge reports every resident chunk in canonical coordinate order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VoxelMeshEvidenceRequest {
    pub grid: u64,
    pub chunks: Vec<ChunkCoord>,
}

/// Compact mesh counters suitable for artifacts without inline geometry arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoxelMeshStatsEvidence {
    pub vertices: u32,
    pub indices: u32,
    pub quads: u32,
    pub faces_emitted: u32,
    pub faces_culled: u32,
}

/// Axis-aligned chunk-local mesh bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoxelMeshBoundsEvidence {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Per-chunk compact mesh evidence derived from authoritative voxel state.
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelMeshChunkEvidence {
    pub coord: ChunkCoord,
    pub resident: bool,
    pub visible: bool,
    pub content_hash: Option<String>,
    pub mesh_hash: Option<String>,
    pub stats: Option<VoxelMeshStatsEvidence>,
    pub bounds: Option<VoxelMeshBoundsEvidence>,
    pub material_slots: Vec<u16>,
}

/// Compact mesh snapshot for proof artifacts: no Three.js objects, no inline mesh
/// arrays by default, just stable hashes/stats sufficient to prove remeshing.
#[derive(Debug, Clone, PartialEq)]
pub struct VoxelMeshEvidenceSnapshot {
    pub grid: u64,
    pub fixture_id: String,
    pub voxel_state_hash: String,
    pub meshing_strategy: String,
    pub chunks: Vec<VoxelMeshChunkEvidence>,
    pub diagnostics: Vec<String>,
}

// ── FPS/ECRP RuntimeSession authority payloads (#4347) ───────────────────────
//
// These DTOs are the narrow public intent and readout shape for an FPS domain
// activated by canonical project admission. Runtime topology never enters
// through this surface.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FpsBridgeRole {
    Player,
    Enemy,
    Neutral,
}

impl FpsBridgeRole {
    pub fn label(self) -> &'static str {
        match self {
            FpsBridgeRole::Player => "player",
            FpsBridgeRole::Enemy => "enemy",
            FpsBridgeRole::Neutral => "neutral",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FpsBridgeHealth {
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsRuntimeSessionRestartRequest {
    pub expected_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FpsPrimaryFireRequest {
    pub tick: u64,
    pub origin: [f64; 3],
    pub direction: [f64; 3],
    pub shooter_role: Option<FpsBridgeRole>,
    pub target_role: Option<FpsBridgeRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsReadSetEvidence {
    pub view_kind: String,
    pub owner: String,
    pub read_set: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsReplayEvidence {
    pub replay_unit: String,
    pub entity_hash: u64,
    pub health_hash: u64,
    pub record_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEntityHealthReadout {
    pub entity: u64,
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsPolicyBindingReadout {
    pub entity: u64,
    pub binding_id: String,
    pub policy_id: String,
    pub view_kind: String,
    pub view_version: String,
    pub allowed_intents: Vec<String>,
    pub runtime_moment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FpsBridgeLifecycleStatus {
    Active,
    EnemyDefeated { entity: u64, tick: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsRuntimeSessionSnapshot {
    pub backend: String,
    pub authority_surface: String,
    pub project_bundle: String,
    pub session_epoch: u64,
    pub lifecycle_status: FpsBridgeLifecycleStatus,
    pub player_entity: u64,
    pub enemy_entity: u64,
    pub health: Vec<FpsEntityHealthReadout>,
    pub policy_bindings: Vec<FpsPolicyBindingReadout>,
    pub replay_records: Vec<FpsReplayEvidence>,
    pub read_sets: Vec<FpsReadSetEvidence>,
    pub entity_hash: u64,
    pub health_hash: u64,
    pub replay_hash: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FpsPrimaryFireResult {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub shooter: u64,
    pub target: Option<u64>,
    pub target_health_before: Option<FpsBridgeHealth>,
    pub target_health_after: Option<FpsBridgeHealth>,
    pub lifecycle_status: FpsBridgeLifecycleStatus,
    pub target_render_visible: Option<bool>,
    pub entity_hash: u64,
    pub health_hash: u64,
    pub replay_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayModuleViewScope {
    Session,
    Entity { entity: u64 },
    PrefabInstance { instance: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayModuleViewRequest {
    pub view: GameplayContractRef,
    pub scope: GameplayModuleViewScope,
    pub expected_runtime_session_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayModuleViewSnapshot {
    pub view: GameplayContractRef,
    pub provider_id: String,
    pub scope: GameplayModuleViewScope,
    pub revision: u64,
    pub canonical_payload: Vec<u8>,
    pub view_hash: String,
    pub runtime_session_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayPrefabPartInteractionRequest {
    pub actor: u64,
    pub instance: u64,
    pub role: String,
    pub expected_target: u64,
    pub tick: u64,
    pub expected_runtime_session_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayPrefabPartInteractionReceipt {
    pub actor: u64,
    pub instance: u64,
    pub role: String,
    pub target: u64,
    pub event_hash: String,
    pub reaction_frame_hash: String,
    pub runtime_session_hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameExtensionWeaponEffectInvocationRequest {
    pub hook: WeaponEffectHookRequest,
    pub primary_fire: FpsPrimaryFireRequest,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameExtensionWeaponEffectInvocationResult {
    pub hook_receipt: GameExtensionHookReceipt,
    pub replay_evidence: GameExtensionReplayEvidence,
    pub primary_fire: Option<FpsPrimaryFireResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRuleCatalogValidationReceipt {
    pub accepted: bool,
    pub catalog_hash: String,
    pub diagnostics: Vec<GameRuleDiagnostic>,
    pub trace: Vec<GameRuleTraceEntry>,
    pub evidence: Vec<GameRuleEvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRuleEffectIntentRequest {
    pub catalog: GameRuleCatalog,
    pub request: GameRuleResolutionRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRuleRuntimeReadout {
    pub backend: String,
    pub authority_surface: String,
    pub active_modifiers: Vec<GameRuleModifierState>,
    pub recent_trace: Vec<GameRuleTraceEntry>,
    pub recent_replay_hashes: Vec<String>,
    pub latest_replay_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterLifecycleInput {
    pub outcome_kind: String,
    pub terminal: bool,
    pub enemy_dead: bool,
    pub player_dead: bool,
    pub lifecycle_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterTransitionRequest {
    pub preset_id: String,
    pub action: String,
    pub lifecycle: FpsEncounterLifecycleInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterStateReadout {
    pub preset_id: String,
    pub status: String,
    pub spawned_enemy_ids: Vec<String>,
    pub defeated_enemy_ids: Vec<String>,
    pub revision: u64,
    pub last_transition: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterDirectorSnapshot {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub state: FpsEncounterStateReadout,
    pub lifecycle: FpsEncounterLifecycleInput,
    pub read_sets: Vec<FpsReadSetEvidence>,
    pub encounter_hash: u64,
    pub replay_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpsEncounterTransitionResult {
    pub backend: String,
    pub authority_surface: String,
    pub mutation_owner: String,
    pub workspace_trace: Vec<String>,
    pub accepted: bool,
    pub rejection_reason: Option<String>,
    pub event_kind: Option<String>,
    pub state: FpsEncounterStateReadout,
    pub lifecycle: FpsEncounterLifecycleInput,
    pub encounter_hash: u64,
    pub replay_hash: u64,
}
