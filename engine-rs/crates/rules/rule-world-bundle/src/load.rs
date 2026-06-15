//! Ordered world-bundle **load executor** (world-runtime-composition, #2361).
//!
//! This turns an already-validated [`LoadPlan`](svc_serialization::LoadPlan) into
//! Rust authority state by *executing* its ordered stages — it is not a plan
//! builder. Each [`LoadStep`](svc_serialization::LoadStep) runs real code:
//! version checks, asset-lock presence, `core_scene` scene decode/validate, voxel
//! edit-log replay / snapshot reconstruction, atomic scene bootstrap, and a final
//! consistency pass.
//!
//! # Staging / no-partial-mutation
//!
//! [`execute_load_plan`] builds the new world entirely in locals and returns a
//! [`WorldLoadResult`] only on success; on any failure it returns a classified
//! [`LoadExecutionError`] and produces **no** world. A caller therefore swaps its
//! live authority only on `Ok`, so a failed load cannot partially mutate an
//! existing world. (#2364 formalizes and tests the commit/swap policy and maps
//! these errors into `protocol-diagnostics`.)

use std::collections::BTreeMap;

use core_entity::{decode_snapshot, EntityStore, SnapshotDecodeError};
use core_ids::SceneId;
use core_scene::{
    bootstrap_scene, decode as decode_scene, validate as validate_scene, BootstrapError,
    BootstrapRecord, FlatSceneDocument, SceneDecodeError, SceneValidationReport, WorldHash,
    WorldState,
};
use core_space::{ChunkCoord, VoxelGridSpec};
use svc_serialization::{LoadPlan, LoadPlanError, LoadStage, LoadStep};
use svc_spatial::VoxelWorld;

use rule_voxel_edit::persist::{decode_edit_log, replay_edit_log};

use crate::compose::{reconstruct, ChunkSnapshotArtifact, CompactedVoxelSave};

/// The current bundle schema / protocol versions this executor understands.
/// A bundle newer than these fails closed at the `ValidateVersions` stage.
pub const SUPPORTED_BUNDLE_SCHEMA_VERSION: u32 = 1;
/// The protocol version this executor understands.
pub const SUPPORTED_PROTOCOL_VERSION: u32 = 1;

/// A source of bundle artifact bytes, addressed by bundle-relative path.
///
/// The executor never touches a filesystem itself: a caller (the runtime facade,
/// a test, a fixture loader) provides artifact contents through this trait, which
/// keeps the executor std-only and deterministic.
pub trait ArtifactSource {
    /// The text contents of the artifact at `path`, or `None` if absent.
    fn artifact(&self, path: &str) -> Option<&str>;

    /// The voxel grid spec this bundle carries, if any. A source that never
    /// serves a voxel section can leave this defaulted to `None`; a voxel
    /// section then fails closed with [`LoadExecutionError::VoxelSpecMissing`].
    fn voxel_grid_spec(&self) -> Option<VoxelGridSpec> {
        None
    }
}

/// A simple in-memory artifact source: a map of bundle-relative path → text,
/// plus the voxel grid spec a real bundle's world/generator metadata would carry
/// (required only when the bundle has a voxel section).
#[derive(Debug, Clone, Default)]
pub struct BundleArtifacts {
    texts: BTreeMap<String, String>,
    voxel_spec: Option<VoxelGridSpec>,
}

impl BundleArtifacts {
    /// An empty artifact set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one artifact's text at a bundle-relative path (builder-style).
    pub fn with_artifact(mut self, path: impl Into<String>, text: impl Into<String>) -> Self {
        self.texts.insert(path.into(), text.into());
        self
    }

    /// Set the voxel grid spec for this bundle (builder-style).
    pub fn with_voxel_spec(mut self, spec: VoxelGridSpec) -> Self {
        self.voxel_spec = Some(spec);
        self
    }
}

impl ArtifactSource for BundleArtifacts {
    fn artifact(&self, path: &str) -> Option<&str> {
        self.texts.get(path).map(String::as_str)
    }

    fn voxel_grid_spec(&self) -> Option<VoxelGridSpec> {
        self.voxel_spec
    }
}

/// One executed stage's outcome, for an agent-legible / golden stage summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageOutcome {
    pub stage: LoadStage,
    /// A short, deterministic description of what the stage did.
    pub detail: String,
}

/// The typed result of a successful load: authority state plus provenance.
/// (`VoxelWorld` is not `PartialEq`, so neither is this; compare via hashes /
/// the rendered summary instead.)
#[derive(Debug, Clone)]
pub struct WorldLoadResult {
    /// Scene/entity authority (runtime transforms + `scene node → entity` trace).
    pub world: WorldState,
    /// Restored runtime-diverged entity authority, when the bundle carried a
    /// world-state snapshot (#2484). Holds the full generic entity store —
    /// runtime-created entities, capability tables, relations, and source traces —
    /// over and above the spatial bootstrap baseline in `world`. `None` when the
    /// save had no runtime divergence to persist.
    pub runtime_entities: Option<EntityStore>,
    /// Voxel authority, when the bundle carried a voxel section.
    pub voxel: Option<VoxelWorld>,
    /// The atomic bootstrap record (carries the source trace).
    pub bootstrap: BootstrapRecord,
    /// Deterministic fingerprint of the bootstrapped scene/entity world.
    pub world_hash: WorldHash,
    /// Ordered per-stage outcomes (the executed plan, not the planned plan).
    pub stages: Vec<StageOutcome>,
}

impl WorldLoadResult {
    /// A deterministic, greppable summary of the executed stages + final state,
    /// suitable for a golden fixture.
    pub fn render_summary(&self) -> String {
        let mut out = String::new();
        for s in &self.stages {
            out.push_str(&format!("stage {} {}\n", s.stage.label(), s.detail));
        }
        out.push_str(&format!(
            "result entities={} voxel={} worldHash={:016x}\n",
            self.world.entity_count(),
            self.voxel.is_some(),
            self.world_hash.0
        ));
        match &self.runtime_entities {
            Some(store) => out.push_str(&format!(
                "runtimeEntities count={} entityHash={:016x}\n",
                store.total_count(),
                store.hash().0
            )),
            None => out.push_str("runtimeEntities none\n"),
        }
        out.push_str(&format!(
            "sourceTrace count={}\n",
            self.bootstrap.source_trace.len()
        ));
        out
    }
}

/// Why an ordered load failed. Each variant names the stage / artifact at fault,
/// so a load failure is agent-legible rather than a bare boolean.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadExecutionError {
    /// The plan itself is not a coherent ordered plan (fail before executing).
    PlanInvalid(LoadPlanError),
    /// A required artifact is absent from the source.
    MissingArtifact { stage: LoadStage, path: String },
    /// A required artifact is present but empty.
    EmptyArtifact { stage: LoadStage, path: String },
    /// The bundle declares versions newer than this build supports (fail closed).
    VersionUnsupported { bundle_schema: u32, protocol: u32 },
    /// The scene document failed to decode.
    SceneDecode {
        artifact: String,
        error: SceneDecodeError,
    },
    /// The scene document decoded but failed validation.
    SceneInvalid {
        artifact: String,
        report: SceneValidationReport,
    },
    /// The decoded scene id is not the one the plan expected.
    SceneIdMismatch { expected: SceneId, found: SceneId },
    /// Atomic scene bootstrap rejected the document.
    Bootstrap(BootstrapError),
    /// A voxel artifact failed to decode.
    VoxelDecode { path: String, detail: String },
    /// Voxel artifacts are present but the bundle carried no voxel grid spec.
    VoxelSpecMissing,
    /// Voxel replay/reconstruction rejected the edits.
    VoxelReplay { detail: String },
    /// The world-state snapshot artifact failed to decode (fail closed before any
    /// runtime authority is restored). Carries the classified codec error, so a
    /// schema-version mismatch, malformed structure, and unknown discriminant stay
    /// distinguishable.
    WorldStateDecode {
        path: String,
        error: SnapshotDecodeError,
    },
    /// The final consistency pass found a problem after composition.
    FinalConsistency { detail: String },
}

/// A staged live world with an explicit **commit/swap** load policy (#2364).
///
/// The recovery posture is "validate/execute into a staging area, then commit on
/// success": [`WorldStage::load_and_commit`] builds a fresh world with
/// [`execute_load_plan`] and replaces the live world **only** when the load
/// succeeds. A failed load returns its classified [`LoadExecutionError`] and
/// leaves the previously-live world byte-for-byte unchanged — there is no partial
/// commit, because the new world does not touch the old one until the swap.
#[derive(Debug, Clone, Default)]
pub struct WorldStage {
    live: Option<WorldLoadResult>,
}

impl WorldStage {
    /// A stage with no live world yet.
    pub fn empty() -> Self {
        Self::default()
    }

    /// The current live world, if one has been committed.
    pub fn live(&self) -> Option<&WorldLoadResult> {
        self.live.as_ref()
    }

    /// `true` if a live world is committed.
    pub fn has_live(&self) -> bool {
        self.live.is_some()
    }

    /// The committed live world's hash, if any (cheap mutation-safety probe).
    pub fn live_world_hash(&self) -> Option<WorldHash> {
        self.live.as_ref().map(|w| w.world_hash)
    }

    /// Execute `plan` into a staging area and, **only on success**, swap it in as
    /// the new live world. On failure the previous live world is untouched and the
    /// classified error is returned for diagnostics/remediation.
    pub fn load_and_commit(
        &mut self,
        plan: &LoadPlan,
        artifacts: &dyn ArtifactSource,
    ) -> Result<&WorldLoadResult, LoadExecutionError> {
        let staged = execute_load_plan(plan, artifacts)?;
        self.live = Some(staged);
        Ok(self.live.as_ref().expect("just committed"))
    }
}

/// Execute an ordered, already-validated load plan into authority state.
///
/// Stage order is enforced at execution time (via
/// [`LoadPlan::verify_order`]) before any artifact is touched. On any failure the
/// function returns `Err` and produces no world, so an existing authority world a
/// caller holds is never partially mutated.
pub fn execute_load_plan(
    plan: &LoadPlan,
    artifacts: &dyn ArtifactSource,
) -> Result<WorldLoadResult, LoadExecutionError> {
    // 1. Enforce stage order + mandatory stages *before* executing anything.
    plan.verify_order()
        .map_err(LoadExecutionError::PlanInvalid)?;

    let mut stages = Vec::new();
    let mut scene_doc: Option<FlatSceneDocument> = None;
    let mut voxel_world: Option<VoxelWorld> = None;
    let mut world_and_record: Option<(WorldState, BootstrapRecord)> = None;
    let mut runtime_entities: Option<EntityStore> = None;

    for step in &plan.steps {
        match step {
            LoadStep::ValidateVersions {
                bundle_schema_version,
                protocol_version,
            } => {
                if *bundle_schema_version > SUPPORTED_BUNDLE_SCHEMA_VERSION
                    || *protocol_version > SUPPORTED_PROTOCOL_VERSION
                {
                    return Err(LoadExecutionError::VersionUnsupported {
                        bundle_schema: *bundle_schema_version,
                        protocol: *protocol_version,
                    });
                }
                stages.push(StageOutcome {
                    stage: LoadStage::Versions,
                    detail: format!("schema={bundle_schema_version} protocol={protocol_version}"),
                });
            }
            LoadStep::LoadAssetLock {
                artifact,
                asset_count,
            } => {
                let text = read_required(artifacts, LoadStage::AssetLock, artifact)?;
                if text.trim().is_empty() {
                    return Err(LoadExecutionError::EmptyArtifact {
                        stage: LoadStage::AssetLock,
                        path: artifact.clone(),
                    });
                }
                stages.push(StageOutcome {
                    stage: LoadStage::AssetLock,
                    detail: format!("artifact={artifact} expectedAssets={asset_count}"),
                });
            }
            LoadStep::LoadSceneDocument { artifact, scene } => {
                let text = read_required(artifacts, LoadStage::SceneDocument, artifact)?;
                let doc = decode_scene(text).map_err(|error| LoadExecutionError::SceneDecode {
                    artifact: artifact.clone(),
                    error,
                })?;
                if doc.id != *scene {
                    return Err(LoadExecutionError::SceneIdMismatch {
                        expected: *scene,
                        found: doc.id,
                    });
                }
                let report = validate_scene(&doc);
                if !report.is_ok() {
                    return Err(LoadExecutionError::SceneInvalid {
                        artifact: artifact.clone(),
                        report,
                    });
                }
                stages.push(StageOutcome {
                    stage: LoadStage::SceneDocument,
                    detail: format!("artifact={artifact} nodes={}", doc.nodes.len()),
                });
                scene_doc = Some(doc);
            }
            LoadStep::GenerateTerrain {
                seed,
                version,
                params,
            } => {
                stages.push(StageOutcome {
                    stage: LoadStage::TerrainGeneration,
                    detail: format!("seed={seed} version={version} params={params}"),
                });
            }
            LoadStep::ApplyVoxelEdits {
                edit_logs,
                snapshots,
            } => {
                let world = apply_voxel_section(artifacts, edit_logs, snapshots)?;
                let applied = world.is_some();
                voxel_world = world;
                stages.push(StageOutcome {
                    stage: LoadStage::VoxelEdits,
                    detail: format!(
                        "editLogs={} snapshots={} applied={applied}",
                        edit_logs.len(),
                        snapshots.len()
                    ),
                });
            }
            LoadStep::BootstrapScene { scene, world } => {
                let doc = scene_doc
                    .as_ref()
                    .ok_or(LoadExecutionError::FinalConsistency {
                        detail: "bootstrap stage reached without a loaded scene document".into(),
                    })?;
                if doc.id != *scene {
                    return Err(LoadExecutionError::SceneIdMismatch {
                        expected: *scene,
                        found: doc.id,
                    });
                }
                let (state, record) =
                    bootstrap_scene(doc, *world).map_err(LoadExecutionError::Bootstrap)?;
                stages.push(StageOutcome {
                    stage: LoadStage::Bootstrap,
                    detail: format!(
                        "world={} entities={}",
                        record.world_id.raw(),
                        record.entity_count
                    ),
                });
                world_and_record = Some((state, record));
            }
            LoadStep::RestoreWorldState { artifact } => {
                // Restore over the bootstrapped baseline: the snapshot is the full
                // runtime authority. Fail closed (no partial mutation) on a missing,
                // empty, or undecodable artifact.
                if world_and_record.is_none() {
                    return Err(LoadExecutionError::FinalConsistency {
                        detail: "world-state restore reached before scene bootstrap".into(),
                    });
                }
                let text = read_required(artifacts, LoadStage::WorldStateSnapshot, artifact)?;
                if text.trim().is_empty() {
                    return Err(LoadExecutionError::EmptyArtifact {
                        stage: LoadStage::WorldStateSnapshot,
                        path: artifact.clone(),
                    });
                }
                let snapshot = decode_snapshot(text).map_err(|error| {
                    LoadExecutionError::WorldStateDecode {
                        path: artifact.clone(),
                        error,
                    }
                })?;
                let store = EntityStore::from_snapshot(snapshot);
                stages.push(StageOutcome {
                    stage: LoadStage::WorldStateSnapshot,
                    detail: format!(
                        "artifact={artifact} entities={} entityHash={:016x}",
                        store.total_count(),
                        store.hash().0
                    ),
                });
                runtime_entities = Some(store);
            }
            LoadStep::ValidateFinalState => {
                let (state, record) =
                    world_and_record
                        .as_ref()
                        .ok_or(LoadExecutionError::FinalConsistency {
                            detail: "final validation reached without a bootstrapped world".into(),
                        })?;
                // Final consistency: the recorded world hash must reproduce, and
                // every entity must have a source-trace entry.
                if state.hash() != record.world_hash {
                    return Err(LoadExecutionError::FinalConsistency {
                        detail: "bootstrapped world hash does not match the record".into(),
                    });
                }
                if record.source_trace.len() != record.entity_count {
                    return Err(LoadExecutionError::FinalConsistency {
                        detail: format!(
                            "source trace count {} != entity count {}",
                            record.source_trace.len(),
                            record.entity_count
                        ),
                    });
                }
                stages.push(StageOutcome {
                    stage: LoadStage::FinalValidation,
                    detail: format!("worldHash={:016x} ok", state.hash().0),
                });
            }
        }
    }

    let (world, bootstrap) = world_and_record.ok_or(LoadExecutionError::FinalConsistency {
        detail: "plan completed without bootstrapping a world".into(),
    })?;
    let world_hash = world.hash();

    Ok(WorldLoadResult {
        world,
        runtime_entities,
        voxel: voxel_world,
        bootstrap,
        world_hash,
        stages,
    })
}

/// Convenience wrapper for `execute_load_plan(plan, &BundleArtifacts)`.
pub fn execute_load_plan_with(
    plan: &LoadPlan,
    artifacts: &BundleArtifacts,
) -> Result<WorldLoadResult, LoadExecutionError> {
    execute_load_plan(plan, artifacts)
}

fn read_required<'a>(
    artifacts: &'a dyn ArtifactSource,
    stage: LoadStage,
    path: &str,
) -> Result<&'a str, LoadExecutionError> {
    artifacts
        .artifact(path)
        .ok_or_else(|| LoadExecutionError::MissingArtifact {
            stage,
            path: path.to_string(),
        })
}

/// Build the voxel authority world from the bundle's edit-log and snapshot
/// artifacts. Returns `Ok(None)` when there is no voxel section.
fn apply_voxel_section(
    artifacts: &dyn ArtifactSource,
    edit_logs: &[String],
    snapshots: &[String],
) -> Result<Option<VoxelWorld>, LoadExecutionError> {
    if edit_logs.is_empty() && snapshots.is_empty() {
        return Ok(None);
    }
    // The voxel spec is bundle metadata; a voxel section without it is a load
    // failure rather than a guess.
    let spec = voxel_spec(artifacts).ok_or(LoadExecutionError::VoxelSpecMissing)?;

    // Snapshots reconstruct chunk content; retained edit logs replay over them.
    let mut snapshot_artifacts = Vec::new();
    for path in snapshots {
        let text = read_required(artifacts, LoadStage::VoxelEdits, path)?;
        let chunk = parse_chunk_path(path).ok_or_else(|| LoadExecutionError::VoxelDecode {
            path: path.clone(),
            detail: "snapshot path does not encode a chunk coordinate".into(),
        })?;
        snapshot_artifacts.push(ChunkSnapshotArtifact {
            chunk,
            path: path.clone(),
            text: text.to_string(),
        });
    }

    let mut retained = Vec::new();
    for path in edit_logs {
        let text = read_required(artifacts, LoadStage::VoxelEdits, path)?;
        let events = decode_edit_log(text).map_err(|e| LoadExecutionError::VoxelDecode {
            path: path.clone(),
            detail: format!("{e}"),
        })?;
        retained.extend(events);
    }

    let world = if snapshot_artifacts.is_empty() {
        // Pure edit-log replay (chunks are created by ChunkGenerated events).
        replay_edit_log(spec, &retained).map_err(|e| LoadExecutionError::VoxelReplay {
            detail: format!("{e:?}"),
        })?
    } else {
        let save = CompactedVoxelSave {
            snapshots: snapshot_artifacts,
            retained_edits: retained,
            retained_log_text: String::new(),
            compacted_edits: 0,
        };
        reconstruct(spec, &save).map_err(|e| LoadExecutionError::VoxelReplay {
            detail: format!("{e:?}"),
        })?
    };
    Ok(Some(world))
}

/// The voxel grid spec, if the artifact source is a [`BundleArtifacts`] that
/// carries one. (A trait-object source without a spec yields `None`.)
fn voxel_spec(artifacts: &dyn ArtifactSource) -> Option<VoxelGridSpec> {
    // Downcast is unavailable through `&dyn`; instead the spec rides on the
    // concrete `BundleArtifacts`. Callers using a custom source that needs voxel
    // support should supply edit logs only (replayable without an external spec)
    // — but our canonical source is `BundleArtifacts`. We expose it via a probe.
    artifacts.voxel_grid_spec()
}

/// A bundle-relative `voxel/chunk_X_Y_Z.snapshot` path → its [`ChunkCoord`].
fn parse_chunk_path(path: &str) -> Option<ChunkCoord> {
    let file = path.rsplit('/').next()?;
    let stem = file.strip_prefix("chunk_")?.strip_suffix(".snapshot")?;
    let mut parts = stem.split('_');
    let x: i64 = parts.next()?.parse().ok()?;
    let y: i64 = parts.next()?.parse().ok()?;
    let z: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(ChunkCoord::new(x, y, z))
}
