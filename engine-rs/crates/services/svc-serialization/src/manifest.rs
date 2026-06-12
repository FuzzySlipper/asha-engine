//! The world-bundle manifest (scene-capability-02, subtask #2318).
//!
//! The manifest is the single inspectable index of a world bundle: it records the
//! bundle/protocol schema versions, the world/scene identity, the asset lock, the
//! terrain generator metadata, and the classified artifact table. It is canonical
//! for the *directory* layout; a future `.asha` archive is only a transport
//! wrapper around the same files (see crate docs).
//!
//! Validation fails **closed**: an unknown newer schema/protocol version is
//! rejected with a classified diagnostic rather than guessed at.

use core_ids::{SceneId, WorldId};

use crate::artifact::{ArtifactClass, ArtifactEntry};
use crate::hash::BundleHash;

/// The bundle-manifest schema version this build writes/understands.
pub const BUNDLE_SCHEMA_VERSION: u32 = 1;

/// The generated-contract protocol version this build is compatible with. A real
/// `protocol-codegen`-sourced value (scene-capability-02, "Decisions to make") is
/// future work; pinned here so manifests already carry the field.
pub const SUPPORTED_PROTOCOL_VERSION: u32 = 1;

/// Terrain generation metadata. Durable and compact: current chunk state can be
/// regenerated from `seed` + `version` + `params`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorMetadata {
    pub seed: u64,
    pub version: u32,
    /// Opaque, deterministic params identity (e.g. a hash/name of the param set).
    /// Kept abstract here; the generator owns its real param schema.
    pub params: String,
}

/// The world identity section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSection {
    pub id: WorldId,
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

/// A whole-world bundle manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBundleManifest {
    pub bundle_schema_version: u32,
    pub protocol_version: u32,
    pub world: WorldSection,
    pub scene: SceneSection,
    pub asset_lock: AssetLockSection,
    pub generator: GeneratorMetadata,
    /// The classified artifact table. Canonicalized (sorted by path) by
    /// [`WorldBundleManifest::canonical`].
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
        }
    }
}

impl std::error::Error for ManifestError {}

impl WorldBundleManifest {
    /// A copy with the artifact table sorted by path (deterministic on-disk order).
    pub fn canonical(&self) -> WorldBundleManifest {
        let mut m = self.clone();
        m.artifacts.sort_by(|a, b| a.path.cmp(&b.path));
        m
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

        // Unique paths + durable artifacts must carry a hash.
        let mut seen: Vec<&str> = Vec::with_capacity(self.artifacts.len());
        for a in &self.artifacts {
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
        }

        // Required sections must point at present artifacts.
        self.require(&self.scene.artifact, "scene")?;
        self.require(&self.asset_lock.artifact, "assetLock")?;
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
    pub fn without_cache(&self) -> WorldBundleManifest {
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
            "{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
            self.bundle_schema_version,
            self.protocol_version,
            self.world.id.raw(),
            self.scene.id.raw(),
            self.scene.schema_version,
            self.asset_lock.asset_count,
            self.generator.seed,
            self.generator.version,
            self.generator.params,
        ));
        // Sorted, cache-excluded artifact identities.
        let mut rows: Vec<String> = self
            .artifacts
            .iter()
            .filter(|a| a.class != ArtifactClass::Cache)
            .map(|a| {
                format!(
                    "{}:{}:{}",
                    a.path,
                    a.class.tag(),
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

/// Convenience builder for the common artifact roles.
impl WorldBundleManifest {
    /// Add an artifact entry, returning `self` for chaining.
    pub fn with_artifact(mut self, entry: ArtifactEntry) -> Self {
        self.artifacts.push(entry);
        self
    }
}
