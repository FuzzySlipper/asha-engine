use super::*;

impl EngineBridge {
    pub(super) fn apply_enemy_direct_nav_movement_authority(
        &mut self,
        request: EnemyDirectNavMovementRequest,
    ) -> BridgeResult<EnemyDirectNavMovementResult> {
        self.require_initialized("apply_enemy_direct_nav_movement")?;
        let entity = Self::enemy_entity_id(request.entity)?;
        if self.gameplay.fps_session.is_some() {
            let fps_before = self.fps_session("apply_enemy_direct_nav_movement")?.clone();
            let entities_before = self.scene.entities.clone();
            let mut fps_next = fps_before.clone();
            let mut entities_next = entities_before.clone();
            let receipt = fps_next
                .apply_autonomous_enemy_direct_nav_movement_with_entities(
                    &mut entities_next,
                    entity,
                    request.target.to_array(),
                    request.max_step_units,
                )
                .map_err(Self::fps_runtime_error)?;
            if self.has_static_gameplay_runtime() {
                self.scene.entities = entities_next;
                let tick = self.time.authority_tick;
                let reconcile = self.with_static_gameplay_runtime(
                    "apply_enemy_direct_nav_movement.trigger_reconciliation",
                    |host| {
                        host.reconcile_triggers(
                            tick,
                            gameplay_runtime_host::TriggerReconcileCause::Movement,
                        )
                    },
                );
                if let Err(error) = reconcile {
                    self.scene.entities = entities_before;
                    return Err(error);
                }
                entities_next = core::mem::replace(&mut self.scene.entities, entities_before);
            }
            self.gameplay.fps_session = Some(fps_next);
            if let Err(error) =
                self.commit_entity_authority_change(entities_next, self.time.authority_tick)
            {
                self.gameplay.fps_session = Some(fps_before);
                return Err(error);
            }
            return Ok(EnemyDirectNavMovementResult {
                entity: receipt.entity.raw(),
                authority_source: EnemyDirectNavAuthoritySource::RustEntityStore,
                from: receipt.navigation.from,
                target: receipt.navigation.target,
                next_waypoint: receipt.navigation.next_waypoint,
                distance_units: receipt.navigation.distance_units,
                reached: receipt.navigation.reached,
                path_hash: receipt.navigation.path_hash,
                transform_hash: Self::transform_hash(receipt.entity, receipt.transform),
                projection_changed: receipt.projection_changed,
            });
        }

        let mut entities_next = self.scene.entities.clone();
        let entities = &mut entities_next;
        let (authority_source, current_transform) =
            Self::seed_or_read_enemy_transform(entities, entity, request.seed_position)?;
        let from = current_transform.translation;
        let nav = propose_direct_nav_movement(DirectNavMovementRequest {
            from,
            target: request.target,
            max_step_units: request.max_step_units,
        })
        .map_err(|err| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "enemy direct-nav movement rejected by svc-pathfinding: {}",
                    EnemyDirectNavMovementError::Navigation(err).label()
                ),
            )
        })?;
        let next_transform = EntityTransform {
            translation: nav.next_waypoint,
            ..current_transform
        };
        let transform_event = entities
            .apply_transform(TransformCommand::Set {
                id: entity,
                transform: next_transform,
            })
            .map_err(|err| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "enemy direct-nav movement rejected by core-entity: {}",
                        EnemyDirectNavMovementError::Transform(err).label()
                    ),
                )
            })?;
        self.commit_entity_authority_change(entities_next, self.time.authority_tick)?;
        Ok(EnemyDirectNavMovementResult {
            entity: entity.raw(),
            authority_source,
            from,
            target: nav.target,
            next_waypoint: nav.next_waypoint,
            distance_units: nav.distance_units,
            reached: nav.reached,
            path_hash: nav.path_hash,
            transform_hash: Self::transform_hash(entity, transform_event.transform),
            projection_changed: transform_event.projection_changed,
        })
    }
}
