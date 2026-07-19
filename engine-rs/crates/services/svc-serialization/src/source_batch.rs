//! Host-neutral, bounded ProjectBundle source batches.
//!
//! A host adapter supplies the canonical manifest plus one body per declared
//! artifact. The body carries only its relative path and either compact inline
//! bytes or an opaque staged-resource handle: role, class, and content hash are
//! resolved exclusively from the Rust-owned manifest.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    decode, ArtifactRole, BundleHash, ManifestDecodeError, ManifestError, ProjectBundleManifest,
};

pub const PROJECT_SOURCE_MANIFEST_MAX_BYTES: usize = 2 * 1024 * 1024;
pub const PROJECT_SOURCE_MAX_BODIES: usize = 16_384;
pub const PROJECT_SOURCE_INLINE_BODY_MAX_BYTES: usize = 1024 * 1024;
pub const PROJECT_SOURCE_INLINE_TOTAL_MAX_BYTES: usize = 16 * 1024 * 1024;
pub const PROJECT_SOURCE_RESOURCE_MAX_BYTES: usize = 256 * 1024 * 1024;
pub const PROJECT_SOURCE_RESOURCE_TOTAL_MAX_BYTES: usize = 512 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectResourceHandle(u64);

impl ProjectResourceHandle {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectResourceTransaction {
    generation: u64,
    manifest_hash: BundleHash,
}

impl ProjectResourceTransaction {
    pub fn generation(self) -> u64 {
        self.generation
    }

    pub fn manifest_hash(self) -> BundleHash {
        self.manifest_hash
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StagedProjectResource {
    pub handle: ProjectResourceHandle,
    pub generation: u64,
    pub version: u32,
    pub byte_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectSourceBody {
    Inline {
        path: String,
        bytes: Vec<u8>,
    },
    Resource {
        path: String,
        resource: StagedProjectResource,
    },
}

impl ProjectSourceBody {
    pub fn inline(path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self::Inline {
            path: path.into(),
            bytes: bytes.into(),
        }
    }

    pub fn resource(path: impl Into<String>, resource: StagedProjectResource) -> Self {
        Self::Resource {
            path: path.into(),
            resource,
        }
    }

    pub fn path(&self) -> &str {
        match self {
            ProjectSourceBody::Inline { path, .. } | ProjectSourceBody::Resource { path, .. } => {
                path
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProjectSourceBatch {
    pub manifest_json: String,
    /// Transaction generation that owns every resource body. Inline-only
    /// batches use `None`; carrying it at batch level lets rejection clean a
    /// transaction even when the body list itself is malformed.
    pub resource_generation: Option<u64>,
    pub bodies: Vec<ProjectSourceBody>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectSourceBatchErrorCode {
    ManifestTooLarge,
    ManifestDecodeFailed,
    ManifestInvalid,
    TooManyBodies,
    DuplicateBody,
    DuplicateResourceHandle,
    MissingBody,
    ExtraBody,
    InlineBodyTooLarge,
    InlineBodyForbidden,
    InlineQuotaExceeded,
    ResourceBodyTooLarge,
    ResourceQuotaExceeded,
    UnknownResourceHandle,
    ResourceGenerationMismatch,
    ResourceVersionMismatch,
    ResourceLengthMismatch,
    ResourceManifestMismatch,
    ResourcePathMismatch,
    ContentHashMismatch,
}

impl ProjectSourceBatchErrorCode {
    pub fn label(self) -> &'static str {
        match self {
            ProjectSourceBatchErrorCode::ManifestTooLarge => "manifestTooLarge",
            ProjectSourceBatchErrorCode::ManifestDecodeFailed => "manifestDecodeFailed",
            ProjectSourceBatchErrorCode::ManifestInvalid => "manifestInvalid",
            ProjectSourceBatchErrorCode::TooManyBodies => "tooManyBodies",
            ProjectSourceBatchErrorCode::DuplicateBody => "duplicateBody",
            ProjectSourceBatchErrorCode::DuplicateResourceHandle => "duplicateResourceHandle",
            ProjectSourceBatchErrorCode::MissingBody => "missingBody",
            ProjectSourceBatchErrorCode::ExtraBody => "extraBody",
            ProjectSourceBatchErrorCode::InlineBodyTooLarge => "inlineBodyTooLarge",
            ProjectSourceBatchErrorCode::InlineBodyForbidden => "inlineBodyForbidden",
            ProjectSourceBatchErrorCode::InlineQuotaExceeded => "inlineQuotaExceeded",
            ProjectSourceBatchErrorCode::ResourceBodyTooLarge => "resourceBodyTooLarge",
            ProjectSourceBatchErrorCode::ResourceQuotaExceeded => "resourceQuotaExceeded",
            ProjectSourceBatchErrorCode::UnknownResourceHandle => "unknownResourceHandle",
            ProjectSourceBatchErrorCode::ResourceGenerationMismatch => "resourceGenerationMismatch",
            ProjectSourceBatchErrorCode::ResourceVersionMismatch => "resourceVersionMismatch",
            ProjectSourceBatchErrorCode::ResourceLengthMismatch => "resourceLengthMismatch",
            ProjectSourceBatchErrorCode::ResourceManifestMismatch => "resourceManifestMismatch",
            ProjectSourceBatchErrorCode::ResourcePathMismatch => "resourcePathMismatch",
            ProjectSourceBatchErrorCode::ContentHashMismatch => "contentHashMismatch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSourceBatchError {
    pub code: ProjectSourceBatchErrorCode,
    pub path: Option<String>,
    pub message: String,
}

impl ProjectSourceBatchError {
    fn new(
        code: ProjectSourceBatchErrorCode,
        path: Option<&str>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            path: path.map(str::to_string),
            message: message.into(),
        }
    }
}

impl core::fmt::Display for ProjectSourceBatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "{} at `{path}`: {}", self.code.label(), self.message)
        } else {
            write!(f, "{}: {}", self.code.label(), self.message)
        }
    }
}

impl std::error::Error for ProjectSourceBatchError {}

#[derive(Debug)]
struct ProjectResourceEntry {
    generation: u64,
    version: u32,
    manifest_hash: BundleHash,
    path: String,
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct ProjectResourceTransactionState {
    manifest: ProjectBundleManifest,
    manifest_hash: BundleHash,
}

/// Stages binary/large project inputs before domain invocation. Transactions
/// are manifest-bound and monotonic; abort or commit removes every handle in the
/// transaction so stale/replayed references fail closed.
#[derive(Debug, Default)]
pub struct ProjectResourceStaging {
    next_handle: u64,
    next_generation: u64,
    active_transactions: BTreeMap<u64, ProjectResourceTransactionState>,
    entries: BTreeMap<ProjectResourceHandle, ProjectResourceEntry>,
}

impl ProjectResourceStaging {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.next_handle = 0;
        self.next_generation = 0;
        self.active_transactions.clear();
        self.entries.clear();
    }

    pub fn begin_for_manifest(
        &mut self,
        manifest_json: &str,
    ) -> Result<ProjectResourceTransaction, ProjectSourceBatchError> {
        let manifest = decode_and_validate_manifest(manifest_json)?;
        self.next_generation = self
            .next_generation
            .checked_add(1)
            .expect("project-resource generation overflow is unreachable");
        let transaction = ProjectResourceTransaction {
            generation: self.next_generation,
            manifest_hash: manifest.durable_hash(),
        };
        self.active_transactions.insert(
            transaction.generation,
            ProjectResourceTransactionState {
                manifest,
                manifest_hash: transaction.manifest_hash,
            },
        );
        Ok(transaction)
    }

    pub fn stage(
        &mut self,
        transaction: ProjectResourceTransaction,
        path: &str,
        bytes: Vec<u8>,
    ) -> Result<StagedProjectResource, ProjectSourceBatchError> {
        let transaction_state = self
            .active_transactions
            .get(&transaction.generation)
            .filter(|state| state.manifest_hash == transaction.manifest_hash)
            .ok_or_else(|| {
                ProjectSourceBatchError::new(
                    ProjectSourceBatchErrorCode::ResourceGenerationMismatch,
                    Some(path),
                    format!(
                        "resource transaction generation {} is not active",
                        transaction.generation
                    ),
                )
            })?;
        let artifact = transaction_state.manifest.artifact(path).ok_or_else(|| {
            ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::ResourcePathMismatch,
                Some(path),
                "resource path is not declared by the transaction manifest",
            )
        })?;
        if !artifact.class.is_load_required() {
            return Err(ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::ResourcePathMismatch,
                Some(path),
                "cache-only artifact is not part of the runtime source closure",
            ));
        }
        if bytes.len() > PROJECT_SOURCE_RESOURCE_MAX_BYTES {
            return Err(ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::ResourceBodyTooLarge,
                None,
                format!(
                    "resource is {} bytes; limit is {}",
                    bytes.len(),
                    PROJECT_SOURCE_RESOURCE_MAX_BYTES
                ),
            ));
        }
        let handle = ProjectResourceHandle(self.next_handle);
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .expect("project-resource handle overflow is unreachable");
        let byte_len = bytes.len() as u64;
        self.entries.insert(
            handle,
            ProjectResourceEntry {
                generation: transaction.generation,
                version: 1,
                manifest_hash: transaction.manifest_hash,
                path: path.to_string(),
                bytes,
            },
        );
        Ok(StagedProjectResource {
            handle,
            generation: transaction.generation,
            version: 1,
            byte_len,
        })
    }

    pub fn abort(&mut self, transaction: ProjectResourceTransaction) -> usize {
        self.active_transactions.remove(&transaction.generation);
        let before = self.entries.len();
        self.entries.retain(|_, entry| {
            entry.generation != transaction.generation
                || entry.manifest_hash != transaction.manifest_hash
        });
        before - self.entries.len()
    }

    pub fn staged_count(&self) -> usize {
        self.entries.len()
    }

    pub fn stage_generation(
        &mut self,
        generation: u64,
        path: &str,
        bytes: Vec<u8>,
    ) -> Result<StagedProjectResource, ProjectSourceBatchError> {
        let manifest_hash = self
            .active_transactions
            .get(&generation)
            .map(|state| state.manifest_hash)
            .ok_or_else(|| {
                ProjectSourceBatchError::new(
                    ProjectSourceBatchErrorCode::ResourceGenerationMismatch,
                    None,
                    format!("resource transaction generation {generation} is not active"),
                )
            })?;
        self.stage(
            ProjectResourceTransaction {
                generation,
                manifest_hash,
            },
            path,
            bytes,
        )
    }

    fn entry(
        &self,
        resource: StagedProjectResource,
    ) -> Result<&ProjectResourceEntry, ProjectSourceBatchError> {
        let entry = self.entries.get(&resource.handle).ok_or_else(|| {
            ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::UnknownResourceHandle,
                None,
                format!("unknown or consumed handle {}", resource.handle.raw()),
            )
        })?;
        if entry.generation != resource.generation {
            return Err(ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::ResourceGenerationMismatch,
                None,
                format!(
                    "handle {} belongs to generation {}, not {}",
                    resource.handle.raw(),
                    entry.generation,
                    resource.generation
                ),
            ));
        }
        if entry.version != resource.version {
            return Err(ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::ResourceVersionMismatch,
                None,
                format!(
                    "handle {} version is {}, not {}",
                    resource.handle.raw(),
                    entry.version,
                    resource.version
                ),
            ));
        }
        if entry.bytes.len() as u64 != resource.byte_len {
            return Err(ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::ResourceLengthMismatch,
                None,
                format!(
                    "handle {} byte length is {}, not {}",
                    resource.handle.raw(),
                    entry.bytes.len(),
                    resource.byte_len
                ),
            ));
        }
        Ok(entry)
    }

    fn take(
        &mut self,
        resource: StagedProjectResource,
    ) -> Result<Vec<u8>, ProjectSourceBatchError> {
        self.entry(resource)?;
        Ok(self
            .entries
            .remove(&resource.handle)
            .expect("entry checked immediately above")
            .bytes)
    }

    fn abort_referenced(&mut self, batch: &RuntimeProjectSourceBatch) {
        let mut generations: BTreeSet<u64> = batch
            .bodies
            .iter()
            .filter_map(|body| match body {
                ProjectSourceBody::Resource { resource, .. } => Some(resource.generation),
                ProjectSourceBody::Inline { .. } => None,
            })
            .chain(batch.resource_generation)
            .collect();
        // A malformed reference may lie about its generation. Once its opaque
        // handle resolves, clean the authority-owned generation too so the
        // rejected transaction cannot leak staged bytes.
        for body in &batch.bodies {
            if let ProjectSourceBody::Resource { resource, .. } = body {
                if let Some(entry) = self.entries.get(&resource.handle) {
                    generations.insert(entry.generation);
                }
            }
        }
        self.entries
            .retain(|_, entry| !generations.contains(&entry.generation));
        self.active_transactions
            .retain(|generation, _| !generations.contains(generation));
    }

    fn abort_generation(&mut self, generation: u64) {
        self.active_transactions.remove(&generation);
        self.entries
            .retain(|_, entry| entry.generation != generation);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ValidatedProjectSourceBody {
    Inline(Vec<u8>),
    Resource(StagedProjectResource),
}

/// Opaque decode-and-validate artifact. It can only be created after the exact
/// manifest closure, body hashes, quotas, and staged-resource bindings pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedRuntimeProjectSourceBatch {
    manifest: ProjectBundleManifest,
    manifest_hash: BundleHash,
    resource_generation: Option<u64>,
    bodies: BTreeMap<String, ValidatedProjectSourceBody>,
}

impl ValidatedRuntimeProjectSourceBatch {
    pub fn manifest(&self) -> &ProjectBundleManifest {
        &self.manifest
    }

    pub fn manifest_hash(&self) -> BundleHash {
        self.manifest_hash
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.bodies.keys().map(String::as_str)
    }

    pub fn commit(
        self,
        staging: &mut ProjectResourceStaging,
    ) -> Result<AdmittedRuntimeProjectSourceBatch, ProjectSourceBatchError> {
        // Preflight every staged handle before removing any, so a stale or
        // externally-aborted transaction cannot partially consume the batch.
        for body in self.bodies.values() {
            if let ValidatedProjectSourceBody::Resource(resource) = body {
                staging.entry(*resource)?;
            }
        }
        let mut bodies = BTreeMap::new();
        for (path, body) in self.bodies {
            let bytes = match body {
                ValidatedProjectSourceBody::Inline(bytes) => bytes,
                ValidatedProjectSourceBody::Resource(resource) => staging.take(resource)?,
            };
            bodies.insert(path, bytes);
        }
        if let Some(generation) = self.resource_generation {
            staging.abort_generation(generation);
        }
        Ok(AdmittedRuntimeProjectSourceBatch {
            manifest: self.manifest,
            manifest_hash: self.manifest_hash,
            bodies,
        })
    }
}

/// Fully owned source bodies ready for compilation/linking. Construction is
/// private so downstream callers cannot bypass manifest closure validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedRuntimeProjectSourceBatch {
    manifest: ProjectBundleManifest,
    manifest_hash: BundleHash,
    bodies: BTreeMap<String, Vec<u8>>,
}

impl AdmittedRuntimeProjectSourceBatch {
    pub fn manifest(&self) -> &ProjectBundleManifest {
        &self.manifest
    }

    pub fn manifest_hash(&self) -> BundleHash {
        self.manifest_hash
    }

    pub fn body(&self, path: &str) -> Option<&[u8]> {
        self.bodies.get(path).map(Vec::as_slice)
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.bodies.keys().map(String::as_str)
    }
}

/// Validate one batch and clean every resource transaction referenced by a
/// rejected batch. Valid handles remain staged after success until the caller
/// atomically commits or explicitly aborts the transaction.
pub fn validate_runtime_project_source_batch(
    batch: &RuntimeProjectSourceBatch,
    staging: &mut ProjectResourceStaging,
) -> Result<ValidatedRuntimeProjectSourceBatch, ProjectSourceBatchError> {
    let result = validate_runtime_project_source_batch_inner(batch, staging);
    if result.is_err() {
        staging.abort_referenced(batch);
    }
    result
}

fn validate_runtime_project_source_batch_inner(
    batch: &RuntimeProjectSourceBatch,
    staging: &ProjectResourceStaging,
) -> Result<ValidatedRuntimeProjectSourceBatch, ProjectSourceBatchError> {
    if batch.bodies.len() > PROJECT_SOURCE_MAX_BODIES {
        return Err(ProjectSourceBatchError::new(
            ProjectSourceBatchErrorCode::TooManyBodies,
            None,
            format!(
                "batch has {} bodies; limit is {}",
                batch.bodies.len(),
                PROJECT_SOURCE_MAX_BODIES
            ),
        ));
    }
    let manifest = decode_and_validate_manifest(&batch.manifest_json)?;
    let manifest_hash = manifest.durable_hash();

    let mut bodies = BTreeMap::new();
    let mut resource_handles = BTreeSet::new();
    let mut observed_resource_generation = None;
    let mut inline_total = 0usize;
    let mut resource_total = 0usize;
    for body in &batch.bodies {
        let path = body.path();
        let artifact = manifest.artifact(path).ok_or_else(|| {
            ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::ExtraBody,
                Some(path),
                "body is not declared by the manifest",
            )
        })?;
        if bodies.contains_key(path) {
            return Err(ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::DuplicateBody,
                Some(path),
                "body path appears more than once",
            ));
        }

        let (validated, bytes) = match body {
            ProjectSourceBody::Inline { bytes, .. } => {
                if requires_resource_handle(&artifact.role) {
                    return Err(ProjectSourceBatchError::new(
                        ProjectSourceBatchErrorCode::InlineBodyForbidden,
                        Some(path),
                        format!(
                            "role `{}` must use staged binary resource transport",
                            artifact.role.tag()
                        ),
                    ));
                }
                if bytes.len() > PROJECT_SOURCE_INLINE_BODY_MAX_BYTES {
                    return Err(ProjectSourceBatchError::new(
                        ProjectSourceBatchErrorCode::InlineBodyTooLarge,
                        Some(path),
                        format!(
                            "inline body is {} bytes; per-body limit is {}",
                            bytes.len(),
                            PROJECT_SOURCE_INLINE_BODY_MAX_BYTES
                        ),
                    ));
                }
                inline_total = inline_total.saturating_add(bytes.len());
                if inline_total > PROJECT_SOURCE_INLINE_TOTAL_MAX_BYTES {
                    return Err(ProjectSourceBatchError::new(
                        ProjectSourceBatchErrorCode::InlineQuotaExceeded,
                        Some(path),
                        format!(
                            "inline total exceeds {} bytes",
                            PROJECT_SOURCE_INLINE_TOTAL_MAX_BYTES
                        ),
                    ));
                }
                (
                    ValidatedProjectSourceBody::Inline(bytes.clone()),
                    bytes.as_slice(),
                )
            }
            ProjectSourceBody::Resource { resource, .. } => {
                if !resource_handles.insert(resource.handle) {
                    return Err(ProjectSourceBatchError::new(
                        ProjectSourceBatchErrorCode::DuplicateResourceHandle,
                        Some(path),
                        format!(
                            "staged handle {} is referenced more than once",
                            resource.handle.raw()
                        ),
                    ));
                }
                if let Some(expected) = batch.resource_generation {
                    if resource.generation != expected {
                        return Err(ProjectSourceBatchError::new(
                            ProjectSourceBatchErrorCode::ResourceGenerationMismatch,
                            Some(path),
                            format!(
                                "resource generation {} does not match batch generation {}",
                                resource.generation, expected
                            ),
                        ));
                    }
                } else {
                    return Err(ProjectSourceBatchError::new(
                        ProjectSourceBatchErrorCode::ResourceGenerationMismatch,
                        Some(path),
                        "resource body requires a batch resource generation",
                    ));
                }
                observed_resource_generation = Some(resource.generation);
                let entry = staging.entry(*resource).map_err(|mut error| {
                    error.path = Some(path.to_string());
                    error
                })?;
                if entry.path != path {
                    return Err(ProjectSourceBatchError::new(
                        ProjectSourceBatchErrorCode::ResourcePathMismatch,
                        Some(path),
                        format!(
                            "staged handle {} is bound to `{}`, not `{path}`",
                            resource.handle.raw(),
                            entry.path
                        ),
                    ));
                }
                if entry.manifest_hash != manifest_hash {
                    return Err(ProjectSourceBatchError::new(
                        ProjectSourceBatchErrorCode::ResourceManifestMismatch,
                        Some(path),
                        "staged handle belongs to a different manifest closure",
                    ));
                }
                resource_total = resource_total.saturating_add(entry.bytes.len());
                if resource_total > PROJECT_SOURCE_RESOURCE_TOTAL_MAX_BYTES {
                    return Err(ProjectSourceBatchError::new(
                        ProjectSourceBatchErrorCode::ResourceQuotaExceeded,
                        Some(path),
                        format!(
                            "resource total exceeds {} bytes",
                            PROJECT_SOURCE_RESOURCE_TOTAL_MAX_BYTES
                        ),
                    ));
                }
                (
                    ValidatedProjectSourceBody::Resource(*resource),
                    entry.bytes.as_slice(),
                )
            }
        };

        let expected_hash = artifact.content_hash.ok_or_else(|| {
            ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::ManifestInvalid,
                Some(path),
                "provided artifact has no manifest content hash",
            )
        })?;
        let actual_hash = BundleHash::of(bytes);
        if actual_hash != expected_hash {
            return Err(ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::ContentHashMismatch,
                Some(path),
                format!(
                    "body hash {} does not match manifest hash {}",
                    actual_hash.to_hex(),
                    expected_hash.to_hex()
                ),
            ));
        }
        bodies.insert(path.to_string(), validated);
    }

    if batch.resource_generation.is_some() && observed_resource_generation.is_none() {
        return Err(ProjectSourceBatchError::new(
            ProjectSourceBatchErrorCode::ResourceGenerationMismatch,
            None,
            "batch declares a resource generation but carries no resource body",
        ));
    }

    for artifact in manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.class.is_load_required())
    {
        if !bodies.contains_key(&artifact.path) {
            return Err(ProjectSourceBatchError::new(
                ProjectSourceBatchErrorCode::MissingBody,
                Some(&artifact.path),
                "load-required manifest artifact has no source body",
            ));
        }
    }

    Ok(ValidatedRuntimeProjectSourceBatch {
        manifest,
        manifest_hash,
        resource_generation: batch.resource_generation,
        bodies,
    })
}

fn decode_and_validate_manifest(
    manifest_json: &str,
) -> Result<ProjectBundleManifest, ProjectSourceBatchError> {
    if manifest_json.len() > PROJECT_SOURCE_MANIFEST_MAX_BYTES {
        return Err(ProjectSourceBatchError::new(
            ProjectSourceBatchErrorCode::ManifestTooLarge,
            None,
            format!(
                "manifest is {} bytes; limit is {}",
                manifest_json.len(),
                PROJECT_SOURCE_MANIFEST_MAX_BYTES
            ),
        ));
    }
    let manifest = decode(manifest_json).map_err(map_manifest_decode_error)?;
    manifest.validate().map_err(map_manifest_error)?;
    Ok(manifest.canonical())
}

fn map_manifest_decode_error(error: ManifestDecodeError) -> ProjectSourceBatchError {
    ProjectSourceBatchError::new(
        ProjectSourceBatchErrorCode::ManifestDecodeFailed,
        None,
        error.to_string(),
    )
}

fn map_manifest_error(error: ManifestError) -> ProjectSourceBatchError {
    ProjectSourceBatchError::new(
        ProjectSourceBatchErrorCode::ManifestInvalid,
        None,
        error.to_string(),
    )
}

fn requires_resource_handle(role: &ArtifactRole) -> bool {
    matches!(
        role,
        ArtifactRole::VoxelVolumeAsset
            | ArtifactRole::VoxelChunkSnapshot
            | ArtifactRole::Resource(_)
    )
}

#[cfg(test)]
mod tests {
    use core_ids::{ProjectId, SceneId};

    use super::*;
    use crate::{
        encode, ArtifactEntry, AssetLockSection, ProjectSection, SceneSection,
        BUNDLE_SCHEMA_VERSION, SUPPORTED_PROTOCOL_VERSION,
    };

    fn source_fixture() -> (ProjectBundleManifest, BTreeMap<String, Vec<u8>>) {
        let bodies = BTreeMap::from([
            ("assets/lock.json".to_string(), b"asset-lock".to_vec()),
            (
                "content/gameplay.json".to_string(),
                b"project-content".to_vec(),
            ),
            ("scene/entry.json".to_string(), b"entry-scene".to_vec()),
            ("scene/other.json".to_string(), b"other-scene".to_vec()),
            ("voxel/house.avox".to_string(), b"voxel-house".to_vec()),
        ]);
        let manifest = ProjectBundleManifest {
            bundle_schema_version: BUNDLE_SCHEMA_VERSION,
            protocol_version: SUPPORTED_PROTOCOL_VERSION,
            project: ProjectSection {
                id: ProjectId::new(7),
                name: Some("source-fixture".into()),
            },
            entry_scene: SceneId::new(10),
            scenes: vec![
                SceneSection {
                    id: SceneId::new(10),
                    schema_version: 1,
                    artifact: "scene/entry.json".into(),
                },
                SceneSection {
                    id: SceneId::new(11),
                    schema_version: 1,
                    artifact: "scene/other.json".into(),
                },
            ],
            asset_lock: AssetLockSection {
                artifact: "assets/lock.json".into(),
                asset_count: 0,
            },
            generation_provenance: None,
            artifacts: vec![
                ArtifactEntry::durable(
                    "assets/lock.json",
                    ArtifactRole::AssetLock,
                    &bodies["assets/lock.json"],
                ),
                ArtifactEntry::durable(
                    "content/gameplay.json",
                    ArtifactRole::ProjectContent,
                    &bodies["content/gameplay.json"],
                ),
                ArtifactEntry::durable(
                    "scene/entry.json",
                    ArtifactRole::SceneDocument,
                    &bodies["scene/entry.json"],
                ),
                ArtifactEntry::durable(
                    "scene/other.json",
                    ArtifactRole::SceneDocument,
                    &bodies["scene/other.json"],
                ),
                ArtifactEntry::durable(
                    "voxel/house.avox",
                    ArtifactRole::VoxelVolumeAsset,
                    &bodies["voxel/house.avox"],
                ),
            ],
        };
        (manifest, bodies)
    }

    fn valid_batch(
        staging: &mut ProjectResourceStaging,
    ) -> (RuntimeProjectSourceBatch, ProjectResourceTransaction) {
        let (manifest, bodies) = source_fixture();
        let manifest_json = encode(&manifest);
        let transaction = staging
            .begin_for_manifest(&manifest_json)
            .expect("begin transaction");
        let voxel = staging
            .stage(
                transaction,
                "voxel/house.avox",
                bodies["voxel/house.avox"].clone(),
            )
            .expect("stage voxel");
        let batch = RuntimeProjectSourceBatch {
            manifest_json,
            resource_generation: Some(transaction.generation()),
            bodies: vec![
                ProjectSourceBody::inline("assets/lock.json", bodies["assets/lock.json"].clone()),
                ProjectSourceBody::inline(
                    "content/gameplay.json",
                    bodies["content/gameplay.json"].clone(),
                ),
                ProjectSourceBody::inline("scene/entry.json", bodies["scene/entry.json"].clone()),
                ProjectSourceBody::inline("scene/other.json", bodies["scene/other.json"].clone()),
                ProjectSourceBody::resource("voxel/house.avox", voxel),
            ],
        };
        (batch, transaction)
    }

    #[test]
    fn valid_batch_commits_complete_manifest_owned_closure_once() {
        let mut staging = ProjectResourceStaging::new();
        let (batch, _) = valid_batch(&mut staging);
        let validated = validate_runtime_project_source_batch(&batch, &mut staging)
            .expect("validate complete batch");
        assert_eq!(validated.paths().count(), 5);
        assert_eq!(staging.staged_count(), 1);

        let admitted = validated
            .commit(&mut staging)
            .expect("commit staged bodies");
        assert_eq!(admitted.paths().count(), 5);
        assert_eq!(
            admitted.body("voxel/house.avox"),
            Some(b"voxel-house".as_slice())
        );
        assert_eq!(staging.staged_count(), 0);

        let replay = validate_runtime_project_source_batch(&batch, &mut staging)
            .expect_err("consumed handle cannot be replayed");
        assert_eq!(
            replay.code,
            ProjectSourceBatchErrorCode::UnknownResourceHandle
        );
    }

    #[test]
    fn missing_extra_duplicate_and_wrong_hash_are_classified() {
        let mut staging = ProjectResourceStaging::new();
        let (mut missing, _) = valid_batch(&mut staging);
        missing.bodies.remove(0);
        let error = validate_runtime_project_source_batch(&missing, &mut staging)
            .expect_err("missing body rejected");
        assert_eq!(error.code, ProjectSourceBatchErrorCode::MissingBody);

        let mut staging = ProjectResourceStaging::new();
        let (mut extra, _) = valid_batch(&mut staging);
        extra
            .bodies
            .push(ProjectSourceBody::inline("not-declared.json", b"extra"));
        let error = validate_runtime_project_source_batch(&extra, &mut staging)
            .expect_err("extra body rejected");
        assert_eq!(error.code, ProjectSourceBatchErrorCode::ExtraBody);

        let mut staging = ProjectResourceStaging::new();
        let (mut duplicate, _) = valid_batch(&mut staging);
        duplicate.bodies.push(duplicate.bodies[0].clone());
        let error = validate_runtime_project_source_batch(&duplicate, &mut staging)
            .expect_err("duplicate body rejected");
        assert_eq!(error.code, ProjectSourceBatchErrorCode::DuplicateBody);

        let mut staging = ProjectResourceStaging::new();
        let (mut wrong_hash, _) = valid_batch(&mut staging);
        let ProjectSourceBody::Inline { bytes, .. } = &mut wrong_hash.bodies[0] else {
            panic!("fixture first body is inline");
        };
        bytes.push(0);
        let error = validate_runtime_project_source_batch(&wrong_hash, &mut staging)
            .expect_err("wrong hash rejected");
        assert_eq!(error.code, ProjectSourceBatchErrorCode::ContentHashMismatch);
    }

    #[test]
    fn binary_role_requires_manifest_bound_current_resource_handle() {
        let mut staging = ProjectResourceStaging::new();
        let (mut inline_voxel, _) = valid_batch(&mut staging);
        inline_voxel.bodies.pop();
        inline_voxel.bodies.push(ProjectSourceBody::inline(
            "voxel/house.avox",
            b"voxel-house",
        ));
        let error = validate_runtime_project_source_batch(&inline_voxel, &mut staging)
            .expect_err("voxel binary inline rejected");
        assert_eq!(error.code, ProjectSourceBatchErrorCode::InlineBodyForbidden);
        assert_eq!(staging.staged_count(), 0, "rejection aborts transaction");

        let mut staging = ProjectResourceStaging::new();
        let (mut stale, _) = valid_batch(&mut staging);
        let ProjectSourceBody::Resource { resource, .. } =
            stale.bodies.last_mut().expect("resource fixture body")
        else {
            panic!("last body is resource");
        };
        resource.version += 1;
        let error = validate_runtime_project_source_batch(&stale, &mut staging)
            .expect_err("wrong version rejected");
        assert_eq!(
            error.code,
            ProjectSourceBatchErrorCode::ResourceVersionMismatch
        );
        assert_eq!(staging.staged_count(), 0, "stale rejection cleans staging");
    }

    #[test]
    fn oversized_inline_and_wrong_generation_reject_without_leaking_staging() {
        let mut staging = ProjectResourceStaging::new();
        let (mut oversized, _) = valid_batch(&mut staging);
        let ProjectSourceBody::Inline { bytes, .. } = &mut oversized.bodies[0] else {
            panic!("fixture first body is inline");
        };
        bytes.resize(PROJECT_SOURCE_INLINE_BODY_MAX_BYTES + 1, 0);
        let error = validate_runtime_project_source_batch(&oversized, &mut staging)
            .expect_err("oversized inline body rejected");
        assert_eq!(error.code, ProjectSourceBatchErrorCode::InlineBodyTooLarge);
        assert_eq!(staging.staged_count(), 0);

        let mut staging = ProjectResourceStaging::new();
        let (mut wrong_generation, _) = valid_batch(&mut staging);
        let ProjectSourceBody::Resource { resource, .. } =
            wrong_generation.bodies.last_mut().expect("resource body")
        else {
            panic!("fixture last body is resource");
        };
        resource.generation += 1;
        wrong_generation.resource_generation = Some(resource.generation);
        let error = validate_runtime_project_source_batch(&wrong_generation, &mut staging)
            .expect_err("wrong resource generation rejected");
        assert_eq!(
            error.code,
            ProjectSourceBatchErrorCode::ResourceGenerationMismatch
        );
        assert_eq!(staging.staged_count(), 0);
    }

    #[test]
    fn resource_from_another_manifest_is_rejected_and_cleaned() {
        let mut staging = ProjectResourceStaging::new();
        let (mut batch, _) = valid_batch(&mut staging);
        let mut other_manifest = source_fixture().0;
        other_manifest.project.name = Some("other-project".into());
        let other_json = encode(&other_manifest);
        let other_transaction = staging
            .begin_for_manifest(&other_json)
            .expect("other transaction");
        let other = staging
            .stage(
                other_transaction,
                "voxel/house.avox",
                b"voxel-house".to_vec(),
            )
            .expect("other resource");
        batch.bodies.pop();
        batch.resource_generation = Some(other_transaction.generation());
        batch
            .bodies
            .push(ProjectSourceBody::resource("voxel/house.avox", other));

        let error = validate_runtime_project_source_batch(&batch, &mut staging)
            .expect_err("cross-manifest handle rejected");
        assert_eq!(
            error.code,
            ProjectSourceBatchErrorCode::ResourceManifestMismatch
        );
        assert_eq!(
            staging.staged_count(),
            1,
            "only referenced generation is aborted"
        );
    }

    #[test]
    fn staged_resource_cannot_move_to_another_manifest_role() {
        let mut staging = ProjectResourceStaging::new();
        let (mut batch, _) = valid_batch(&mut staging);
        batch.bodies.remove(0);
        let ProjectSourceBody::Resource { path, .. } =
            batch.bodies.last_mut().expect("resource body")
        else {
            panic!("fixture last body is resource");
        };
        *path = "assets/lock.json".to_string();

        let error = validate_runtime_project_source_batch(&batch, &mut staging)
            .expect_err("manifest-path role substitution rejected");
        assert_eq!(
            error.code,
            ProjectSourceBatchErrorCode::ResourcePathMismatch
        );
        assert_eq!(staging.staged_count(), 0);
    }

    #[test]
    fn abort_discards_every_handle_in_a_transaction() {
        let (manifest, _) = source_fixture();
        let manifest_json = encode(&manifest);
        let mut staging = ProjectResourceStaging::new();
        let transaction = staging
            .begin_for_manifest(&manifest_json)
            .expect("transaction");
        staging
            .stage(transaction, "voxel/house.avox", vec![1, 2, 3])
            .expect("first resource");
        staging
            .stage(transaction, "voxel/house.avox", vec![4, 5, 6])
            .expect("second resource");
        assert_eq!(staging.abort(transaction), 2);
        assert_eq!(staging.staged_count(), 0);
    }

    #[test]
    fn malformed_manifest_never_opens_a_resource_transaction() {
        let mut staging = ProjectResourceStaging::new();
        let error = staging
            .begin_for_manifest("{\"bundleSchemaVersion\":2,\"garbage\":true}")
            .expect_err("malformed manifest rejected");
        assert_eq!(
            error.code,
            ProjectSourceBatchErrorCode::ManifestDecodeFailed
        );
        assert_eq!(staging.staged_count(), 0);
    }
}
