use super::fps_animation_catalog::{
    animation_authority_error, animation_projection_error, primary_fire_animation_catalog,
};
use super::fps_project_diagnostics::{
    runtime_project_seed_domain_error, CanonicalFpsDefinitionSource, CanonicalFpsProjectSeed,
};
use super::*;

impl EngineBridge {
    pub(super) fn project_primary_fire_feedback(
        &mut self,
        request: FpsPrimaryFireRequest,
        result: &FpsPrimaryFireResult,
    ) -> BridgeResult<()> {
        self.project_primary_fire_audio(request, result)?;
        self.project_primary_fire_particles(request, result)?;
        self.project_primary_fire_billboards(request, result)?;
        self.project_primary_fire_animation(request, result)?;
        self.project_primary_fire_telemetry_overlay(request.tick, result)
    }

    pub(super) fn require_initialized(&self, op: &str) -> BridgeResult<()> {
        if self.runtime_project.engine.is_none() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{op} called before initialize_engine"),
            ));
        }
        Ok(())
    }

    pub(super) fn fps_runtime_error(error: FpsRuntimeError) -> RuntimeBridgeError {
        RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("FPS RuntimeSession authority rejected request: {error:?}"),
        )
    }

    pub(super) fn fps_session(&self, op: &str) -> BridgeResult<&FpsRuntimeSessionState> {
        self.gameplay.fps_session.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{op} called before canonical FPS project activation"),
            )
        })
    }

    /// Derive the required FPS domain seed from the canonical admission plan.
    /// The statically installed adapter selects this function before any
    /// project data is inspected; missing or incompatible semantics reject.
    pub(super) fn convert_runtime_project_fps_seed(
        seeds: Vec<gameplay_runtime_host::RuntimeProjectEntitySeed>,
        entry_scene: SceneId,
    ) -> Result<CanonicalFpsProjectSeed, RuntimeProjectLoadError> {
        let project_bundle = seeds
            .first()
            .map(|seed| seed.definition.source.project_bundle.clone())
            .ok_or_else(|| RuntimeProjectLoadError::Domain {
                code: "missingEntityDefinitions".to_owned(),
                document_id: Some(entry_scene.raw().to_string()),
                path: Some("nodes".to_owned()),
                message: "FPS domain activation requires at least one canonical entity definition instance in the entry scene".to_owned(),
            })?;
        if let Some(seed) = seeds
            .iter()
            .find(|seed| seed.definition.source.project_bundle != project_bundle)
        {
            return Err(runtime_project_seed_domain_error(
                "projectBundleMismatch",
                seed,
                "source.projectBundle",
                format!(
                    "entity definition `{}` belongs to ProjectBundle `{}` instead of `{project_bundle}`",
                    seed.definition.stable_id, seed.definition.source.project_bundle
                ),
            ));
        }

        let mut definitions = Vec::with_capacity(seeds.len());
        let mut sources = BTreeMap::new();
        for seed in seeds {
            let mut definition = seed.definition.clone();
            let declared_spawn_marker =
                definition
                    .capabilities
                    .iter()
                    .find_map(|capability| match capability {
                        EntityDefinitionCapability::SpawnMarker { marker_id } => {
                            Some(marker_id.as_str())
                        }
                        _ => None,
                    });
            if declared_spawn_marker != seed.spawn_marker_id.as_deref() {
                return Err(runtime_project_seed_domain_error(
                    "spawnMarkerMismatch",
                    &seed,
                    "capabilities",
                    format!(
                        "canonical entity instance `{}` binds spawn marker {:?}, but definition `{}` declares {:?}",
                        seed.instance_id,
                        seed.spawn_marker_id,
                        definition.stable_id,
                        declared_spawn_marker
                    ),
                ));
            }
            if let Some(transform) =
                definition
                    .capabilities
                    .iter_mut()
                    .find_map(|capability| match capability {
                        EntityDefinitionCapability::Transform { transform } => Some(transform),
                        _ => None,
                    })
            {
                *transform = AuthoringTransform {
                    translation: seed.world_translation,
                    rotation: seed.world_rotation,
                    scale: seed.world_scale,
                };
            }

            let controller =
                definition
                    .capabilities
                    .iter()
                    .find_map(|capability| match capability {
                        EntityDefinitionCapability::Controller { controller_id } => {
                            Some(controller_id.as_str())
                        }
                        _ => None,
                    });
            let faction = definition
                .capabilities
                .iter()
                .find_map(|capability| match capability {
                    EntityDefinitionCapability::Faction { faction_id } => Some(faction_id.as_str()),
                    _ => None,
                });
            let player_signal = controller == Some("player_input") || faction == Some("player");
            let enemy_signal = controller == Some("enemy_policy") || faction == Some("hostile");
            if player_signal && enemy_signal {
                return Err(runtime_project_seed_domain_error(
                    "conflictingFpsRole",
                    &seed,
                    "capabilities",
                    format!(
                        "canonical entity definition `{}` has conflicting player/enemy role capabilities",
                        definition.stable_id
                    ),
                ));
            }
            let role = if player_signal {
                FpsRuntimeRole::Player
            } else if enemy_signal {
                FpsRuntimeRole::Enemy
            } else {
                FpsRuntimeRole::Neutral
            };

            let health = definition
                .capabilities
                .iter()
                .find_map(|capability| match capability {
                    EntityDefinitionCapability::Health { current, max } => {
                        Some(HealthState::new(*current, *max))
                    }
                    _ => None,
                });
            let weapon = definition
                .capabilities
                .iter()
                .find_map(|capability| match capability {
                    EntityDefinitionCapability::WeaponMount {
                        weapon_id,
                        damage,
                        range_units,
                        ammo,
                        cooldown_ticks_after_fire,
                    } => Some(FpsWeaponMount {
                        weapon_id: weapon_id.clone(),
                        damage: *damage,
                        range_units: *range_units,
                        ammo: *ammo,
                        cooldown_ticks_after_fire: *cooldown_ticks_after_fire,
                    }),
                    _ => None,
                });
            let render_projection =
                definition
                    .capabilities
                    .iter()
                    .find_map(|capability| match capability {
                        EntityDefinitionCapability::RenderProjection {
                            projection_id,
                            visible,
                        } => Some(FpsRenderProjectionState {
                            projection: projection_id.clone(),
                            visible: *visible,
                        }),
                        _ => None,
                    });
            let policy_binding =
                definition
                    .capabilities
                    .iter()
                    .find_map(|capability| match capability {
                        EntityDefinitionCapability::PolicyBinding {
                            binding_id,
                            policy_id,
                            view_kind,
                            view_version,
                            allowed_intents,
                            runtime_moment,
                        } => Some(FpsPolicyBinding {
                            binding_id: binding_id.clone(),
                            policy_id: policy_id.clone(),
                            view_kind: view_kind.clone(),
                            view_version: view_version.clone(),
                            allowed_intents: allowed_intents.clone(),
                            runtime_moment: runtime_moment.clone(),
                        }),
                        _ => None,
                    });

            sources.insert(
                seed.entity,
                CanonicalFpsDefinitionSource {
                    document_id: seed.document_id,
                    source_path: seed.source_path,
                },
            );
            definitions.push(FpsStoredEntityDefinition {
                entity: seed.entity,
                definition,
                role,
                health,
                weapon,
                render_projection,
                policy_binding,
            });
        }

        Ok(CanonicalFpsProjectSeed {
            input: FpsProjectBundleLoadInput {
                project_bundle,
                definitions,
            },
            sources,
        })
    }

    pub(super) fn fps_runtime_role(role: FpsBridgeRole) -> FpsRuntimeRole {
        match role {
            FpsBridgeRole::Player => FpsRuntimeRole::Player,
            FpsBridgeRole::Enemy => FpsRuntimeRole::Enemy,
            FpsBridgeRole::Neutral => FpsRuntimeRole::Neutral,
        }
    }

    pub(super) fn validate_loaded_game_rule_module<'a>(
        loaded: &'a BTreeMap<String, GameRuleModuleManifest>,
        request: &WeaponEffectHookRequest,
    ) -> BridgeResult<&'a GameRuleModuleManifest> {
        let manifest = loaded.get(&request.module_ref.module_id).ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "game rule module '{}' is not declared by the loaded RuntimeSession",
                    request.module_ref.module_id
                ),
            )
        })?;
        if manifest.module_ref != request.module_ref {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "game rule module ref does not match the loaded RuntimeSession declaration",
            ));
        }
        if !manifest.declared_hooks.iter().any(|hook| {
            hook.hook_id == request.hook_id && hook.kind == GameExtensionHookKind::WeaponEffect
        }) {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "game rule module '{}' does not declare weapon-effect hook '{}'",
                    request.module_ref.module_id, request.hook_id
                ),
            ));
        }
        Ok(manifest)
    }

    pub(super) fn resolve_weapon_effect_game_rule_module(
        loaded: &BTreeMap<String, GameRuleModuleManifest>,
        request: &WeaponEffectHookRequest,
    ) -> BridgeResult<ResolvedGameRuleModule> {
        let manifest = Self::validate_loaded_game_rule_module(loaded, request)?;
        if manifest.module_ref.module_id == BUILT_IN_GAME_RULE_MODULE_ID {
            Ok(ResolvedGameRuleModule::BuiltIn(
                BuiltInDamageModifierModule::new(request.module_ref.clone()),
            ))
        } else {
            Ok(ResolvedGameRuleModule::Registered(
                RegisteredDamageModifierModule::new(manifest.clone()),
            ))
        }
    }

    pub(super) fn game_extension_diagnostic(
        code: GameExtensionDiagnosticCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> GameExtensionDiagnostic {
        GameExtensionDiagnostic {
            code,
            severity: DiagnosticSeverity::Error,
            path: path.into(),
            message: message.into(),
        }
    }

    pub(super) fn extension_replay_evidence(
        receipt: &GameExtensionHookReceipt,
        validation_status: impl Into<String>,
        event_hashes: Vec<String>,
    ) -> GameExtensionReplayEvidence {
        let validation_status = validation_status.into();
        let rejection_hashes = receipt
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "fnv1a64:{}",
                    Self::fnv1a64(&format!(
                        "{}|{}|{}",
                        diagnostic.code.as_str(),
                        diagnostic.path,
                        diagnostic.message
                    ))
                )
            })
            .collect::<Vec<_>>();
        let replay_hash = format!(
            "fnv1a64:{}",
            Self::fnv1a64(&format!(
                "{}|{}|{}|{}|{}|{:?}|{:?}",
                receipt.module_ref.module_id,
                receipt.hook_id,
                receipt.request_id,
                receipt.input_hash,
                validation_status,
                event_hashes,
                rejection_hashes
            ))
        );
        GameExtensionReplayEvidence {
            module_ref: receipt.module_ref.clone(),
            hook_id: receipt.hook_id.clone(),
            request_id: receipt.request_id.clone(),
            input_hash: receipt.input_hash.clone(),
            proposal_hash: receipt.proposal_hash.clone(),
            validation_status,
            event_hashes,
            rejection_hashes,
            replay_hash,
        }
    }

    pub(super) fn fps_lifecycle_status(status: FpsLifecycleStatus) -> FpsBridgeLifecycleStatus {
        match status {
            FpsLifecycleStatus::Active => FpsBridgeLifecycleStatus::Active,
            FpsLifecycleStatus::EnemyDefeated { entity, tick } => {
                FpsBridgeLifecycleStatus::EnemyDefeated {
                    entity: entity.raw(),
                    tick,
                }
            }
        }
    }

    pub(super) fn fps_read_sets() -> Vec<FpsReadSetEvidence> {
        vec![
            FpsReadSetEvidence {
                view_kind: "runtime_session.lifecycle.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec![
                    "EntityStore.lifecycle".to_string(),
                    "FpsRuntimeSessionState.lifecycle_status".to_string(),
                ],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.health.v0".to_string(),
                owner: "svc-combat".to_string(),
                read_set: vec![
                    "CombatState.health".to_string(),
                    "CombatState.health_hash".to_string(),
                ],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.policy_binding.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec!["FpsStoredEntityDefinition.policy_binding".to_string()],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.replay.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec!["FpsRuntimeSessionState.replay_records".to_string()],
            },
        ]
    }

    pub(super) fn fps_encounter_read_sets() -> Vec<FpsReadSetEvidence> {
        vec![
            FpsReadSetEvidence {
                view_kind: "runtime_session.encounter_director.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec![
                    "FpsRuntimeSessionState.encounter".to_string(),
                    "FpsRuntimeSessionState.lifecycle_status".to_string(),
                ],
            },
            FpsReadSetEvidence {
                view_kind: "runtime_session.encounter_replay.v0".to_string(),
                owner: "rule-lifecycle".to_string(),
                read_set: vec!["FpsRuntimeSessionState.replay_records".to_string()],
            },
        ]
    }

    pub(super) fn bridge_encounter_lifecycle(
        lifecycle: FpsEncounterLifecycleInput,
    ) -> RuleFpsEncounterLifecycleInput {
        RuleFpsEncounterLifecycleInput {
            outcome_kind: lifecycle.outcome_kind,
            terminal: lifecycle.terminal,
            enemy_dead: lifecycle.enemy_dead,
            player_dead: lifecycle.player_dead,
            lifecycle_hash: lifecycle.lifecycle_hash,
        }
    }

    pub(super) fn bridge_encounter_state(state: &FpsEncounterState) -> FpsEncounterStateReadout {
        FpsEncounterStateReadout {
            preset_id: state.preset_id.clone(),
            status: Self::encounter_status_label(state.status).to_string(),
            spawned_enemy_ids: state.spawned_enemy_ids.clone(),
            defeated_enemy_ids: state.defeated_enemy_ids.clone(),
            revision: state.revision,
            last_transition: Self::encounter_last_transition_label(state.last_transition)
                .to_string(),
        }
    }

    pub(super) fn encounter_status_label(status: FpsEncounterStatus) -> &'static str {
        match status {
            FpsEncounterStatus::Pending => "pending",
            FpsEncounterStatus::Active => "active",
            FpsEncounterStatus::Cleared => "cleared",
            FpsEncounterStatus::Failed => "failed",
        }
    }

    pub(super) fn encounter_last_transition_label(
        transition: FpsEncounterLastTransition,
    ) -> &'static str {
        match transition {
            FpsEncounterLastTransition::Initialized => "initialized",
            FpsEncounterLastTransition::Activated => "activated",
            FpsEncounterLastTransition::Cleared => "cleared",
            FpsEncounterLastTransition::Failed => "failed",
            FpsEncounterLastTransition::Reset => "reset",
        }
    }

    pub(super) fn encounter_action(action: &str) -> BridgeResult<FpsEncounterTransitionAction> {
        match action {
            "activate" => Ok(FpsEncounterTransitionAction::Activate),
            "sync_lifecycle" => Ok(FpsEncounterTransitionAction::SyncLifecycle),
            "reset" => Ok(FpsEncounterTransitionAction::Reset),
            other => Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("unknown FPS encounter transition action '{other}'"),
            )),
        }
    }

    pub(super) fn encounter_hash(
        state: &FpsEncounterState,
        lifecycle: &FpsEncounterLifecycleInput,
    ) -> u64 {
        let key = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            state.preset_id,
            Self::encounter_status_label(state.status),
            state.spawned_enemy_ids.join(","),
            state.defeated_enemy_ids.join(","),
            state.revision,
            Self::encounter_last_transition_label(state.last_transition),
            lifecycle.outcome_kind,
            lifecycle.terminal,
            lifecycle.enemy_dead,
            lifecycle.player_dead,
            lifecycle.lifecycle_hash
        );
        u64::from_str_radix(&Self::fnv1a64(&key), 16).expect("fnv1a64 emits hex")
    }

    pub(super) fn encounter_snapshot(
        session: &FpsRuntimeSessionState,
        lifecycle: FpsEncounterLifecycleInput,
    ) -> FpsEncounterDirectorSnapshot {
        let latest = session.replay_records.last();
        let encounter_hash = Self::encounter_hash(&session.encounter, &lifecycle);
        FpsEncounterDirectorSnapshot {
            backend: "engine_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.encounter_director.v0".to_string(),
            mutation_owner: "rule-lifecycle".to_string(),
            workspace_trace: vec!["projected encounter state from rule-lifecycle".to_string()],
            state: Self::bridge_encounter_state(&session.encounter),
            lifecycle,
            read_sets: Self::fps_encounter_read_sets(),
            encounter_hash,
            replay_hash: latest
                .map(|record| record.record_hash)
                .unwrap_or(encounter_hash),
        }
    }

    pub(super) fn encounter_transition_result(
        receipt: FpsEncounterTransitionReceipt,
        lifecycle: FpsEncounterLifecycleInput,
    ) -> FpsEncounterTransitionResult {
        FpsEncounterTransitionResult {
            backend: "engine_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.encounter_transition.v0".to_string(),
            mutation_owner: "rule-lifecycle".to_string(),
            workspace_trace: vec![
                "validated encounter transition against rule-lifecycle".to_string(),
                "serialized accepted encounter transition into replay evidence".to_string(),
            ],
            accepted: receipt.accepted,
            rejection_reason: receipt.rejection_reason.map(str::to_string),
            event_kind: receipt.event_kind.map(str::to_string),
            state: Self::bridge_encounter_state(&receipt.state),
            lifecycle,
            encounter_hash: receipt.encounter_hash,
            replay_hash: receipt.replay_hash,
        }
    }

    pub(super) fn fps_snapshot(
        session: &FpsRuntimeSessionState,
        entities: &EntityStore,
        epoch: u64,
    ) -> BridgeResult<FpsRuntimeSessionSnapshot> {
        let player = session
            .role_entity(FpsRuntimeRole::Player)
            .map_err(Self::fps_runtime_error)?;
        let enemy = session
            .role_entity(FpsRuntimeRole::Enemy)
            .map_err(Self::fps_runtime_error)?;
        let mut health = Vec::new();
        let mut policy_bindings = Vec::new();
        for (entity, definition) in &session.definitions {
            if let Some(state) = session.health(*entity) {
                health.push(FpsEntityHealthReadout {
                    entity: entity.raw(),
                    current: state.current,
                    max: state.max,
                });
            }
            if let Some(binding) = &definition.policy_binding {
                policy_bindings.push(FpsPolicyBindingReadout {
                    entity: entity.raw(),
                    binding_id: binding.binding_id.clone(),
                    policy_id: binding.policy_id.clone(),
                    view_kind: binding.view_kind.clone(),
                    view_version: binding.view_version.clone(),
                    allowed_intents: binding.allowed_intents.clone(),
                    runtime_moment: binding.runtime_moment.clone(),
                });
            }
        }
        let replay_records = session
            .replay_records
            .iter()
            .map(|record| FpsReplayEvidence {
                replay_unit: record.kind.to_string(),
                entity_hash: record.entity_hash,
                health_hash: record.health_hash,
                record_hash: record.record_hash,
            })
            .collect::<Vec<_>>();
        let latest = session.replay_records.last();
        Ok(FpsRuntimeSessionSnapshot {
            backend: "engine_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.authority.v0".to_string(),
            project_bundle: session.project_bundle.clone(),
            session_epoch: epoch,
            lifecycle_status: Self::fps_lifecycle_status(session.lifecycle_status),
            player_entity: player.raw(),
            enemy_entity: enemy.raw(),
            health,
            policy_bindings,
            replay_records,
            read_sets: Self::fps_read_sets(),
            entity_hash: entities.hash().0,
            health_hash: session.combat.health_hash(),
            replay_hash: latest.map(|record| record.record_hash).unwrap_or(0),
        })
    }

    pub(super) fn bridge_health(state: HealthState) -> FpsBridgeHealth {
        FpsBridgeHealth {
            current: state.current,
            max: state.max,
        }
    }

    pub(super) fn primary_fire_result(receipt: FpsPrimaryFireReceipt) -> FpsPrimaryFireResult {
        FpsPrimaryFireResult {
            backend: "engine_bridge_rust".to_string(),
            authority_surface: "runtime_session.fps.primary_fire.v0".to_string(),
            mutation_owner: "rule-lifecycle + svc-combat".to_string(),
            workspace_trace: vec![
                "validated FireIntentCommand against svc-combat".to_string(),
                "serialized accepted combat/lifecycle outcome into replay evidence".to_string(),
            ],
            shooter: receipt.shooter.raw(),
            target: receipt.target.map(EntityId::raw),
            target_health_before: receipt.target_health_before.map(Self::bridge_health),
            target_health_after: receipt.target_health_after.map(Self::bridge_health),
            lifecycle_status: Self::fps_lifecycle_status(receipt.lifecycle_status),
            target_render_visible: receipt.target_render_visible,
            entity_hash: receipt.entity_hash,
            health_hash: receipt.health_hash,
            replay_hash: receipt.replay_hash,
        }
    }

    pub(super) fn ray_from_primary_fire(request: FpsPrimaryFireRequest) -> BridgeResult<Ray> {
        if !request.origin.iter().all(|value| value.is_finite())
            || !request.direction.iter().all(|value| value.is_finite())
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "primary fire origin/direction must be finite",
            ));
        }
        Ok(Ray::new(
            WorldPos::new(request.origin[0], request.origin[1], request.origin[2]),
            WorldVec::new(
                request.direction[0],
                request.direction[1],
                request.direction[2],
            ),
        ))
    }

    pub(super) fn enemy_entity_id(raw: u64) -> BridgeResult<EntityId> {
        if raw == 0 {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                EnemyDirectNavMovementError::InvalidEntity.label(),
            ));
        }
        Ok(EntityId::new(raw))
    }

    pub(super) fn seed_or_read_enemy_transform(
        entities: &mut EntityStore,
        entity: EntityId,
        seed_position: Vec3,
    ) -> BridgeResult<(EnemyDirectNavAuthoritySource, EntityTransform)> {
        if let Some(transform) = entities.transform(entity) {
            return Ok((
                EnemyDirectNavAuthoritySource::RustEntityStore,
                transform.transform,
            ));
        }
        entities
            .apply(EntityLifecycleCommand::Create {
                id: entity,
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .map_err(|err| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!("enemy direct-nav entity seed rejected: {err}"),
                )
            })?;
        let transform = EntityTransform::at(seed_position);
        let attached = entities.attach_transform(entity, transform);
        debug_assert!(attached);
        Ok((EnemyDirectNavAuthoritySource::SeededFromRequest, transform))
    }

    pub(super) fn transform_hash(entity: EntityId, transform: EntityTransform) -> u64 {
        let key = format!(
            "{}|{:.3},{:.3},{:.3}|{:.3},{:.3},{:.3},{:.3}|{:.3},{:.3},{:.3}",
            entity.raw(),
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
            transform.rotation.w,
            transform.scale.x,
            transform.scale.y,
            transform.scale.z
        );
        u64::from_str_radix(&Self::fnv1a64(&key), 16).expect("fnv1a64 emits hex")
    }

    fn primary_fire_presentation_origin(
        &self,
        authority_tick: u64,
        result: &FpsPrimaryFireResult,
    ) -> PresentationOriginRef {
        PresentationOriginRef {
            kind: PresentationOriginKind::OwnerFact,
            id: format!("combat.primary-fire.accepted:{}", result.replay_hash),
            authority_tick,
            causation_id: Some(format!("combat.primary-fire:{}", result.replay_hash)),
            correlation_id: Some(format!("fps.session:{}", self.gameplay.fps_epoch)),
        }
    }

    pub(super) fn project_primary_fire_audio(
        &mut self,
        request: FpsPrimaryFireRequest,
        result: &FpsPrimaryFireResult,
    ) -> BridgeResult<()> {
        let cue = self
            .projection
            .presentation_catalog
            .audio(presentation_catalog::PRIMARY_FIRE_PRESENTATION_SIGNAL)
            .cloned()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "active project has no admitted primary-fire audio cue",
                )
            })?;
        let sequence = self
            .projection
            .projection_frame
            .as_ref()
            .filter(|frame| frame.authority_tick == request.tick)
            .map_or(0, |frame| frame.presentation.ops.len() as u32);
        let meta = PresentationOpMeta {
            sequence,
            origin: Some(self.primary_fire_presentation_origin(request.tick, result)),
        };
        let op = AudioProjectionOp::Emit {
            signal_id: format!(
                "primary-fire:{}:{}",
                self.gameplay.fps_epoch, result.replay_hash
            ),
            descriptor: AudioSourceDescriptor {
                clip: AudioClipRef {
                    asset: cue.asset_id.clone(),
                    content_hash: cue.content_hash.clone(),
                },
                bus: AudioBus::Sfx,
                volume: cue.value,
                pitch: 1.0,
                looping: false,
                spatial_blend: 1.0,
                attenuation: 24.0,
                pan: 0.0,
                emitter: AudioEmitter::World3d {
                    position: [
                        request.origin[0] as f32,
                        request.origin[1] as f32,
                        request.origin[2] as f32,
                    ],
                },
            },
        };
        if self.projection.audio_projector.is_none() {
            self.record_developer_console(DeveloperConsoleEmission {
                severity: DiagnosticSeverity::Error,
                category: DeveloperConsoleCategory::Resource,
                source: DeveloperConsoleSource::Projection,
                message: "primary-fire audio presentation is unavailable".to_owned(),
                correlation: Some(format!("primary-fire:{}", result.replay_hash)),
                authority_tick: Some(request.tick),
                detail: DeveloperConsoleDetail {
                    code: "resource_degraded".to_owned(),
                    operation: Some("apply_fps_primary_fire".to_owned()),
                    resource_kind: Some("audio_projector".to_owned()),
                    resource_id: Some(cue.asset_id.clone()),
                    reason: Some("audio projector unavailable".to_owned()),
                },
            });
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "audio projector is unavailable after initialization",
            ));
        }
        let projection_result = self
            .projection
            .audio_projector
            .as_mut()
            .expect("audio projector availability checked")
            .project(meta, op);
        let projected = match projection_result {
            Ok(projected) => projected,
            Err(diagnostic) => {
                self.record_developer_console(DeveloperConsoleEmission {
                    severity: DiagnosticSeverity::Warning,
                    category: DeveloperConsoleCategory::Resource,
                    source: DeveloperConsoleSource::Projection,
                    message: "primary-fire audio resource was rejected by projection".to_owned(),
                    correlation: Some(format!("primary-fire:{}", result.replay_hash)),
                    authority_tick: Some(request.tick),
                    detail: DeveloperConsoleDetail {
                        code: "resource_degraded".to_owned(),
                        operation: Some("apply_fps_primary_fire".to_owned()),
                        resource_kind: Some("audio_clip".to_owned()),
                        resource_id: Some(cue.asset_id.clone()),
                        reason: Some(format!("{:?}", diagnostic.code)),
                    },
                });
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!(
                        "project primary-fire audio projection rejected: {:?}",
                        diagnostic.code
                    ),
                ));
            }
        };

        if self
            .projection
            .projection_frame
            .as_ref()
            .is_none_or(|frame| frame.authority_tick != request.tick)
        {
            self.projection.projection_frame = Some(RuntimeProjectionFrame::empty(request.tick));
        }
        self.projection
            .projection_frame
            .as_mut()
            .expect("projection frame was initialized")
            .presentation
            .ops
            .push(projected);
        Ok(())
    }

    pub(super) fn project_primary_fire_billboards(
        &mut self,
        request: FpsPrimaryFireRequest,
        result: &FpsPrimaryFireResult,
    ) -> BridgeResult<()> {
        let shooter_role = request.shooter_role.unwrap_or(FpsBridgeRole::Player);
        let target_role = request.target_role.unwrap_or(FpsBridgeRole::Enemy);
        let (shooter_localization_key, shooter_fallback_text) = match shooter_role {
            FpsBridgeRole::Player => ("asha.fps.player.name", "Player"),
            FpsBridgeRole::Enemy => ("asha.fps.enemy.name", "Enemy"),
            FpsBridgeRole::Neutral => ("asha.fps.neutral.name", "Neutral"),
        };
        let (target_health_label_key, target_health_fallback_label) = match target_role {
            FpsBridgeRole::Player => ("asha.fps.player.health", "Player health"),
            FpsBridgeRole::Enemy => ("asha.fps.enemy.health", "Enemy health"),
            FpsBridgeRole::Neutral => ("asha.fps.neutral.health", "Neutral health"),
        };
        let shooter_handle =
            BillboardHandle::new(result.shooter.checked_mul(2).ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "shooter entity id cannot be represented as a billboard handle",
                )
            })?);
        if self
            .projection
            .billboard_projector
            .as_ref()
            .is_some_and(|projector| projector.descriptor(shooter_handle).is_none())
        {
            let shooter_descriptor = BillboardDescriptor {
                anchor: BillboardAnchor::EntityAttached {
                    entity: result.shooter,
                    offset: [0.0, 1.9, 0.0],
                },
                content: BillboardContent::Text {
                    localization_key: shooter_localization_key.to_string(),
                    fallback_text: shooter_fallback_text.to_string(),
                    arguments: Vec::new(),
                },
                font: BillboardFontRef::System {
                    family: "sans-serif".to_string(),
                },
                height_pixels: 20.0,
                color: [0.8, 0.95, 1.0, 1.0],
                background: [0.03, 0.08, 0.12, 0.8],
                max_distance: 35.0,
                layer: BillboardLayer::AlwaysOnTop,
                visible: true,
            };
            self.project_billboard_operation(
                request.tick,
                self.primary_fire_presentation_origin(request.tick, result),
                BillboardProjectionOp::Create {
                    handle: shooter_handle,
                    descriptor: shooter_descriptor,
                },
            )?;
        }

        let Some(target) = result.target else {
            return Ok(());
        };
        let target_handle = BillboardHandle::new(
            target
                .checked_mul(2)
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::Internal,
                        "target entity id cannot be represented as a billboard handle",
                    )
                })?,
        );
        let health = result.target_health_after;
        let content = BillboardContent::Value {
            label_key: target_health_label_key.to_string(),
            fallback_label: target_health_fallback_label.to_string(),
            value: health
                .map(|state| format!("{}/{}", state.current, state.max))
                .unwrap_or_else(|| "unknown".to_string()),
            unit_key: None,
            fallback_unit: None,
        };
        let operation = if self
            .projection
            .billboard_projector
            .as_ref()
            .is_some_and(|projector| projector.descriptor(target_handle).is_some())
        {
            BillboardProjectionOp::Update {
                handle: target_handle,
                patch: BillboardPatch {
                    content: Some(content),
                    ..BillboardPatch::default()
                },
            }
        } else {
            BillboardProjectionOp::Create {
                handle: target_handle,
                descriptor: BillboardDescriptor {
                    anchor: BillboardAnchor::EntityAttached {
                        entity: target,
                        offset: [0.0, 1.9, 0.0],
                    },
                    content,
                    font: BillboardFontRef::System {
                        family: "sans-serif".to_string(),
                    },
                    height_pixels: 24.0,
                    color: [1.0, 0.9, 0.75, 1.0],
                    background: [0.18, 0.04, 0.03, 0.85],
                    max_distance: 45.0,
                    layer: BillboardLayer::Occluded,
                    visible: true,
                },
            }
        };
        self.project_billboard_operation(
            request.tick,
            self.primary_fire_presentation_origin(request.tick, result),
            operation,
        )
    }

    pub(super) fn project_primary_fire_particles(
        &mut self,
        request: FpsPrimaryFireRequest,
        result: &FpsPrimaryFireResult,
    ) -> BridgeResult<()> {
        let cue = self
            .projection
            .presentation_catalog
            .particle(presentation_catalog::PRIMARY_FIRE_PRESENTATION_SIGNAL)
            .cloned()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "active project has no admitted primary-fire particle cue",
                )
            })?;
        let authority_tick = request.tick;
        if self
            .projection
            .projection_frame
            .as_ref()
            .is_none_or(|frame| frame.authority_tick != authority_tick)
        {
            self.projection.projection_frame = Some(RuntimeProjectionFrame::empty(authority_tick));
        }
        let sequence = self
            .projection
            .projection_frame
            .as_ref()
            .expect("projection frame was initialized")
            .presentation
            .ops
            .len() as u32;
        let anchor = result.target.map_or_else(
            || ParticleAnchor::World {
                position: request.origin.map(|value| value as f32),
            },
            |entity| ParticleAnchor::EntityAttached {
                entity,
                offset: [0.0, 1.0, 0.0],
            },
        );
        let origin = self.primary_fire_presentation_origin(authority_tick, result);
        let projected = self
            .projection
            .particle_projector
            .as_mut()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "particle projector is unavailable after initialization",
                )
            })?
            .project(
                PresentationOpMeta {
                    sequence,
                    origin: Some(origin),
                },
                ParticleProjectionOp::Emit {
                    signal_id: format!(
                        "primary-fire-particles:{}:{}",
                        self.gameplay.fps_epoch, result.replay_hash
                    ),
                    descriptor: ParticleEmitterDescriptor {
                        anchor,
                        sprite: ParticleSpriteRef {
                            asset: cue.asset_id,
                            content_hash: cue.content_hash,
                            frame_count: 1,
                        },
                        rate_per_second: 0.0,
                        burst_count: (12.0 * cue.value).round().max(1.0) as u32,
                        lifetime_seconds: [0.6, 1.1],
                        velocity_min: [-1.8, 0.8, -1.8],
                        velocity_max: [1.8, 3.2, 1.8],
                        acceleration: [0.0, -5.5, 0.0],
                        size_curve: vec![
                            ParticleScalarKey {
                                age: 0.0,
                                value: 0.22 * cue.value,
                            },
                            ParticleScalarKey {
                                age: 1.0,
                                value: 0.0,
                            },
                        ],
                        color_curve: vec![
                            ParticleColorKey {
                                age: 0.0,
                                color: [1.0, 0.9, 0.3, 1.0],
                            },
                            ParticleColorKey {
                                age: 1.0,
                                color: [1.0, 0.15, 0.0, 0.0],
                            },
                        ],
                        flipbook_frames_per_second: 0.0,
                        seed: result.replay_hash & ((1_u64 << 53) - 1),
                        max_particles: 32,
                        visible: true,
                    },
                },
            )
            .map_err(|diagnostic| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!(
                        "project primary-fire particle projection rejected: {:?}",
                        diagnostic.code
                    ),
                )
            })?;
        self.projection
            .projection_frame
            .as_mut()
            .expect("projection frame was initialized")
            .presentation
            .ops
            .push(projected);
        Ok(())
    }

    pub(super) fn project_primary_fire_animation(
        &mut self,
        request: FpsPrimaryFireRequest,
        result: &FpsPrimaryFireResult,
    ) -> BridgeResult<()> {
        let entity = EntityId::new(result.shooter);
        let player = self
            .fps_session("project_primary_fire_animation")?
            .role_entity(FpsRuntimeRole::Player)
            .map_err(Self::fps_runtime_error)?;
        if entity != player {
            return Ok(());
        }
        let animation_cue = self
            .projection
            .presentation_catalog
            .animation(super::presentation_catalog::PRIMARY_FIRE_ANIMATION_CUE)
            .cloned()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    "FPS primary-fire animation cue is unavailable from admitted project content",
                )
            })?;
        let presentation_origin = self.primary_fire_presentation_origin(request.tick, result);
        let source_fact_id = presentation_origin.id;
        let causation_id = presentation_origin
            .causation_id
            .expect("primary-fire presentation origin has causation identity");
        let correlation_id = presentation_origin
            .correlation_id
            .expect("primary-fire presentation origin has correlation identity");
        let origin = rule_animation_controller::AnimationInputOrigin {
            source_fact_id: source_fact_id.clone(),
            authority_tick: request.tick,
            causation_id: causation_id.clone(),
            correlation_id: correlation_id.clone(),
        };

        let create_change = if self.projection.animation_controller.is_none() {
            let catalog = rule_animation_controller::validate_animation_catalog(
                primary_fire_animation_catalog(&animation_cue.asset_id, &animation_cue.clip_ids),
            )
            .map_err(|error| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!("built-in animation catalog rejected: {error}"),
                )
            })?;
            let mut controller =
                rule_animation_controller::AnimationControllerAuthority::new(catalog);
            let change = controller
                .attach(entity, "fps.primary-fire")
                .map_err(animation_authority_error)?
                .change;
            self.projection.animation_controller = Some(controller);
            change
        } else {
            None
        };

        if let Some(change) = create_change {
            let meta = self.animation_presentation_meta(
                request.tick,
                request.tick,
                source_fact_id.clone(),
                causation_id.clone(),
                correlation_id.clone(),
            );
            let projected = self
                .projection
                .animation_projector
                .as_mut()
                .ok_or_else(|| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::Internal,
                        "animation projector is unavailable after initialization",
                    )
                })?
                .create(
                    entity,
                    protocol_render::RenderHandle::new(4_100),
                    &animation_cue.asset_id,
                    50,
                    &change,
                    meta,
                )
                .map_err(animation_projection_error)?;
            self.push_animation_projection(request.tick, projected);
        }

        {
            let controller = self
                .projection
                .animation_controller
                .as_mut()
                .ok_or_else(|| {
                    RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::Internal,
                        "animation controller is unavailable after initialization",
                    )
                })?;
            controller
                .set_float(entity, "intensity", 650)
                .map_err(animation_authority_error)?;
            controller
                .set_bool(entity, "active", true)
                .map_err(animation_authority_error)?;
        }
        // One gameplay action advances two fixed controller quanta: the first
        // accepts the semantic transition and the second publishes inspectable
        // blend progress. Both are replayed from the same accepted owner fact.
        for _ in 0..2 {
            self.projection.animation_tick = self.projection.animation_tick.saturating_add(1);
            let change = self
                .projection
                .animation_controller
                .as_mut()
                .expect("animation controller exists")
                .tick_from_fact(entity, self.projection.animation_tick, origin.clone())
                .map_err(animation_authority_error)?
                .change;
            if let Some(change) = change {
                let timing_source = change.state.timing_fact.as_ref().map(|fact| &fact.source);
                let meta = self.animation_presentation_meta(
                    request.tick,
                    timing_source.map_or(request.tick, |source| source.authority_tick),
                    timing_source.map_or_else(
                        || source_fact_id.clone(),
                        |source| source.source_fact_id.clone(),
                    ),
                    timing_source.map_or_else(
                        || causation_id.clone(),
                        |source| source.causation_id.clone(),
                    ),
                    timing_source.map_or_else(
                        || correlation_id.clone(),
                        |source| source.correlation_id.clone(),
                    ),
                );
                let projected = self
                    .projection
                    .animation_projector
                    .as_ref()
                    .ok_or_else(|| {
                        RuntimeBridgeError::new(
                            RuntimeBridgeErrorKind::Internal,
                            "animation projector is unavailable after initialization",
                        )
                    })?
                    .update(entity, &change, meta)
                    .map_err(animation_projection_error)?;
                self.push_animation_projection(request.tick, projected);
            }
        }
        Ok(())
    }

    fn animation_presentation_meta(
        &self,
        frame_tick: u64,
        origin_tick: u64,
        source_fact_id: String,
        causation_id: String,
        correlation_id: String,
    ) -> PresentationOpMeta {
        let sequence = self
            .projection
            .projection_frame
            .as_ref()
            .filter(|frame| frame.authority_tick == frame_tick)
            .map_or(0, |frame| frame.presentation.ops.len() as u32);
        PresentationOpMeta {
            sequence,
            origin: Some(PresentationOriginRef {
                kind: PresentationOriginKind::OwnerFact,
                id: source_fact_id,
                authority_tick: origin_tick,
                causation_id: Some(causation_id),
                correlation_id: Some(correlation_id),
            }),
        }
    }

    fn push_animation_projection(&mut self, authority_tick: u64, projected: PresentationOp) {
        if self
            .projection
            .projection_frame
            .as_ref()
            .is_none_or(|frame| frame.authority_tick != authority_tick)
        {
            self.projection.projection_frame = Some(RuntimeProjectionFrame::empty(authority_tick));
        }
        self.projection
            .projection_frame
            .as_mut()
            .expect("projection frame was initialized")
            .presentation
            .ops
            .push(projected);
    }

    fn project_billboard_operation(
        &mut self,
        authority_tick: u64,
        origin: PresentationOriginRef,
        op: BillboardProjectionOp,
    ) -> BridgeResult<()> {
        if self
            .projection
            .projection_frame
            .as_ref()
            .is_none_or(|frame| frame.authority_tick != authority_tick)
        {
            self.projection.projection_frame = Some(RuntimeProjectionFrame::empty(authority_tick));
        }
        let sequence = self
            .projection
            .projection_frame
            .as_ref()
            .expect("projection frame was initialized")
            .presentation
            .ops
            .len() as u32;
        let projected = self
            .projection
            .billboard_projector
            .as_mut()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "billboard projector is unavailable after initialization",
                )
            })?
            .project(
                PresentationOpMeta {
                    sequence,
                    origin: Some(origin),
                },
                op,
            )
            .map_err(|diagnostic| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!(
                        "built-in FPS billboard projection rejected: {:?}",
                        diagnostic.code
                    ),
                )
            })?;
        self.projection
            .projection_frame
            .as_mut()
            .expect("projection frame was initialized")
            .presentation
            .ops
            .push(projected);
        Ok(())
    }

    pub(super) fn project_primary_fire_telemetry_overlay(
        &mut self,
        authority_tick: u64,
        result: &FpsPrimaryFireResult,
    ) -> BridgeResult<()> {
        if self
            .projection
            .projection_frame
            .as_ref()
            .is_none_or(|frame| frame.authority_tick != authority_tick)
        {
            self.projection.projection_frame = Some(RuntimeProjectionFrame::empty(authority_tick));
        }
        let sequence = self
            .projection
            .projection_frame
            .as_ref()
            .expect("projection frame was initialized")
            .presentation
            .ops
            .len() as u32;
        let handle = TelemetryOverlayHandle::new(1);
        let origin = self.primary_fire_presentation_origin(authority_tick, result);
        let projector = self
            .projection
            .telemetry_overlay_projector
            .as_mut()
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "telemetry overlay projector is unavailable after initialization",
                )
            })?;
        let op = if projector.descriptor(handle).is_some() {
            TelemetryOverlayProjectionOp::Update {
                handle,
                patch: TelemetryOverlayPatch {
                    visible: Some(true),
                    ..TelemetryOverlayPatch::default()
                },
            }
        } else {
            TelemetryOverlayProjectionOp::Create {
                handle,
                descriptor: TelemetryOverlayDescriptor {
                    title: "ASHA runtime".to_string(),
                    corner: TelemetryOverlayCorner::TopRight,
                    refresh_interval_ms: 250,
                    max_frame_time_samples: 60,
                    visible: true,
                },
            }
        };
        let projected = projector
            .project(
                PresentationOpMeta {
                    sequence,
                    origin: Some(origin),
                },
                op,
            )
            .map_err(|diagnostic| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!(
                        "built-in telemetry overlay projection rejected: {:?}",
                        diagnostic.code
                    ),
                )
            })?;
        self.projection
            .projection_frame
            .as_mut()
            .expect("projection frame was initialized")
            .presentation
            .ops
            .push(projected);
        Ok(())
    }

    pub(super) fn reset_presentation_projection(&mut self) {
        self.projection.projection_frame = Some(RuntimeProjectionFrame::empty(0));
        self.projection
            .audio_projector
            .as_mut()
            .expect("audio projector exists after initialization")
            .reset();
        self.projection
            .billboard_projector
            .as_mut()
            .expect("billboard projector exists after initialization")
            .reset();
        self.projection
            .particle_projector
            .as_mut()
            .expect("particle projector exists after initialization")
            .reset();
        self.projection.animation_controller = None;
        self.projection.animation_projector =
            Some(render_animation::AnimationControllerProjector::new());
        self.projection.animation_tick = 0;
        self.projection
            .telemetry_overlay_projector
            .as_mut()
            .expect("telemetry overlay projector exists after initialization")
            .reset();
    }
}
