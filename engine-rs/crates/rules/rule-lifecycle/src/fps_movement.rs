use super::*;

impl FpsRuntimeSessionState {
    pub fn apply_autonomous_enemy_direct_nav_movement_with_entities(
        &mut self,
        entities: &mut EntityStore,
        entity: EntityId,
        target: [f32; 3],
        max_step_units: f32,
    ) -> Result<FpsAutonomousMovementReceipt, FpsRuntimeError> {
        self.require_autonomous_enemy_movement(entity)?;
        let current = entities
            .transform(entity)
            .ok_or(FpsRuntimeError::RuleMutationRejected {
                entity,
                command: "readTransform",
            })?
            .transform;
        let mut target_position = current.translation;
        target_position.x = target[0];
        target_position.y = target[1];
        target_position.z = target[2];
        let navigation = propose_direct_nav_movement(DirectNavMovementRequest {
            from: current.translation,
            target: target_position,
            max_step_units,
        })
        .map_err(FpsRuntimeError::NavigationRejected)?;
        let next = AuthoringTransform {
            translation: navigation.next_waypoint.to_array(),
            rotation: [
                current.rotation.x,
                current.rotation.y,
                current.rotation.z,
                current.rotation.w,
            ],
            scale: current.scale.to_array(),
        };
        let outcome = svc_entity_authoring::validate_and_apply_rule_owned(
            entities,
            EcrpRuleOwner::TransformRule,
            &EntityAuthoringCommand::SetTransform {
                id: entity,
                transform: next,
            },
        );
        match outcome {
            RuleOwnedEntityAuthoringOutcome::Accepted { .. } => {}
            RuleOwnedEntityAuthoringOutcome::Rejected { .. }
            | RuleOwnedEntityAuthoringOutcome::Forbidden { .. } => {
                return Err(FpsRuntimeError::RuleMutationRejected {
                    entity,
                    command: "autonomousDirectNavMovement",
                });
            }
        }

        let transform = entities
            .transform(entity)
            .expect("accepted transform remains attached")
            .transform;
        let entity_hash = entities.hash().0;
        let health_hash = self.combat.health_hash();
        let replay_hash = hash_autonomous_movement(entity, navigation.path_hash, entity_hash);
        self.replay_records.push(FpsReplayRecord {
            kind: "runtime_session.fps.autonomous_movement.v0",
            entity_hash,
            health_hash,
            record_hash: replay_hash,
        });
        Ok(FpsAutonomousMovementReceipt {
            entity,
            navigation,
            transform,
            entity_hash,
            health_hash,
            replay_hash,
            projection_changed: self
                .render_projection
                .get(&entity)
                .is_some_and(|projection| projection.visible),
        })
    }

    fn require_autonomous_enemy_movement(&self, entity: EntityId) -> Result<(), FpsRuntimeError> {
        let registered_enemy = self.roles.get(&FpsRuntimeRole::Enemy).copied();
        let binding_allows_movement = self
            .definitions
            .get(&entity)
            .and_then(|definition| definition.policy_binding.as_ref())
            .is_some_and(|binding| {
                binding
                    .allowed_intents
                    .iter()
                    .any(|intent| intent == FPS_AUTONOMOUS_DIRECT_NAV_INTENT)
            });
        if registered_enemy != Some(entity) || !binding_allows_movement {
            return Err(FpsRuntimeError::UnauthorizedAutonomousMovement { entity });
        }
        Ok(())
    }
}

impl LoadedFpsRuntimeSession {
    pub fn apply_autonomous_enemy_direct_nav_movement(
        &mut self,
        entity: EntityId,
        target: [f32; 3],
        max_step_units: f32,
    ) -> Result<FpsAutonomousMovementReceipt, FpsRuntimeError> {
        self.session
            .apply_autonomous_enemy_direct_nav_movement_with_entities(
                &mut self.entities,
                entity,
                target,
                max_step_units,
            )
    }
}

fn hash_autonomous_movement(entity: EntityId, path_hash: u64, entity_hash: u64) -> u64 {
    let mut h = Fnv1a::new();
    h.write_str("runtime_session.fps.autonomous_movement.v0");
    h.write_u64(entity.raw());
    h.write_u64(path_hash);
    h.write_u64(entity_hash);
    h.finish()
}
