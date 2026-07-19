use super::*;

impl EngineBridge {
    const PUBLIC_SCENE_HASH_MASK: u64 = (1_u64 << 53) - 1;

    pub(super) fn initial_scene_document() -> core_scene::FlatSceneDocument {
        core_scene::FlatSceneDocument {
            id: SceneId::new(1),
            schema_version: 2,
            metadata: core_scene::SceneMetadata {
                name: Some("Runtime scene".to_string()),
                authoring_format_version: 2,
            },
            dependencies: Vec::new(),
            nodes: vec![core_scene::SceneNodeRecord {
                id: SceneNodeId::new(1),
                parent: None,
                child_order: 0,
                transform: core_scene::SceneTransform::IDENTITY,
                kind: core_scene::SceneNodeKind::EmptyGroup,
                metadata: core_scene::NodeMetadata {
                    label: Some("Root".to_string()),
                    tags: Vec::new(),
                },
            }],
        }
    }

    pub(super) fn read_model_material_preview_authority(
        &self,
        request: ModelMaterialPreviewRequest,
    ) -> BridgeResult<ModelMaterialPreviewSnapshot> {
        self.require_initialized("read_model_material_preview")?;
        let material = request.catalog_entry.material.clone().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "catalog entry {:?} does not carry a material projection",
                    request.catalog_entry.id
                ),
            )
        })?;
        if request.catalog_entry.kind != "material" {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "catalog entry {:?} has kind {:?}, expected material",
                    request.catalog_entry.id, request.catalog_entry.kind
                ),
            ));
        }
        request.mesh_asset.validate().map_err(|error| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("preview mesh asset is invalid: {error:?}"),
            )
        })?;
        if !request
            .mesh_asset
            .material_slots
            .iter()
            .any(|slot| slot.material == request.catalog_entry.id)
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "mesh asset {:?} does not reference material {:?}",
                    request.mesh_asset.asset, request.catalog_entry.id
                ),
            ));
        }

        let render = &material.render;
        let uv_strategy = match render.uv_strategy.as_str() {
            "flat" => protocol_render::MaterialUvStrategy::Flat,
            "planar" => protocol_render::MaterialUvStrategy::Planar,
            "atlas" => protocol_render::MaterialUvStrategy::Atlas,
            other => {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!("material uv strategy {other:?} is not supported"),
                ))
            }
        };
        let descriptor = protocol_render::RenderMaterialDescriptor {
            schema_version: 2,
            id: request.catalog_entry.id.clone(),
            color: [
                render.color.r,
                render.color.g,
                render.color.b,
                render.color.a,
            ],
            texture: render.texture.as_ref().map(|asset| asset.id.clone()),
            roughness: render.roughness,
            texture_tint: [
                render.texture_tint.r,
                render.texture_tint.g,
                render.texture_tint.b,
                render.texture_tint.a,
            ],
            emission_color: [
                render.emission_color.r,
                render.emission_color.g,
                render.emission_color.b,
            ],
            emission_intensity: render.emissive,
            uv_strategy,
        };
        let preview_diff = protocol_render::RenderFrameDiff {
            ops: vec![
                protocol_render::RenderDiff::DefineMaterial {
                    material: descriptor,
                },
                protocol_render::RenderDiff::DefineStaticMesh {
                    asset: request.mesh_asset.clone(),
                },
                protocol_render::RenderDiff::CreateStaticMeshInstance {
                    handle: request.instance_handle,
                    parent: None,
                    instance: protocol_render::StaticMeshInstanceDescriptor {
                        asset: request.mesh_asset.asset.clone(),
                        transform: protocol_render::Transform::IDENTITY,
                        material_overrides: Vec::new(),
                        metadata: protocol_render::RenderMetadata {
                            source: None,
                            tags: Vec::new(),
                            label: Some(format!("Preview {}", request.mesh_asset.asset)),
                        },
                    },
                },
            ],
        };
        Ok(ModelMaterialPreviewSnapshot {
            catalog_entry: request.catalog_entry,
            material,
            mesh_asset: request.mesh_asset,
            preview_diff,
            renderer_classification: "runtime_readback".to_string(),
            diagnostics: Vec::new(),
        })
    }

    pub(super) fn read_scene_object_snapshot_authority(
        &self,
    ) -> BridgeResult<SceneObjectSnapshotDto> {
        self.require_initialized("read_scene_object_snapshot")?;
        let document = self.scene.scene_document.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "initialized bridge has no scene document",
            )
        })?;
        Ok(Self::scene_snapshot_dto(core_scene::scene_object_snapshot(
            document,
        )))
    }

    pub(super) fn decode_scene_document_authority(
        &mut self,
        request: SceneDocumentDecodeRequestDto,
    ) -> BridgeResult<SceneDocumentCodecResultDto> {
        self.require_runtime_or_workspace_authoring("decode_scene_document")?;
        let document = match core_scene::decode(&request.source_text) {
            Ok(document) => document,
            Err(error) => {
                let (code, message) = match error {
                    core_scene::SceneDecodeError::Json(message) => {
                        (SceneDocumentCodecDiagnosticCode::InvalidJson, message)
                    }
                    core_scene::SceneDecodeError::Field(message) => {
                        (SceneDocumentCodecDiagnosticCode::InvalidField, message)
                    }
                    core_scene::SceneDecodeError::Asset(message) => {
                        (SceneDocumentCodecDiagnosticCode::InvalidAsset, message)
                    }
                    core_scene::SceneDecodeError::UnknownKind(kind) => (
                        SceneDocumentCodecDiagnosticCode::UnknownKind,
                        format!("unknown scene node kind {kind:?}"),
                    ),
                    core_scene::SceneDecodeError::UnknownVersionReq(requirement) => (
                        SceneDocumentCodecDiagnosticCode::UnknownVersionRequirement,
                        format!("unknown scene asset version requirement {requirement:?}"),
                    ),
                    core_scene::SceneDecodeError::LegacyDemoScene => (
                        SceneDocumentCodecDiagnosticCode::LegacyDemoScene,
                        "legacy Demo SceneDocument shape is unsupported; migrate to canonical schemaVersion/id/metadata/dependencies/nodes data".to_string(),
                    ),
                };
                return Ok(Self::scene_codec_rejection(code, message));
            }
        };
        let result = Self::scene_codec_result(document);
        self.remember_project_content_scene(&result);
        Ok(result)
    }

    pub(super) fn encode_scene_document_authority(
        &self,
        request: SceneDocumentEncodeRequestDto,
    ) -> BridgeResult<SceneDocumentCodecResultDto> {
        self.require_runtime_or_workspace_authoring("encode_scene_document")?;
        let document = match Self::scene_document_from_dto(request.document) {
            Ok(document) => document,
            Err(error) => {
                return Ok(Self::scene_codec_rejection(
                    SceneDocumentCodecDiagnosticCode::InvalidDocument,
                    error.message,
                ))
            }
        };
        Ok(Self::scene_codec_result(document))
    }

    pub(super) fn apply_scene_document_authoring_authority(
        &mut self,
        request: SceneDocumentAuthoringRequestDto,
    ) -> BridgeResult<SceneDocumentAuthoringResultDto> {
        self.require_runtime_or_workspace_authoring("apply_scene_document_authoring")?;
        let current = match Self::scene_document_from_dto(request.current_document) {
            Ok(document) => document,
            Err(error) => {
                return Ok(Self::scene_authoring_rejection(
                    SceneDocumentAuthoringRejectionCode::InvalidCurrentDocument,
                    error.message,
                    None,
                    None,
                ))
            }
        };
        let current_result = Self::scene_codec_result(current);
        let actual_hash = current_result.content_hash.clone();
        let Some(current_dto) = current_result.document else {
            return Ok(Self::scene_authoring_rejection(
                SceneDocumentAuthoringRejectionCode::InvalidCurrentDocument,
                Self::scene_codec_rejection_message(&current_result),
                None,
                None,
            ));
        };
        let current = Self::scene_document_from_dto(current_dto.clone())?;
        let Some(actual_hash) = actual_hash else {
            return Ok(Self::scene_authoring_rejection(
                SceneDocumentAuthoringRejectionCode::InvalidCurrentDocument,
                "Rust accepted the current document without issuing its content hash",
                None,
                None,
            ));
        };
        if request.expected_content_hash != actual_hash {
            return Ok(Self::scene_authoring_rejection(
                SceneDocumentAuthoringRejectionCode::StaleDocument,
                "stored scene authoring expected hash does not match the current document",
                Some(request.expected_content_hash),
                Some(actual_hash),
            ));
        }

        let mutates_document = !matches!(
            &request.command,
            SceneDocumentAuthoringCommandDto::RefreshProjection { .. }
        );
        let target = request.command.target();
        if target.project_id != request.current_project_id || target.scene_id != current.id {
            return Ok(Self::scene_authoring_rejection(
                SceneDocumentAuthoringRejectionCode::ForeignDocumentIdentity,
                "stored scene authoring command targets a foreign project or scene identity",
                Some(request.expected_content_hash.clone()),
                Some(actual_hash.clone()),
            ));
        }

        let candidate = match request.command {
            SceneDocumentAuthoringCommandDto::RefreshProjection { .. } => current,
            command => {
                let command = match Self::stored_scene_command(command) {
                    Ok(command) => command,
                    Err(error) => {
                        return Ok(Self::scene_authoring_rejection(
                            SceneDocumentAuthoringRejectionCode::InvalidCommand,
                            error.message,
                            Some(request.expected_content_hash.clone()),
                            Some(actual_hash.clone()),
                        ));
                    }
                };
                let expected = core_scene::scene_object_snapshot(&current).document_hash;
                match core_scene::apply_scene_object_command(&current, expected, command) {
                    Ok(outcome) => outcome.document,
                    Err(rejection) => {
                        let (code, message) = Self::stored_scene_command_rejection(rejection);
                        return Ok(Self::scene_authoring_rejection(
                            code,
                            message,
                            Some(request.expected_content_hash.clone()),
                            Some(actual_hash.clone()),
                        ));
                    }
                }
            }
        };
        let candidate_result = Self::scene_codec_result(candidate);
        let content_hash = candidate_result.content_hash.clone();
        let Some(document) = candidate_result.document else {
            return Ok(Self::scene_authoring_rejection(
                SceneDocumentAuthoringRejectionCode::InvalidResultingDocument,
                Self::scene_codec_rejection_message(&candidate_result),
                Some(request.expected_content_hash),
                Some(actual_hash),
            ));
        };
        let canonical = Self::scene_document_from_dto(document.clone())?;
        let authored_light_frame = render_bridge::project_authored_scene_lights(&canonical);
        let result = SceneDocumentAuthoringResultDto {
            accepted: true,
            document: Some(document),
            content_hash,
            authored_light_frame: Some(authored_light_frame),
            rejection: None,
        };
        if let Some(document) = result.document.as_ref() {
            self.remember_project_content_scene_document(document);
            if mutates_document {
                self.record_workspace_authoring_mutation();
            }
        }
        Ok(result)
    }

    fn remember_project_content_scene(&mut self, result: &SceneDocumentCodecResultDto) {
        let (Some(authority), Some(document)) =
            (self.workspace_authoring.as_mut(), result.document.as_ref())
        else {
            return;
        };
        Self::replace_project_content_scene(authority, document);
    }

    fn remember_project_content_scene_document(&mut self, document: &FlatSceneDocumentDto) {
        let Some(authority) = self.workspace_authoring.as_mut() else {
            return;
        };
        Self::replace_project_content_scene(authority, document);
    }

    fn replace_project_content_scene(
        authority: &mut WorkspaceAuthoringAuthority,
        document: &FlatSceneDocumentDto,
    ) {
        let changed = authority.project_content_scenes.get(&document.id.raw()) != Some(document);
        if changed {
            authority
                .project_content_scenes
                .insert(document.id.raw(), document.clone());
            authority.project_content_reference_revision = authority
                .project_content_reference_revision
                .saturating_add(1);
            authority.pending_save_candidate = None;
            authority.pending_project_write = None;
            authority.pending_procedural_environment = None;
        }
    }

    fn stored_scene_command(
        command: SceneDocumentAuthoringCommandDto,
    ) -> BridgeResult<core_scene::SceneObjectCommand> {
        match command {
            SceneDocumentAuthoringCommandDto::RefreshProjection { .. } => {
                Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "refresh projection command must not reach scene mutation dispatch",
                ))
            }
            SceneDocumentAuthoringCommandDto::Create { record, .. } => {
                Ok(core_scene::SceneObjectCommand::Create {
                    record: Self::scene_record_from_dto(record)?,
                })
            }
            SceneDocumentAuthoringCommandDto::Delete { id, .. } => {
                Ok(core_scene::SceneObjectCommand::Delete { id })
            }
            SceneDocumentAuthoringCommandDto::Rename { id, label, .. } => {
                Ok(core_scene::SceneObjectCommand::Rename { id, label })
            }
            SceneDocumentAuthoringCommandDto::Reparent {
                id,
                parent,
                child_order,
                ..
            } => Ok(core_scene::SceneObjectCommand::Reparent {
                id,
                parent,
                child_order,
            }),
            SceneDocumentAuthoringCommandDto::SetTransform { id, transform, .. } => {
                Ok(core_scene::SceneObjectCommand::SetTransform {
                    id,
                    transform: Self::scene_transform_from_dto(transform),
                })
            }
            SceneDocumentAuthoringCommandDto::UpdateLight {
                id, scene_light, ..
            } => Ok(core_scene::SceneObjectCommand::UpdateLight {
                id,
                light: Self::scene_light_from_dto(scene_light),
            }),
            SceneDocumentAuthoringCommandDto::RetargetVoxelAsset {
                id, asset, tags, ..
            } => Ok(core_scene::SceneObjectCommand::RetargetVoxelAsset {
                id,
                asset: Self::scene_asset_from_dto(asset)?,
                tags,
            }),
        }
    }

    fn stored_scene_command_rejection(
        rejection: core_scene::SceneObjectCommandRejection,
    ) -> (SceneDocumentAuthoringRejectionCode, String) {
        let code = match rejection {
            core_scene::SceneObjectCommandRejection::MissingObject { .. } => {
                SceneDocumentAuthoringRejectionCode::MissingTarget
            }
            core_scene::SceneObjectCommandRejection::InvalidBefore { .. }
            | core_scene::SceneObjectCommandRejection::InvalidAfter { .. } => {
                SceneDocumentAuthoringRejectionCode::InvalidResultingDocument
            }
            _ => SceneDocumentAuthoringRejectionCode::InvalidCommand,
        };
        (code, rejection.label().to_string())
    }

    fn scene_codec_rejection_message(result: &SceneDocumentCodecResultDto) -> String {
        result
            .diagnostics
            .first()
            .map(|diagnostic| diagnostic.message.clone())
            .or_else(|| {
                result
                    .validation
                    .errors
                    .first()
                    .map(|error| error.code.as_str().to_string())
            })
            .unwrap_or_else(|| "Rust rejected the stored scene document".to_string())
    }

    fn scene_authoring_rejection(
        code: SceneDocumentAuthoringRejectionCode,
        message: impl Into<String>,
        expected_hash: Option<String>,
        actual_hash: Option<String>,
    ) -> SceneDocumentAuthoringResultDto {
        SceneDocumentAuthoringResultDto {
            accepted: false,
            document: None,
            content_hash: None,
            authored_light_frame: None,
            rejection: Some(SceneDocumentAuthoringRejectionDto {
                code,
                message: message.into(),
                expected_hash,
                actual_hash,
            }),
        }
    }

    pub(super) fn apply_scene_object_command_authority(
        &mut self,
        request: SceneObjectCommandRequestDto,
    ) -> BridgeResult<SceneObjectCommandResultDto> {
        self.require_initialized("apply_scene_object_command")?;
        let command = match request.command {
            SceneObjectCommandDto::Create { record } => core_scene::SceneObjectCommand::Create {
                record: Self::scene_record_from_dto(record)?,
            },
            SceneObjectCommandDto::Delete { id } => core_scene::SceneObjectCommand::Delete { id },
            SceneObjectCommandDto::Rename { id, label } => {
                core_scene::SceneObjectCommand::Rename { id, label }
            }
            SceneObjectCommandDto::Reparent {
                id,
                parent,
                child_order,
            } => core_scene::SceneObjectCommand::Reparent {
                id,
                parent,
                child_order,
            },
            SceneObjectCommandDto::UpdateLight { id, scene_light } => {
                core_scene::SceneObjectCommand::UpdateLight {
                    id,
                    light: Self::scene_light_from_dto(scene_light),
                }
            }
            SceneObjectCommandDto::Select { id } => core_scene::SceneObjectCommand::Select { id },
            SceneObjectCommandDto::Translate { id, .. }
            | SceneObjectCommandDto::Rotate { id, .. } => {
                return Ok(SceneObjectCommandResultDto {
                    accepted: false,
                    outcome: None,
                    rejection: Some(SceneObjectCommandRejectionDto {
                        code: SceneObjectCommandRejectionCode::ReadonlyTransform,
                        id: Some(id),
                        parent: None,
                        expected_hash: None,
                        actual_hash: None,
                        validation_errors: Vec::new(),
                    }),
                })
            }
        };
        let document = self.scene.scene_document.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "initialized bridge has no scene document",
            )
        })?;
        let authority_hash = core_scene::scene_object_snapshot(document).document_hash;
        let public_hash = authority_hash.0 & Self::PUBLIC_SCENE_HASH_MASK;
        if request.expected_document_hash != public_hash {
            return Ok(SceneObjectCommandResultDto {
                accepted: false,
                outcome: None,
                rejection: Some(SceneObjectCommandRejectionDto {
                    code: SceneObjectCommandRejectionCode::StaleSnapshot,
                    id: None,
                    parent: None,
                    expected_hash: Some(request.expected_document_hash),
                    actual_hash: Some(public_hash),
                    validation_errors: Vec::new(),
                }),
            });
        }
        match core_scene::apply_scene_object_command(document, authority_hash, command) {
            Ok(outcome) => {
                let result = SceneObjectCommandResultDto {
                    accepted: true,
                    outcome: Some(SceneObjectCommandOutcomeDto {
                        document: Self::scene_document_dto(&outcome.document),
                        snapshot: Self::scene_snapshot_dto(outcome.snapshot),
                        selected: outcome.selected,
                    }),
                    rejection: None,
                };
                self.scene.scene_document = Some(outcome.document);
                Ok(result)
            }
            Err(rejection) => Ok(SceneObjectCommandResultDto {
                accepted: false,
                outcome: None,
                rejection: Some(Self::scene_rejection_dto(rejection)),
            }),
        }
    }

    fn scene_snapshot_dto(snapshot: core_scene::SceneObjectSnapshot) -> SceneObjectSnapshotDto {
        SceneObjectSnapshotDto {
            // Generated TypeScript represents this as `number`; retain 53 bits so
            // a snapshot hash can make an exact JSON round trip into the guarded
            // command request instead of being rounded by JavaScript.
            document_hash: snapshot.document_hash.0 & Self::PUBLIC_SCENE_HASH_MASK,
            objects: snapshot
                .objects
                .into_iter()
                .map(|object| SceneObjectRecordDto {
                    id: object.id,
                    parent: object.parent,
                    child_order: object.child_order,
                    label: object.label,
                    kind: Self::scene_kind_tag(object.kind),
                    has_renderable_asset: object.has_renderable_asset,
                })
                .collect(),
        }
    }

    fn scene_kind_tag(kind: &str) -> protocol_scene::SceneNodeKindTag {
        match kind {
            "staticMesh" => protocol_scene::SceneNodeKindTag::StaticMesh,
            "sprite" => protocol_scene::SceneNodeKindTag::Sprite,
            "voxelVolume" => protocol_scene::SceneNodeKindTag::VoxelVolume,
            "light" => protocol_scene::SceneNodeKindTag::Light,
            "entityInstance" => protocol_scene::SceneNodeKindTag::EntityInstance,
            "bootstrap" => protocol_scene::SceneNodeKindTag::Bootstrap,
            _ => protocol_scene::SceneNodeKindTag::EmptyGroup,
        }
    }

    pub(super) fn scene_document_dto(
        document: &core_scene::FlatSceneDocument,
    ) -> FlatSceneDocumentDto {
        let document = document.canonical();
        FlatSceneDocumentDto {
            schema_version: document.schema_version,
            id: document.id,
            metadata: SceneMetadataDto {
                name: document.metadata.name,
                authoring_format_version: document.metadata.authoring_format_version,
            },
            dependencies: document
                .dependencies
                .iter()
                .map(Self::scene_asset_dto)
                .collect(),
            nodes: document
                .nodes
                .into_iter()
                .map(|record| SceneNodeRecordDto {
                    id: record.id,
                    parent: record.parent,
                    child_order: record.child_order,
                    label: record.metadata.label,
                    tags: record.metadata.tags,
                    transform: SceneTransformDto {
                        translation: record.transform.translation.to_array(),
                        rotation: [
                            record.transform.rotation.x,
                            record.transform.rotation.y,
                            record.transform.rotation.z,
                            record.transform.rotation.w,
                        ],
                        scale: record.transform.scale.to_array(),
                    },
                    kind: match record.kind {
                        core_scene::SceneNodeKind::EmptyGroup => SceneNodeKindDto::EmptyGroup,
                        core_scene::SceneNodeKind::StaticMesh(asset) => {
                            SceneNodeKindDto::StaticMesh(Self::scene_asset_dto(&asset))
                        }
                        core_scene::SceneNodeKind::Sprite(asset) => {
                            SceneNodeKindDto::Sprite(Self::scene_asset_dto(&asset))
                        }
                        core_scene::SceneNodeKind::VoxelVolume(asset) => {
                            SceneNodeKindDto::VoxelVolume(Self::scene_asset_dto(&asset))
                        }
                        core_scene::SceneNodeKind::Light(light) => {
                            SceneNodeKindDto::Light(Self::scene_light_dto(light))
                        }
                        core_scene::SceneNodeKind::Marker(marker) => SceneNodeKindDto::Marker {
                            marker_id: marker.marker_id,
                        },
                        core_scene::SceneNodeKind::EntityInstance(instance) => {
                            SceneNodeKindDto::EntityInstance {
                                instance: SceneEntityInstanceDto {
                                    instance_id: instance.instance_id,
                                    reference: match instance.reference {
                                        core_scene::SceneEntityReference::EntityDefinition {
                                            stable_id,
                                        } => {
                                            SceneEntityReferenceDto::EntityDefinition { stable_id }
                                        }
                                        core_scene::SceneEntityReference::Prefab {
                                            prefab_id,
                                            variant_id,
                                            instantiation_seed,
                                        } => SceneEntityReferenceDto::Prefab {
                                            prefab_id,
                                            variant_id,
                                            instantiation_seed,
                                        },
                                    },
                                    spawn_marker_id: instance.spawn_marker_id,
                                },
                            }
                        }
                        core_scene::SceneNodeKind::Bootstrap(bindings) => {
                            SceneNodeKindDto::Bootstrap {
                                bindings: Self::scene_bootstrap_bindings_dto(bindings),
                            }
                        }
                    },
                })
                .collect(),
        }
    }

    pub(super) fn scene_document_from_dto(
        document: FlatSceneDocumentDto,
    ) -> BridgeResult<core_scene::FlatSceneDocument> {
        let dependencies = document
            .dependencies
            .into_iter()
            .map(Self::scene_asset_from_dto)
            .collect::<BridgeResult<Vec<_>>>()?;
        let nodes = document
            .nodes
            .into_iter()
            .map(Self::scene_record_from_dto)
            .collect::<BridgeResult<Vec<_>>>()?;
        Ok(core_scene::FlatSceneDocument {
            id: document.id,
            schema_version: document.schema_version,
            metadata: core_scene::SceneMetadata {
                name: document.metadata.name,
                authoring_format_version: document.metadata.authoring_format_version,
            },
            dependencies,
            nodes,
        })
    }

    fn scene_codec_result(document: core_scene::FlatSceneDocument) -> SceneDocumentCodecResultDto {
        let document = document.canonical();
        let mut diagnostics = Vec::new();
        if !(1..=4).contains(&document.schema_version) {
            diagnostics.push(SceneDocumentCodecDiagnosticDto {
                code: SceneDocumentCodecDiagnosticCode::UnsupportedSchema,
                message: format!(
                    "scene schema version {} is unsupported; expected 1, 2, 3, or 4",
                    document.schema_version
                ),
            });
        }
        if !(1..=4).contains(&document.metadata.authoring_format_version) {
            diagnostics.push(SceneDocumentCodecDiagnosticDto {
                code: SceneDocumentCodecDiagnosticCode::UnsupportedAuthoringFormat,
                message: format!(
                    "scene authoring format version {} is unsupported; expected 1, 2, 3, or 4",
                    document.metadata.authoring_format_version
                ),
            });
        }
        let has_lights = document
            .nodes
            .iter()
            .any(|node| matches!(node.kind, core_scene::SceneNodeKind::Light(_)));
        if has_lights
            && (document.schema_version < 2 || document.metadata.authoring_format_version < 2)
        {
            diagnostics.push(SceneDocumentCodecDiagnosticDto {
                code: SceneDocumentCodecDiagnosticCode::UnsupportedAuthoringFormat,
                message: "stored light nodes require scene schema and authoring format version 2"
                    .to_string(),
            });
        }
        let validation = SceneValidationReportDto {
            errors: core_scene::validate(&document)
                .errors
                .into_iter()
                .map(Self::scene_validation_dto)
                .collect(),
        };
        if !diagnostics.is_empty() || !validation.is_ok() {
            return SceneDocumentCodecResultDto {
                accepted: false,
                document: None,
                canonical_json: None,
                content_hash: None,
                diagnostics,
                validation,
            };
        }
        let canonical_json = core_scene::encode(&document);
        let content_hash = format!("fnv1a64:{}", Self::fnv1a64(&canonical_json));
        SceneDocumentCodecResultDto {
            accepted: true,
            document: Some(Self::scene_document_dto(&document)),
            canonical_json: Some(canonical_json),
            content_hash: Some(content_hash),
            diagnostics,
            validation,
        }
    }

    fn scene_codec_rejection(
        code: SceneDocumentCodecDiagnosticCode,
        message: String,
    ) -> SceneDocumentCodecResultDto {
        SceneDocumentCodecResultDto {
            accepted: false,
            document: None,
            canonical_json: None,
            content_hash: None,
            diagnostics: vec![SceneDocumentCodecDiagnosticDto { code, message }],
            validation: SceneValidationReportDto::default(),
        }
    }

    fn scene_record_from_dto(
        record: SceneNodeRecordDto,
    ) -> BridgeResult<core_scene::SceneNodeRecord> {
        let kind = match record.kind {
            SceneNodeKindDto::EmptyGroup => core_scene::SceneNodeKind::EmptyGroup,
            SceneNodeKindDto::StaticMesh(asset) => {
                core_scene::SceneNodeKind::StaticMesh(Self::scene_asset_from_dto(asset)?)
            }
            SceneNodeKindDto::Sprite(asset) => {
                core_scene::SceneNodeKind::Sprite(Self::scene_asset_from_dto(asset)?)
            }
            SceneNodeKindDto::VoxelVolume(asset) => {
                core_scene::SceneNodeKind::VoxelVolume(Self::scene_asset_from_dto(asset)?)
            }
            SceneNodeKindDto::Light(light) => {
                core_scene::SceneNodeKind::Light(Self::scene_light_from_dto(light))
            }
            SceneNodeKindDto::Marker { marker_id } => {
                core_scene::SceneNodeKind::Marker(core_scene::SceneMarker { marker_id })
            }
            SceneNodeKindDto::EntityInstance { instance } => {
                core_scene::SceneNodeKind::EntityInstance(core_scene::SceneEntityInstance {
                    instance_id: instance.instance_id,
                    reference: match instance.reference {
                        SceneEntityReferenceDto::EntityDefinition { stable_id } => {
                            core_scene::SceneEntityReference::EntityDefinition { stable_id }
                        }
                        SceneEntityReferenceDto::Prefab {
                            prefab_id,
                            variant_id,
                            instantiation_seed,
                        } => core_scene::SceneEntityReference::Prefab {
                            prefab_id,
                            variant_id,
                            instantiation_seed,
                        },
                    },
                    spawn_marker_id: instance.spawn_marker_id,
                })
            }
            SceneNodeKindDto::Bootstrap { bindings } => {
                core_scene::SceneNodeKind::Bootstrap(core_scene::SceneBootstrapBindings {
                    generator: bindings.generator.map(|generator| {
                        core_scene::SceneGeneratorBinding {
                            provider_id: generator.provider_id,
                            preset_id: generator.preset_id,
                            seed: generator.seed,
                        }
                    }),
                    catalogs: bindings
                        .catalogs
                        .into_iter()
                        .map(|catalog| core_scene::SceneCatalogBinding {
                            binding_id: catalog.binding_id,
                            catalog_id: catalog.catalog_id,
                            source_path: catalog.source_path,
                        })
                        .collect(),
                })
            }
        };
        Ok(core_scene::SceneNodeRecord {
            id: record.id,
            parent: record.parent,
            child_order: record.child_order,
            transform: Self::scene_transform_from_dto(record.transform),
            kind,
            metadata: core_scene::NodeMetadata {
                label: record.label,
                tags: record.tags,
            },
        })
    }

    pub(super) fn scene_transform_from_dto(
        transform: SceneTransformDto,
    ) -> core_scene::SceneTransform {
        core_scene::SceneTransform {
            translation: Vec3::new(
                transform.translation[0],
                transform.translation[1],
                transform.translation[2],
            ),
            rotation: core_scene::Quat::new(
                transform.rotation[0],
                transform.rotation[1],
                transform.rotation[2],
                transform.rotation[3],
            ),
            scale: Vec3::new(transform.scale[0], transform.scale[1], transform.scale[2]),
        }
    }

    fn scene_asset_from_dto(asset: AssetReferenceDto) -> BridgeResult<AssetReference> {
        let id = AssetId::parse(&asset.id).map_err(|error| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("invalid scene asset id {:?}: {error}", asset.id),
            )
        })?;
        let version = match asset.version {
            AssetVersionReqDto::Any => AssetVersionReq::Any,
            AssetVersionReqDto::Exact(value) => AssetVersionReq::Exact(value),
            AssetVersionReqDto::AtLeast(value) => AssetVersionReq::AtLeast(value),
        };
        let hash = asset
            .hash
            .as_deref()
            .map(AssetHash::parse)
            .transpose()
            .map_err(|error| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!("invalid scene asset hash: {error}"),
                )
            })?;
        Ok(AssetReference::new(id, version, hash))
    }

    fn scene_light_dto(light: core_scene::SceneLight) -> SceneLightDto {
        let shadow = |intent| match intent {
            core_scene::SceneLightShadowIntent::Disabled => SceneLightShadowIntentDto::Disabled,
            core_scene::SceneLightShadowIntent::Requested => SceneLightShadowIntentDto::Requested,
        };
        match light {
            core_scene::SceneLight::Ambient {
                color,
                intensity,
                enabled,
                shadow_intent,
            } => SceneLightDto::Ambient {
                color,
                intensity,
                enabled,
                shadow_intent: shadow(shadow_intent),
            },
            core_scene::SceneLight::Directional {
                color,
                intensity,
                enabled,
                shadow_intent,
            } => SceneLightDto::Directional {
                color,
                intensity,
                enabled,
                shadow_intent: shadow(shadow_intent),
            },
            core_scene::SceneLight::Point {
                color,
                intensity,
                enabled,
                range,
                decay,
                shadow_intent,
            } => SceneLightDto::Point {
                color,
                intensity,
                enabled,
                range,
                decay,
                shadow_intent: shadow(shadow_intent),
            },
            core_scene::SceneLight::Spot {
                color,
                intensity,
                enabled,
                range,
                decay,
                outer_angle_radians,
                penumbra,
                shadow_intent,
            } => SceneLightDto::Spot {
                color,
                intensity,
                enabled,
                range,
                decay,
                outer_angle_radians,
                penumbra,
                shadow_intent: shadow(shadow_intent),
            },
        }
    }

    fn scene_light_from_dto(light: SceneLightDto) -> core_scene::SceneLight {
        let shadow = |intent| match intent {
            SceneLightShadowIntentDto::Disabled => core_scene::SceneLightShadowIntent::Disabled,
            SceneLightShadowIntentDto::Requested => core_scene::SceneLightShadowIntent::Requested,
        };
        match light {
            SceneLightDto::Ambient {
                color,
                intensity,
                enabled,
                shadow_intent,
            } => core_scene::SceneLight::Ambient {
                color,
                intensity,
                enabled,
                shadow_intent: shadow(shadow_intent),
            },
            SceneLightDto::Directional {
                color,
                intensity,
                enabled,
                shadow_intent,
            } => core_scene::SceneLight::Directional {
                color,
                intensity,
                enabled,
                shadow_intent: shadow(shadow_intent),
            },
            SceneLightDto::Point {
                color,
                intensity,
                enabled,
                range,
                decay,
                shadow_intent,
            } => core_scene::SceneLight::Point {
                color,
                intensity,
                enabled,
                range,
                decay,
                shadow_intent: shadow(shadow_intent),
            },
            SceneLightDto::Spot {
                color,
                intensity,
                enabled,
                range,
                decay,
                outer_angle_radians,
                penumbra,
                shadow_intent,
            } => core_scene::SceneLight::Spot {
                color,
                intensity,
                enabled,
                range,
                decay,
                outer_angle_radians,
                penumbra,
                shadow_intent: shadow(shadow_intent),
            },
        }
    }

    fn scene_asset_dto(asset: &AssetReference) -> AssetReferenceDto {
        AssetReferenceDto {
            id: asset.id().as_str().to_string(),
            version: match asset.version() {
                AssetVersionReq::Any => AssetVersionReqDto::Any,
                AssetVersionReq::Exact(value) => AssetVersionReqDto::Exact(value),
                AssetVersionReq::AtLeast(value) => AssetVersionReqDto::AtLeast(value),
            },
            hash: asset.hash().map(|hash| hash.as_str().to_string()),
        }
    }

    fn scene_rejection_dto(
        rejection: core_scene::SceneObjectCommandRejection,
    ) -> SceneObjectCommandRejectionDto {
        let mut dto = SceneObjectCommandRejectionDto {
            code: SceneObjectCommandRejectionCode::InvalidAfter,
            id: None,
            parent: None,
            expected_hash: None,
            actual_hash: None,
            validation_errors: Vec::new(),
        };
        match rejection {
            core_scene::SceneObjectCommandRejection::StaleSnapshot { expected, actual } => {
                dto.code = SceneObjectCommandRejectionCode::StaleSnapshot;
                dto.expected_hash = Some(expected.0);
                dto.actual_hash = Some(actual.0);
            }
            core_scene::SceneObjectCommandRejection::InvalidBefore { errors } => {
                dto.code = SceneObjectCommandRejectionCode::InvalidBefore;
                dto.validation_errors =
                    errors.into_iter().map(Self::scene_validation_dto).collect();
            }
            core_scene::SceneObjectCommandRejection::InvalidAfter { errors } => {
                dto.code = SceneObjectCommandRejectionCode::InvalidAfter;
                dto.validation_errors =
                    errors.into_iter().map(Self::scene_validation_dto).collect();
            }
            core_scene::SceneObjectCommandRejection::MissingObject { id } => {
                dto.code = SceneObjectCommandRejectionCode::MissingObject;
                dto.id = Some(id);
            }
            core_scene::SceneObjectCommandRejection::DuplicateObject { id } => {
                dto.code = SceneObjectCommandRejectionCode::DuplicateObject;
                dto.id = Some(id);
            }
            core_scene::SceneObjectCommandRejection::MissingParent { id, parent } => {
                dto.code = SceneObjectCommandRejectionCode::MissingParent;
                dto.id = Some(id);
                dto.parent = Some(parent);
            }
            core_scene::SceneObjectCommandRejection::SelfParent { id } => {
                dto.code = SceneObjectCommandRejectionCode::SelfParent;
                dto.id = Some(id);
            }
            core_scene::SceneObjectCommandRejection::BlankLabel { id } => {
                dto.code = SceneObjectCommandRejectionCode::BlankLabel;
                dto.id = Some(id);
            }
            core_scene::SceneObjectCommandRejection::WrongObjectKind { id } => {
                dto.code = SceneObjectCommandRejectionCode::WrongObjectKind;
                dto.id = Some(id);
            }
        }
        dto
    }

    fn scene_validation_dto(error: core_scene::SceneValidationError) -> SceneValidationErrorDto {
        let mut dto = SceneValidationErrorDto::of(SceneValidationCode::InvalidTransform);
        match error {
            core_scene::SceneValidationError::DuplicateNodeId { id } => {
                dto.code = SceneValidationCode::DuplicateNodeId;
                dto.node = Some(id);
            }
            core_scene::SceneValidationError::UnknownParent { node, parent } => {
                dto.code = SceneValidationCode::UnknownParent;
                dto.node = Some(node);
                dto.parent = Some(parent);
            }
            core_scene::SceneValidationError::Cycle { path } => {
                dto.code = SceneValidationCode::Cycle;
                dto.cycle_path = path;
            }
            core_scene::SceneValidationError::InvalidTransform { node, reason } => {
                dto.code = SceneValidationCode::InvalidTransform;
                dto.node = Some(node);
                dto.transform_reason = Some(format!("{reason:?}"));
            }
            core_scene::SceneValidationError::InvalidVoxelVolumeTransform { node, reason } => {
                dto.code = SceneValidationCode::InvalidVoxelVolumeTransform;
                dto.node = Some(node);
                dto.detail_reason = Some(reason);
            }
            core_scene::SceneValidationError::AssetKindMismatch {
                node,
                expected,
                actual,
            } => {
                dto.code = SceneValidationCode::AssetKindMismatch;
                dto.node = Some(node);
                dto.expected_kind = Some(expected.prefix().to_string());
                dto.actual_kind = Some(actual.prefix().to_string());
            }
            core_scene::SceneValidationError::InvalidLight { node, reason } => {
                dto.code = SceneValidationCode::InvalidLight;
                dto.node = Some(node);
                dto.light_reason = Some(reason.label().to_string());
            }
            core_scene::SceneValidationError::DuplicateMarkerId { node, marker_id } => {
                dto.code = SceneValidationCode::DuplicateMarkerId;
                dto.node = Some(node);
                dto.detail_reason = Some(marker_id);
            }
            core_scene::SceneValidationError::InvalidMarker { node, reason } => {
                dto.code = SceneValidationCode::InvalidMarker;
                dto.node = Some(node);
                dto.detail_reason = Some(reason);
            }
            core_scene::SceneValidationError::DuplicateEntityInstanceId { node, instance_id } => {
                dto.code = SceneValidationCode::DuplicateEntityInstanceId;
                dto.node = Some(node);
                dto.instance_id = Some(instance_id);
            }
            core_scene::SceneValidationError::InvalidEntityInstance { node, reason } => {
                dto.code = SceneValidationCode::InvalidEntityInstance;
                dto.node = Some(node);
                dto.detail_reason = Some(reason);
            }
            core_scene::SceneValidationError::DuplicateBootstrapNode { node } => {
                dto.code = SceneValidationCode::DuplicateBootstrapNode;
                dto.node = Some(node);
            }
            core_scene::SceneValidationError::InvalidBootstrap { node, reason } => {
                dto.code = SceneValidationCode::InvalidBootstrap;
                dto.node = Some(node);
                dto.detail_reason = Some(reason);
            }
            core_scene::SceneValidationError::DuplicateCatalogBinding { node, binding_id } => {
                dto.code = SceneValidationCode::DuplicateCatalogBinding;
                dto.node = Some(node);
                dto.binding_id = Some(binding_id);
            }
        }
        dto
    }

    fn scene_bootstrap_bindings_dto(
        bindings: core_scene::SceneBootstrapBindings,
    ) -> SceneBootstrapBindingsDto {
        SceneBootstrapBindingsDto {
            generator: bindings
                .generator
                .map(|generator| SceneGeneratorBindingDto {
                    provider_id: generator.provider_id,
                    preset_id: generator.preset_id,
                    seed: generator.seed,
                }),
            catalogs: bindings
                .catalogs
                .into_iter()
                .map(|catalog| SceneCatalogBindingDto {
                    binding_id: catalog.binding_id,
                    catalog_id: catalog.catalog_id,
                    source_path: catalog.source_path,
                })
                .collect(),
        }
    }
}
