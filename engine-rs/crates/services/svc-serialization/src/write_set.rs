//! Canonical, Rust-owned ProjectBundle persistence candidates.
//!
//! Engine authority describes the complete next stored state. A trusted host may
//! apply that description to a staging directory, but it must first present the
//! exact prior store identity and may confirm publication only with the exact
//! next identity. This keeps filesystem access outside Engine core without
//! moving manifest closure or compare-and-swap authority into TypeScript.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    encode, manifest::is_canonical_relative_path, ArtifactEntry, BundleHash, ManifestError,
    ProjectBundleManifest,
};

pub const PROJECT_BUNDLE_MANIFEST_PATH: &str = "asha.project-bundle.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStoreIdentity {
    pub revision: u64,
    pub manifest_hash: BundleHash,
    pub content_set_hash: BundleHash,
    pub index_hash: Option<BundleHash>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectArtifactExpectation {
    pub path: String,
    pub content_hash: Option<BundleHash>,
}

impl ProjectStoreIdentity {
    pub fn from_manifest(
        revision: u64,
        manifest: &ProjectBundleManifest,
        index_hash: Option<BundleHash>,
    ) -> Result<Self, ManifestError> {
        manifest.validate()?;
        let canonical = manifest.canonical();
        let manifest_json = encode(&canonical);
        Ok(Self {
            revision,
            manifest_hash: BundleHash::of_str(&manifest_json),
            content_set_hash: content_set_hash(&canonical),
            index_hash,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalProjectWrite {
    path: String,
    bytes: Vec<u8>,
    content_hash: BundleHash,
}

impl CanonicalProjectWrite {
    pub fn new(path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        let path = path.into();
        let bytes = bytes.into();
        let content_hash = BundleHash::of(&bytes);
        Self {
            path,
            bytes,
            content_hash,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn content_hash(&self) -> BundleHash {
        self.content_hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalProjectMove {
    pub from: String,
    pub to: String,
    pub expected_content_hash: Option<BundleHash>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalProjectDelete {
    pub path: String,
    pub expected_content_hash: Option<BundleHash>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWriteSetDraft {
    pub next_manifest: ProjectBundleManifest,
    pub writes: Vec<CanonicalProjectWrite>,
    pub moves: Vec<CanonicalProjectMove>,
    pub deletes: Vec<CanonicalProjectDelete>,
    /// Optional host/tooling index. It is part of the atomic publication but is
    /// deliberately absent from runtime closure.
    pub index_replacement: Option<CanonicalProjectWrite>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectWriteSetError {
    InvalidPriorManifest(ManifestError),
    InvalidNextManifest(ManifestError),
    RevisionOverflow,
    InvalidPath(String),
    DuplicateTarget(String),
    ConflictingOperation(String),
    MissingPriorArtifact(String),
    MissingNextArtifact(String),
    HashMismatch(String),
    MetadataMismatch(String),
    UnaccountedPriorChange(String),
    UnaccountedNextChange(String),
    StaleStore,
    PublicationMismatch,
}

impl core::fmt::Display for ProjectWriteSetError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidPriorManifest(error) => {
                write!(formatter, "invalid prior manifest: {error}")
            }
            Self::InvalidNextManifest(error) => write!(formatter, "invalid next manifest: {error}"),
            Self::RevisionOverflow => write!(formatter, "project store revision overflowed"),
            Self::InvalidPath(path) => write!(formatter, "invalid project-relative path `{path}`"),
            Self::DuplicateTarget(path) => write!(formatter, "multiple writes target `{path}`"),
            Self::ConflictingOperation(path) => {
                write!(formatter, "conflicting project operations mention `{path}`")
            }
            Self::MissingPriorArtifact(path) => {
                write!(
                    formatter,
                    "operation references missing prior artifact `{path}`"
                )
            }
            Self::MissingNextArtifact(path) => {
                write!(
                    formatter,
                    "operation references missing next artifact `{path}`"
                )
            }
            Self::HashMismatch(path) => write!(formatter, "content hash mismatch for `{path}`"),
            Self::MetadataMismatch(path) => {
                write!(formatter, "artifact metadata mismatch for `{path}`")
            }
            Self::UnaccountedPriorChange(path) => {
                write!(
                    formatter,
                    "prior artifact change is not declared for `{path}`"
                )
            }
            Self::UnaccountedNextChange(path) => {
                write!(
                    formatter,
                    "next artifact change is not declared for `{path}`"
                )
            }
            Self::StaleStore => write!(formatter, "project store identity is stale"),
            Self::PublicationMismatch => {
                write!(
                    formatter,
                    "published project does not match the authorized candidate"
                )
            }
        }
    }
}

impl std::error::Error for ProjectWriteSetError {}

#[derive(Debug, PartialEq, Eq)]
pub struct ProjectWriteCandidate {
    expected_prior: ProjectStoreIdentity,
    expected_next: ProjectStoreIdentity,
    expected_prior_artifacts: Vec<ProjectArtifactExpectation>,
    expected_next_artifacts: Vec<ProjectArtifactExpectation>,
    manifest_json: String,
    writes: Vec<CanonicalProjectWrite>,
    moves: Vec<CanonicalProjectMove>,
    deletes: Vec<CanonicalProjectDelete>,
    index_replacement: Option<CanonicalProjectWrite>,
    asset_lock_path: String,
    candidate_hash: BundleHash,
}

impl ProjectWriteCandidate {
    pub fn build(
        prior_revision: u64,
        prior_manifest: &ProjectBundleManifest,
        prior_index_hash: Option<BundleHash>,
        draft: ProjectWriteSetDraft,
    ) -> Result<Self, ProjectWriteSetError> {
        prior_manifest
            .validate()
            .map_err(ProjectWriteSetError::InvalidPriorManifest)?;
        draft
            .next_manifest
            .validate()
            .map_err(ProjectWriteSetError::InvalidNextManifest)?;
        let next_revision = prior_revision
            .checked_add(1)
            .ok_or(ProjectWriteSetError::RevisionOverflow)?;
        validate_operations(prior_manifest, &draft)?;

        let asset_lock_path = draft.next_manifest.asset_lock.artifact.clone();
        let expected_prior =
            ProjectStoreIdentity::from_manifest(prior_revision, prior_manifest, prior_index_hash)
                .map_err(ProjectWriteSetError::InvalidPriorManifest)?;
        let next_index_hash = draft
            .index_replacement
            .as_ref()
            .map(CanonicalProjectWrite::content_hash)
            .or(prior_index_hash);
        let expected_next = ProjectStoreIdentity::from_manifest(
            next_revision,
            &draft.next_manifest,
            next_index_hash,
        )
        .map_err(ProjectWriteSetError::InvalidNextManifest)?;
        let manifest_json = encode(&draft.next_manifest);
        let candidate_hash = candidate_hash(
            &expected_prior,
            &expected_next,
            &manifest_json,
            &draft.writes,
            &draft.moves,
            &draft.deletes,
            draft.index_replacement.as_ref(),
        );

        Ok(Self {
            expected_prior,
            expected_next,
            expected_prior_artifacts: artifact_expectations(prior_manifest),
            expected_next_artifacts: artifact_expectations(&draft.next_manifest),
            manifest_json,
            writes: sorted_writes(draft.writes),
            moves: sorted_moves(draft.moves),
            deletes: sorted_deletes(draft.deletes),
            index_replacement: draft.index_replacement,
            asset_lock_path,
            candidate_hash,
        })
    }

    pub fn expected_prior(&self) -> &ProjectStoreIdentity {
        &self.expected_prior
    }

    pub fn expected_next(&self) -> &ProjectStoreIdentity {
        &self.expected_next
    }

    pub fn candidate_hash(&self) -> BundleHash {
        self.candidate_hash
    }

    /// Hashes the host must observe before touching staging. Cache entries with
    /// no manifest hash are optional and intentionally carry `None`.
    pub fn expected_prior_artifacts(&self) -> &[ProjectArtifactExpectation] {
        &self.expected_prior_artifacts
    }

    /// Hashes the host must observe in staging before atomic publication.
    pub fn expected_next_artifacts(&self) -> &[ProjectArtifactExpectation] {
        &self.expected_next_artifacts
    }

    pub fn manifest_json(&self) -> &str {
        &self.manifest_json
    }

    pub fn writes(&self) -> &[CanonicalProjectWrite] {
        &self.writes
    }

    pub fn moves(&self) -> &[CanonicalProjectMove] {
        &self.moves
    }

    pub fn deletes(&self) -> &[CanonicalProjectDelete] {
        &self.deletes
    }

    pub fn index_replacement(&self) -> Option<&CanonicalProjectWrite> {
        self.index_replacement.as_ref()
    }

    pub fn asset_lock_replacement(&self) -> Option<&CanonicalProjectWrite> {
        self.writes
            .iter()
            .find(|write| write.path == self.asset_lock_path)
    }

    /// Consume the candidate only when the host's pre-write observation is an
    /// exact match. No filesystem mutation should happen before this succeeds.
    pub fn authorize(
        self,
        observed: &ProjectStoreIdentity,
    ) -> Result<AuthorizedProjectWriteCandidate, ProjectWriteSetError> {
        if observed != &self.expected_prior {
            return Err(ProjectWriteSetError::StaleStore);
        }
        Ok(AuthorizedProjectWriteCandidate { candidate: self })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AuthorizedProjectWriteCandidate {
    candidate: ProjectWriteCandidate,
}

impl AuthorizedProjectWriteCandidate {
    pub fn candidate(&self) -> &ProjectWriteCandidate {
        &self.candidate
    }

    /// Consume authorization after atomic publication. Partial staging or a
    /// failed rename cannot manufacture the complete next identity.
    pub fn confirm(
        self,
        published: &ProjectStoreIdentity,
    ) -> Result<ProjectWriteConfirmation, ProjectWriteSetError> {
        if published != &self.candidate.expected_next {
            return Err(ProjectWriteSetError::PublicationMismatch);
        }
        Ok(ProjectWriteConfirmation {
            candidate_hash: self.candidate.candidate_hash,
            stored: published.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWriteConfirmation {
    pub candidate_hash: BundleHash,
    pub stored: ProjectStoreIdentity,
}

fn validate_operations(
    prior_manifest: &ProjectBundleManifest,
    draft: &ProjectWriteSetDraft,
) -> Result<(), ProjectWriteSetError> {
    let prior = artifact_map(prior_manifest);
    let next = artifact_map(&draft.next_manifest);
    let writes = draft
        .writes
        .iter()
        .map(|write| (write.path.as_str(), write))
        .collect::<BTreeMap<_, _>>();
    if writes.len() != draft.writes.len() {
        return Err(duplicate_write_path(&draft.writes));
    }

    let mut mentioned_sources = BTreeSet::new();
    let mut mentioned_targets = BTreeSet::new();
    for write in &draft.writes {
        validate_path(&write.path)?;
        if !mentioned_targets.insert(write.path.as_str()) {
            return Err(ProjectWriteSetError::DuplicateTarget(write.path.clone()));
        }
        let entry = next
            .get(write.path.as_str())
            .ok_or_else(|| ProjectWriteSetError::MissingNextArtifact(write.path.clone()))?;
        if entry
            .content_hash
            .is_some_and(|hash| hash != write.content_hash)
        {
            return Err(ProjectWriteSetError::HashMismatch(write.path.clone()));
        }
    }
    for movement in &draft.moves {
        validate_path(&movement.from)?;
        validate_path(&movement.to)?;
        if !mentioned_sources.insert(movement.from.as_str())
            || !mentioned_targets.insert(movement.to.as_str())
        {
            return Err(ProjectWriteSetError::ConflictingOperation(
                movement.from.clone(),
            ));
        }
        let source = prior
            .get(movement.from.as_str())
            .ok_or_else(|| ProjectWriteSetError::MissingPriorArtifact(movement.from.clone()))?;
        let target = next
            .get(movement.to.as_str())
            .ok_or_else(|| ProjectWriteSetError::MissingNextArtifact(movement.to.clone()))?;
        if source.content_hash != movement.expected_content_hash {
            return Err(ProjectWriteSetError::HashMismatch(movement.from.clone()));
        }
        if source.class != target.class
            || source.role != target.role
            || source.content_hash != target.content_hash
        {
            return Err(ProjectWriteSetError::MetadataMismatch(movement.to.clone()));
        }
    }
    for deletion in &draft.deletes {
        validate_path(&deletion.path)?;
        if !mentioned_sources.insert(deletion.path.as_str()) {
            return Err(ProjectWriteSetError::ConflictingOperation(
                deletion.path.clone(),
            ));
        }
        let source = prior
            .get(deletion.path.as_str())
            .ok_or_else(|| ProjectWriteSetError::MissingPriorArtifact(deletion.path.clone()))?;
        if source.content_hash != deletion.expected_content_hash {
            return Err(ProjectWriteSetError::HashMismatch(deletion.path.clone()));
        }
        if next.contains_key(deletion.path.as_str()) {
            return Err(ProjectWriteSetError::ConflictingOperation(
                deletion.path.clone(),
            ));
        }
    }
    if let Some(index) = &draft.index_replacement {
        validate_path(&index.path)?;
        if index.path == PROJECT_BUNDLE_MANIFEST_PATH || next.contains_key(index.path.as_str()) {
            return Err(ProjectWriteSetError::ConflictingOperation(
                index.path.clone(),
            ));
        }
    }

    for (path, prior_entry) in &prior {
        match next.get(path) {
            Some(next_entry) if *prior_entry == *next_entry => {}
            Some(_) if writes.contains_key(path) => {}
            Some(_) => {
                return Err(ProjectWriteSetError::UnaccountedPriorChange(
                    (*path).to_owned(),
                ));
            }
            None if mentioned_sources.contains(path) => {}
            None => {
                return Err(ProjectWriteSetError::UnaccountedPriorChange(
                    (*path).to_owned(),
                ));
            }
        }
    }
    for (path, next_entry) in &next {
        match prior.get(path) {
            Some(prior_entry) if *prior_entry == *next_entry => {}
            Some(_) if writes.contains_key(path) => {}
            None if mentioned_targets.contains(path) => {}
            _ => {
                return Err(ProjectWriteSetError::UnaccountedNextChange(
                    (*path).to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn artifact_map(manifest: &ProjectBundleManifest) -> BTreeMap<&str, &ArtifactEntry> {
    manifest
        .artifacts
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect()
}

fn artifact_expectations(manifest: &ProjectBundleManifest) -> Vec<ProjectArtifactExpectation> {
    manifest
        .canonical()
        .artifacts
        .into_iter()
        .map(|artifact| ProjectArtifactExpectation {
            path: artifact.path,
            content_hash: artifact.content_hash,
        })
        .collect()
}

fn duplicate_write_path(writes: &[CanonicalProjectWrite]) -> ProjectWriteSetError {
    let mut seen = BTreeSet::new();
    let duplicate = writes
        .iter()
        .find(|write| !seen.insert(write.path.as_str()))
        .map(|write| write.path.clone())
        .unwrap_or_else(|| "<unknown>".to_owned());
    ProjectWriteSetError::DuplicateTarget(duplicate)
}

fn validate_path(path: &str) -> Result<(), ProjectWriteSetError> {
    if is_canonical_relative_path(path) && path != PROJECT_BUNDLE_MANIFEST_PATH {
        Ok(())
    } else {
        Err(ProjectWriteSetError::InvalidPath(path.to_owned()))
    }
}

fn content_set_hash(manifest: &ProjectBundleManifest) -> BundleHash {
    let mut canonical = String::new();
    for artifact in &manifest.canonical().artifacts {
        canonical.push_str(&artifact.path);
        canonical.push('\0');
        canonical.push_str(artifact.class.tag());
        canonical.push('\0');
        canonical.push_str(artifact.role.tag());
        canonical.push('\0');
        canonical.push_str(
            artifact
                .content_hash
                .map(BundleHash::to_hex)
                .as_deref()
                .unwrap_or("-"),
        );
        canonical.push('\n');
    }
    BundleHash::of_str(&canonical)
}

fn candidate_hash(
    prior: &ProjectStoreIdentity,
    next: &ProjectStoreIdentity,
    manifest_json: &str,
    writes: &[CanonicalProjectWrite],
    moves: &[CanonicalProjectMove],
    deletes: &[CanonicalProjectDelete],
    index: Option<&CanonicalProjectWrite>,
) -> BundleHash {
    let mut canonical = format!(
        "prior:{}:{}:{}:{:?}\nnext:{}:{}:{}:{:?}\nmanifest:{}\n",
        prior.revision,
        prior.manifest_hash.to_hex(),
        prior.content_set_hash.to_hex(),
        prior.index_hash.map(BundleHash::to_hex),
        next.revision,
        next.manifest_hash.to_hex(),
        next.content_set_hash.to_hex(),
        next.index_hash.map(BundleHash::to_hex),
        BundleHash::of_str(manifest_json).to_hex(),
    );
    for write in sorted_writes(writes.to_vec()) {
        canonical.push_str(&format!(
            "write:{}:{}:{}\n",
            write.path,
            write.content_hash.to_hex(),
            write.bytes.len()
        ));
    }
    for movement in sorted_moves(moves.to_vec()) {
        canonical.push_str(&format!(
            "move:{}:{}:{:?}\n",
            movement.from,
            movement.to,
            movement.expected_content_hash.map(BundleHash::to_hex)
        ));
    }
    for deletion in sorted_deletes(deletes.to_vec()) {
        canonical.push_str(&format!(
            "delete:{}:{:?}\n",
            deletion.path,
            deletion.expected_content_hash.map(BundleHash::to_hex)
        ));
    }
    if let Some(index) = index {
        canonical.push_str(&format!(
            "index:{}:{}:{}\n",
            index.path,
            index.content_hash.to_hex(),
            index.bytes.len()
        ));
    }
    BundleHash::of_str(&canonical)
}

fn sorted_writes(mut writes: Vec<CanonicalProjectWrite>) -> Vec<CanonicalProjectWrite> {
    writes.sort_by(|left, right| left.path.cmp(&right.path));
    writes
}

fn sorted_moves(mut moves: Vec<CanonicalProjectMove>) -> Vec<CanonicalProjectMove> {
    moves.sort_by(|left, right| left.from.cmp(&right.from).then(left.to.cmp(&right.to)));
    moves
}

fn sorted_deletes(mut deletes: Vec<CanonicalProjectDelete>) -> Vec<CanonicalProjectDelete> {
    deletes.sort_by(|left, right| left.path.cmp(&right.path));
    deletes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ArtifactRole, AssetLockSection, ProjectSection, SceneSection, BUNDLE_SCHEMA_VERSION,
        SUPPORTED_PROTOCOL_VERSION,
    };
    use core_ids::{ProjectId, SceneId};
    use std::collections::BTreeMap;

    fn manifest(scene_paths: &[(u64, &str, &[u8])]) -> ProjectBundleManifest {
        let lock = b"{\"assets\":[]}";
        ProjectBundleManifest {
            bundle_schema_version: BUNDLE_SCHEMA_VERSION,
            protocol_version: SUPPORTED_PROTOCOL_VERSION,
            project: ProjectSection {
                id: ProjectId::new(1),
                name: Some("write-set".to_owned()),
            },
            entry_scene: SceneId::new(scene_paths[0].0),
            scenes: scene_paths
                .iter()
                .map(|(id, path, _)| SceneSection {
                    id: SceneId::new(*id),
                    schema_version: 1,
                    artifact: (*path).to_owned(),
                })
                .collect(),
            asset_lock: AssetLockSection {
                artifact: "assets/lock.json".to_owned(),
                asset_count: 0,
            },
            generation_provenance: None,
            artifacts: scene_paths
                .iter()
                .map(|(_, path, bytes)| {
                    ArtifactEntry::durable(*path, ArtifactRole::SceneDocument, bytes)
                })
                .chain([ArtifactEntry::durable(
                    "assets/lock.json",
                    ArtifactRole::AssetLock,
                    lock,
                )])
                .collect(),
        }
        .canonical()
    }

    #[test]
    fn add_move_split_delete_and_rename_are_one_atomic_candidate() {
        let old_a = b"scene-a-old";
        let old_b = b"scene-b-old";
        let removed = b"scene-removed";
        let prior = manifest(&[
            (1, "scenes/a.json", old_a),
            (2, "scenes/b.json", old_b),
            (4, "scenes/removed.json", removed),
        ]);
        let new_a = b"scene-a-split";
        let new_c = b"scene-c-added";
        let next = manifest(&[
            (1, "scenes/a.json", new_a),
            (2, "scenes/archive/b-renamed.json", old_b),
            (3, "scenes/c.json", new_c),
        ]);
        let draft = ProjectWriteSetDraft {
            next_manifest: next.clone(),
            writes: vec![
                CanonicalProjectWrite::new("scenes/c.json", new_c),
                CanonicalProjectWrite::new("scenes/a.json", new_a),
            ],
            moves: vec![CanonicalProjectMove {
                from: "scenes/b.json".to_owned(),
                to: "scenes/archive/b-renamed.json".to_owned(),
                expected_content_hash: Some(BundleHash::of(old_b)),
            }],
            deletes: vec![CanonicalProjectDelete {
                path: "scenes/removed.json".to_owned(),
                expected_content_hash: Some(BundleHash::of(removed)),
            }],
            index_replacement: Some(CanonicalProjectWrite::new(
                ".asha/project-index.json",
                b"{\"scenes\":3}",
            )),
        };
        let candidate = ProjectWriteCandidate::build(7, &prior, None, draft).unwrap();
        assert_eq!(candidate.expected_prior.revision, 7);
        assert_eq!(candidate.expected_next.revision, 8);
        assert_eq!(candidate.writes[0].path(), "scenes/a.json");
        assert_eq!(candidate.writes[1].path(), "scenes/c.json");

        let stale = ProjectStoreIdentity::from_manifest(6, &prior, None).unwrap();
        let candidate = match candidate.authorize(&stale) {
            Err(ProjectWriteSetError::StaleStore) => ProjectWriteCandidate::build(
                7,
                &prior,
                None,
                ProjectWriteSetDraft {
                    next_manifest: next.clone(),
                    writes: vec![
                        CanonicalProjectWrite::new("scenes/a.json", new_a),
                        CanonicalProjectWrite::new("scenes/c.json", new_c),
                    ],
                    moves: vec![CanonicalProjectMove {
                        from: "scenes/b.json".to_owned(),
                        to: "scenes/archive/b-renamed.json".to_owned(),
                        expected_content_hash: Some(BundleHash::of(old_b)),
                    }],
                    deletes: vec![CanonicalProjectDelete {
                        path: "scenes/removed.json".to_owned(),
                        expected_content_hash: Some(BundleHash::of(removed)),
                    }],
                    index_replacement: Some(CanonicalProjectWrite::new(
                        ".asha/project-index.json",
                        b"{\"scenes\":3}",
                    )),
                },
            )
            .unwrap(),
            other => panic!("expected stale rejection, got {other:?}"),
        };
        let prior_identity = candidate.expected_prior().clone();
        let authorized = candidate.authorize(&prior_identity).unwrap();
        let partial = ProjectStoreIdentity::from_manifest(8, &prior, None).unwrap();
        assert_eq!(
            authorized.confirm(&partial),
            Err(ProjectWriteSetError::PublicationMismatch)
        );

        let candidate = ProjectWriteCandidate::build(
            7,
            &prior,
            None,
            ProjectWriteSetDraft {
                next_manifest: next,
                writes: vec![
                    CanonicalProjectWrite::new("scenes/a.json", new_a),
                    CanonicalProjectWrite::new("scenes/c.json", new_c),
                ],
                moves: vec![CanonicalProjectMove {
                    from: "scenes/b.json".to_owned(),
                    to: "scenes/archive/b-renamed.json".to_owned(),
                    expected_content_hash: Some(BundleHash::of(old_b)),
                }],
                deletes: vec![CanonicalProjectDelete {
                    path: "scenes/removed.json".to_owned(),
                    expected_content_hash: Some(BundleHash::of(removed)),
                }],
                index_replacement: Some(CanonicalProjectWrite::new(
                    ".asha/project-index.json",
                    b"{\"scenes\":3}",
                )),
            },
        )
        .unwrap();
        let prior_identity = candidate.expected_prior().clone();
        let next_identity = candidate.expected_next().clone();
        let confirmation = candidate
            .authorize(&prior_identity)
            .unwrap()
            .confirm(&next_identity)
            .unwrap();
        assert_eq!(confirmation.stored.revision, 8);
    }

    #[test]
    fn explicit_delete_is_required_and_stale_hashes_fail_before_authorization() {
        let old_a = b"scene-a";
        let old_b = b"scene-b";
        let prior = manifest(&[(1, "scenes/a.json", old_a), (2, "scenes/b.json", old_b)]);
        let next = manifest(&[(1, "scenes/a.json", old_a)]);
        let missing_delete = ProjectWriteCandidate::build(
            1,
            &prior,
            None,
            ProjectWriteSetDraft {
                next_manifest: next.clone(),
                writes: Vec::new(),
                moves: Vec::new(),
                deletes: Vec::new(),
                index_replacement: None,
            },
        );
        assert_eq!(
            missing_delete,
            Err(ProjectWriteSetError::UnaccountedPriorChange(
                "scenes/b.json".to_owned()
            ))
        );
        let wrong_hash = ProjectWriteCandidate::build(
            1,
            &prior,
            None,
            ProjectWriteSetDraft {
                next_manifest: next,
                writes: Vec::new(),
                moves: Vec::new(),
                deletes: vec![CanonicalProjectDelete {
                    path: "scenes/b.json".to_owned(),
                    expected_content_hash: Some(BundleHash::of(b"stale")),
                }],
                index_replacement: None,
            },
        );
        assert_eq!(
            wrong_hash,
            Err(ProjectWriteSetError::HashMismatch(
                "scenes/b.json".to_owned()
            ))
        );
    }

    #[test]
    fn candidate_hash_covers_bytes_and_operation_shape() {
        let prior = manifest(&[(1, "scenes/a.json", b"old")]);
        let next_a = manifest(&[(1, "scenes/a.json", b"new-a")]);
        let next_b = manifest(&[(1, "scenes/a.json", b"new-b")]);
        let build = |next, bytes: &'static [u8]| {
            ProjectWriteCandidate::build(
                0,
                &prior,
                None,
                ProjectWriteSetDraft {
                    next_manifest: next,
                    writes: vec![CanonicalProjectWrite::new("scenes/a.json", bytes)],
                    moves: Vec::new(),
                    deletes: Vec::new(),
                    index_replacement: None,
                },
            )
            .unwrap()
        };
        assert_ne!(
            build(next_a, b"new-a").candidate_hash(),
            build(next_b, b"new-b").candidate_hash()
        );
    }

    #[test]
    fn role_metadata_cannot_be_changed_by_a_move() {
        let prior = manifest(&[(1, "scenes/a.json", b"scene")]);
        let mut next = manifest(&[(1, "scenes/moved.json", b"scene")]);
        next.artifacts[1].role = ArtifactRole::Resource("resource:other".to_owned());
        let result = ProjectWriteCandidate::build(
            0,
            &prior,
            None,
            ProjectWriteSetDraft {
                next_manifest: next,
                writes: Vec::new(),
                moves: vec![CanonicalProjectMove {
                    from: "scenes/a.json".to_owned(),
                    to: "scenes/moved.json".to_owned(),
                    expected_content_hash: Some(BundleHash::of(b"scene")),
                }],
                deletes: Vec::new(),
                index_replacement: None,
            },
        );
        assert!(matches!(
            result,
            Err(ProjectWriteSetError::InvalidNextManifest(_))
                | Err(ProjectWriteSetError::MetadataMismatch(_))
        ));
    }

    #[test]
    fn asset_lock_is_a_runtime_artifact_not_a_tooling_write_policy() {
        let prior = manifest(&[(1, "scenes/a.json", b"scene")]);
        let mut next = prior.clone();
        next.artifacts
            .iter_mut()
            .find(|entry| entry.role == ArtifactRole::AssetLock)
            .unwrap()
            .content_hash = Some(BundleHash::of(b"new-lock"));
        let candidate = ProjectWriteCandidate::build(
            0,
            &prior,
            None,
            ProjectWriteSetDraft {
                next_manifest: next,
                writes: vec![CanonicalProjectWrite::new("assets/lock.json", b"new-lock")],
                moves: Vec::new(),
                deletes: Vec::new(),
                index_replacement: None,
            },
        )
        .unwrap();
        assert_eq!(
            candidate.asset_lock_replacement().map(|write| write.path()),
            Some("assets/lock.json")
        );
    }

    #[test]
    fn saved_file_set_reloads_through_the_unchanged_source_admission_path() {
        let prior = manifest(&[
            (1, "scenes/main.json", b"main-old"),
            (2, "scenes/remove.json", b"remove-old"),
        ]);
        let next = manifest(&[
            (1, "scenes/archive/main.json", b"main-old"),
            (3, "scenes/added.json", b"added-new"),
        ]);
        let candidate = ProjectWriteCandidate::build(
            0,
            &prior,
            None,
            ProjectWriteSetDraft {
                next_manifest: next,
                writes: vec![CanonicalProjectWrite::new(
                    "scenes/added.json",
                    b"added-new",
                )],
                moves: vec![CanonicalProjectMove {
                    from: "scenes/main.json".to_owned(),
                    to: "scenes/archive/main.json".to_owned(),
                    expected_content_hash: Some(BundleHash::of(b"main-old")),
                }],
                deletes: vec![CanonicalProjectDelete {
                    path: "scenes/remove.json".to_owned(),
                    expected_content_hash: Some(BundleHash::of(b"remove-old")),
                }],
                index_replacement: None,
            },
        )
        .unwrap();
        let prior_identity = candidate.expected_prior().clone();
        let next_identity = candidate.expected_next().clone();
        let authorized = candidate.authorize(&prior_identity).unwrap();
        let candidate = authorized.candidate();

        let mut stored = BTreeMap::from([
            ("assets/lock.json".to_owned(), b"{\"assets\":[]}".to_vec()),
            ("scenes/main.json".to_owned(), b"main-old".to_vec()),
            ("scenes/remove.json".to_owned(), b"remove-old".to_vec()),
        ]);
        for movement in candidate.moves() {
            let bytes = stored.remove(&movement.from).unwrap();
            stored.insert(movement.to.clone(), bytes);
        }
        for write in candidate.writes() {
            stored.insert(write.path().to_owned(), write.bytes().to_vec());
        }
        for deletion in candidate.deletes() {
            stored.remove(&deletion.path);
        }
        let batch = crate::RuntimeProjectSourceBatch {
            manifest_json: candidate.manifest_json().to_owned(),
            resource_generation: None,
            bodies: stored
                .into_iter()
                .map(|(path, bytes)| crate::ProjectSourceBody::inline(path, bytes))
                .collect(),
        };
        let mut staging = crate::ProjectResourceStaging::default();
        let admitted = crate::validate_runtime_project_source_batch(&batch, &mut staging)
            .unwrap()
            .commit(&mut staging)
            .unwrap();
        assert_eq!(
            admitted.paths().collect::<Vec<_>>(),
            vec![
                "assets/lock.json",
                "scenes/added.json",
                "scenes/archive/main.json"
            ]
        );
        assert_eq!(admitted.body("scenes/added.json"), Some(&b"added-new"[..]));
        authorized.confirm(&next_identity).unwrap();
    }
}
