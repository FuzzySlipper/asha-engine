use super::*;

const ENTITY_APPEARANCE_HANDLE_BASE: u64 = 8_000_000;

#[derive(Debug, Clone)]
pub(super) struct EntityAppearanceProjectionSeed {
    entity: EntityId,
    label: String,
    tags: Vec<core_ids::TagId>,
    visible: bool,
    resource_id: String,
    initial_clip_id: Option<String>,
    model_scale: [f32; 3],
}

impl EngineBridge {
    pub(super) fn install_entity_appearance_bindings(
        &mut self,
        runtime_seeds: &[gameplay_runtime_host::RuntimeProjectEntitySeed],
    ) {
        self.projection.entity_appearances = runtime_seeds
            .iter()
            .filter_map(|runtime_seed| {
                let (visible, appearance) =
                    runtime_seed
                        .definition
                        .capabilities
                        .iter()
                        .find_map(|capability| match capability {
                            EntityDefinitionCapability::RenderProjection {
                                visible,
                                appearance: Some(appearance),
                                ..
                            } => Some((*visible, appearance)),
                            _ => None,
                        })?;
                Some((
                    runtime_seed.entity,
                    EntityAppearanceProjectionSeed {
                        entity: runtime_seed.entity,
                        label: runtime_seed.definition.display_name.clone(),
                        tags: runtime_seed.definition.tags.clone(),
                        visible,
                        resource_id: appearance.resource_id.clone(),
                        initial_clip_id: appearance.initial_clip_id.clone(),
                        model_scale: appearance.model_scale,
                    },
                ))
            })
            .collect();
    }

    /// Rebuild the retained entity-appearance scene from canonical project
    /// state. Existing handles are destroyed first so restart is a real retained
    /// scene lifecycle rather than an out-of-band renderer reset.
    pub(super) fn recreate_entity_appearance_projection(
        &mut self,
        authority_tick: u64,
    ) -> BridgeResult<()> {
        let previous_handles = self
            .projection
            .entity_appearance_handles
            .values()
            .copied()
            .collect::<Vec<_>>();
        let seeds = self
            .projection
            .entity_appearances
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut frame = RuntimeProjectionFrame::empty(authority_tick);
        frame.scene.ops.extend(
            previous_handles
                .into_iter()
                .map(|handle| protocol_render::RenderDiff::Destroy { handle }),
        );
        self.projection.entity_appearance_handles.clear();

        let mut defined_assets = BTreeSet::new();
        for (index, seed) in seeds.into_iter().enumerate() {
            let asset = self
                .projection
                .presentation_catalog
                .animated_mesh(&seed.resource_id)
                .cloned()
                .ok_or_else(|| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::Internal,
                        format!(
                            "validated entity appearance resource `{}` is unavailable at projection",
                            seed.resource_id
                        ),
                    )
                })?;
            if defined_assets.insert(asset.asset.clone()) {
                frame
                    .scene
                    .ops
                    .push(protocol_render::RenderDiff::DefineAnimatedMesh {
                        asset: asset.clone(),
                    });
            }
            let handle_offset = u64::try_from(index).map_err(|_| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "entity appearance count cannot be represented as a render handle",
                )
            })?;
            let handle = protocol_render::RenderHandle::new(
                ENTITY_APPEARANCE_HANDLE_BASE.saturating_add(handle_offset),
            );
            let transform = self.entity_appearance_transform(seed.entity, seed.model_scale)?;
            let playback_clip = seed.initial_clip_id.or_else(|| asset.default_clip.clone());
            let playback =
                playback_clip.map(|clip| protocol_render::AnimatedMeshPlaybackCommand::Play {
                    clip,
                    r#loop: protocol_render::AnimationLoopMode::Repeat,
                    speed: 1.0,
                    weight: 1.0,
                    restart: true,
                    fade_seconds: None,
                });
            frame
                .scene
                .ops
                .push(protocol_render::RenderDiff::CreateAnimatedMeshInstance {
                    handle,
                    parent: None,
                    instance: protocol_render::AnimatedMeshInstanceDescriptor {
                        asset: asset.asset,
                        transform,
                        material_overrides: Vec::new(),
                        playback,
                        metadata: protocol_render::RenderMetadata {
                            source: Some(seed.entity),
                            tags: seed.tags,
                            label: Some(seed.label),
                        },
                    },
                });
            if !seed.visible {
                frame.scene.ops.push(protocol_render::RenderDiff::Update {
                    handle,
                    transform: None,
                    material: None,
                    visible: Some(false),
                    metadata: None,
                });
            }
            self.projection
                .entity_appearance_handles
                .insert(seed.entity, handle);
        }
        self.projection.projection_frame = Some(frame);
        Ok(())
    }

    pub(super) fn project_entity_appearance_transform(
        &mut self,
        entity: EntityId,
        authority_tick: u64,
    ) -> BridgeResult<()> {
        let Some(handle) = self
            .projection
            .entity_appearance_handles
            .get(&entity)
            .copied()
        else {
            return Ok(());
        };
        let model_scale = self
            .projection
            .entity_appearances
            .get(&entity)
            .map(|appearance| appearance.model_scale)
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "retained appearance handle has no canonical appearance binding",
                )
            })?;
        let transform = self.entity_appearance_transform(entity, model_scale)?;
        let mut frame = RuntimeProjectionFrame::empty(authority_tick);
        frame.scene.ops.push(protocol_render::RenderDiff::Update {
            handle,
            transform: Some(transform),
            material: None,
            visible: None,
            metadata: None,
        });
        self.projection.projection_frame = Some(frame);
        Ok(())
    }

    pub(super) fn project_entity_appearance_visibility(
        &mut self,
        entity: EntityId,
        authority_tick: u64,
    ) {
        let Some(handle) = self
            .projection
            .entity_appearance_handles
            .get(&entity)
            .copied()
        else {
            return;
        };
        let visible = self
            .scene
            .entities
            .render_projection(entity)
            .is_some_and(|projection| projection.visible);
        let mut frame = RuntimeProjectionFrame::empty(authority_tick);
        frame.scene.ops.push(protocol_render::RenderDiff::Update {
            handle,
            transform: None,
            material: None,
            visible: Some(visible),
            metadata: None,
        });
        self.projection.projection_frame = Some(frame);
    }

    fn entity_appearance_transform(
        &self,
        entity: EntityId,
        model_scale: [f32; 3],
    ) -> BridgeResult<protocol_render::Transform> {
        let transform = self.scene.entities.transform(entity).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!(
                    "entity {} has an appearance but no authoritative transform",
                    entity.raw()
                ),
            )
        })?;
        let transform = transform.transform;
        Ok(protocol_render::Transform {
            translation: [
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
            ],
            rotation: [
                transform.rotation.x,
                transform.rotation.y,
                transform.rotation.z,
                transform.rotation.w,
            ],
            scale: [
                transform.scale.x * model_scale[0],
                transform.scale.y * model_scale[1],
                transform.scale.z * model_scale[2],
            ],
        })
    }
}
