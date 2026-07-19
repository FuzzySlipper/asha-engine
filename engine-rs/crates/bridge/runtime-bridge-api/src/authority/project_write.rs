use super::*;

impl EngineBridge {
    pub(super) fn prepare_project_write_authority(
        &mut self,
        request: ProjectWritePrepareRequest,
    ) -> BridgeResult<ProjectWritePrepareReceipt> {
        self.require_workspace_authoring_revision(
            "prepare_project_write",
            &request.expected_workspace_id,
            request.expected_generation,
            request.expected_working_revision,
        )?;

        let (canonical_files, scenes) = {
            let authority = self.require_open_workspace_authoring_mut("prepare_project_write")?;
            let Some(content) = authority.project_content_current.as_ref() else {
                return Ok(project_write_rejection(
                    "projectContentNotLoaded",
                    None::<String>,
                    "prepareProjectWrite requires the manifest-discovered ProjectContent set to be decoded first",
                ));
            };
            (
                content.result().canonical_files.clone(),
                authority.project_content_scenes.clone(),
            )
        };

        let prior_manifest = match svc_serialization::decode(&request.prior_manifest_json) {
            Ok(manifest) => manifest,
            Err(error) => {
                return Ok(project_write_rejection(
                    "invalidPriorManifest",
                    Some("priorManifestJson"),
                    error.to_string(),
                ))
            }
        };
        let observed_prior = match service_store_identity(&request.observed_prior) {
            Ok(identity) => identity,
            Err(diagnostic) => {
                return Ok(ProjectWritePrepareReceipt {
                    accepted: false,
                    candidate: None,
                    diagnostics: vec![diagnostic],
                })
            }
        };
        let draft = match project_write_draft(
            &prior_manifest,
            &canonical_files,
            &scenes,
            &request.relocations,
        ) {
            Ok(draft) => draft,
            Err(diagnostic) => {
                return Ok(ProjectWritePrepareReceipt {
                    accepted: false,
                    candidate: None,
                    diagnostics: vec![diagnostic],
                })
            }
        };
        let candidate = match svc_serialization::ProjectWriteCandidate::build(
            observed_prior.revision,
            &prior_manifest,
            observed_prior.index_hash,
            draft,
        ) {
            Ok(candidate) => candidate,
            Err(error) => {
                return Ok(project_write_rejection(
                    "invalidWriteSet",
                    None::<String>,
                    error.to_string(),
                ))
            }
        };
        let candidate_hash = candidate.candidate_hash().to_hex();
        let authorized = match candidate.authorize(&observed_prior) {
            Ok(candidate) => candidate,
            Err(error) => {
                return Ok(project_write_rejection(
                    "staleStore",
                    None::<String>,
                    error.to_string(),
                ))
            }
        };
        let candidate_dto = self.project_write_candidate_dto(authorized.candidate())?;
        let authority = self.require_open_workspace_authoring_mut("prepare_project_write")?;
        authority.pending_project_write = Some(PendingProjectWriteCandidate {
            candidate_hash,
            working_revision: request.expected_working_revision,
            authorized,
        });
        authority.pending_save_candidate = None;
        Ok(ProjectWritePrepareReceipt {
            accepted: true,
            candidate: Some(candidate_dto),
            diagnostics: Vec::new(),
        })
    }

    pub(super) fn confirm_project_write_authority(
        &mut self,
        request: ProjectWriteConfirmRequest,
    ) -> BridgeResult<ProjectWriteConfirmReceipt> {
        self.require_workspace_authoring_revision(
            "confirm_project_write",
            &request.expected_workspace_id,
            request.expected_generation,
            request.expected_working_revision,
        )?;
        let published = match service_store_identity(&request.publication.published) {
            Ok(identity) => identity,
            Err(diagnostic) => {
                return Ok(ProjectWriteConfirmReceipt {
                    accepted: false,
                    stored: None,
                    diagnostics: vec![diagnostic],
                })
            }
        };
        {
            let authority = self.require_open_workspace_authoring_mut("confirm_project_write")?;
            let Some(pending) = authority.pending_project_write.as_ref() else {
                return Ok(project_write_confirmation_rejection(
                    "missingCandidate",
                    "no Rust-authorized project write candidate is pending",
                ));
            };
            if pending.candidate_hash != request.publication.candidate_hash
                || pending.working_revision != request.expected_working_revision
            {
                return Ok(project_write_confirmation_rejection(
                    "staleCandidate",
                    "publication does not match the current workspace revision and candidate",
                ));
            }
            if pending.authorized.candidate().expected_next() != &published {
                return Ok(project_write_confirmation_rejection(
                    "publicationMismatch",
                    "published project identity does not match the authorized Rust candidate",
                ));
            }
        }

        let pending = self
            .require_open_workspace_authoring_mut("confirm_project_write")?
            .pending_project_write
            .take()
            .expect("pending candidate checked above");
        let confirmation = pending.authorized.confirm(&published).map_err(|error| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("authorized project write confirmation diverged: {error}"),
            )
        })?;
        let stored = protocol_store_identity(&confirmation.stored);
        let authority = self.require_open_workspace_authoring_mut("confirm_project_write")?;
        authority.stored_revision = authority.working_revision;
        authority.last_stored_canonical_json_hash = Some(confirmation.candidate_hash.to_hex());
        authority.pending_save_candidate = None;
        authority.pending_procedural_environment = None;
        Ok(ProjectWriteConfirmReceipt {
            accepted: true,
            stored: Some(stored),
            diagnostics: Vec::new(),
        })
    }

    fn project_write_candidate_dto(
        &mut self,
        candidate: &svc_serialization::ProjectWriteCandidate,
    ) -> BridgeResult<ProjectWriteCandidate> {
        let writes = candidate
            .writes()
            .iter()
            .map(|write| self.project_write_dto(write))
            .collect::<BridgeResult<Vec<_>>>()?;
        let index_replacement = candidate
            .index_replacement()
            .map(|write| self.project_write_dto(write))
            .transpose()?;
        Ok(ProjectWriteCandidate {
            candidate_hash: candidate.candidate_hash().to_hex(),
            expected_prior: protocol_store_identity(candidate.expected_prior()),
            expected_next: protocol_store_identity(candidate.expected_next()),
            expected_prior_artifacts: candidate
                .expected_prior_artifacts()
                .iter()
                .map(protocol_artifact_expectation)
                .collect(),
            expected_next_artifacts: candidate
                .expected_next_artifacts()
                .iter()
                .map(protocol_artifact_expectation)
                .collect(),
            manifest_json: candidate.manifest_json().to_owned(),
            writes,
            moves: candidate
                .moves()
                .iter()
                .map(|movement| protocol_project_bundle::CanonicalProjectMove {
                    from: movement.from.clone(),
                    to: movement.to.clone(),
                    expected_content_hash: movement
                        .expected_content_hash
                        .map(svc_serialization::BundleHash::to_hex),
                })
                .collect(),
            deletes: candidate
                .deletes()
                .iter()
                .map(|deletion| protocol_project_bundle::CanonicalProjectDelete {
                    path: deletion.path.clone(),
                    expected_content_hash: deletion
                        .expected_content_hash
                        .map(svc_serialization::BundleHash::to_hex),
                })
                .collect(),
            index_replacement,
        })
    }

    fn project_write_dto(
        &mut self,
        write: &svc_serialization::CanonicalProjectWrite,
    ) -> BridgeResult<protocol_project_bundle::CanonicalProjectWrite> {
        let byte_len = u64::try_from(write.bytes().len()).map_err(|_| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "canonical project write exceeds the supported buffer length",
            )
        })?;
        let handle = self.voxel.buffers.create(
            buffer_provider::BufferKind::Opaque,
            buffer_provider::BufferLifetime::Manual,
            None,
            write.bytes().to_vec(),
        );
        Ok(protocol_project_bundle::CanonicalProjectWrite {
            path: write.path().to_owned(),
            content_hash: write.content_hash().to_hex(),
            resource: protocol_project_bundle::ProjectWriteResourceRef {
                handle: handle.raw(),
                version: 1,
                byte_len,
            },
        })
    }
}

fn project_write_draft(
    prior: &svc_serialization::ProjectBundleManifest,
    canonical_files: &[protocol_project_content::ProjectContentCanonicalFileDto],
    scenes: &BTreeMap<u64, protocol_scene::FlatSceneDocumentDto>,
    relocations: &[ProjectArtifactRelocation],
) -> Result<svc_serialization::ProjectWriteSetDraft, ProjectWriteDiagnostic> {
    let prior_by_path = prior
        .artifacts
        .iter()
        .map(|artifact| (artifact.path.clone(), artifact.clone()))
        .collect::<BTreeMap<_, _>>();
    let relocation_by_source = relocation_map(relocations, &prior_by_path)?;
    let relocation_targets = relocation_by_source
        .values()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut next = prior.clone();
    next.artifacts.retain(|artifact| {
        !is_project_content_role(&artifact.role)
            && artifact.role != svc_serialization::ArtifactRole::SceneDocument
    });
    let mut generated_bodies = BTreeMap::<String, Vec<u8>>::new();

    for file in canonical_files {
        let path = file.document_id.clone();
        let bytes = file.canonical_json.as_bytes().to_vec();
        let role = prior_by_path
            .get(&path)
            .filter(|artifact| is_project_content_role(&artifact.role))
            .map(|artifact| artifact.role.clone())
            .unwrap_or(svc_serialization::ArtifactRole::ProjectContent);
        next.artifacts
            .push(svc_serialization::ArtifactEntry::durable(
                path.clone(),
                role,
                &bytes,
            ));
        generated_bodies.insert(path, bytes);
    }

    if scenes.len() != next.scenes.len() {
        return Err(write_diagnostic(
            "incompleteSceneSet",
            Some("workspace.scenes"),
            "every manifest scene must be opened in the workspace before preparing a project write",
        ));
    }
    for section in &mut next.scenes {
        let Some(scene) = scenes.get(&section.id.raw()) else {
            return Err(write_diagnostic(
                "missingScene",
                Some(section.artifact.as_str()),
                format!(
                    "scene {} is not open in workspace authoring",
                    section.id.raw()
                ),
            ));
        };
        let canonical = EngineBridge::scene_document_from_dto(scene.clone()).map_err(|error| {
            write_diagnostic(
                "invalidScene",
                Some(section.artifact.as_str()),
                error.to_string(),
            )
        })?;
        let bytes = core_scene::encode(&canonical).into_bytes();
        let prior_path = section.artifact.clone();
        let next_path = relocation_by_source
            .get(&prior_path)
            .cloned()
            .unwrap_or(prior_path);
        section.artifact = next_path.clone();
        next.artifacts
            .push(svc_serialization::ArtifactEntry::durable(
                next_path.clone(),
                svc_serialization::ArtifactRole::SceneDocument,
                &bytes,
            ));
        generated_bodies.insert(next_path, bytes);
    }

    for artifact in &mut next.artifacts {
        if generated_bodies.contains_key(&artifact.path) {
            continue;
        }
        if let Some(target) = relocation_by_source.get(&artifact.path) {
            artifact.path = target.clone();
        }
    }
    if let Some(target) = relocation_by_source.get(&next.asset_lock.artifact) {
        next.asset_lock.artifact = target.clone();
    }
    next = next.canonical();
    let next_by_path = next
        .artifacts
        .iter()
        .map(|artifact| (artifact.path.clone(), artifact.clone()))
        .collect::<BTreeMap<_, _>>();

    let mut moves = Vec::new();
    let mut moved_sources = BTreeSet::new();
    let mut moved_targets = BTreeSet::new();
    for (from, to) in &relocation_by_source {
        let prior_artifact = prior_by_path.get(from).expect("relocation was validated");
        let Some(next_artifact) = next_by_path.get(to) else {
            return Err(write_diagnostic(
                "relocationNotRetained",
                Some(from.as_str()),
                "relocated artifact is not present in the next canonical closure",
            ));
        };
        if prior_artifact.class == next_artifact.class
            && prior_artifact.role == next_artifact.role
            && prior_artifact.content_hash == next_artifact.content_hash
        {
            moves.push(svc_serialization::CanonicalProjectMove {
                from: from.clone(),
                to: to.clone(),
                expected_content_hash: prior_artifact.content_hash,
            });
            moved_sources.insert(from.clone());
            moved_targets.insert(to.clone());
        } else if !generated_bodies.contains_key(to) {
            return Err(write_diagnostic(
                "changedRelocationRequiresCanonicalBody",
                Some(to.as_str()),
                "a changed relocated artifact must be owned by a loaded Rust codec",
            ));
        }
    }

    let mut writes = Vec::new();
    for (path, bytes) in generated_bodies {
        if moved_targets.contains(&path) {
            continue;
        }
        let next_artifact = next_by_path
            .get(&path)
            .expect("generated body has a next manifest row");
        let unchanged = prior_by_path.get(&path).is_some_and(|prior_artifact| {
            prior_artifact.class == next_artifact.class
                && prior_artifact.role == next_artifact.role
                && prior_artifact.content_hash == next_artifact.content_hash
        });
        if !unchanged {
            writes.push(svc_serialization::CanonicalProjectWrite::new(path, bytes));
        }
    }

    let deletes = prior_by_path
        .iter()
        .filter(|(path, _)| !next_by_path.contains_key(*path) && !moved_sources.contains(*path))
        .map(
            |(path, artifact)| svc_serialization::CanonicalProjectDelete {
                path: path.clone(),
                expected_content_hash: artifact.content_hash,
            },
        )
        .collect();
    for (path, artifact) in &next_by_path {
        if prior_by_path.contains_key(path) || moved_targets.contains(path) {
            continue;
        }
        if !writes.iter().any(|write| write.path() == path) {
            return Err(write_diagnostic(
                "missingCanonicalBody",
                Some(path.as_str()),
                format!(
                    "new {:?} artifact has no Rust-owned canonical body",
                    artifact.role
                ),
            ));
        }
    }
    if relocation_targets.len() != relocation_by_source.len() {
        return Err(write_diagnostic(
            "duplicateRelocationTarget",
            None::<String>,
            "artifact relocation targets must be unique",
        ));
    }
    Ok(svc_serialization::ProjectWriteSetDraft {
        next_manifest: next,
        writes,
        moves,
        deletes,
        index_replacement: None,
    })
}

fn relocation_map(
    relocations: &[ProjectArtifactRelocation],
    prior: &BTreeMap<String, svc_serialization::ArtifactEntry>,
) -> Result<BTreeMap<String, String>, ProjectWriteDiagnostic> {
    let mut values = BTreeMap::new();
    let mut targets = BTreeSet::new();
    for relocation in relocations {
        let Some(artifact) = prior.get(&relocation.from) else {
            return Err(write_diagnostic(
                "unknownRelocationSource",
                Some(relocation.from.as_str()),
                "relocation source is not in the prior ProjectBundle closure",
            ));
        };
        if is_project_content_role(&artifact.role) {
            return Err(write_diagnostic(
                "projectContentPathOwnedByDocument",
                Some(relocation.from.as_str()),
                "ProjectContent paths are changed through their typed document id, not a second relocation list",
            ));
        }
        if values
            .insert(relocation.from.clone(), relocation.to.clone())
            .is_some()
        {
            return Err(write_diagnostic(
                "duplicateRelocationSource",
                Some(relocation.from.as_str()),
                "relocation source is repeated",
            ));
        }
        if !targets.insert(relocation.to.clone()) {
            return Err(write_diagnostic(
                "duplicateRelocationTarget",
                Some(relocation.to.as_str()),
                "relocation target is repeated",
            ));
        }
        if prior.contains_key(&relocation.to)
            && !relocations
                .iter()
                .any(|candidate| candidate.from == relocation.to)
        {
            return Err(write_diagnostic(
                "occupiedRelocationTarget",
                Some(relocation.to.as_str()),
                "relocation target is already occupied by a retained artifact",
            ));
        }
    }
    Ok(values)
}

fn is_project_content_role(role: &svc_serialization::ArtifactRole) -> bool {
    matches!(
        role,
        svc_serialization::ArtifactRole::ProjectContent
            | svc_serialization::ArtifactRole::PrefabRegistry
            | svc_serialization::ArtifactRole::EntityDefinitionCatalog
            | svc_serialization::ArtifactRole::MaterialCatalog
    )
}

fn service_store_identity(
    identity: &ProjectStoreIdentity,
) -> Result<svc_serialization::ProjectStoreIdentity, ProjectWriteDiagnostic> {
    Ok(svc_serialization::ProjectStoreIdentity {
        revision: identity.revision,
        manifest_hash: parse_store_hash(&identity.manifest_hash, "observedPrior.manifestHash")?,
        content_set_hash: parse_store_hash(
            &identity.content_set_hash,
            "observedPrior.contentSetHash",
        )?,
        index_hash: identity
            .index_hash
            .as_deref()
            .map(|hash| parse_store_hash(hash, "observedPrior.indexHash"))
            .transpose()?,
    })
}

fn parse_store_hash(
    value: &str,
    path: &'static str,
) -> Result<svc_serialization::BundleHash, ProjectWriteDiagnostic> {
    svc_serialization::BundleHash::parse_hex(value).ok_or_else(|| {
        write_diagnostic(
            "invalidStoreHash",
            Some(path),
            "project store hashes must be 16 lowercase hexadecimal digits",
        )
    })
}

fn protocol_store_identity(
    identity: &svc_serialization::ProjectStoreIdentity,
) -> ProjectStoreIdentity {
    ProjectStoreIdentity {
        revision: identity.revision,
        manifest_hash: identity.manifest_hash.to_hex(),
        content_set_hash: identity.content_set_hash.to_hex(),
        index_hash: identity
            .index_hash
            .map(svc_serialization::BundleHash::to_hex),
    }
}

fn protocol_artifact_expectation(
    expectation: &svc_serialization::ProjectArtifactExpectation,
) -> protocol_project_bundle::ProjectArtifactExpectation {
    protocol_project_bundle::ProjectArtifactExpectation {
        path: expectation.path.clone(),
        content_hash: expectation
            .content_hash
            .map(svc_serialization::BundleHash::to_hex),
    }
}

fn project_write_rejection(
    code: impl Into<String>,
    path: Option<impl Into<String>>,
    message: impl Into<String>,
) -> ProjectWritePrepareReceipt {
    ProjectWritePrepareReceipt {
        accepted: false,
        candidate: None,
        diagnostics: vec![write_diagnostic(code, path, message)],
    }
}

fn project_write_confirmation_rejection(
    code: impl Into<String>,
    message: impl Into<String>,
) -> ProjectWriteConfirmReceipt {
    ProjectWriteConfirmReceipt {
        accepted: false,
        stored: None,
        diagnostics: vec![write_diagnostic(code, None::<String>, message)],
    }
}

fn write_diagnostic(
    code: impl Into<String>,
    path: Option<impl Into<String>>,
    message: impl Into<String>,
) -> ProjectWriteDiagnostic {
    ProjectWriteDiagnostic {
        code: code.into(),
        path: path.map(Into::into),
        message: message.into(),
    }
}
