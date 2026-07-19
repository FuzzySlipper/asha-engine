//! Ordered ProjectBundle **load executor** (runtime-session composition, #2361).
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
//! [`execute_load_plan`] builds the new runtime authority entirely in locals and
//! returns a [`ProjectBundleLoadResult`] only on success; on any failure it returns
//! a classified [`LoadExecutionError`] and produces **no** replacement session. A
//! caller therefore swaps its live authority only on `Ok`, so a failed load cannot
//! partially mutate an existing runtime session. (#2364 formalizes and tests the
//! commit/swap policy and maps these errors into `protocol-diagnostics`.)

use std::collections::BTreeMap;

use core_entity::{decode_snapshot, EntityStore, SnapshotDecodeError};
use core_ids::SceneId;
use core_scene::{
    bootstrap_scene, decode as decode_scene, validate as validate_scene, BootstrapError,
    BootstrapPlan, BootstrapRecord, BootstrapResolutionContext, FlatSceneDocument,
    SceneDecodeError, SceneValidationReport, SpatialSessionHash, SpatialSessionState,
};
use core_space::{ChunkCoord, VoxelGridSpec};
use protocol_voxel_annotation::{
    VoxelAnnotationDiagnostic, VoxelAnnotationLayer, VoxelAnnotationLayerValidationInput,
    VoxelAnnotationLayerValidationRequest,
};
use svc_serialization::{LoadPlan, LoadPlanError, LoadStage, LoadStep, ValidatedPrefabRegistry};
use svc_spatial::VoxelWorld;

use rule_voxel_edit::history::{
    decode_project_bundle_history_with_material_hash, VoxelEditHistory,
};
use rule_voxel_edit::persist::{decode_edit_log, replay_edit_log};
use rule_voxel_edit::voxel_world_hash;

use crate::compose::{reconstruct, ChunkSnapshotArtifact, CompactedVoxelSave};
use crate::prefab_instance::{
    InstantiatePrefabCommand, PrefabInstanceAuthority, PrefabInstantiationCatalog,
    PrefabInstantiationError, PrefabInstantiationReceipt,
};
use crate::prefab_snapshot::{decode_embedded_prefab_snapshot, PrefabSnapshotDecodeError};
use crate::session_state::{compose_session_state_snapshot_with_prefabs, SessionStateArtifact};

/// The current bundle schema / protocol versions this executor understands.
/// A bundle newer than these fails closed at the `ValidateVersions` stage.
pub const SUPPORTED_BUNDLE_SCHEMA_VERSION: u32 = svc_serialization::BUNDLE_SCHEMA_VERSION;
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

    /// The authority-visible voxel-data hash for a stored voxel-volume asset id,
    /// if this bundle source has loaded or indexed that target. Annotation layers
    /// fail closed when their target id is absent or their recorded target hash
    /// is stale.
    fn voxel_volume_data_hash(&self, _asset_id: &str) -> Option<&str> {
        None
    }

    /// The material catalog hash associated with persisted voxel edit history.
    /// History load fails closed when a history artifact is present and this
    /// hash is absent or differs from the artifact header.
    fn voxel_material_catalog_hash(&self) -> Option<&str> {
        None
    }
}

/// A simple in-memory artifact source: a map of bundle-relative path → text,
/// plus the voxel grid spec a real bundle's project/generator metadata would carry
/// (required only when the bundle has a voxel section).
#[derive(Debug, Clone, Default)]
pub struct BundleArtifacts {
    texts: BTreeMap<String, String>,
    voxel_spec: Option<VoxelGridSpec>,
    voxel_volume_data_hashes: BTreeMap<String, String>,
    voxel_material_catalog_hash: Option<String>,
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

    /// Declare the authority-visible data hash for a stored voxel-volume asset
    /// targeted by annotation layers.
    pub fn with_voxel_volume_data_hash(
        mut self,
        asset_id: impl Into<String>,
        voxel_data_hash: impl Into<String>,
    ) -> Self {
        self.voxel_volume_data_hashes
            .insert(asset_id.into(), voxel_data_hash.into());
        self
    }

    /// Declare the material catalog hash expected by voxel edit history.
    pub fn with_voxel_material_catalog_hash(mut self, hash: impl Into<String>) -> Self {
        self.voxel_material_catalog_hash = Some(hash.into());
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

    fn voxel_volume_data_hash(&self, asset_id: &str) -> Option<&str> {
        self.voxel_volume_data_hashes
            .get(asset_id)
            .map(String::as_str)
    }

    fn voxel_material_catalog_hash(&self) -> Option<&str> {
        self.voxel_material_catalog_hash.as_deref()
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
pub struct ProjectBundleLoadResult {
    /// Scene/entity authority (runtime transforms + `scene node → entity` trace).
    pub spatial_session: SpatialSessionState,
    /// Restored runtime-diverged entity authority, when the bundle carried a
    /// session-state snapshot (#2484). Holds the full generic entity store —
    /// runtime-created entities, capability tables, relations, and source traces —
    /// over and above the spatial bootstrap baseline in `spatial_session`. `None`
    /// when the save had no runtime divergence to persist.
    pub runtime_entities: Option<EntityStore>,
    /// Prefab instance/role authority. Created prefab entities live in the same
    /// `runtime_entities` store as every other non-scene Session entity.
    pub prefab_instances: PrefabInstanceAuthority,
    /// Voxel authority, when the bundle carried a voxel section.
    pub voxel: Option<VoxelWorld>,
    /// Voxel edit history/cursor authority, when the bundle carried a history
    /// artifact alongside the voxel section.
    pub voxel_history: Option<VoxelEditHistory>,
    /// Validated semantic annotation layers loaded from ProjectBundle artifacts.
    /// These are stored metadata/readout layers over target voxel-volume assets;
    /// they do not mutate voxel occupancy authority.
    pub voxel_annotations: Vec<VoxelAnnotationLayer>,
    /// The atomic bootstrap record (carries the source trace).
    pub bootstrap: BootstrapRecord,
    /// Deterministic fingerprint of the bootstrapped scene/entity spatial session.
    pub spatial_session_hash: SpatialSessionHash,
    /// Ordered per-stage outcomes (the executed plan, not the planned plan).
    pub stages: Vec<StageOutcome>,
}

impl ProjectBundleLoadResult {
    /// Compose the full current Session save artifact, including prefab role and
    /// override metadata beside the shared EntityStore snapshot.
    pub fn compose_session_state_snapshot(&self) -> Option<SessionStateArtifact> {
        let entities = self.runtime_entities.as_ref()?;
        let prefabs = self.prefab_instances.snapshot(entities);
        Some(compose_session_state_snapshot_with_prefabs(
            &entities.snapshot_durable(),
            &prefabs,
        ))
    }

    /// A deterministic, greppable summary of the executed stages + final state,
    /// suitable for a golden fixture.
    pub fn render_summary(&self) -> String {
        let mut out = String::new();
        for s in &self.stages {
            out.push_str(&format!("stage {} {}\n", s.stage.label(), s.detail));
        }
        out.push_str(&format!(
            "result entities={} voxel={} spatialSessionHash={:016x}\n",
            self.spatial_session.entity_count(),
            self.voxel.is_some(),
            self.spatial_session_hash.0
        ));
        out.push_str(&format!(
            "voxelAnnotations count={}\n",
            self.voxel_annotations.len()
        ));
        match &self.voxel_history {
            Some(history) => out.push_str(&format!(
                "voxelHistory cursor={} undoDepth={} redoDepth={} worldHash={:016x}\n",
                history.cursor().index,
                history.cursor().undo_depth,
                history.cursor().redo_depth,
                history.current_world_hash()
            )),
            None => out.push_str("voxelHistory none\n"),
        }
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
    /// A voxel history artifact failed to decode or did not match loaded voxel state.
    VoxelHistory { path: String, detail: String },
    /// A voxel annotation artifact failed to decode structurally.
    VoxelAnnotationDecode { path: String, detail: String },
    /// A voxel annotation layer targets a voxel-volume asset this source did not
    /// load or index.
    VoxelAnnotationTargetMissing { path: String, asset_id: String },
    /// A voxel annotation layer decoded but failed Rust authority validation.
    VoxelAnnotationInvalid {
        path: String,
        layer_id: String,
        diagnostics: Vec<VoxelAnnotationDiagnostic>,
    },
    /// The session-state snapshot artifact failed to decode (fail closed before any
    /// runtime authority is restored). Carries the classified codec error, so a
    /// schema-version mismatch, malformed structure, and unknown discriminant stay
    /// distinguishable.
    SessionStateDecode {
        path: String,
        error: SnapshotDecodeError,
    },
    /// The entity snapshot decoded, but its embedded prefab-instance metadata was
    /// malformed or did not match the restored owning EntityStore.
    PrefabSessionStateDecode {
        path: String,
        error: PrefabSnapshotDecodeError,
    },
    PrefabSessionStateDiverged {
        path: String,
        error: PrefabInstantiationError,
    },
    /// The final consistency pass found a problem after composition.
    FinalConsistency { detail: String },
}

/// A staged live ProjectBundle load with an explicit **commit/swap** policy (#2364).
///
/// The recovery posture is "validate/execute into a staging area, then commit on
/// success": [`ProjectBundleStage::load_and_commit`] builds fresh authority with
/// [`execute_load_plan`] and replaces the live session **only** when the load
/// succeeds. A failed load returns its classified [`LoadExecutionError`] and
/// leaves the previously-live session byte-for-byte unchanged — there is no
/// partial commit, because the new authority does not touch the old one until the
/// swap.
#[derive(Debug, Clone, Default)]
pub struct ProjectBundleStage {
    live: Option<ProjectBundleLoadResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectBundlePrefabError {
    NoLiveSession,
    Instantiate(PrefabInstantiationError),
}

impl core::fmt::Display for ProjectBundlePrefabError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProjectBundlePrefabError {}

impl ProjectBundleStage {
    /// A stage with no live ProjectBundle load yet.
    pub fn empty() -> Self {
        Self::default()
    }

    /// The current live ProjectBundle load, if one has been committed.
    pub fn live(&self) -> Option<&ProjectBundleLoadResult> {
        self.live.as_ref()
    }

    /// `true` if a live ProjectBundle load is committed.
    pub fn has_live(&self) -> bool {
        self.live.is_some()
    }

    /// The committed live spatial session hash, if any (cheap mutation-safety probe).
    pub fn live_spatial_session_hash(&self) -> Option<SpatialSessionHash> {
        self.live.as_ref().map(|w| w.spatial_session_hash)
    }

    /// Apply the same authoritative prefab command for authored or player
    /// placement. Both the instance map and the owning Session EntityStore are
    /// staged and swapped together, so rejection leaves the live Session exact.
    pub fn instantiate_prefab(
        &mut self,
        registry: &ValidatedPrefabRegistry,
        catalog: &PrefabInstantiationCatalog,
        command: InstantiatePrefabCommand,
    ) -> Result<PrefabInstantiationReceipt, ProjectBundlePrefabError> {
        let mut staged = self
            .live
            .clone()
            .ok_or(ProjectBundlePrefabError::NoLiveSession)?;
        let entities = staged.runtime_entities.get_or_insert_with(EntityStore::new);
        let receipt = staged
            .prefab_instances
            .instantiate(entities, registry, catalog, command)
            .map_err(ProjectBundlePrefabError::Instantiate)?;
        self.live = Some(staged);
        Ok(receipt)
    }

    /// Execute `plan` into a staging area and, **only on success**, swap it in as
    /// the new live ProjectBundle load. On failure the previous live load is
    /// untouched and the classified error is returned for diagnostics/remediation.
    pub fn load_and_commit(
        &mut self,
        plan: &LoadPlan,
        artifacts: &dyn ArtifactSource,
    ) -> Result<&ProjectBundleLoadResult, LoadExecutionError> {
        let staged = execute_load_plan(plan, artifacts)?;
        self.live = Some(staged);
        Ok(self.live.as_ref().expect("just committed"))
    }
}

/// Execute an ordered, already-validated load plan into authority state.
///
/// Stage order is enforced at execution time (via
/// [`LoadPlan::verify_order`]) before any artifact is touched. On any failure the
/// function returns `Err` and produces no replacement session, so existing
/// authority held by a caller is never partially mutated.
pub fn execute_load_plan(
    plan: &LoadPlan,
    artifacts: &dyn ArtifactSource,
) -> Result<ProjectBundleLoadResult, LoadExecutionError> {
    execute_load_plan_internal(plan, artifacts, None)
}

/// Execute a ProjectBundle whose scene carries typed stored references. The
/// immutable registry contains only independently validated external ids;
/// marker ids are always derived from the SceneDocument itself.
pub fn execute_load_plan_resolved(
    plan: &LoadPlan,
    artifacts: &dyn ArtifactSource,
    resolution: &BootstrapResolutionContext,
) -> Result<ProjectBundleLoadResult, LoadExecutionError> {
    execute_load_plan_internal(plan, artifacts, Some(resolution))
}

fn execute_load_plan_internal(
    plan: &LoadPlan,
    artifacts: &dyn ArtifactSource,
    resolution: Option<&BootstrapResolutionContext>,
) -> Result<ProjectBundleLoadResult, LoadExecutionError> {
    // 1. Enforce stage order + mandatory stages *before* executing anything.
    plan.verify_order()
        .map_err(LoadExecutionError::PlanInvalid)?;

    let mut stages = Vec::new();
    let mut scene_doc: Option<FlatSceneDocument> = None;
    let mut voxel_state: Option<VoxelWorld> = None;
    let mut voxel_history: Option<VoxelEditHistory> = None;
    let mut voxel_annotations: Vec<VoxelAnnotationLayer> = Vec::new();
    let mut spatial_session_and_record: Option<(SpatialSessionState, BootstrapRecord)> = None;
    let mut runtime_entities: Option<EntityStore> = None;
    let mut prefab_instances = PrefabInstanceAuthority::new();

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
                histories,
            } => {
                let (world, history) =
                    apply_voxel_section(artifacts, edit_logs, snapshots, histories)?;
                let applied = world.is_some();
                voxel_state = world;
                voxel_history = history;
                stages.push(StageOutcome {
                    stage: LoadStage::VoxelEdits,
                    detail: format!(
                        "editLogs={} snapshots={} histories={} applied={applied}",
                        edit_logs.len(),
                        snapshots.len(),
                        histories.len()
                    ),
                });
            }
            LoadStep::LoadVoxelAnnotations {
                artifacts: annotation_artifacts,
            } => {
                voxel_annotations = load_voxel_annotations(artifacts, annotation_artifacts)?;
                let region_count: usize = voxel_annotations
                    .iter()
                    .map(|layer| layer.regions.len())
                    .sum();
                stages.push(StageOutcome {
                    stage: LoadStage::VoxelAnnotations,
                    detail: format!(
                        "artifacts={} regions={region_count}",
                        voxel_annotations.len()
                    ),
                });
            }
            LoadStep::BootstrapScene {
                scene,
                runtime_session,
            } => {
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
                let (state, record) = match resolution {
                    Some(resolution) => {
                        BootstrapPlan::prepare_resolved(doc, *runtime_session, resolution)
                            .map(|plan| plan.apply())
                    }
                    None => bootstrap_scene(doc, *runtime_session),
                }
                .map_err(LoadExecutionError::Bootstrap)?;
                stages.push(StageOutcome {
                    stage: LoadStage::Bootstrap,
                    detail: format!(
                        "runtimeSession={} entities={}",
                        record.runtime_session_id.raw(),
                        record.entity_count
                    ),
                });
                spatial_session_and_record = Some((state, record));
            }
            LoadStep::RestoreSessionState { artifact } => {
                // Restore over the bootstrapped baseline: the snapshot is the full
                // runtime authority. Fail closed (no partial mutation) on a missing,
                // empty, or undecodable artifact.
                if spatial_session_and_record.is_none() {
                    return Err(LoadExecutionError::FinalConsistency {
                        detail: "session-state restore reached before scene bootstrap".into(),
                    });
                }
                let text = read_required(artifacts, LoadStage::SessionStateSnapshot, artifact)?;
                if text.trim().is_empty() {
                    return Err(LoadExecutionError::EmptyArtifact {
                        stage: LoadStage::SessionStateSnapshot,
                        path: artifact.clone(),
                    });
                }
                let snapshot = decode_snapshot(text).map_err(|error| {
                    LoadExecutionError::SessionStateDecode {
                        path: artifact.clone(),
                        error,
                    }
                })?;
                let store = EntityStore::from_snapshot(snapshot);
                if let Some(prefab_snapshot) =
                    decode_embedded_prefab_snapshot(text).map_err(|error| {
                        LoadExecutionError::PrefabSessionStateDecode {
                            path: artifact.clone(),
                            error,
                        }
                    })?
                {
                    prefab_instances =
                        PrefabInstanceAuthority::restore_persisted(&prefab_snapshot, &store)
                            .map_err(|error| LoadExecutionError::PrefabSessionStateDiverged {
                                path: artifact.clone(),
                                error,
                            })?;
                } else if store.snapshot().records.iter().any(|record| {
                    matches!(
                        record.core.source,
                        core_entity::EntitySource::PrefabInstance { .. }
                    )
                }) {
                    return Err(LoadExecutionError::PrefabSessionStateDecode {
                        path: artifact.clone(),
                        error: PrefabSnapshotDecodeError::Field(
                            "prefab-created entities require `prefabInstances` metadata".into(),
                        ),
                    });
                }
                stages.push(StageOutcome {
                    stage: LoadStage::SessionStateSnapshot,
                    detail: format!(
                        "artifact={artifact} entities={} entityHash={:016x}",
                        store.total_count(),
                        store.hash().0
                    ),
                });
                runtime_entities = Some(store);
            }
            LoadStep::ValidateFinalState => {
                let (state, record) = spatial_session_and_record.as_ref().ok_or(
                    LoadExecutionError::FinalConsistency {
                        detail: "final validation reached without a bootstrapped spatial session"
                            .into(),
                    },
                )?;
                // Final consistency: the recorded spatial session hash must reproduce, and
                // every entity must have a source-trace entry.
                if state.hash() != record.spatial_session_hash {
                    return Err(LoadExecutionError::FinalConsistency {
                        detail: "bootstrapped spatial session hash does not match the record"
                            .into(),
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
                    detail: format!("spatialSessionHash={:016x} ok", state.hash().0),
                });
            }
        }
    }

    let (spatial_session, bootstrap) =
        spatial_session_and_record.ok_or(LoadExecutionError::FinalConsistency {
            detail: "plan completed without bootstrapping a spatial session".into(),
        })?;
    let spatial_session_hash = spatial_session.hash();

    Ok(ProjectBundleLoadResult {
        spatial_session,
        runtime_entities,
        prefab_instances,
        voxel: voxel_state,
        voxel_history,
        voxel_annotations,
        bootstrap,
        spatial_session_hash,
        stages,
    })
}

/// Convenience wrapper for `execute_load_plan(plan, &BundleArtifacts)`.
pub fn execute_load_plan_with(
    plan: &LoadPlan,
    artifacts: &BundleArtifacts,
) -> Result<ProjectBundleLoadResult, LoadExecutionError> {
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

/// Build the voxel authority state from the bundle's edit-log and snapshot
/// artifacts. Returns `Ok(None)` when there is no voxel section.
fn apply_voxel_section(
    artifacts: &dyn ArtifactSource,
    edit_logs: &[String],
    snapshots: &[String],
    histories: &[String],
) -> Result<(Option<VoxelWorld>, Option<VoxelEditHistory>), LoadExecutionError> {
    if edit_logs.is_empty() && snapshots.is_empty() && histories.is_empty() {
        return Ok((None, None));
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

    let history_base_world = if snapshot_artifacts.is_empty() {
        VoxelWorld::new(spec)
    } else {
        let save = CompactedVoxelSave {
            snapshots: snapshot_artifacts.clone(),
            retained_edits: Vec::new(),
            retained_log_text: String::new(),
            compacted_edits: 0,
        };
        reconstruct(spec, &save).map_err(|e| LoadExecutionError::VoxelReplay {
            detail: format!("{e:?}"),
        })?
    };

    let voxel_authority = if snapshot_artifacts.is_empty() {
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

    let history = load_voxel_history(artifacts, histories, history_base_world, &voxel_authority)?;
    Ok((Some(voxel_authority), history))
}

fn load_voxel_history(
    artifacts: &dyn ArtifactSource,
    histories: &[String],
    base_world: VoxelWorld,
    voxel_authority: &VoxelWorld,
) -> Result<Option<VoxelEditHistory>, LoadExecutionError> {
    if histories.is_empty() {
        return Ok(None);
    }
    if histories.len() > 1 {
        return Err(LoadExecutionError::VoxelHistory {
            path: histories.join(","),
            detail: "multiple voxel history artifacts are not supported yet".into(),
        });
    }
    let path = &histories[0];
    let text = read_required(artifacts, LoadStage::VoxelEdits, path)?;
    let material_hash = artifacts.voxel_material_catalog_hash().ok_or_else(|| {
        LoadExecutionError::VoxelHistory {
            path: path.clone(),
            detail: "voxel history requires an expected material catalog hash".into(),
        }
    })?;
    let history = decode_project_bundle_history_with_material_hash(text, base_world, material_hash)
        .map_err(|detail| LoadExecutionError::VoxelHistory {
            path: path.clone(),
            detail,
        })?;
    let expected = voxel_world_hash(voxel_authority);
    let actual = history.current_world_hash();
    if expected != actual {
        return Err(LoadExecutionError::VoxelHistory {
            path: path.clone(),
            detail: format!(
                "history current voxel hash {actual:016x} does not match loaded voxel hash {expected:016x}"
            ),
        });
    }
    Ok(Some(history))
}

fn load_voxel_annotations(
    artifacts: &dyn ArtifactSource,
    annotation_artifacts: &[String],
) -> Result<Vec<VoxelAnnotationLayer>, LoadExecutionError> {
    let mut layers = Vec::with_capacity(annotation_artifacts.len());
    for path in annotation_artifacts {
        let text = read_required(artifacts, LoadStage::VoxelAnnotations, path)?;
        let layer: VoxelAnnotationLayer =
            serde_json::from_str(text).map_err(|e| LoadExecutionError::VoxelAnnotationDecode {
                path: path.clone(),
                detail: e.to_string(),
            })?;
        let target_hash = artifacts
            .voxel_volume_data_hash(&layer.target_voxel_volume_asset_id)
            .ok_or_else(|| LoadExecutionError::VoxelAnnotationTargetMissing {
                path: path.clone(),
                asset_id: layer.target_voxel_volume_asset_id.clone(),
            })?;
        let request = VoxelAnnotationLayerValidationRequest {
            input: VoxelAnnotationLayerValidationInput::Finalized { layer },
            expected_target_voxel_volume_asset_id: None,
            expected_target_voxel_data_hash: Some(target_hash.to_string()),
            max_regions: svc_voxel_annotation::DEFAULT_MAX_REGIONS,
            max_sparse_runs_per_region: svc_voxel_annotation::DEFAULT_MAX_SPARSE_RUNS_PER_REGION,
            max_total_assigned_cells: svc_voxel_annotation::DEFAULT_MAX_TOTAL_ASSIGNED_CELLS,
        };
        let report = svc_voxel_annotation::validate_layer(&request);
        if !report.valid {
            return Err(LoadExecutionError::VoxelAnnotationInvalid {
                path: path.clone(),
                layer_id: report.layer_id,
                diagnostics: report.diagnostics,
            });
        }
        layers.push(
            report
                .normalized_layer
                .expect("valid annotation report has a normalized layer"),
        );
    }
    Ok(layers)
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
