use super::*;

pub type RuntimeProjectLifecycleVersion = protocol_project_bundle::RuntimeProjectLifecycleVersion;
pub type RuntimeProjectActivationReceipt = protocol_project_bundle::ActiveRuntimeProjectIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProjectUnloadReceipt {
    pub project_id: u64,
    pub manifest_hash: String,
    pub lifecycle: RuntimeProjectLifecycleVersion,
}

#[derive(Debug)]
pub enum RuntimeProjectLoadError {
    NotInitialized,
    MissingStaticComposition,
    MissingAdmittedSource,
    AlreadyActive {
        project_id: u64,
        lifecycle: RuntimeProjectLifecycleVersion,
    },
    NoActiveProject,
    StaleLifecycle {
        expected: RuntimeProjectLifecycleVersion,
        actual: RuntimeProjectLifecycleVersion,
    },
    Admission(gameplay_runtime_host::RuntimeProjectAdmissionReport),
    Activation(gameplay_runtime_host::GameplayRuntimeHostError),
    Domain {
        code: String,
        document_id: Option<String>,
        path: Option<String>,
        message: String,
    },
    Resource(String),
}

impl core::fmt::Display for RuntimeProjectLoadError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RuntimeProjectLoadError {}

impl EngineBridge {
    pub fn runtime_project_lifecycle_version(&self) -> RuntimeProjectLifecycleVersion {
        RuntimeProjectLifecycleVersion {
            generation: self.runtime_project.runtime_project_generation,
            revision: self.runtime_project.runtime_project_revision,
        }
    }

    pub fn active_runtime_project(&self) -> Option<RuntimeProjectActivationReceipt> {
        self.runtime_project
            .active_runtime_project
            .as_ref()
            .map(|active| RuntimeProjectActivationReceipt {
                project_id: active.project_id,
                manifest_hash: active.manifest_hash.clone(),
                admission_hash: active.admission_hash.clone(),
                content_set_hash: active.content_set_hash.clone(),
                composition_hash: active.composition_hash.clone(),
                entry_scene_id: active.entry_scene_id,
                scene_count: active.scene_count,
                entity_count: active.entity_count,
                voxel_asset_count: active.voxel_asset_count,
                voxel_bindings: active.voxel_bindings.clone(),
                lifecycle: self.runtime_project_lifecycle_version(),
            })
    }

    pub fn read_active_runtime_project_content_authority(
        &self,
    ) -> BridgeResult<ActiveRuntimeProjectContentReadoutDto> {
        self.require_initialized("read_active_runtime_project_content")?;
        let active = self
            .runtime_project
            .active_runtime_project
            .as_ref()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "read_active_runtime_project_content called without an active canonical project",
            )
            })?;
        let host = self.gameplay.static_gameplay_host.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "active canonical project is missing its gameplay host",
            )
        })?;
        let content = host
            .activated_project_content_readout()
            .cloned()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "active canonical project is missing its admitted content readout",
                )
            })?;
        let entry_scene = host.activated_entry_scene().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "active canonical project is missing its entry scene",
            )
        })?;
        let active_domains = match self.gameplay.static_project_domain_adapter {
            Some(RuntimeProjectDomainAdapter::Fps) => {
                let seed = self.gameplay.fps_seed.as_ref().ok_or_else(|| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::Internal,
                        "active FPS project is missing its Rust-owned domain seed",
                    )
                })?;
                vec![ActiveRuntimeProjectDomainReadoutDto {
                    kind: ActiveRuntimeProjectDomainKind::Fps,
                    entity_roles: seed
                        .definitions
                        .iter()
                        .map(|definition| ActiveRuntimeProjectEntityRoleReadoutDto {
                            entity: definition.entity.raw(),
                            role: match definition.role {
                                FpsRuntimeRole::Player => ActiveRuntimeProjectEntityRole::Player,
                                FpsRuntimeRole::Enemy => ActiveRuntimeProjectEntityRole::Enemy,
                                FpsRuntimeRole::Neutral => ActiveRuntimeProjectEntityRole::Neutral,
                            },
                        })
                        .collect(),
                }]
            }
            None => Vec::new(),
        };
        Ok(ActiveRuntimeProjectContentReadoutDto {
            project_id: active.project_id,
            manifest_hash: active.manifest_hash.clone(),
            content_set_hash: active.content_set_hash.clone(),
            entry_scene: Self::scene_document_dto(entry_scene),
            content,
            active_domains,
        })
    }

    /// Consume the complete admitted source closure and publish a new runtime
    /// authority graph only after linking, scene/bootstrap, gameplay, stored
    /// voxel collision, and initial projection all succeed in staging.
    pub fn activate_pending_runtime_project(
        &mut self,
        expected: RuntimeProjectLifecycleVersion,
    ) -> Result<RuntimeProjectActivationReceipt, RuntimeProjectLoadError> {
        let actual = self.runtime_project_lifecycle_version();
        if expected != actual {
            return self.reject_pending_runtime_project(RuntimeProjectLoadError::StaleLifecycle {
                expected,
                actual,
            });
        }
        if let Some(active) = &self.runtime_project.active_runtime_project {
            return self.reject_pending_runtime_project(RuntimeProjectLoadError::AlreadyActive {
                project_id: active.project_id,
                lifecycle: actual,
            });
        }
        let engine = match self.runtime_project.engine {
            Some(engine) => engine,
            None => {
                return self.reject_pending_runtime_project(RuntimeProjectLoadError::NotInitialized)
            }
        };
        let composition = match self.gameplay.static_gameplay_composition.clone() {
            Some(composition) => composition,
            None => {
                return self.reject_pending_runtime_project(
                    RuntimeProjectLoadError::MissingStaticComposition,
                )
            }
        };
        let domain_adapter = self.gameplay.static_project_domain_adapter;
        let source = match self.runtime_project.pending_project_source.take() {
            Some(source) => source,
            None => {
                return self
                    .reject_pending_runtime_project(RuntimeProjectLoadError::MissingAdmittedSource)
            }
        };
        self.runtime_project.project_resource_staging.reset();

        let admission =
            gameplay_runtime_host::compile_runtime_project_admission(source, composition.clone())
                .map_err(RuntimeProjectLoadError::Admission)?;
        let content_set_hash = admission.project_content_set_hash().to_owned();
        let composition_hash = admission.composition_registry_digest().to_owned();
        let scene_count = admission.scene_count() as u32;
        let mut gameplay_host =
            gameplay_runtime_host::GameplayRuntimeHost::activate_validated_project(admission)
                .map_err(RuntimeProjectLoadError::Activation)?;
        let identity = gameplay_host
            .activated_project_identity()
            .expect("validated activation retains project identity")
            .clone();
        let entry_scene = gameplay_host
            .activated_entry_scene()
            .expect("validated activation retains entry scene")
            .clone();
        let project_content = gameplay_host
            .activated_project_content_readout()
            .expect("validated activation retains project content")
            .clone();
        let installed_presentation =
            presentation_catalog::InstalledPresentationCatalog::from_documents(
                &project_content.documents,
            )
            .map_err(RuntimeProjectLoadError::Resource)?;
        if domain_adapter == Some(RuntimeProjectDomainAdapter::Fps)
            && (installed_presentation
                .audio(presentation_catalog::PRIMARY_FIRE_PRESENTATION_SIGNAL)
                .is_none()
                || installed_presentation
                    .particle(presentation_catalog::PRIMARY_FIRE_PRESENTATION_SIGNAL)
                    .is_none()
                || installed_presentation
                    .animation(presentation_catalog::PRIMARY_FIRE_ANIMATION_CUE)
                    .is_none())
        {
            return Err(RuntimeProjectLoadError::Resource(
                "FPS project content must bind typed audio and particle cues to `fps.primary-fire.accepted` and an animation cue named `fps.primary-fire.animation`"
                    .to_owned(),
            ));
        }
        let voxel_assets = gameplay_host.take_activated_voxel_assets();
        let runtime_entity_seeds = gameplay_host.take_activated_runtime_entity_seeds();
        let fps_seed = match domain_adapter {
            Some(RuntimeProjectDomainAdapter::Fps) => Some(Self::convert_runtime_project_fps_seed(
                runtime_entity_seeds,
                entry_scene.id,
            )?),
            None => None,
        };

        let mut staged = EngineBridge::new();
        initialization::initialize(&mut staged, EngineConfig { seed: engine.raw() })
            .map_err(|error| RuntimeProjectLoadError::Resource(error.to_string()))?;
        staged.projection.audio_projector =
            Some(AudioProjector::new(installed_presentation.catalog()));
        staged.projection.billboard_projector =
            Some(BillboardProjector::new(installed_presentation.catalog()));
        staged.projection.particle_projector = Some(ParticleProjector::new(
            installed_presentation.catalog(),
            ParticleProjectionLimits::default(),
        ));
        staged.projection.presentation_catalog = installed_presentation;
        staged.gameplay.static_gameplay_composition = Some(composition.clone());
        staged.gameplay.static_project_domain_adapter = domain_adapter;
        staged.gameplay.static_project_content_admission =
            Some(rule_project_bundle::GameplayProjectContentAdmission::new(
                composition.project_configuration_authority(),
            ));
        staged.scene.scene_document = Some(entry_scene.clone());

        let reset_checkpoint = gameplay_host.checkpoint_reset_state();
        staged.scene.entities = gameplay_host
            .take_entity_authority()
            .map_err(RuntimeProjectLoadError::Activation)?;
        let entity_count = staged.scene.entities.snapshot().records.len() as u32;
        staged.gameplay.static_gameplay_base_entities = Some(staged.scene.entities.clone());
        staged.gameplay.static_gameplay_reset_checkpoint = Some(reset_checkpoint);
        staged.gameplay.static_gameplay_host = Some(gameplay_host);
        if let Some(seed) = fps_seed {
            let fps_session = load_fps_project_bundle_from_existing_entities(
                &mut staged.scene.entities,
                seed.input.clone(),
            )
            .map_err(|error| {
                super::fps_project_diagnostics::runtime_project_fps_activation_error(&seed, error)
            })?;
            staged.gameplay.fps_session = Some(fps_session);
            staged.gameplay.fps_seed = Some(seed.input);
            staged.gameplay.fps_epoch = 1;
            staged.reset_presentation_projection();
        }

        let referenced_voxel_assets = entry_scene
            .nodes
            .iter()
            .filter_map(|node| match &node.kind {
                core_scene::SceneNodeKind::VoxelVolume(reference) => {
                    Some(reference.id().as_str().to_owned())
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        if referenced_voxel_assets.len() > 1 {
            return Err(RuntimeProjectLoadError::Resource(
                "the current RuntimeSession supports one collision-authoritative voxel asset in the entry scene"
                    .to_owned(),
            ));
        }
        if let Some(asset_id) = referenced_voxel_assets.first() {
            let asset = voxel_assets.get(asset_id).cloned().ok_or_else(|| {
                RuntimeProjectLoadError::Resource(format!(
                    "entry scene voxel asset `{asset_id}` was not retained by admission"
                ))
            })?;
            let material_frame = material_catalog::project_voxel_material_frame(
                &project_content.documents,
                &asset.material_palette,
            )
            .map_err(|error| RuntimeProjectLoadError::Resource(error.to_string()))?;
            staged
                .projection
                .pending_voxel_frame
                .ops
                .extend(material_frame.ops);
            staged.voxel.materials = MaterialCatalog::new(
                asset
                    .material_palette
                    .iter()
                    .map(|binding| VoxelMaterialId::new(binding.voxel_material)),
            );
            let receipt = RuntimeBridge::load_voxel_volume_asset(
                &mut staged,
                VoxelVolumeAssetLoadRequest {
                    target_grid: runtime_project_grid_id(asset_id),
                    target_volume_asset_id: Some(asset_id.clone()),
                    asset,
                    replace_existing: true,
                    include_material_counts: true,
                },
            )
            .map_err(|error| RuntimeProjectLoadError::Resource(error.to_string()))?;
            if !receipt.loaded {
                return Err(RuntimeProjectLoadError::Resource(format!(
                    "stored voxel asset `{asset_id}` was rejected during staged activation: {:?}",
                    receipt.diagnostics
                )));
            }
        }

        let lifecycle = RuntimeProjectLifecycleVersion {
            generation: actual.generation.saturating_add(1),
            revision: actual.revision.saturating_add(1),
        };
        let voxel_bindings = referenced_voxel_assets
            .iter()
            .map(
                |asset_id| protocol_project_bundle::RuntimeProjectVoxelBinding {
                    asset_id: asset_id.clone(),
                    grid: runtime_project_grid_id(asset_id),
                },
            )
            .collect::<Vec<_>>();
        let active = ActiveRuntimeProjectAuthority {
            project_id: identity.project_id(),
            manifest_hash: identity.manifest_hash().to_hex(),
            admission_hash: identity.admission_hash().to_owned(),
            content_set_hash,
            composition_hash,
            entry_scene_id: entry_scene.id.raw(),
            scene_count,
            entity_count,
            voxel_asset_count: referenced_voxel_assets.len() as u32,
            voxel_bindings,
        };
        let receipt = RuntimeProjectActivationReceipt {
            project_id: active.project_id,
            manifest_hash: active.manifest_hash.clone(),
            admission_hash: active.admission_hash.clone(),
            content_set_hash: active.content_set_hash.clone(),
            composition_hash: active.composition_hash.clone(),
            entry_scene_id: active.entry_scene_id,
            scene_count: active.scene_count,
            entity_count: active.entity_count,
            voxel_asset_count: active.voxel_asset_count,
            voxel_bindings: active.voxel_bindings.clone(),
            lifecycle,
        };
        staged.runtime_project.runtime_project_generation = lifecycle.generation;
        staged.runtime_project.runtime_project_revision = lifecycle.revision;
        staged.runtime_project.active_runtime_project = Some(active);
        *self = staged;
        Ok(receipt)
    }

    pub fn unload_runtime_project(
        &mut self,
        expected: RuntimeProjectLifecycleVersion,
    ) -> Result<RuntimeProjectUnloadReceipt, RuntimeProjectLoadError> {
        let actual = self.runtime_project_lifecycle_version();
        if expected != actual {
            return Err(RuntimeProjectLoadError::StaleLifecycle { expected, actual });
        }
        let active = self
            .runtime_project
            .active_runtime_project
            .clone()
            .ok_or(RuntimeProjectLoadError::NoActiveProject)?;
        let engine = self
            .runtime_project
            .engine
            .ok_or(RuntimeProjectLoadError::NotInitialized)?;
        let composition = self
            .gameplay
            .static_gameplay_composition
            .clone()
            .ok_or(RuntimeProjectLoadError::MissingStaticComposition)?;
        let domain_adapter = self.gameplay.static_project_domain_adapter;
        let mut unloaded = EngineBridge::new();
        initialization::initialize(&mut unloaded, EngineConfig { seed: engine.raw() })
            .map_err(|error| RuntimeProjectLoadError::Resource(error.to_string()))?;
        unloaded.gameplay.static_project_content_admission =
            Some(rule_project_bundle::GameplayProjectContentAdmission::new(
                composition.project_configuration_authority(),
            ));
        unloaded.gameplay.static_gameplay_composition = Some(composition);
        unloaded.gameplay.static_project_domain_adapter = domain_adapter;
        let lifecycle = RuntimeProjectLifecycleVersion {
            generation: actual.generation,
            revision: actual.revision.saturating_add(1),
        };
        unloaded.runtime_project.runtime_project_generation = lifecycle.generation;
        unloaded.runtime_project.runtime_project_revision = lifecycle.revision;
        let receipt = RuntimeProjectUnloadReceipt {
            project_id: active.project_id,
            manifest_hash: active.manifest_hash,
            lifecycle,
        };
        *self = unloaded;
        Ok(receipt)
    }

    fn reject_pending_runtime_project<T>(
        &mut self,
        error: RuntimeProjectLoadError,
    ) -> Result<T, RuntimeProjectLoadError> {
        self.runtime_project.pending_project_source = None;
        self.runtime_project.project_resource_staging.reset();
        Err(error)
    }

    /// Begin one manifest-bound input-resource transaction. Hosts use this for
    /// large/binary bodies before submitting the compact source batch. No
    /// project authority is activated here.
    #[doc(hidden)]
    pub fn begin_runtime_project_source_resources(
        &mut self,
        manifest_json: &str,
    ) -> BridgeResult<svc_serialization::ProjectResourceTransaction> {
        self.require_initialized("begin_runtime_project_source_resources")?;
        self.runtime_project
            .project_resource_staging
            .begin_for_manifest(manifest_json)
            .map_err(project_source_bridge_error)
    }

    /// Stage one raw resource body through the non-JSON transport owner.
    #[doc(hidden)]
    pub fn stage_runtime_project_source_resource(
        &mut self,
        transaction: svc_serialization::ProjectResourceTransaction,
        path: &str,
        bytes: Vec<u8>,
    ) -> BridgeResult<svc_serialization::StagedProjectResource> {
        self.require_initialized("stage_runtime_project_source_resource")?;
        self.runtime_project
            .project_resource_staging
            .stage(transaction, path, bytes)
            .map_err(project_source_bridge_error)
    }

    /// Transport helper for staging by the opaque generation returned from
    /// `begin_runtime_project_source_resources` without reconstructing the
    /// manifest-bound transaction in TypeScript.
    #[doc(hidden)]
    pub fn stage_runtime_project_source_resource_generation(
        &mut self,
        generation: u64,
        path: &str,
        bytes: Vec<u8>,
    ) -> BridgeResult<svc_serialization::StagedProjectResource> {
        self.require_initialized("stage_runtime_project_source_resource_generation")?;
        self.runtime_project
            .project_resource_staging
            .stage_generation(generation, path, bytes)
            .map_err(project_source_bridge_error)
    }

    /// Decode, validate, hash, and fully stage one manifest-owned raw batch.
    /// The accepted opaque source remains pre-activation until the runtime
    /// compiler/linker transaction consumes it.
    #[doc(hidden)]
    pub fn admit_runtime_project_source_batch(
        &mut self,
        batch: protocol_project_bundle::RuntimeProjectSourceBatch,
    ) -> BridgeResult<protocol_project_bundle::ProjectSourceBatchValidationReceipt> {
        self.require_initialized("admit_runtime_project_source_batch")?;
        let service_batch = service_project_source_batch(batch);
        let validated = match svc_serialization::validate_runtime_project_source_batch(
            &service_batch,
            &mut self.runtime_project.project_resource_staging,
        ) {
            Ok(validated) => validated,
            Err(error) => {
                return Ok(
                    protocol_project_bundle::ProjectSourceBatchValidationReceipt {
                        accepted: false,
                        manifest_hash: None,
                        paths: Vec::new(),
                        diagnostics: vec![project_source_diagnostic(error)],
                    },
                );
            }
        };
        let manifest_hash = validated.manifest_hash().to_hex();
        let paths = validated.paths().map(str::to_string).collect();
        let admitted = match validated.commit(&mut self.runtime_project.project_resource_staging) {
            Ok(admitted) => admitted,
            Err(error) => {
                return Ok(
                    protocol_project_bundle::ProjectSourceBatchValidationReceipt {
                        accepted: false,
                        manifest_hash: None,
                        paths: Vec::new(),
                        diagnostics: vec![project_source_diagnostic(error)],
                    },
                );
            }
        };
        self.runtime_project.pending_project_source = Some(admitted);
        Ok(
            protocol_project_bundle::ProjectSourceBatchValidationReceipt {
                accepted: true,
                manifest_hash: Some(manifest_hash),
                paths,
                diagnostics: Vec::new(),
            },
        )
    }

    #[cfg(test)]
    pub(super) fn pending_project_source(
        &self,
    ) -> Option<&svc_serialization::AdmittedRuntimeProjectSourceBatch> {
        self.runtime_project.pending_project_source.as_ref()
    }

    pub(super) fn register_voxel_conversion_mesh_asset_authority(
        &mut self,
        request: VoxelConversionMeshAssetRegistrationRequest,
    ) -> BridgeResult<VoxelConversionSourceRegistration> {
        self.require_runtime_or_workspace_authoring("register_voxel_conversion_mesh_asset")?;
        let source = match Self::static_mesh_source_from_project_mesh_asset(&request) {
            Ok(source) => source,
            Err(message) => {
                return Ok(Self::source_registration_diagnostic(
                    &request.source,
                    message,
                ));
            }
        };
        self.voxel
            .voxel_conversion_sources
            .insert(source.asset_id.clone(), source);
        self.voxel.voxel_conversion_source_metadata.insert(
            request.source.asset_id.clone(),
            Self::source_metadata_from_project_mesh_asset(&request),
        );
        self.voxel.voxel_conversion_plan = None;
        let evidence = vec![VoxelConversionEvidenceRef {
            kind: protocol_voxel_conversion::VoxelConversionEvidenceKind::SourceSnapshot,
            uri: format!(
                "asha://voxel-conversion/source/{}",
                request.source.asset_id.as_str()
            ),
            content_hash: request.source.source_hash.clone(),
        }];
        self.remember_voxel_conversion_evidence(evidence.clone());
        Ok(VoxelConversionSourceRegistration {
            source: request.source,
            registered: true,
            material_slots: request.mesh_asset.material_slots,
            diagnostics: Vec::new(),
            evidence,
        })
    }

    pub(super) fn import_voxel_conversion_mesh_source_authority(
        &mut self,
        request: VoxelConversionMeshSourceImportRequest,
    ) -> BridgeResult<VoxelConversionMeshSourceImportReceipt> {
        self.require_runtime_or_workspace_authoring("import_voxel_conversion_mesh_source")?;
        let source_byte_count = request.source_bytes.len() as u64;
        let preflight_error = svc_mesh_import::preflight_import_request(&request).err();
        let source_hash = if preflight_error.is_none() {
            svc_mesh_import::source_sha256(&request.source_bytes)
        } else {
            "sha256:not-computed".to_string()
        };
        let rejected_source = protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: request.source_asset_id.clone(),
            asset_kind: "mesh".to_string(),
            asset_version: request.asset_version,
            source_hash,
            mesh_primitive: request.mesh_primitive.clone(),
        };
        if let Some(error) = preflight_error {
            return Ok(VoxelConversionMeshSourceImportReceipt {
                source: rejected_source,
                imported: false,
                source_path: request.source_path,
                format: request.format,
                source_byte_count,
                mesh_asset: None,
                source_bounds: None,
                vertex_count: 0,
                triangle_count: 0,
                groups: Vec::new(),
                material_slots: Vec::new(),
                diagnostics: vec![Self::voxel_conversion_diagnostic(
                    Self::mesh_import_diagnostic_code(error.kind),
                    "meshImportPreflight",
                    error.message,
                )],
                evidence: Vec::new(),
            });
        }
        let imported = match svc_mesh_import::import_static_mesh(&request) {
            Ok(imported) => imported,
            Err(error) => {
                return Ok(VoxelConversionMeshSourceImportReceipt {
                    source: rejected_source,
                    imported: false,
                    source_path: request.source_path,
                    format: request.format,
                    source_byte_count,
                    mesh_asset: None,
                    source_bounds: None,
                    vertex_count: 0,
                    triangle_count: 0,
                    groups: Vec::new(),
                    material_slots: Vec::new(),
                    diagnostics: vec![Self::voxel_conversion_diagnostic(
                        Self::mesh_import_diagnostic_code(error.kind),
                        "sourceBytes",
                        error.message,
                    )],
                    evidence: Vec::new(),
                });
            }
        };
        let registration_request = VoxelConversionMeshAssetRegistrationRequest {
            source: imported.source.clone(),
            mesh_asset: imported.mesh_asset.clone(),
        };
        let source = match Self::static_mesh_source_from_project_mesh_asset(&registration_request) {
            Ok(source) => source,
            Err(message) => {
                return Ok(VoxelConversionMeshSourceImportReceipt {
                    source: imported.source,
                    imported: false,
                    source_path: request.source_path,
                    format: request.format,
                    source_byte_count,
                    mesh_asset: None,
                    source_bounds: None,
                    vertex_count: 0,
                    triangle_count: 0,
                    groups: Vec::new(),
                    material_slots: Vec::new(),
                    diagnostics: vec![Self::voxel_conversion_diagnostic(
                        VoxelConversionDiagnosticCode::UnsupportedSourceAsset,
                        "canonicalGeometry",
                        message,
                    )],
                    evidence: Vec::new(),
                });
            }
        };
        let metadata = Self::source_metadata_from_project_mesh_asset(&registration_request);
        self.voxel
            .voxel_conversion_sources
            .insert(source.asset_id.clone(), source);
        self.voxel
            .voxel_conversion_source_metadata
            .insert(imported.source.asset_id.clone(), metadata.clone());
        self.voxel.voxel_conversion_plan = None;
        self.remember_voxel_conversion_evidence(metadata.evidence.clone());
        Ok(VoxelConversionMeshSourceImportReceipt {
            source: imported.source,
            imported: true,
            source_path: request.source_path,
            format: request.format,
            source_byte_count,
            mesh_asset: Some(imported.mesh_asset),
            source_bounds: metadata.source_bounds,
            vertex_count: metadata.vertex_count,
            triangle_count: metadata.triangle_count,
            groups: metadata.groups,
            material_slots: metadata.material_slots,
            diagnostics: Vec::new(),
            evidence: metadata.evidence,
        })
    }

    pub(super) fn mesh_import_diagnostic_code(
        kind: svc_mesh_import::MeshImportErrorKind,
    ) -> VoxelConversionDiagnosticCode {
        match kind {
            svc_mesh_import::MeshImportErrorKind::QuotaExceeded => {
                VoxelConversionDiagnosticCode::OutputLimitExceeded
            }
            svc_mesh_import::MeshImportErrorKind::InvalidRequest
            | svc_mesh_import::MeshImportErrorKind::UnsupportedFeature
            | svc_mesh_import::MeshImportErrorKind::InvalidGeometry => {
                VoxelConversionDiagnosticCode::UnsupportedSourceAsset
            }
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    /// Native transport helper for turning strict closed document JSON into
    /// typed protocol values before the authority cell validates the complete
    /// set. It does not install or accept project-content state.
    #[doc(hidden)]
    pub fn decode_project_content_sources(
        sources: &[ProjectContentSourceDto],
    ) -> Result<Vec<ProjectContentDocumentDto>, Vec<ProjectContentDiagnosticDto>> {
        svc_project_content::decode_project_content_sources(sources)
    }

    /// Completes a transport-level strict parse rejection with the catalog
    /// owned by the open project-authoring authority. Parsing itself is pure;
    /// only this authority cell can supply the composed provider context.
    #[doc(hidden)]
    pub fn reject_project_content_parse(
        &self,
        diagnostics: Vec<ProjectContentDiagnosticDto>,
    ) -> BridgeResult<ProjectContentCodecResultDto> {
        let authority = self
            .workspace_authoring
            .as_ref()
            .filter(|authority| authority.open)
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    "project-content parse rejection requested before workspace authoring open",
                )
            })?;
        Ok(svc_project_content::reject_project_content_parse(
            diagnostics,
            &authority.project_content_admission,
        ))
    }

    /// The default launch grid: id 1, voxel size 1.0, cubic 2×2×2 chunks (matches
    /// the canonical voxel fixture). Chunk dims come from the spec, not a global.
    pub(super) fn launch_grid() -> VoxelGridSpec {
        VoxelGridSpec::new(
            GridId::new(1),
            1.0,
            ChunkDims::cubic(2).expect("nonzero dims"),
        )
        .expect("positive voxel size")
    }

    pub(super) fn material_for_chunk(coord: ChunkCoord) -> u16 {
        const MATERIAL_IDS: [u16; 3] = [1, 2, 3];
        let idx = (coord.x * 2 + coord.y).rem_euclid(MATERIAL_IDS.len() as i64) as usize;
        MATERIAL_IDS[idx]
    }

    pub(super) fn launch_world() -> VoxelWorld {
        let spec = Self::launch_grid();
        let mut world = VoxelWorld::new(spec);
        let dims = spec.chunk_dims();
        for coord in [
            ChunkCoord::new(0, 0, 0),
            ChunkCoord::new(1, 0, 0),
            ChunkCoord::new(0, 1, 0),
            ChunkCoord::new(1, 1, 0),
        ] {
            let mut chunk = VoxelChunk::from_spec(&spec);
            chunk
                .fill_region(
                    core_space::LocalVoxelCoord::new(0, 0, 0),
                    core_space::LocalVoxelCoord::new(dims.x(), dims.y(), 1),
                    VoxelValue::solid_raw(Self::material_for_chunk(coord)),
                )
                .expect("canonical launch chunk fill within bounds");
            world.insert(coord, chunk);
        }
        let _ = world.drain_dirty();
        world
    }

    pub(super) fn fixture_quad_source() -> StaticMeshSource {
        StaticMeshSource {
            asset_id: "mesh/quad".to_string(),
            asset_kind: "mesh".to_string(),
            asset_version: 1,
            source_hash: "sha256:quad".to_string(),
            mesh_primitive: None,
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            triangles: vec![
                MeshTriangle {
                    indices: [0, 1, 2],
                    source_material_slot: 0,
                },
                MeshTriangle {
                    indices: [0, 2, 3],
                    source_material_slot: 1,
                },
            ],
        }
    }

    pub(super) fn project_triangle_static_mesh_asset() -> StaticMeshAsset {
        StaticMeshAsset {
            asset: "mesh/import-fixture-a".to_string(),
            payload: MeshPayloadDescriptor {
                layout: MeshBufferLayout {
                    vertex_count: 3,
                    index_count: 3,
                    index_width: MeshIndexWidth::U32,
                    attributes: vec![
                        MeshAttribute {
                            name: MeshAttributeName::Position,
                            components: 3,
                            kind: MeshAttributeKind::F32,
                        },
                        MeshAttribute {
                            name: MeshAttributeName::Normal,
                            components: 3,
                            kind: MeshAttributeKind::F32,
                        },
                    ],
                },
                groups: vec![MeshGroupDescriptor {
                    material_slot: 0,
                    start: 0,
                    count: 3,
                }],
                bounds: MeshBoundsDescriptor {
                    min: [0.0, 0.0, 0.0],
                    max: [1.0, 1.0, 0.0],
                },
                source: MeshPayloadSource::Inline {
                    positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                    normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
                    indices: vec![0, 1, 2],
                },
                provenance: MeshProvenance::StaticAsset,
            },
            material_slots: vec![MeshMaterialSlot {
                slot: 0,
                material: "material/surface-a".to_string(),
            }],
            collision: MeshCollisionPolicy::AabbFallback,
        }
    }

    pub(super) fn static_mesh_source_from_asset(
        asset: &StaticMeshAsset,
        asset_version: u64,
        source_hash: impl Into<String>,
        mesh_primitive: Option<String>,
    ) -> BridgeResult<StaticMeshSource> {
        asset.validate().map_err(|err| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("static mesh asset cannot seed voxel conversion authority: {err:?}"),
            )
        })?;
        if asset.payload.provenance != MeshProvenance::StaticAsset {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel conversion source must be an authored static mesh asset",
            ));
        }
        let MeshPayloadSource::Inline {
            positions, indices, ..
        } = &asset.payload.source
        else {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel conversion source requires authority-visible inline mesh geometry",
            ));
        };
        let mut triangles = Vec::new();
        for group in &asset.payload.groups {
            if group.count % 3 != 0 {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    "voxel conversion source mesh group is not a triangle list",
                ));
            }
            let start = group.start as usize;
            let end = start + group.count as usize;
            for tri in indices[start..end].chunks_exact(3) {
                triangles.push(MeshTriangle {
                    indices: [tri[0], tri[1], tri[2]],
                    source_material_slot: group.material_slot as u32,
                });
            }
        }

        Ok(StaticMeshSource {
            asset_id: asset.asset.clone(),
            asset_kind: "mesh".to_string(),
            asset_version,
            source_hash: source_hash.into(),
            mesh_primitive,
            positions: positions
                .chunks_exact(3)
                .map(|position| [position[0], position[1], position[2]])
                .collect(),
            triangles,
        })
    }

    pub(super) fn static_mesh_source_from_project_mesh_asset(
        request: &VoxelConversionMeshAssetRegistrationRequest,
    ) -> Result<StaticMeshSource, String> {
        let asset = &request.mesh_asset;
        if request.source.asset_id != asset.asset_id
            || request.source.asset_kind != "mesh"
            || request.source.asset_version == 0
            || request.source.source_hash.is_empty()
        {
            return Err(
                "mesh asset source ref does not match a supported project mesh asset identity"
                    .to_string(),
            );
        }
        if !matches!(
            request.source.mesh_primitive.as_deref(),
            None | Some("default")
        ) {
            return Err("only the default mesh primitive is supported for project mesh asset voxel conversion".to_string());
        }
        if asset.positions.is_empty() || asset.indices.is_empty() || asset.groups.is_empty() {
            return Err(
                "project mesh asset must contain positions, indices, and triangle groups"
                    .to_string(),
            );
        }
        if !asset.normals.is_empty() && asset.normals.len() != asset.positions.len() {
            return Err(
                "project mesh asset normals must either be omitted or match position count"
                    .to_string(),
            );
        }
        let material_slots = asset
            .material_slots
            .iter()
            .map(|slot| slot.source_material_slot)
            .collect::<BTreeSet<_>>();
        let mut covered = 0u64;
        let mut triangles = Vec::new();
        for group in &asset.groups {
            if group.count % 3 != 0 {
                return Err("project mesh asset group is not a triangle list".to_string());
            }
            if !material_slots.contains(&group.material_slot) {
                return Err(
                    "project mesh asset group references an unbound material slot".to_string(),
                );
            }
            let start = group.start as usize;
            let end = start + group.count as usize;
            if end > asset.indices.len() {
                return Err(
                    "project mesh asset group range is outside the index buffer".to_string()
                );
            }
            covered += u64::from(group.count);
            for tri in asset.indices[start..end].chunks_exact(3) {
                if tri
                    .iter()
                    .any(|index| *index as usize >= asset.positions.len())
                {
                    return Err(
                        "project mesh asset index references a missing position".to_string()
                    );
                }
                triangles.push(MeshTriangle {
                    indices: [tri[0], tri[1], tri[2]],
                    source_material_slot: group.material_slot,
                });
            }
        }
        if covered != asset.indices.len() as u64 {
            return Err(
                "project mesh asset groups must exactly cover the index buffer".to_string(),
            );
        }
        Ok(StaticMeshSource {
            asset_id: request.source.asset_id.clone(),
            asset_kind: "mesh".to_string(),
            asset_version: request.source.asset_version,
            source_hash: request.source.source_hash.clone(),
            mesh_primitive: request.source.mesh_primitive.clone(),
            positions: asset.positions.clone(),
            triangles,
        })
    }

    pub(super) fn source_registration_diagnostic(
        source: &protocol_voxel_conversion::VoxelConversionSourceRef,
        message: impl Into<String>,
    ) -> VoxelConversionSourceRegistration {
        VoxelConversionSourceRegistration {
            source: source.clone(),
            registered: false,
            material_slots: Vec::new(),
            diagnostics: vec![Self::voxel_conversion_diagnostic(
                VoxelConversionDiagnosticCode::UnsupportedSourceAsset,
                source.asset_id.clone(),
                message,
            )],
            evidence: Vec::new(),
        }
    }

    pub(super) fn static_mesh_source_from_registration(
        request: &VoxelConversionSourceRegistrationRequest,
    ) -> Result<StaticMeshSource, String> {
        if request.source.asset_id.trim().is_empty() {
            return Err("voxel conversion source asset id is required".to_string());
        }
        if request.source.asset_kind != "mesh" {
            return Err(format!(
                "voxel conversion source asset kind must be mesh, got {}",
                request.source.asset_kind
            ));
        }
        if request.source.asset_version == 0 {
            return Err("voxel conversion source asset version must be positive".to_string());
        }
        if !request.source.source_hash.starts_with("sha256:") {
            return Err(
                "voxel conversion source hash must be a sha256: authority hash".to_string(),
            );
        }
        if request.positions.is_empty() {
            return Err(
                "voxel conversion source requires at least one vertex position".to_string(),
            );
        }
        if request.triangles.is_empty() {
            return Err("voxel conversion source requires at least one triangle".to_string());
        }

        for (index, position) in request.positions.iter().enumerate() {
            if !position.iter().all(|component| component.is_finite()) {
                return Err(format!(
                    "voxel conversion source position {index} contains a non-finite component"
                ));
            }
        }

        let mut material_slots = BTreeSet::new();
        for slot in &request.material_slots {
            if !material_slots.insert(slot.source_material_slot) {
                return Err(format!(
                    "voxel conversion source material slot {} is duplicated",
                    slot.source_material_slot
                ));
            }
        }
        if material_slots.is_empty() {
            return Err("voxel conversion source requires material slot bindings".to_string());
        }

        let vertex_count = request.positions.len() as u32;
        let mut triangles = Vec::with_capacity(request.triangles.len());
        for (index, triangle) in request.triangles.iter().enumerate() {
            if triangle
                .indices
                .iter()
                .any(|vertex| *vertex >= vertex_count)
            {
                return Err(format!(
                    "voxel conversion source triangle {index} references a missing vertex"
                ));
            }
            if !material_slots.contains(&triangle.source_material_slot) {
                return Err(format!(
                    "voxel conversion source triangle {index} references unbound material slot {}",
                    triangle.source_material_slot
                ));
            }
            triangles.push(MeshTriangle {
                indices: triangle.indices,
                source_material_slot: triangle.source_material_slot,
            });
        }

        Ok(StaticMeshSource {
            asset_id: request.source.asset_id.clone(),
            asset_kind: request.source.asset_kind.clone(),
            asset_version: request.source.asset_version,
            source_hash: request.source.source_hash.clone(),
            mesh_primitive: request.source.mesh_primitive.clone(),
            positions: request.positions.clone(),
            triangles,
        })
    }

    pub(super) fn empty_unsupported_source(
        reference: &protocol_voxel_conversion::VoxelConversionSourceRef,
    ) -> StaticMeshSource {
        StaticMeshSource {
            asset_id: reference.asset_id.clone(),
            asset_kind: reference.asset_kind.clone(),
            asset_version: reference.asset_version,
            source_hash: reference.source_hash.clone(),
            mesh_primitive: reference.mesh_primitive.clone(),
            positions: Vec::new(),
            triangles: Vec::new(),
        }
    }

    pub(super) fn seeded_voxel_conversion_authority() -> BridgeResult<(
        BTreeMap<String, StaticMeshSource>,
        BTreeMap<String, VoxelConversionSourceMetadataAuthority>,
    )> {
        let mut sources = BTreeMap::new();
        let mut metadata = BTreeMap::new();
        let fixture = Self::fixture_quad_source();
        metadata.insert(
            fixture.asset_id.clone(),
            Self::source_metadata_from_static_source(
                &fixture,
                None,
                vec![
                    VoxelConversionSourceMaterialSlot {
                        source_material_slot: 0,
                        source_material_id: Some("mat/a".to_string()),
                    },
                    VoxelConversionSourceMaterialSlot {
                        source_material_slot: 1,
                        source_material_id: Some("mat/b".to_string()),
                    },
                ],
                Self::material_group_metadata_from_source(&fixture),
            ),
        );
        sources.insert(fixture.asset_id.clone(), fixture);

        let project_asset = Self::project_triangle_static_mesh_asset();
        let project_source = Self::static_mesh_source_from_asset(
            &project_asset,
            1,
            "sha256:import-fixture-a",
            Some("default".to_string()),
        )?;
        metadata.insert(
            project_source.asset_id.clone(),
            Self::source_metadata_from_static_mesh_asset(
                &project_source,
                &project_asset,
                Some("asha://fixture/mesh/import-fixture-a".to_string()),
            ),
        );
        sources.insert(project_source.asset_id.clone(), project_source);
        Ok((sources, metadata))
    }

    pub(super) fn seeded_voxel_conversion_targets(
    ) -> BTreeMap<(u64, Option<String>), VoxelConversionTargetAuthority> {
        let launch = Self::launch_grid();
        let studio_grid =
            VoxelGridSpec::new(GridId::new(2), launch.voxel_size(), launch.chunk_dims())
                .expect("positive Studio target voxel size");
        let authored_grid =
            VoxelGridSpec::new(GridId::new(7), launch.voxel_size(), launch.chunk_dims())
                .expect("positive authored target voxel size");
        [
            VoxelConversionTargetAuthority {
                spec: launch,
                volume_asset_id: Some("voxel/generated".to_string()),
            },
            VoxelConversionTargetAuthority {
                spec: studio_grid,
                volume_asset_id: Some("voxel/generated".to_string()),
            },
            VoxelConversionTargetAuthority {
                spec: authored_grid,
                volume_asset_id: Some("voxel/generated".to_string()),
            },
        ]
        .into_iter()
        .map(|target| {
            (
                (
                    target.spec.id().raw() as u64,
                    target.volume_asset_id.clone(),
                ),
                target,
            )
        })
        .collect()
    }

    pub(super) fn source_for_voxel_conversion(
        &self,
        request: &VoxelConversionPlanRequest,
    ) -> StaticMeshSource {
        self.voxel
            .voxel_conversion_sources
            .get(&request.source.asset_id)
            .cloned()
            .unwrap_or_else(|| Self::empty_unsupported_source(&request.source))
    }

    pub(super) fn source_metadata_from_registration(
        request: &VoxelConversionSourceRegistrationRequest,
    ) -> VoxelConversionSourceMetadataAuthority {
        let source = StaticMeshSource {
            asset_id: request.source.asset_id.clone(),
            asset_kind: request.source.asset_kind.clone(),
            asset_version: request.source.asset_version,
            source_hash: request.source.source_hash.clone(),
            mesh_primitive: request.source.mesh_primitive.clone(),
            positions: request.positions.clone(),
            triangles: request
                .triangles
                .iter()
                .map(|triangle| MeshTriangle {
                    indices: triangle.indices,
                    source_material_slot: triangle.source_material_slot,
                })
                .collect(),
        };
        Self::source_metadata_from_static_source(
            &source,
            None,
            request.material_slots.clone(),
            Self::material_group_metadata_from_source(&source),
        )
    }

    pub(super) fn source_metadata_from_project_mesh_asset(
        request: &VoxelConversionMeshAssetRegistrationRequest,
    ) -> VoxelConversionSourceMetadataAuthority {
        let source = StaticMeshSource {
            asset_id: request.source.asset_id.clone(),
            asset_kind: "mesh".to_string(),
            asset_version: request.source.asset_version,
            source_hash: request.source.source_hash.clone(),
            mesh_primitive: request.source.mesh_primitive.clone(),
            positions: request.mesh_asset.positions.clone(),
            triangles: request
                .mesh_asset
                .groups
                .iter()
                .flat_map(|group| {
                    let start = group.start as usize;
                    let end = start + group.count as usize;
                    request.mesh_asset.indices[start..end]
                        .chunks_exact(3)
                        .map(move |tri| MeshTriangle {
                            indices: [tri[0], tri[1], tri[2]],
                            source_material_slot: group.material_slot,
                        })
                })
                .collect(),
        };
        let groups = Self::group_metadata_from_project_mesh_asset(&request.mesh_asset);
        Self::source_metadata_from_static_source(
            &source,
            request.mesh_asset.source_path.clone(),
            request.mesh_asset.material_slots.clone(),
            groups,
        )
    }

    pub(super) fn source_metadata_from_static_mesh_asset(
        source: &StaticMeshSource,
        asset: &StaticMeshAsset,
        source_path: Option<String>,
    ) -> VoxelConversionSourceMetadataAuthority {
        let material_slots = asset
            .material_slots
            .iter()
            .map(|slot| VoxelConversionSourceMaterialSlot {
                source_material_slot: slot.slot as u32,
                source_material_id: Some(slot.material.clone()),
            })
            .collect();
        let groups = Self::group_metadata_from_static_mesh_asset(source, asset);
        Self::source_metadata_from_static_source(source, source_path, material_slots, groups)
    }

    pub(super) fn source_metadata_from_static_source(
        source: &StaticMeshSource,
        source_path: Option<String>,
        material_slots: Vec<VoxelConversionSourceMaterialSlot>,
        groups: Vec<VoxelConversionSourceGroupMetadata>,
    ) -> VoxelConversionSourceMetadataAuthority {
        let source_ref = protocol_voxel_conversion::VoxelConversionSourceRef {
            asset_id: source.asset_id.clone(),
            asset_kind: source.asset_kind.clone(),
            asset_version: source.asset_version,
            source_hash: source.source_hash.clone(),
            mesh_primitive: source.mesh_primitive.clone(),
        };
        VoxelConversionSourceMetadataAuthority {
            source: source_ref,
            source_path,
            source_bounds: Self::source_bounds(&source.positions),
            vertex_count: Self::saturating_u32(source.positions.len()),
            triangle_count: Self::saturating_u32(source.triangles.len()),
            groups,
            material_slots,
            evidence: vec![VoxelConversionEvidenceRef {
                kind: protocol_voxel_conversion::VoxelConversionEvidenceKind::SourceSnapshot,
                uri: format!("asha://voxel-conversion/source/{}", source.asset_id),
                content_hash: source.source_hash.clone(),
            }],
        }
    }

    pub(super) fn material_group_metadata_from_source(
        source: &StaticMeshSource,
    ) -> Vec<VoxelConversionSourceGroupMetadata> {
        let mut by_slot: BTreeMap<u32, Vec<MeshTriangle>> = BTreeMap::new();
        for triangle in &source.triangles {
            by_slot
                .entry(triangle.source_material_slot)
                .or_default()
                .push(*triangle);
        }
        let mut start = 0u32;
        by_slot
            .into_iter()
            .map(|(slot, triangles)| {
                let count = Self::saturating_u32(triangles.len().saturating_mul(3));
                let bounds = Self::bounds_for_triangles(&source.positions, &triangles);
                let group = VoxelConversionSourceGroupMetadata {
                    group_id: format!("material-slot:{slot}"),
                    label: Some(format!("Material slot {slot}")),
                    material_slot: slot,
                    start,
                    count,
                    bounds,
                };
                start = start.saturating_add(count);
                group
            })
            .collect()
    }

    pub(super) fn group_metadata_from_project_mesh_asset(
        asset: &protocol_voxel_conversion::VoxelConversionMeshAsset,
    ) -> Vec<VoxelConversionSourceGroupMetadata> {
        asset
            .groups
            .iter()
            .enumerate()
            .map(|(index, group)| {
                let start = group.start as usize;
                let end = start + group.count as usize;
                let triangles = asset.indices[start..end]
                    .chunks_exact(3)
                    .map(|tri| MeshTriangle {
                        indices: [tri[0], tri[1], tri[2]],
                        source_material_slot: group.material_slot,
                    })
                    .collect::<Vec<_>>();
                VoxelConversionSourceGroupMetadata {
                    group_id: format!("group:{index}:material-slot:{}", group.material_slot),
                    label: Some(format!(
                        "Group {index} / material slot {}",
                        group.material_slot
                    )),
                    material_slot: group.material_slot,
                    start: group.start,
                    count: group.count,
                    bounds: Self::bounds_for_triangles(&asset.positions, &triangles),
                }
            })
            .collect()
    }

    pub(super) fn group_metadata_from_static_mesh_asset(
        source: &StaticMeshSource,
        asset: &StaticMeshAsset,
    ) -> Vec<VoxelConversionSourceGroupMetadata> {
        asset
            .payload
            .groups
            .iter()
            .enumerate()
            .map(|(index, group)| {
                let triangles = source
                    .triangles
                    .iter()
                    .filter(|triangle| triangle.source_material_slot == group.material_slot as u32)
                    .copied()
                    .collect::<Vec<_>>();
                VoxelConversionSourceGroupMetadata {
                    group_id: format!("group:{index}:material-slot:{}", group.material_slot),
                    label: Some(format!(
                        "Group {index} / material slot {}",
                        group.material_slot
                    )),
                    material_slot: group.material_slot as u32,
                    start: group.start,
                    count: group.count,
                    bounds: Self::bounds_for_triangles(&source.positions, &triangles),
                }
            })
            .collect()
    }

    pub(super) fn source_bounds(positions: &[[f32; 3]]) -> Option<VoxelConversionSourceBounds> {
        let mut iter = positions.iter();
        let first = *iter.next()?;
        let mut min = first;
        let mut max = first;
        for position in iter {
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        Some(VoxelConversionSourceBounds { min, max })
    }

    pub(super) fn bounds_for_triangles(
        positions: &[[f32; 3]],
        triangles: &[MeshTriangle],
    ) -> Option<VoxelConversionSourceBounds> {
        let mut group_positions = Vec::new();
        for triangle in triangles {
            for index in triangle.indices {
                let position = positions.get(index as usize)?;
                group_positions.push(*position);
            }
        }
        Self::source_bounds(&group_positions)
    }

    pub(super) fn missing_voxel_conversion_source_metadata(
        request: VoxelConversionSourceMetadataRequest,
        message: impl Into<String>,
    ) -> VoxelConversionSourceMetadataReadout {
        VoxelConversionSourceMetadataReadout {
            request,
            registered: false,
            source: None,
            source_path: None,
            source_bounds: None,
            vertex_count: 0,
            triangle_count: 0,
            groups: Vec::new(),
            material_slots: Vec::new(),
            latest_plan_id: None,
            latest_plan_transform: None,
            diagnostics: vec![Self::voxel_conversion_diagnostic(
                VoxelConversionDiagnosticCode::VoxelConversionUnavailable,
                "source",
                message,
            )],
            evidence: Vec::new(),
        }
    }

    pub(super) fn saturating_u32(value: usize) -> u32 {
        u32::try_from(value).unwrap_or(u32::MAX)
    }
}

fn runtime_project_grid_id(asset_id: &str) -> u64 {
    let raw = svc_serialization::BundleHash::of(asset_id.as_bytes()).0 as u32;
    u64::from(raw.max(1))
}

fn service_project_source_batch(
    batch: protocol_project_bundle::RuntimeProjectSourceBatch,
) -> svc_serialization::RuntimeProjectSourceBatch {
    svc_serialization::RuntimeProjectSourceBatch {
        manifest_json: batch.manifest_json,
        resource_generation: batch.resource_generation,
        bodies: batch
            .bodies
            .into_iter()
            .map(|body| match body {
                protocol_project_bundle::ProjectSourceBody::Inline { path, bytes } => {
                    svc_serialization::ProjectSourceBody::Inline { path, bytes }
                }
                protocol_project_bundle::ProjectSourceBody::Resource { path, resource } => {
                    svc_serialization::ProjectSourceBody::Resource {
                        path,
                        resource: svc_serialization::StagedProjectResource {
                            handle: svc_serialization::ProjectResourceHandle::new(resource.handle),
                            generation: resource.generation,
                            version: resource.version,
                            byte_len: resource.byte_len,
                        },
                    }
                }
            })
            .collect(),
    }
}

fn project_source_diagnostic(
    error: svc_serialization::ProjectSourceBatchError,
) -> protocol_project_bundle::ProjectSourceBatchDiagnostic {
    protocol_project_bundle::ProjectSourceBatchDiagnostic {
        code: match error.code {
            svc_serialization::ProjectSourceBatchErrorCode::ManifestTooLarge => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ManifestTooLarge
            }
            svc_serialization::ProjectSourceBatchErrorCode::ManifestDecodeFailed => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ManifestDecodeFailed
            }
            svc_serialization::ProjectSourceBatchErrorCode::ManifestInvalid => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ManifestInvalid
            }
            svc_serialization::ProjectSourceBatchErrorCode::TooManyBodies => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::TooManyBodies
            }
            svc_serialization::ProjectSourceBatchErrorCode::DuplicateBody => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::DuplicateBody
            }
            svc_serialization::ProjectSourceBatchErrorCode::DuplicateResourceHandle => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::DuplicateResourceHandle
            }
            svc_serialization::ProjectSourceBatchErrorCode::MissingBody => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::MissingBody
            }
            svc_serialization::ProjectSourceBatchErrorCode::ExtraBody => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ExtraBody
            }
            svc_serialization::ProjectSourceBatchErrorCode::InlineBodyTooLarge => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::InlineBodyTooLarge
            }
            svc_serialization::ProjectSourceBatchErrorCode::InlineBodyForbidden => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::InlineBodyForbidden
            }
            svc_serialization::ProjectSourceBatchErrorCode::InlineQuotaExceeded => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::InlineQuotaExceeded
            }
            svc_serialization::ProjectSourceBatchErrorCode::ResourceBodyTooLarge => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ResourceBodyTooLarge
            }
            svc_serialization::ProjectSourceBatchErrorCode::ResourceQuotaExceeded => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ResourceQuotaExceeded
            }
            svc_serialization::ProjectSourceBatchErrorCode::UnknownResourceHandle => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::UnknownResourceHandle
            }
            svc_serialization::ProjectSourceBatchErrorCode::ResourceGenerationMismatch => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ResourceGenerationMismatch
            }
            svc_serialization::ProjectSourceBatchErrorCode::ResourceVersionMismatch => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ResourceVersionMismatch
            }
            svc_serialization::ProjectSourceBatchErrorCode::ResourceLengthMismatch => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ResourceLengthMismatch
            }
            svc_serialization::ProjectSourceBatchErrorCode::ResourceManifestMismatch => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ResourceManifestMismatch
            }
            svc_serialization::ProjectSourceBatchErrorCode::ResourcePathMismatch => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ResourcePathMismatch
            }
            svc_serialization::ProjectSourceBatchErrorCode::ContentHashMismatch => {
                protocol_project_bundle::ProjectSourceBatchErrorCode::ContentHashMismatch
            }
        },
        path: error.path,
        message: error.message,
    }
}

fn project_source_bridge_error(
    error: svc_serialization::ProjectSourceBatchError,
) -> RuntimeBridgeError {
    RuntimeBridgeError::new(RuntimeBridgeErrorKind::InvalidInput, error.to_string())
}
