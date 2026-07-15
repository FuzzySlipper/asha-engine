use super::*;

impl EngineBridge {
    pub(super) fn configure_voxel_projection_instances_authority(
        &mut self,
        request: VoxelProjectionBindingRequest,
    ) -> BridgeResult<VoxelProjectionBindingReceipt> {
        self.require_initialized("configure_voxel_projection_instances")?;
        if request.workspace_id.trim().is_empty() || request.registry_digest.trim().is_empty() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "voxel projection binding requires nonempty workspace and registry identities",
            ));
        }
        let world = self.voxel.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "configure_voxel_projection_instances called before voxel authority was initialized",
            )
        })?;

        let mut by_id = BTreeMap::new();
        let mut projector_instances = Vec::with_capacity(request.instances.len());
        for binding in &request.instances {
            if by_id
                .insert(binding.instance_id.clone(), binding.clone())
                .is_some()
            {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "duplicate voxel projection instance {:?}",
                        binding.instance_id
                    ),
                ));
            }
            projector_instances.push(VoxelProjectionInstance {
                instance_id: binding.instance_id.clone(),
                asset_id: binding.asset_id.clone(),
                transform: Self::voxel_instance_transform(binding.transform),
            });
        }

        let binding_hash = Self::voxel_instance_binding_hash(&request, world);
        let frame = self
            .projection
            .voxel_projector
            .set_instances(world, projector_instances)
            .map_err(|error| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!("invalid voxel projection instance binding: {error:?}"),
                )
            })?;
        let projection_op_count = frame.ops.len().min(u32::MAX as usize) as u32;
        self.projection.pending_voxel_frame.ops.extend(frame.ops);
        self.projection.voxel_instance_binding = Some(VoxelInstanceBindingAuthority {
            workspace_id: request.workspace_id.clone(),
            workspace_generation: request.workspace_generation,
            working_revision: request.working_revision,
            registry_digest: request.registry_digest.clone(),
            binding_hash: binding_hash.clone(),
            world_hash: rule_voxel_edit::voxel_world_hash(world),
            instances: by_id,
        });

        Ok(VoxelProjectionBindingReceipt {
            workspace_id: request.workspace_id,
            workspace_generation: request.workspace_generation,
            working_revision: request.working_revision,
            registry_digest: request.registry_digest,
            binding_hash,
            instance_count: request.instances.len().min(u32::MAX as usize) as u32,
            projection_op_count,
        })
    }

    pub(super) fn pick_voxel_instance_authority(
        &self,
        request: VoxelInstancePickRequest,
    ) -> BridgeResult<VoxelInstancePickResult> {
        self.require_initialized("pick_voxel_instance")?;
        let Some(binding) = &self.projection.voxel_instance_binding else {
            return Ok(Self::voxel_instance_pick_rejected(
                request,
                VoxelInstancePickRejection::BindingHashMismatch,
            ));
        };
        let world = self.voxel.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "pick_voxel_instance called before voxel authority was initialized",
            )
        })?;
        let rejection = if request.workspace_id != binding.workspace_id
            || request.workspace_generation != binding.workspace_generation
        {
            Some(VoxelInstancePickRejection::StaleWorkspaceGeneration)
        } else if request.working_revision != binding.working_revision
            || rule_voxel_edit::voxel_world_hash(world) != binding.world_hash
        {
            Some(VoxelInstancePickRejection::StaleWorkingRevision)
        } else if request.registry_digest != binding.registry_digest {
            Some(VoxelInstancePickRejection::RegistryDigestChanged)
        } else if request.binding_hash != binding.binding_hash {
            Some(VoxelInstancePickRejection::BindingHashMismatch)
        } else if !binding.instances.contains_key(&request.instance_id) {
            Some(VoxelInstancePickRejection::UnknownInstance)
        } else {
            None
        };
        if let Some(rejection) = rejection {
            return Ok(Self::voxel_instance_pick_rejected(request, rejection));
        }

        let instance = &binding.instances[&request.instance_id];
        let projection = CollisionProjection::build(world);
        let result = rule_voxel_edit::picking::validate_instance_pick(
            &projection,
            Self::voxel_instance_transform(instance.transform),
            Ray::new(
                WorldPos::new(request.origin[0], request.origin[1], request.origin[2]),
                WorldVec::new(
                    request.direction[0],
                    request.direction[1],
                    request.direction[2],
                ),
            ),
            request.max_distance,
            request.renderer_hint.local_voxel,
            request.renderer_hint.local_face,
        );
        let outcome = match result {
            Ok(hit) => VoxelInstancePickOutcome::Hit(VoxelInstancePickHit {
                local_voxel: hit.local.hit.voxel,
                local_chunk: hit.local.hit.chunk,
                local_face: hit.local.hit.face,
                local_place_anchor: hit.local.place_anchor,
                world_point: hit.world_point,
                world_distance: hit.world_distance,
            }),
            Err(rule_voxel_edit::picking::InstancePickInvalidOrRejected::Invalid(_)) => {
                VoxelInstancePickOutcome::Rejected(VoxelInstancePickRejection::InvalidRay)
            }
            Err(rule_voxel_edit::picking::InstancePickInvalidOrRejected::Rejected(
                rule_voxel_edit::picking::PickRejection::NoHit,
            )) => VoxelInstancePickOutcome::Rejected(VoxelInstancePickRejection::NoHit),
            Err(rule_voxel_edit::picking::InstancePickInvalidOrRejected::Rejected(
                rule_voxel_edit::picking::PickRejection::HitMismatch { .. },
            )) => {
                VoxelInstancePickOutcome::Rejected(VoxelInstancePickRejection::RendererHintMismatch)
            }
        };
        Ok(VoxelInstancePickResult {
            workspace_id: request.workspace_id,
            workspace_generation: request.workspace_generation,
            working_revision: request.working_revision,
            binding_hash: request.binding_hash,
            instance_id: request.instance_id,
            outcome,
        })
    }

    fn voxel_instance_pick_rejected(
        request: VoxelInstancePickRequest,
        rejection: VoxelInstancePickRejection,
    ) -> VoxelInstancePickResult {
        VoxelInstancePickResult {
            workspace_id: request.workspace_id,
            workspace_generation: request.workspace_generation,
            working_revision: request.working_revision,
            binding_hash: request.binding_hash,
            instance_id: request.instance_id,
            outcome: VoxelInstancePickOutcome::Rejected(rejection),
        }
    }

    fn voxel_instance_transform(transform: SceneTransformDto) -> core_scene::SceneTransform {
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

    fn voxel_instance_binding_hash(
        request: &VoxelProjectionBindingRequest,
        world: &VoxelWorld,
    ) -> String {
        let mut instances = request.instances.clone();
        instances.sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
        let mut canonical = format!(
            "{}|{}|{}|{}|{:016x}",
            request.workspace_id,
            request.workspace_generation,
            request.working_revision,
            request.registry_digest,
            rule_voxel_edit::voxel_world_hash(world),
        );
        for instance in instances {
            canonical.push_str(&format!(
                "|{}|{}|{}|{:?}|{:?}|{:?}",
                instance.instance_id,
                instance.scene_node_id,
                instance.asset_id,
                instance.transform.translation.map(f32::to_bits),
                instance.transform.rotation.map(f32::to_bits),
                instance.transform.scale.map(f32::to_bits),
            ));
        }
        format!("fnv1a64:{}", Self::fnv1a64(&canonical))
    }
}
