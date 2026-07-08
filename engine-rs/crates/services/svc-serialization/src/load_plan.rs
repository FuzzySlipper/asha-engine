//! Deterministic project-bundle load plan (scene-capability-02, subtask #2319).
//!
//! Loading order is an **authority constraint**, not an implementation detail. A
//! [`LoadPlan`] is the ordered, typed sequence of steps that turn a manifest into
//! loaded authority:
//!
//! 1. validate manifest/version compatibility,
//! 2. load asset lock / catalog,
//! 3. load + validate the flat scene document,
//! 4. generate terrain from seed + version + params,
//! 5. apply voxel edit log / load compacted snapshots,
//! 6. atomically bootstrap runtime entities from the scene document,
//! 7. validate final state.
//!
//! [`LoadPlan::build`] constructs this sequence deterministically from a manifest;
//! [`LoadPlan::verify_order`] rejects an out-of-order or prerequisite-missing
//! sequence with a classified diagnostic. Parallel *decoding* may happen
//! internally, but final authority application must respect this order.

use core_ids::{RuntimeSessionId, SceneId};

use crate::artifact::ArtifactRole;
use crate::manifest::{ManifestError, ProjectBundleManifest};

/// The ordered authority-application stages. A load plan's steps must appear in
/// non-decreasing stage order; the numeric index defines that order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadStage {
    Versions,
    AssetLock,
    SceneDocument,
    TerrainGeneration,
    VoxelEdits,
    Bootstrap,
    /// Restore a runtime-diverged session-state snapshot over the bootstrapped scene
    /// baseline. Optional: present only when the save carried runtime divergence
    /// (post-launchable-02, #2484).
    SessionStateSnapshot,
    FinalValidation,
}

impl LoadStage {
    /// Position in the canonical load order (lower runs first).
    pub fn index(self) -> u8 {
        match self {
            LoadStage::Versions => 0,
            LoadStage::AssetLock => 1,
            LoadStage::SceneDocument => 2,
            LoadStage::TerrainGeneration => 3,
            LoadStage::VoxelEdits => 4,
            LoadStage::Bootstrap => 5,
            LoadStage::SessionStateSnapshot => 6,
            LoadStage::FinalValidation => 7,
        }
    }

    /// Stable label for diagnostics/fixtures.
    pub fn label(self) -> &'static str {
        match self {
            LoadStage::Versions => "versions",
            LoadStage::AssetLock => "assetLock",
            LoadStage::SceneDocument => "sceneDocument",
            LoadStage::TerrainGeneration => "terrainGeneration",
            LoadStage::VoxelEdits => "voxelEdits",
            LoadStage::Bootstrap => "bootstrap",
            LoadStage::SessionStateSnapshot => "sessionStateSnapshot",
            LoadStage::FinalValidation => "finalValidation",
        }
    }
}

/// One ordered step of a load plan, carrying the typed inputs it consumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadStep {
    /// Validate bundle schema + protocol version compatibility.
    ValidateVersions {
        bundle_schema_version: u32,
        protocol_version: u32,
    },
    /// Load the asset lock from its artifact.
    LoadAssetLock { artifact: String, asset_count: u32 },
    /// Load + validate the flat scene document from its artifact.
    LoadSceneDocument { artifact: String, scene: SceneId },
    /// Generate terrain from seed/version/params.
    GenerateTerrain {
        seed: u64,
        version: u32,
        params: String,
    },
    /// Apply voxel edit logs and/or load compacted chunk snapshots, in artifact
    /// path order.
    ApplyVoxelEdits {
        edit_logs: Vec<String>,
        snapshots: Vec<String>,
    },
    /// Atomically bootstrap runtime entities from the scene document.
    BootstrapScene {
        scene: SceneId,
        runtime_session: RuntimeSessionId,
    },
    /// Restore the runtime-diverged session-state snapshot from its artifact, over
    /// the bootstrapped scene baseline (#2484).
    RestoreSessionState { artifact: String },
    /// Validate final state (hashes, required assets, source traces).
    ValidateFinalState,
}

impl LoadStep {
    /// The stage this step belongs to.
    pub fn stage(&self) -> LoadStage {
        match self {
            LoadStep::ValidateVersions { .. } => LoadStage::Versions,
            LoadStep::LoadAssetLock { .. } => LoadStage::AssetLock,
            LoadStep::LoadSceneDocument { .. } => LoadStage::SceneDocument,
            LoadStep::GenerateTerrain { .. } => LoadStage::TerrainGeneration,
            LoadStep::ApplyVoxelEdits { .. } => LoadStage::VoxelEdits,
            LoadStep::BootstrapScene { .. } => LoadStage::Bootstrap,
            LoadStep::RestoreSessionState { .. } => LoadStage::SessionStateSnapshot,
            LoadStep::ValidateFinalState => LoadStage::FinalValidation,
        }
    }
}

/// Why a load plan could not be built or verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadPlanError {
    /// The manifest itself failed validation (fail closed before planning).
    Manifest(ManifestError),
    /// A step references an artifact role not present in the manifest table.
    MissingPrerequisiteArtifact { role: String },
    /// A step appears before a stage it depends on (authority order violated).
    OutOfOrder { step: LoadStage, after: LoadStage },
    /// A mandatory stage is missing from the plan.
    MissingStage { stage: LoadStage },
}

impl core::fmt::Display for LoadPlanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LoadPlanError::Manifest(e) => write!(f, "manifest invalid: {e}"),
            LoadPlanError::MissingPrerequisiteArtifact { role } => {
                write!(f, "load step references missing artifact role `{role}`")
            }
            LoadPlanError::OutOfOrder { step, after } => write!(
                f,
                "load step `{}` runs after `{}` (authority order violated)",
                step.label(),
                after.label()
            ),
            LoadPlanError::MissingStage { stage } => {
                write!(f, "mandatory load stage `{}` is missing", stage.label())
            }
        }
    }
}

impl std::error::Error for LoadPlanError {}

/// A deterministic, ordered load plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadPlan {
    pub steps: Vec<LoadStep>,
}

/// The stages that must always be present in a load plan.
const MANDATORY_STAGES: [LoadStage; 5] = [
    LoadStage::Versions,
    LoadStage::AssetLock,
    LoadStage::SceneDocument,
    LoadStage::Bootstrap,
    LoadStage::FinalValidation,
];

impl LoadPlan {
    /// Build the canonical load plan for a manifest. Validates the manifest first
    /// (fail closed), then emits steps in authority order. Voxel edit logs and
    /// chunk snapshots are listed in artifact-path order for determinism.
    pub fn build(manifest: &ProjectBundleManifest) -> Result<LoadPlan, LoadPlanError> {
        manifest.validate().map_err(LoadPlanError::Manifest)?;

        let edit_logs = artifacts_with_role(manifest, &ArtifactRole::VoxelEditLog);
        let snapshots = artifacts_with_role(manifest, &ArtifactRole::VoxelChunkSnapshot);
        // The runtime session-state snapshot is optional: a save only carries one
        // when runtime authority diverged from the bootstrapped scene (#2484).
        let session_state_snapshot =
            artifacts_with_role(manifest, &ArtifactRole::SessionStateSnapshot)
                .into_iter()
                .next();

        let mut steps = vec![
            LoadStep::ValidateVersions {
                bundle_schema_version: manifest.bundle_schema_version,
                protocol_version: manifest.protocol_version,
            },
            LoadStep::LoadAssetLock {
                artifact: manifest.asset_lock.artifact.clone(),
                asset_count: manifest.asset_lock.asset_count,
            },
            LoadStep::LoadSceneDocument {
                artifact: manifest.scene.artifact.clone(),
                scene: manifest.scene.id,
            },
            LoadStep::GenerateTerrain {
                seed: manifest.generator.seed,
                version: manifest.generator.version,
                params: manifest.generator.params.clone(),
            },
            LoadStep::ApplyVoxelEdits {
                edit_logs,
                snapshots,
            },
            LoadStep::BootstrapScene {
                scene: manifest.scene.id,
                runtime_session: RuntimeSessionId::new(manifest.project.id.raw()),
            },
        ];
        if let Some(artifact) = session_state_snapshot {
            steps.push(LoadStep::RestoreSessionState { artifact });
        }
        steps.push(LoadStep::ValidateFinalState);

        let plan = LoadPlan { steps };
        plan.verify_order()?;
        Ok(plan)
    }

    /// Verify the plan respects authority order: steps appear in non-decreasing
    /// stage order and every mandatory stage is present. Out-of-order or missing
    /// prerequisites yield a classified [`LoadPlanError`].
    pub fn verify_order(&self) -> Result<(), LoadPlanError> {
        let mut last: Option<LoadStage> = None;
        for step in &self.steps {
            let stage = step.stage();
            if let Some(prev) = last {
                if stage.index() < prev.index() {
                    return Err(LoadPlanError::OutOfOrder {
                        step: stage,
                        after: prev,
                    });
                }
            }
            last = Some(stage);
        }
        let present: Vec<LoadStage> = self.steps.iter().map(LoadStep::stage).collect();
        for mandatory in MANDATORY_STAGES {
            if !present.contains(&mandatory) {
                return Err(LoadPlanError::MissingStage { stage: mandatory });
            }
        }
        Ok(())
    }

    /// Deterministic one-line-per-step rendering for golden fixtures/diagnostics.
    pub fn render(&self) -> String {
        use core::fmt::Write;
        let mut s = String::new();
        for (i, step) in self.steps.iter().enumerate() {
            let _ = writeln!(s, "{i} {} {}", step.stage().label(), render_step(step));
        }
        s
    }
}

fn artifacts_with_role(manifest: &ProjectBundleManifest, role: &ArtifactRole) -> Vec<String> {
    let mut v: Vec<String> = manifest
        .artifacts
        .iter()
        .filter(|a| &a.role == role)
        .map(|a| a.path.clone())
        .collect();
    v.sort();
    v
}

fn render_step(step: &LoadStep) -> String {
    match step {
        LoadStep::ValidateVersions {
            bundle_schema_version,
            protocol_version,
        } => format!("schema={bundle_schema_version} protocol={protocol_version}"),
        LoadStep::LoadAssetLock {
            artifact,
            asset_count,
        } => format!("{artifact} assets={asset_count}"),
        LoadStep::LoadSceneDocument { artifact, scene } => {
            format!("{artifact} scene={}", scene.raw())
        }
        LoadStep::GenerateTerrain {
            seed,
            version,
            params,
        } => format!("seed={seed} version={version} params={params}"),
        LoadStep::ApplyVoxelEdits {
            edit_logs,
            snapshots,
        } => format!(
            "editLogs=[{}] snapshots=[{}]",
            edit_logs.join(","),
            snapshots.join(",")
        ),
        LoadStep::BootstrapScene {
            scene,
            runtime_session,
        } => {
            format!(
                "scene={} runtimeSession={}",
                scene.raw(),
                runtime_session.raw()
            )
        }
        LoadStep::RestoreSessionState { artifact } => artifact.clone(),
        LoadStep::ValidateFinalState => String::new(),
    }
}
