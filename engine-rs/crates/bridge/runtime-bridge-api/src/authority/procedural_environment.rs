use super::*;

impl EngineBridge {
    pub(super) fn preview_procedural_environment_authority(
        &mut self,
        request: ProceduralEnvironmentPreviewRequestDto,
    ) -> BridgeResult<ProceduralEnvironmentPreviewResultDto> {
        self.require_workspace_authoring_revision(
            "preview_procedural_environment",
            &request.expected_workspace_id,
            request.expected_generation,
            request.expected_working_revision,
        )?;

        let (scene_dto, working_revision, replacement_asset) = {
            let authority =
                self.require_open_workspace_authoring_mut("preview_procedural_environment")?;
            let Some(scene) = authority
                .project_content_scenes
                .get(&request.target.scene_id.raw())
                .cloned()
            else {
                authority.pending_procedural_environment = None;
                return Ok(Self::procedural_preview_rejection(
                    ProceduralEnvironmentDiagnosticCode::MissingScene,
                    "target.sceneId",
                    "target scene is not loaded in the current workspace generation",
                ));
            };
            let replacement_asset = authority
                .loaded_voxel_assets
                .get(&request.target.asset_id)
                .cloned();
            (scene, authority.working_revision, replacement_asset)
        };
        let scene = Self::scene_document_from_dto(scene_dto)?;
        let canonical_scene = scene.canonical();
        let current_scene_json = core_scene::encode(&canonical_scene);
        let current_scene_hash = format!("fnv1a64:{}", Self::fnv1a64(&current_scene_json));
        if current_scene_hash != request.expected_scene_content_hash {
            self.require_open_workspace_authoring_mut("preview_procedural_environment")?
                .pending_procedural_environment = None;
            return Ok(Self::procedural_preview_rejection(
                ProceduralEnvironmentDiagnosticCode::StaleScene,
                "expectedSceneContentHash",
                "request targeted a stale Engine-owned scene",
            ));
        }

        let target = svc_environment_authoring::EnvironmentTarget {
            scene_path: request.target.scene_path.clone(),
            asset_id: request.target.asset_id.clone(),
            asset_path: request.target.asset_path.clone(),
            voxel_node_id: request.target.voxel_node_id,
            voxel_parent_id: request.target.voxel_parent_id,
            voxel_child_order: request.target.voxel_child_order,
            voxel_label: request.target.voxel_label.clone(),
            voxel_transform: Self::scene_transform_from_dto(request.target.voxel_transform),
            marker_targets: request.target.marker_targets.clone(),
        };
        let materialized = match svc_environment_authoring::materialize_environment(
            &canonical_scene,
            &svc_environment_authoring::EnvironmentMaterializationInput {
                provider_id: request.provider_id,
                preset_id: request.preset_id,
                seed: request.seed,
                replacement_asset,
                target,
                material_palette: request.material_palette,
                authoring: request.authoring,
                limits: request.limits,
            },
        ) {
            Ok(candidate) => candidate,
            Err(diagnostics) => {
                self.require_open_workspace_authoring_mut("preview_procedural_environment")?
                    .pending_procedural_environment = None;
                return Ok(ProceduralEnvironmentPreviewResultDto {
                    accepted: false,
                    candidate: None,
                    preview_frame: None,
                    preview_projection_hash: None,
                    preview_diff_count: 0,
                    diagnostics,
                });
            }
        };
        let bound_candidate_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "workspace-procedural-candidate-v1|{}|{}|{}|{}|{}",
                request.expected_workspace_id,
                request.expected_generation,
                working_revision,
                current_scene_hash,
                materialized.candidate_hash
            ))
        );
        let mut projector = VoxelChunkProjector::default();
        let preview_frame = projector
            .set_instances(
                &materialized.world,
                vec![VoxelProjectionInstance {
                    instance_id: format!("scene-node/{}", request.target.voxel_node_id.raw()),
                    asset_id: materialized.asset.asset_id.clone(),
                    transform: materialized.instance_transform,
                }],
            )
            .map_err(|error| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!("materialization preview projection failed: {error:?}"),
                )
            })?;
        let preview_frame_json = render_bridge::json::encode_frame(&preview_frame);
        let preview_diff_count = preview_frame.ops.len() as u64;
        let preview_projection_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "procedural-preview-v1|{}|{}",
                bound_candidate_hash, preview_frame_json
            ))
        );
        let candidate_dto = Self::procedural_candidate_dto(
            &bound_candidate_hash,
            &request.target.scene_path,
            &request.target.asset_path,
            &materialized,
        );
        self.require_open_workspace_authoring_mut("preview_procedural_environment")?
            .pending_procedural_environment = Some(PendingProceduralEnvironmentCandidate {
            candidate_hash: bound_candidate_hash,
            base_scene_hash: current_scene_hash,
            working_revision,
            scene_path: request.target.scene_path,
            asset_path: request.target.asset_path,
            voxel_node_id: request.target.voxel_node_id,
            materialized,
        });
        Ok(ProceduralEnvironmentPreviewResultDto {
            accepted: true,
            candidate: Some(candidate_dto),
            preview_frame: Some(preview_frame),
            preview_projection_hash: Some(preview_projection_hash),
            preview_diff_count,
            diagnostics: Vec::new(),
        })
    }

    pub(super) fn apply_procedural_environment_authority(
        &mut self,
        request: ProceduralEnvironmentApplyRequestDto,
    ) -> BridgeResult<ProceduralEnvironmentApplyResultDto> {
        self.require_workspace_authoring_revision(
            "apply_procedural_environment",
            &request.expected_workspace_id,
            request.expected_generation,
            request.expected_working_revision,
        )?;
        let pending = {
            let authority =
                self.require_open_workspace_authoring_mut("apply_procedural_environment")?;
            let matches = authority
                .pending_procedural_environment
                .as_ref()
                .is_some_and(|candidate| {
                    candidate.candidate_hash == request.candidate_hash
                        && candidate.working_revision == authority.working_revision
                });
            if !matches {
                return Ok(Self::procedural_apply_rejection(
                    authority.working_revision,
                    ProceduralEnvironmentDiagnosticCode::StaleCandidate,
                    "candidateHash",
                    "candidate is missing, stale, foreign, or already consumed",
                ));
            }
            authority
                .pending_procedural_environment
                .take()
                .expect("candidate match checked")
        };

        let current_scene_hash = {
            let authority =
                self.require_open_workspace_authoring_mut("apply_procedural_environment")?;
            let scene = authority
                .project_content_scenes
                .get(&pending.materialized.scene.id.raw())
                .cloned()
                .ok_or_else(|| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::StaleAuthoritySnapshot,
                        "candidate scene disappeared before apply",
                    )
                })?;
            let scene = Self::scene_document_from_dto(scene)?;
            format!(
                "fnv1a64:{}",
                Self::fnv1a64(&core_scene::encode(&scene.canonical()))
            )
        };
        if current_scene_hash != pending.base_scene_hash {
            return Ok(Self::procedural_apply_rejection(
                request.expected_working_revision,
                ProceduralEnvironmentDiagnosticCode::StaleScene,
                "scene",
                "Engine-owned scene changed after preview",
            ));
        }

        let scene_dto = Self::scene_document_dto(&pending.materialized.scene);
        {
            let authority =
                self.require_open_workspace_authoring_mut("apply_procedural_environment")?;
            authority
                .project_content_scenes
                .insert(scene_dto.id.raw(), scene_dto);
            authority.project_content_reference_revision = authority
                .project_content_reference_revision
                .saturating_add(1);
        }

        let offset = pending
            .materialized
            .instance_transform
            .translation
            .to_array()
            .map(f64::from);
        self.reset_voxel_edit_history_with_collision_offset(
            pending.materialized.world.clone(),
            offset,
        );
        let applied_frame = self
            .projection
            .voxel_projector
            .set_instances(
                &pending.materialized.world,
                vec![VoxelProjectionInstance {
                    instance_id: format!("scene-node/{}", pending.voxel_node_id.raw()),
                    asset_id: pending.materialized.asset.asset_id.clone(),
                    transform: pending.materialized.instance_transform,
                }],
            )
            .map_err(|error| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!("applied environment projection failed: {error:?}"),
                )
            })?;
        self.projection
            .pending_voxel_frame
            .ops
            .extend(applied_frame.ops);
        self.remember_workspace_authoring_voxel_write(
            pending.asset_path.clone(),
            pending.materialized.asset.clone(),
        );
        if let Some(authority) = self.workspace_authoring.as_mut().filter(|value| value.open) {
            authority.project_write_generation_provenance =
                Some(svc_serialization::GeneratorMetadata {
                    provider: pending.materialized.provenance.provider_id.clone(),
                    seed: pending.materialized.provenance.seed,
                    version: pending.materialized.provenance.provider_version,
                    params: pending.materialized.provenance.config_hash.clone(),
                });
        }
        self.record_workspace_authoring_mutation();
        self.remember_workspace_authoring_save_candidate(
            pending.materialized.artifact_set_hash.clone(),
        );
        let working_revision = self
            .require_open_workspace_authoring_mut("apply_procedural_environment")?
            .working_revision;
        let candidate = Self::procedural_candidate_dto(
            &pending.candidate_hash,
            &pending.scene_path,
            &pending.asset_path,
            &pending.materialized,
        );
        Ok(ProceduralEnvironmentApplyResultDto {
            accepted: true,
            working_revision,
            save_candidate_hash: Some(pending.materialized.artifact_set_hash),
            candidate: Some(candidate),
            diagnostics: Vec::new(),
        })
    }

    fn procedural_candidate_dto(
        bound_candidate_hash: &str,
        scene_path: &str,
        asset_path: &str,
        candidate: &svc_environment_authoring::MaterializedEnvironment,
    ) -> ProceduralEnvironmentArtifactCandidateDto {
        ProceduralEnvironmentArtifactCandidateDto {
            candidate_hash: bound_candidate_hash.to_owned(),
            scene_file: ProceduralEnvironmentCanonicalFileDto {
                path: scene_path.to_owned(),
                media_type: "application/vnd.asha.scene+json;version=4".to_owned(),
                canonical_json: candidate.scene_json.clone(),
                content_hash: candidate.scene_hash.clone(),
            },
            voxel_file: ProceduralEnvironmentCanonicalFileDto {
                path: asset_path.to_owned(),
                media_type: VOXEL_ASSET_MEDIA_TYPE.to_owned(),
                canonical_json: candidate.asset_json.clone(),
                content_hash: candidate.asset.content_hashes.canonical_json.clone(),
            },
            artifact_set_hash: candidate.artifact_set_hash.clone(),
            scene: Self::scene_document_dto(&candidate.scene),
            asset: candidate.asset.clone(),
            provenance: candidate.provenance.clone(),
            markers: candidate.markers.clone(),
            sources: candidate.sources.clone(),
        }
    }

    fn procedural_preview_rejection(
        code: ProceduralEnvironmentDiagnosticCode,
        path: &str,
        message: &str,
    ) -> ProceduralEnvironmentPreviewResultDto {
        ProceduralEnvironmentPreviewResultDto {
            accepted: false,
            candidate: None,
            preview_frame: None,
            preview_projection_hash: None,
            preview_diff_count: 0,
            diagnostics: vec![ProceduralEnvironmentDiagnosticDto {
                code,
                path: path.to_owned(),
                message: message.to_owned(),
            }],
        }
    }

    fn procedural_apply_rejection(
        working_revision: u64,
        code: ProceduralEnvironmentDiagnosticCode,
        path: &str,
        message: &str,
    ) -> ProceduralEnvironmentApplyResultDto {
        ProceduralEnvironmentApplyResultDto {
            accepted: false,
            working_revision,
            save_candidate_hash: None,
            candidate: None,
            diagnostics: vec![ProceduralEnvironmentDiagnosticDto {
                code,
                path: path.to_owned(),
                message: message.to_owned(),
            }],
        }
    }
}
