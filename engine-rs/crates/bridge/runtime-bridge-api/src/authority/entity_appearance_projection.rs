use super::*;

const ENTITY_APPEARANCE_HANDLE_BASE: u64 = 8_000_000;

#[derive(Debug, Clone)]
pub(super) struct EntityAppearanceProjectionSeed {
    entity: EntityId,
    label: String,
    tags: Vec<core_ids::TagId>,
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
                let appearance =
                    runtime_seed
                        .definition
                        .capabilities
                        .iter()
                        .find_map(|capability| match capability {
                            EntityDefinitionCapability::RenderProjection {
                                appearance: Some(appearance),
                                ..
                            } => Some(appearance),
                            _ => None,
                        })?;
                Some((
                    runtime_seed.entity,
                    EntityAppearanceProjectionSeed {
                        entity: runtime_seed.entity,
                        label: runtime_seed.definition.display_name.clone(),
                        tags: runtime_seed.definition.tags.clone(),
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
            let asset = self.entity_appearance_asset(&seed)?;
            if defined_assets.insert(asset.asset.clone()) {
                frame
                    .scene
                    .ops
                    .push(protocol_render::RenderDiff::DefineAnimatedMesh {
                        asset: asset.clone(),
                    });
            }
            let handle = Self::entity_appearance_handle_for_index(index)?;
            let transform = Self::entity_appearance_transform_in(
                &self.scene.entities,
                seed.entity,
                seed.model_scale,
            )?
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!(
                        "entity {} has a retained appearance but is absent from authority",
                        seed.entity.raw()
                    ),
                )
            })?;
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
                        playback: Self::entity_appearance_playback(
                            seed.initial_clip_id.as_deref(),
                            asset.default_clip.as_deref(),
                        ),
                        metadata: protocol_render::RenderMetadata {
                            source: Some(seed.entity),
                            tags: seed.tags,
                            label: Some(seed.label),
                        },
                    },
                });
            if !Self::entity_appearance_visible_in(&self.scene.entities, seed.entity) {
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

    /// Publish a newly accepted EntityStore through the single authority-to-
    /// projection boundary. Domain operations stage mutations in a cloned store
    /// and call this only after their own validation succeeds. Appearance
    /// projection thus follows canonical entity deltas rather than FPS-specific
    /// movement or lifecycle hooks.
    pub(super) fn commit_entity_authority_change(
        &mut self,
        next_entities: EntityStore,
        authority_tick: u64,
    ) -> BridgeResult<()> {
        let seeds = self
            .projection
            .entity_appearances
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut operations = Vec::new();
        let mut next_handles = self.projection.entity_appearance_handles.clone();

        for seed in seeds {
            let before_transform = Self::entity_appearance_transform_in(
                &self.scene.entities,
                seed.entity,
                seed.model_scale,
            )?;
            let after_transform = Self::entity_appearance_transform_in(
                &next_entities,
                seed.entity,
                seed.model_scale,
            )?;
            let before_visible =
                Self::entity_appearance_visible_in(&self.scene.entities, seed.entity);
            let after_visible = Self::entity_appearance_visible_in(&next_entities, seed.entity);
            let current_handle = next_handles.get(&seed.entity).copied();

            match (current_handle, before_transform, after_transform) {
                (Some(handle), _, None) => {
                    operations.push(protocol_render::RenderDiff::Destroy { handle });
                    next_handles.remove(&seed.entity);
                }
                (Some(handle), before, Some(after)) => {
                    let transform = (before != Some(after)).then_some(after);
                    let visible = (before_visible != after_visible).then_some(after_visible);
                    if transform.is_some() || visible.is_some() {
                        operations.push(protocol_render::RenderDiff::Update {
                            handle,
                            transform,
                            material: None,
                            visible,
                            metadata: None,
                        });
                    }
                }
                (None, _, Some(transform)) => {
                    let asset = self.entity_appearance_asset(&seed)?;
                    let handle = self.entity_appearance_handle(seed.entity)?;
                    operations.push(protocol_render::RenderDiff::DefineAnimatedMesh {
                        asset: asset.clone(),
                    });
                    operations.push(protocol_render::RenderDiff::CreateAnimatedMeshInstance {
                        handle,
                        parent: None,
                        instance: protocol_render::AnimatedMeshInstanceDescriptor {
                            asset: asset.asset,
                            transform,
                            material_overrides: Vec::new(),
                            playback: Self::entity_appearance_playback(
                                seed.initial_clip_id.as_deref(),
                                asset.default_clip.as_deref(),
                            ),
                            metadata: protocol_render::RenderMetadata {
                                source: Some(seed.entity),
                                tags: seed.tags,
                                label: Some(seed.label),
                            },
                        },
                    });
                    if !after_visible {
                        operations.push(protocol_render::RenderDiff::Update {
                            handle,
                            transform: None,
                            material: None,
                            visible: Some(false),
                            metadata: None,
                        });
                    }
                    next_handles.insert(seed.entity, handle);
                }
                (None, _, None) => {}
            }
        }

        self.scene.entities = next_entities;
        self.projection.entity_appearance_handles = next_handles;
        if !operations.is_empty() {
            if self
                .projection
                .projection_frame
                .as_ref()
                .is_none_or(|frame| frame.authority_tick != authority_tick)
            {
                self.projection.projection_frame =
                    Some(RuntimeProjectionFrame::empty(authority_tick));
            }
            self.projection
                .projection_frame
                .as_mut()
                .expect("projection frame initialized for accepted entity delta")
                .scene
                .ops
                .extend(operations);
        }
        Ok(())
    }

    fn entity_appearance_asset(
        &self,
        seed: &EntityAppearanceProjectionSeed,
    ) -> BridgeResult<protocol_render::AnimatedMeshAsset> {
        self.projection
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
            })
    }

    fn entity_appearance_handle(
        &self,
        entity: EntityId,
    ) -> BridgeResult<protocol_render::RenderHandle> {
        let index = self
            .projection
            .entity_appearances
            .keys()
            .position(|candidate| *candidate == entity)
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "entity appearance handle requested for an unknown binding",
                )
            })?;
        Self::entity_appearance_handle_for_index(index)
    }

    fn entity_appearance_handle_for_index(
        index: usize,
    ) -> BridgeResult<protocol_render::RenderHandle> {
        let handle_offset = u64::try_from(index).map_err(|_| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "entity appearance count cannot be represented as a render handle",
            )
        })?;
        Ok(protocol_render::RenderHandle::new(
            ENTITY_APPEARANCE_HANDLE_BASE.saturating_add(handle_offset),
        ))
    }

    fn entity_appearance_playback(
        initial_clip_id: Option<&str>,
        default_clip: Option<&str>,
    ) -> Option<protocol_render::AnimatedMeshPlaybackCommand> {
        initial_clip_id.or(default_clip).map(|clip| {
            protocol_render::AnimatedMeshPlaybackCommand::Play {
                clip: clip.to_owned(),
                r#loop: protocol_render::AnimationLoopMode::Repeat,
                speed: 1.0,
                weight: 1.0,
                restart: true,
                fade_seconds: None,
            }
        })
    }

    fn entity_appearance_visible_in(entities: &EntityStore, entity: EntityId) -> bool {
        entities.is_alive(entity)
            && entities
                .render_projection(entity)
                .is_some_and(|projection| projection.visible)
    }

    fn entity_appearance_transform_in(
        entities: &EntityStore,
        entity: EntityId,
        model_scale: [f32; 3],
    ) -> BridgeResult<Option<protocol_render::Transform>> {
        if !entities.contains(entity) {
            return Ok(None);
        }
        let transform = entities.transform(entity).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!(
                    "entity {} has an appearance but no authoritative transform",
                    entity.raw()
                ),
            )
        })?;
        let transform = transform.transform;
        Ok(Some(protocol_render::Transform {
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
        }))
    }
}
