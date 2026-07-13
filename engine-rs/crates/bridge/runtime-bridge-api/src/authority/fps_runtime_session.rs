use super::*;

impl EngineBridge {
    pub(super) fn load_fps_runtime_session_authority(
        &mut self,
        request: FpsRuntimeSessionLoadRequest,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        self.require_initialized("load_fps_runtime_session")?;
        let input = Self::convert_fps_load_request(&request)?;
        let game_rule_modules = Self::verify_game_rule_modules(&request.game_rule_modules)?;
        let mut entities = self
            .gameplay
            .static_gameplay_base_entities
            .clone()
            .unwrap_or_default();
        let loaded =
            load_fps_project_bundle_into(&mut entities, input).map_err(Self::fps_runtime_error)?;
        self.scene.entities = entities;
        self.gameplay.fps_session = Some(loaded);
        self.gameplay.fps_seed = Some(request);
        self.gameplay.fps_epoch = self.gameplay.fps_epoch.saturating_add(1);
        self.gameplay.game_rule_modules = game_rule_modules;
        self.reset_presentation_projection();
        Self::fps_snapshot(
            self.gameplay.fps_session.as_ref().expect("just committed"),
            &self.scene.entities,
            self.gameplay.fps_epoch,
        )
    }

    pub(super) fn apply_fps_primary_fire_authority(
        &mut self,
        request: FpsPrimaryFireRequest,
    ) -> BridgeResult<FpsPrimaryFireResult> {
        self.require_initialized("apply_fps_primary_fire")?;
        let shooter_role = request
            .shooter_role
            .map(Self::fps_runtime_role)
            .unwrap_or(FpsRuntimeRole::Player);
        let target_role = request
            .target_role
            .map(Self::fps_runtime_role)
            .unwrap_or(FpsRuntimeRole::Enemy);
        let ray = Self::ray_from_primary_fire(request)?;
        let world = self.voxel.voxel.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "apply_fps_primary_fire called before initialize_engine",
            )
        })?;
        let projection = self.collision_projection(world);
        let fps_before = self.fps_session("apply_fps_primary_fire")?.clone();
        let entities_before = self.scene.entities.clone();
        let session = self.gameplay.fps_session.as_mut().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "apply_fps_primary_fire called before load_fps_runtime_session",
            )
        })?;
        let receipt = session
            .apply_primary_fire_for_roles_with_entities(FpsPrimaryFireAuthorityInput {
                entities: &mut self.scene.entities,
                projection: &projection,
                ray,
                tick: request.tick,
                shooter_role,
                target_role,
                damage_delta: 0,
            })
            .map_err(Self::fps_runtime_error)?;
        if let Err(error) =
            self.deliver_static_gameplay_owner_events(receipt.gameplay_events.clone())
        {
            self.gameplay.fps_session = Some(fps_before);
            self.scene.entities = entities_before;
            return Err(error);
        }
        let result = Self::primary_fire_result(receipt);
        self.project_primary_fire_feedback(request, &result)?;
        Ok(result)
    }

    pub(super) fn restart_fps_runtime_session_authority(
        &mut self,
        request: FpsRuntimeSessionRestartRequest,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        self.require_initialized("restart_fps_runtime_session")?;
        if request.expected_epoch != self.gameplay.fps_epoch {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "restart expected epoch {} but current epoch is {}",
                    request.expected_epoch, self.gameplay.fps_epoch
                ),
            ));
        }
        let seed = self.gameplay.fps_seed.clone().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "restart_fps_runtime_session called before load_fps_runtime_session",
            )
        })?;
        let input = Self::convert_fps_load_request(&seed)?;
        let mut entities = self
            .gameplay
            .static_gameplay_base_entities
            .clone()
            .unwrap_or_default();
        let loaded =
            load_fps_project_bundle_into(&mut entities, input).map_err(Self::fps_runtime_error)?;
        self.scene.entities = entities;
        self.gameplay.fps_session = Some(loaded);
        self.gameplay.fps_epoch = self.gameplay.fps_epoch.saturating_add(1);
        self.reset_presentation_projection();
        Self::fps_snapshot(
            self.gameplay.fps_session.as_ref().expect("just restarted"),
            &self.scene.entities,
            self.gameplay.fps_epoch,
        )
    }

    pub(super) fn apply_fps_encounter_transition_authority(
        &mut self,
        request: FpsEncounterTransitionRequest,
    ) -> BridgeResult<FpsEncounterTransitionResult> {
        self.require_initialized("apply_fps_encounter_transition")?;
        let action = Self::encounter_action(&request.action)?;
        let lifecycle = request.lifecycle;
        let rule_lifecycle = Self::bridge_encounter_lifecycle(lifecycle.clone());
        let entities = &self.scene.entities;
        let receipt = self
            .gameplay
            .fps_session
            .as_mut()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    "apply_fps_encounter_transition called before load_fps_runtime_session",
                )
            })?
            .apply_encounter_transition_with_entities(
                entities,
                &request.preset_id,
                action,
                &rule_lifecycle,
            )
            .map_err(Self::fps_runtime_error)?;
        Ok(Self::encounter_transition_result(receipt, lifecycle))
    }
}
