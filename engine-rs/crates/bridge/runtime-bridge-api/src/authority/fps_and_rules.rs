use super::*;

impl EngineBridge {
    pub(super) fn require_initialized(&self, op: &str) -> BridgeResult<()> {
        if self.engine.is_none() {
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
        self.fps_session.as_ref().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{op} called before load_fps_runtime_session"),
            )
        })
    }

    pub(super) fn fps_session_mut(
        &mut self,
        op: &str,
    ) -> BridgeResult<&mut FpsRuntimeSessionState> {
        self.fps_session.as_mut().ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                format!("{op} called before load_fps_runtime_session"),
            )
        })
    }

    pub(super) fn convert_fps_load_request(
        request: &FpsRuntimeSessionLoadRequest,
    ) -> BridgeResult<FpsProjectBundleLoadInput> {
        let mut definitions = Vec::with_capacity(request.definitions.len());
        for entry in &request.definitions {
            let entity = EntityId::new(entry.entity);
            let mut capabilities = Vec::new();
            if let Some(transform) = &entry.transform {
                capabilities.push(EntityDefinitionCapability::Transform {
                    transform: AuthoringTransform {
                        translation: transform.translation,
                        rotation: transform.rotation,
                        scale: transform.scale,
                    },
                });
            }
            if let Some(bounds) = entry.bounds {
                capabilities.push(EntityDefinitionCapability::Bounds {
                    min: bounds.min,
                    max: bounds.max,
                });
            }
            if let Some(visible) = entry.render_visible {
                capabilities.push(EntityDefinitionCapability::Render { visible });
            }
            if let Some(static_collider) = entry.static_collider {
                capabilities.push(EntityDefinitionCapability::Collision { static_collider });
            }

            definitions.push(FpsStoredEntityDefinition {
                entity,
                definition: EntityDefinition {
                    stable_id: entry.stable_id.clone(),
                    display_name: entry.display_name.clone(),
                    source: EntityDefinitionSourceTrace {
                        project_bundle: request.project_bundle.clone(),
                        relative_path: entry.source_path.clone(),
                    },
                    tags: Vec::new(),
                    metadata: Vec::new(),
                    capabilities,
                },
                role: Self::fps_runtime_role(entry.role),
                health: entry
                    .health
                    .map(|health| HealthState::new(health.current, health.max)),
                weapon: entry.weapon.as_ref().map(|weapon| FpsWeaponMount {
                    weapon_id: weapon.weapon_id.clone(),
                    damage: weapon.damage,
                    range_units: weapon.range_units,
                    ammo: weapon.ammo,
                    cooldown_ticks_after_fire: weapon.cooldown_ticks_after_fire,
                }),
                render_projection: entry
                    .render_visible
                    .map(|visible| FpsRenderProjectionState {
                        projection: match entry.role {
                            FpsBridgeRole::Player => "first_person_camera",
                            FpsBridgeRole::Enemy => "target_actor",
                            FpsBridgeRole::Neutral => "neutral_actor",
                        }
                        .to_string(),
                        visible,
                    }),
                policy_binding: entry
                    .policy_binding
                    .as_ref()
                    .map(|binding| FpsPolicyBinding {
                        binding_id: binding.binding_id.clone(),
                        policy_id: binding.policy_id.clone(),
                        view_kind: binding.view_kind.clone(),
                        view_version: binding.view_version.clone(),
                        allowed_intents: binding.allowed_intents.clone(),
                        runtime_moment: binding.runtime_moment.clone(),
                    }),
            });
        }

        Ok(FpsProjectBundleLoadInput {
            project_bundle: request.project_bundle.clone(),
            definitions,
        })
    }

    pub(super) fn fps_runtime_role(role: FpsBridgeRole) -> FpsRuntimeRole {
        match role {
            FpsBridgeRole::Player => FpsRuntimeRole::Player,
            FpsBridgeRole::Enemy => FpsRuntimeRole::Enemy,
            FpsBridgeRole::Neutral => FpsRuntimeRole::Neutral,
        }
    }

    pub(super) fn verify_game_rule_modules(
        manifests: &[GameRuleModuleManifest],
    ) -> BridgeResult<BTreeMap<String, GameRuleModuleManifest>> {
        let mut loaded = BTreeMap::new();
        for manifest in manifests {
            Self::verify_game_rule_module_manifest(manifest)?;
            if loaded
                .insert(manifest.module_ref.module_id.clone(), manifest.clone())
                .is_some()
            {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "duplicate game rule module declaration '{}'",
                        manifest.module_ref.module_id
                    ),
                ));
            }
        }
        Ok(loaded)
    }

    pub(super) fn verify_game_rule_module_manifest(
        manifest: &GameRuleModuleManifest,
    ) -> BridgeResult<()> {
        if manifest.module_ref.module_id.trim().is_empty() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "declared game rule module id is required",
            ));
        }
        if manifest.module_ref.version.trim().is_empty() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "declared game rule module '{}' is missing a version",
                    manifest.module_ref.module_id
                ),
            ));
        }
        if !manifest.module_ref.contract_hash.starts_with("sha256:") {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "declared game rule module '{}' contract hash must be sha256",
                    manifest.module_ref.module_id
                ),
            ));
        }
        if !manifest.source_hash.starts_with("sha256:") {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "declared game rule module '{}' source hash must be sha256",
                    manifest.module_ref.module_id
                ),
            ));
        }
        for required in GAME_RULE_DETERMINISTIC_REQUIREMENTS {
            if !manifest
                .deterministic_requirements
                .iter()
                .any(|requirement| requirement == required)
            {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "declared game rule module is missing deterministic requirement '{required}'"
                    ),
                ));
            }
        }
        if manifest.declared_hooks.is_empty() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "declared game rule module '{}' must declare at least one hook",
                    manifest.module_ref.module_id
                ),
            ));
        }
        let mut hook_ids = BTreeSet::new();
        let mut declares_supported_weapon_effect = false;
        for hook in &manifest.declared_hooks {
            if hook.hook_id.trim().is_empty() {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "declared game rule module '{}' has an empty hook id",
                        manifest.module_ref.module_id
                    ),
                ));
            }
            if !hook_ids.insert(hook.hook_id.clone()) {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "declared game rule module '{}' declares duplicate hook '{}'",
                        manifest.module_ref.module_id, hook.hook_id
                    ),
                ));
            }
            if hook.kind != GameExtensionHookKind::WeaponEffect {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "declared game rule module '{}' uses unsupported hook kind '{}'",
                        manifest.module_ref.module_id,
                        hook.kind.as_str()
                    ),
                ));
            }
            if hook.input_contract != WEAPON_EFFECT_INPUT_CONTRACT
                || hook.output_contract != GAME_EXTENSION_PROPOSAL_CONTRACT
            {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "declared game rule module '{}' hook '{}' has an incompatible contract",
                        manifest.module_ref.module_id, hook.hook_id
                    ),
                ));
            }
            if !hook.required_capabilities.contains(&"health".to_string())
                || !hook
                    .required_capabilities
                    .contains(&"weaponMount".to_string())
            {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "declared game rule module '{}' hook '{}' is missing required capabilities",
                        manifest.module_ref.module_id, hook.hook_id
                    ),
                ));
            }
            declares_supported_weapon_effect = true;
        }
        if !declares_supported_weapon_effect {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "declared game rule module '{}' does not expose a supported weapon-effect hook",
                    manifest.module_ref.module_id
                ),
            ));
        }
        Ok(())
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

    pub(super) fn validated_damage_modifier_delta(
        request: &WeaponEffectHookRequest,
        receipt: &GameExtensionHookReceipt,
    ) -> Result<i64, GameExtensionDiagnostic> {
        let Some(GameExtensionProposal::DamageModifier {
            target,
            channel_id,
            amount_delta,
            proposal_hash,
            ..
        }) = &receipt.proposal
        else {
            return Err(Self::game_extension_diagnostic(
                GameExtensionDiagnosticCode::InvalidProposal,
                "proposal.kind",
                "weapon-effect hook must return a damageModifier proposal",
            ));
        };
        if Some(*target) != request.target {
            return Err(Self::game_extension_diagnostic(
                GameExtensionDiagnosticCode::InvalidProposal,
                "proposal.target",
                "damageModifier target must match the hook target",
            ));
        }
        if channel_id != "combat.primary_fire.damage" {
            return Err(Self::game_extension_diagnostic(
                GameExtensionDiagnosticCode::InvalidProposal,
                "proposal.channelId",
                "damageModifier channel must be combat.primary_fire.damage",
            ));
        }
        if !proposal_hash.starts_with("fnv1a64:") {
            return Err(Self::game_extension_diagnostic(
                GameExtensionDiagnosticCode::InvalidProposal,
                "proposal.proposalHash",
                "damageModifier proposal hash must be deterministic",
            ));
        }
        Ok(*amount_delta)
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
            entity_hash: session.entities.hash().0,
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
}
