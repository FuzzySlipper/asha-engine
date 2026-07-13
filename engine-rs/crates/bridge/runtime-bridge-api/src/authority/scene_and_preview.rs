use super::*;

impl EngineBridge {
    const PUBLIC_SCENE_HASH_MASK: u64 = (1_u64 << 53) - 1;

    pub(super) fn initial_scene_document() -> core_scene::FlatSceneDocument {
        core_scene::FlatSceneDocument {
            id: SceneId::new(1),
            schema_version: 1,
            metadata: core_scene::SceneMetadata {
                name: Some("Runtime scene".to_string()),
                authoring_format_version: 1,
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
        let document = self.scene_document.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "initialized bridge has no scene document",
            )
        })?;
        Ok(Self::scene_snapshot_dto(core_scene::scene_object_snapshot(
            document,
        )))
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
        let document = self.scene_document.as_ref().ok_or_else(|| {
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
                self.scene_document = Some(outcome.document);
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
            _ => protocol_scene::SceneNodeKindTag::EmptyGroup,
        }
    }

    fn scene_document_dto(document: &core_scene::FlatSceneDocument) -> FlatSceneDocumentDto {
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
                    },
                })
                .collect(),
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
        };
        Ok(core_scene::SceneNodeRecord {
            id: record.id,
            parent: record.parent,
            child_order: record.child_order,
            transform: core_scene::SceneTransform {
                translation: Vec3::new(
                    record.transform.translation[0],
                    record.transform.translation[1],
                    record.transform.translation[2],
                ),
                rotation: core_scene::Quat::new(
                    record.transform.rotation[0],
                    record.transform.rotation[1],
                    record.transform.rotation[2],
                    record.transform.rotation[3],
                ),
                scale: Vec3::new(
                    record.transform.scale[0],
                    record.transform.scale[1],
                    record.transform.scale[2],
                ),
            },
            kind,
            metadata: core_scene::NodeMetadata {
                label: record.label,
                tags: record.tags,
            },
        })
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
        }
        dto
    }
}
