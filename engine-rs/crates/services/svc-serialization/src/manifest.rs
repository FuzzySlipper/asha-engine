//! The project-bundle manifest (scene-capability-02, subtask #2318).
//!
//! The manifest is the single inspectable index of a project bundle: it records the
//! bundle/protocol schema versions, project and entry-scene identity, the asset
//! lock, optional authoring provenance, and the classified artifact table. It is canonical
//! for the *directory* layout; a future `.asha` archive is only a transport
//! wrapper around the same files (see crate docs).
//!
//! Validation fails **closed**: an unknown newer schema/protocol version is
//! rejected with a classified diagnostic rather than guessed at.

use core_ids::{ProjectId, SceneId};

use crate::artifact::{ArtifactClass, ArtifactEntry};
use crate::hash::BundleHash;

/// The bundle-manifest schema version this build writes/understands.
pub const BUNDLE_SCHEMA_VERSION: u32 = 2;

/// The generated-contract protocol version this build is compatible with. A real
/// `protocol-codegen`-sourced value (scene-capability-02, "Decisions to make") is
/// future work; pinned here so manifests already carry the field.
pub const SUPPORTED_PROTOCOL_VERSION: u32 = 1;

/// Authoring-only procedural generation provenance. Runtime admission never
/// resolves or invokes this provider: materialized scene/resource artifacts are
/// the runtime source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorMetadata {
    /// Stable provider identity retained for inspection and later re-authoring.
    pub provider: String,
    pub seed: u64,
    pub version: u32,
    /// Opaque, deterministic params identity (e.g. a hash/name of the param set).
    /// Kept abstract here; the generator owns its real param schema.
    pub params: String,
}

/// The project identity section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSection {
    pub id: ProjectId,
    pub name: Option<String>,
}

/// The scene section: identity plus the artifact carrying the flat scene document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneSection {
    pub id: SceneId,
    pub schema_version: u32,
    /// Bundle-relative path of the scene-document artifact.
    pub artifact: String,
}

/// The asset-lock section: the artifact carrying resolved asset references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetLockSection {
    /// Bundle-relative path of the asset-lock artifact.
    pub artifact: String,
    /// Number of locked asset references (legibility/cross-check only).
    pub asset_count: u32,
}

/// A whole-project bundle manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBundleManifest {
    pub bundle_schema_version: u32,
    pub protocol_version: u32,
    pub project: ProjectSection,
    /// Scene selected for initial runtime activation.
    pub entry_scene: SceneId,
    /// Every stored scene in the runtime closure. Canonical order is scene id,
    /// then artifact path.
    pub scenes: Vec<SceneSection>,
    pub asset_lock: AssetLockSection,
    /// Optional authoring provenance. It is hashed and persisted, but is never a
    /// load-plan prerequisite.
    pub generation_provenance: Option<GeneratorMetadata>,
    /// The classified artifact table. Canonicalized (sorted by path) by
    /// [`ProjectBundleManifest::canonical`].
    pub artifacts: Vec<ArtifactEntry>,
}

/// A classified manifest validation failure. Returned eagerly so a bad bundle is
/// rejected before any authority load is attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// The manifest schema version is newer than this build supports (fail closed).
    UnsupportedSchema { found: u32, supported: u32 },
    /// The protocol version is newer than this build supports (fail closed).
    UnsupportedProtocol { found: u32, supported: u32 },
    /// Two artifacts share a bundle-relative path.
    DuplicateArtifact { path: String },
    /// A required section references an artifact missing from the table.
    MissingArtifact { role: String, path: String },
    /// A durable artifact has no content hash (durable artifacts must be hashed).
    DurableMissingHash { path: String },
    /// A load-required v2 artifact has no content hash.
    LoadRequiredMissingHash { path: String },
    /// An artifact path is not a canonical safe bundle-relative path.
    InvalidArtifactPath { path: String },
    /// Two scene sections share a stable scene id.
    DuplicateScene { scene: u64 },
    /// The entry scene is not present in `scenes`.
    MissingEntryScene { scene: u64 },
    /// A scene section points at a non-scene or non-durable artifact.
    SceneArtifactMismatch { scene: u64, path: String },
    /// A scene-document artifact is not owned by any scene section.
    UnreferencedSceneArtifact { path: String },
    /// A v2 manifest used an opaque legacy role instead of a known role or the
    /// `resource:<kind>` extension namespace.
    UnknownArtifactRole { role: String, path: String },
    /// More than one artifact claims a singleton ProjectBundle role.
    DuplicateArtifactRole { role: String },
    /// An artifact role was stored with a durability class that cannot preserve it.
    ArtifactClassMismatch {
        path: String,
        expected: String,
        found: String,
    },
}

impl core::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ManifestError::UnsupportedSchema { found, supported } => write!(
                f,
                "bundle schema version {found} is newer than supported {supported} (fail closed)"
            ),
            ManifestError::UnsupportedProtocol { found, supported } => write!(
                f,
                "protocol version {found} is newer than supported {supported} (fail closed)"
            ),
            ManifestError::DuplicateArtifact { path } => {
                write!(f, "duplicate artifact path `{path}`")
            }
            ManifestError::MissingArtifact { role, path } => {
                write!(f, "{role} references missing artifact `{path}`")
            }
            ManifestError::DurableMissingHash { path } => {
                write!(f, "durable artifact `{path}` has no content hash")
            }
            ManifestError::LoadRequiredMissingHash { path } => {
                write!(f, "load-required artifact `{path}` has no content hash")
            }
            ManifestError::InvalidArtifactPath { path } => {
                write!(f, "artifact path `{path}` is not a canonical bundle-relative path")
            }
            ManifestError::DuplicateScene { scene } => {
                write!(f, "duplicate scene id `{scene}`")
            }
            ManifestError::MissingEntryScene { scene } => {
                write!(f, "entry scene `{scene}` is not declared in scenes")
            }
            ManifestError::SceneArtifactMismatch { scene, path } => write!(
                f,
                "scene `{scene}` references `{path}`, which is not a durable sceneDocument artifact"
            ),
            ManifestError::UnreferencedSceneArtifact { path } => {
                write!(f, "sceneDocument artifact `{path}` has no scene section")
            }
            ManifestError::UnknownArtifactRole { role, path } => write!(
                f,
                "artifact `{path}` uses unknown v2 role `{role}`; use a known role or resource:<kind>"
            ),
            ManifestError::DuplicateArtifactRole { role } => {
                write!(f, "multiple artifacts claim singleton role `{role}`")
            }
            ManifestError::ArtifactClassMismatch {
                path,
                expected,
                found,
            } => write!(
                f,
                "artifact `{path}` must use class `{expected}`, found `{found}`"
            ),
        }
    }
}

impl std::error::Error for ManifestError {}

impl ProjectBundleManifest {
    /// A v2 copy with scene and artifact tables in deterministic order. This is
    /// also the explicit in-memory migration used by canonical writes.
    pub fn canonical(&self) -> ProjectBundleManifest {
        let mut m = self.clone();
        m.bundle_schema_version = BUNDLE_SCHEMA_VERSION;
        m.scenes.sort_by(|left, right| {
            left.id
                .raw()
                .cmp(&right.id.raw())
                .then_with(|| left.artifact.cmp(&right.artifact))
        });
        m.artifacts.sort_by(|a, b| a.path.cmp(&b.path));
        m
    }

    /// The scene selected for initial activation.
    pub fn entry_scene(&self) -> Option<&SceneSection> {
        self.scenes
            .iter()
            .find(|scene| scene.id == self.entry_scene)
    }

    /// Find an artifact by bundle-relative path.
    pub fn artifact(&self, path: &str) -> Option<&ArtifactEntry> {
        self.artifacts.iter().find(|a| a.path == path)
    }

    /// Validate the manifest. Fails closed on unknown newer versions, and checks
    /// that required sections reference present artifacts, paths are unique, and
    /// durable artifacts are hashed.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.bundle_schema_version > BUNDLE_SCHEMA_VERSION {
            return Err(ManifestError::UnsupportedSchema {
                found: self.bundle_schema_version,
                supported: BUNDLE_SCHEMA_VERSION,
            });
        }
        if self.protocol_version > SUPPORTED_PROTOCOL_VERSION {
            return Err(ManifestError::UnsupportedProtocol {
                found: self.protocol_version,
                supported: SUPPORTED_PROTOCOL_VERSION,
            });
        }

        // Unique canonical paths + load-required artifacts must carry a hash.
        let mut seen: Vec<&str> = Vec::with_capacity(self.artifacts.len());
        for a in &self.artifacts {
            if !is_canonical_relative_path(&a.path) {
                return Err(ManifestError::InvalidArtifactPath {
                    path: a.path.clone(),
                });
            }
            if seen.contains(&a.path.as_str()) {
                return Err(ManifestError::DuplicateArtifact {
                    path: a.path.clone(),
                });
            }
            seen.push(&a.path);
            if a.class == ArtifactClass::Durable && a.content_hash.is_none() {
                return Err(ManifestError::DurableMissingHash {
                    path: a.path.clone(),
                });
            }
            if self.bundle_schema_version >= BUNDLE_SCHEMA_VERSION
                && a.class.is_load_required()
                && a.content_hash.is_none()
            {
                return Err(ManifestError::LoadRequiredMissingHash {
                    path: a.path.clone(),
                });
            }
            if self.bundle_schema_version >= BUNDLE_SCHEMA_VERSION
                && !a.role.is_known_runtime_role()
            {
                return Err(ManifestError::UnknownArtifactRole {
                    role: a.role.tag().to_string(),
                    path: a.path.clone(),
                });
            }
            if self.bundle_schema_version >= BUNDLE_SCHEMA_VERSION
                && !role_accepts_class(&a.role, a.class)
            {
                return Err(ManifestError::ArtifactClassMismatch {
                    path: a.path.clone(),
                    expected: expected_classes(&a.role).to_string(),
                    found: a.class.tag().to_string(),
                });
            }
        }

        let mut scene_ids = Vec::with_capacity(self.scenes.len());
        let mut scene_artifacts = Vec::with_capacity(self.scenes.len());
        for scene in &self.scenes {
            if scene_ids.contains(&scene.id.raw()) {
                return Err(ManifestError::DuplicateScene {
                    scene: scene.id.raw(),
                });
            }
            scene_ids.push(scene.id.raw());
            if scene_artifacts.contains(&scene.artifact.as_str()) {
                return Err(ManifestError::SceneArtifactMismatch {
                    scene: scene.id.raw(),
                    path: scene.artifact.clone(),
                });
            }
            scene_artifacts.push(scene.artifact.as_str());
            let Some(artifact) = self.artifact(&scene.artifact) else {
                return Err(ManifestError::MissingArtifact {
                    role: "scene".to_string(),
                    path: scene.artifact.clone(),
                });
            };
            if artifact.role != crate::ArtifactRole::SceneDocument
                || artifact.class != ArtifactClass::Durable
            {
                return Err(ManifestError::SceneArtifactMismatch {
                    scene: scene.id.raw(),
                    path: scene.artifact.clone(),
                });
            }
        }
        if self.entry_scene().is_none() {
            return Err(ManifestError::MissingEntryScene {
                scene: self.entry_scene.raw(),
            });
        }
        for artifact in self
            .artifacts
            .iter()
            .filter(|artifact| artifact.role == crate::ArtifactRole::SceneDocument)
        {
            if !scene_artifacts.contains(&artifact.path.as_str()) {
                return Err(ManifestError::UnreferencedSceneArtifact {
                    path: artifact.path.clone(),
                });
            }
        }

        for singleton_role in [
            crate::ArtifactRole::AssetLock,
            crate::ArtifactRole::PrefabRegistry,
        ] {
            let matching: Vec<&ArtifactEntry> = self
                .artifacts
                .iter()
                .filter(|artifact| artifact.role == singleton_role)
                .collect();
            if matching.len() > 1 {
                return Err(ManifestError::DuplicateArtifactRole {
                    role: singleton_role.tag().to_string(),
                });
            }
            if let Some(artifact) = matching.first() {
                if artifact.class != ArtifactClass::Durable {
                    return Err(ManifestError::ArtifactClassMismatch {
                        path: artifact.path.clone(),
                        expected: ArtifactClass::Durable.tag().to_string(),
                        found: artifact.class.tag().to_string(),
                    });
                }
            }
        }

        // Required asset-lock section must point at the matching durable role.
        self.require(&self.asset_lock.artifact, "assetLock")?;
        let lock = self
            .artifact(&self.asset_lock.artifact)
            .expect("require above established asset lock artifact");
        if lock.role != crate::ArtifactRole::AssetLock || lock.class != ArtifactClass::Durable {
            return Err(ManifestError::ArtifactClassMismatch {
                path: lock.path.clone(),
                expected: format!(
                    "{}:{}",
                    ArtifactClass::Durable.tag(),
                    crate::ArtifactRole::AssetLock.tag()
                ),
                found: format!("{}:{}", lock.class.tag(), lock.role.tag()),
            });
        }
        Ok(())
    }

    fn require(&self, path: &str, role: &str) -> Result<(), ManifestError> {
        if self.artifact(path).is_some() {
            Ok(())
        } else {
            Err(ManifestError::MissingArtifact {
                role: role.to_string(),
                path: path.to_string(),
            })
        }
    }

    /// The artifacts required for an authority load, in canonical path order.
    /// Cache artifacts are excluded — deleting them must not affect the load.
    pub fn load_required_artifacts(&self) -> Vec<&ArtifactEntry> {
        let mut v: Vec<&ArtifactEntry> = self
            .artifacts
            .iter()
            .filter(|a| a.class.is_load_required())
            .collect();
        v.sort_by(|a, b| a.path.cmp(&b.path));
        v
    }

    /// A copy of the manifest with every cache artifact removed. Used to prove an
    /// authority load is unaffected by cache disposal: the result still validates
    /// and yields the same [`load_required_artifacts`](Self::load_required_artifacts).
    pub fn without_cache(&self) -> ProjectBundleManifest {
        let mut m = self.clone();
        m.artifacts.retain(|a| a.class != ArtifactClass::Cache);
        m
    }

    /// Hash of the manifest's own durable identity (versions + sections + the
    /// durable/generated artifact hashes). Changes iff durable content changes;
    /// stable when only cache artifacts are added/removed.
    pub fn durable_hash(&self) -> BundleHash {
        let mut s = String::new();
        s.push_str(&format!(
            "{}|{}|{}|{}|{}|{}\n",
            BUNDLE_SCHEMA_VERSION,
            self.protocol_version,
            self.project.id.raw(),
            self.project.name.as_deref().unwrap_or_default(),
            self.asset_lock.asset_count,
            self.entry_scene.raw(),
        ));
        let mut scenes = self.scenes.clone();
        scenes.sort_by(|left, right| {
            left.id
                .raw()
                .cmp(&right.id.raw())
                .then_with(|| left.artifact.cmp(&right.artifact))
        });
        for scene in scenes {
            s.push_str(&format!(
                "scene:{}:{}:{}\n",
                scene.id.raw(),
                scene.schema_version,
                scene.artifact
            ));
        }
        if let Some(provenance) = &self.generation_provenance {
            s.push_str(&format!(
                "generation:{}:{}:{}:{}\n",
                provenance.provider, provenance.seed, provenance.version, provenance.params
            ));
        }
        // Sorted, cache-excluded artifact identities.
        let mut rows: Vec<String> = self
            .artifacts
            .iter()
            .filter(|a| a.class != ArtifactClass::Cache)
            .map(|a| {
                format!(
                    "{}:{}:{}:{}",
                    a.path,
                    a.class.tag(),
                    a.role.tag(),
                    a.content_hash.map(|h| h.to_hex()).unwrap_or_default(),
                )
            })
            .collect();
        rows.sort();
        for r in rows {
            s.push_str(&r);
            s.push('\n');
        }
        BundleHash::of_str(&s)
    }
}

pub(crate) fn is_canonical_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.ends_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn role_accepts_class(role: &crate::ArtifactRole, class: ArtifactClass) -> bool {
    use crate::ArtifactRole;
    match role {
        ArtifactRole::SceneDocument
        | ArtifactRole::AssetLock
        | ArtifactRole::PrefabRegistry
        | ArtifactRole::ProjectContent
        | ArtifactRole::EntityDefinitionCatalog
        | ArtifactRole::MaterialCatalog
        | ArtifactRole::VoxelVolumeAsset
        | ArtifactRole::SessionStateSnapshot
        | ArtifactRole::VoxelEditLog
        | ArtifactRole::VoxelEditHistory
        | ArtifactRole::VoxelAnnotationLayer
        | ArtifactRole::ReplayRecord => class == ArtifactClass::Durable,
        ArtifactRole::VoxelChunkSnapshot
        | ArtifactRole::GeneratedMetadata
        | ArtifactRole::Resource(_) => {
            matches!(class, ArtifactClass::Durable | ArtifactClass::Generated)
        }
        ArtifactRole::Cache => class == ArtifactClass::Cache,
        ArtifactRole::Other(_) => true,
    }
}

fn expected_classes(role: &crate::ArtifactRole) -> &'static str {
    use crate::ArtifactRole;
    match role {
        ArtifactRole::VoxelChunkSnapshot
        | ArtifactRole::GeneratedMetadata
        | ArtifactRole::Resource(_) => "durable|generated",
        ArtifactRole::Cache => "cache",
        ArtifactRole::Other(_) => "legacy",
        _ => "durable",
    }
}

/// Convenience builder for the common artifact roles.
impl ProjectBundleManifest {
    /// Add an artifact entry, returning `self` for chaining.
    pub fn with_artifact(mut self, entry: ArtifactEntry) -> Self {
        self.artifacts.push(entry);
        self
    }
}
