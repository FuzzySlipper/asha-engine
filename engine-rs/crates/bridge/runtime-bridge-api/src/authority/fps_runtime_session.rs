use super::*;

use gameplay_runtime_host::{
    GameplayRuntimeDecisionOwner, GameplayRuntimeDecisionOwnerOutput,
    GameplayRuntimeTransactionCheckpoint,
};
use protocol_game_extension::{
    GameplayCausationRef, GameplayEmitterRef, GameplayEntityRef, GameplayOwnerRef,
    GameplayProposalEnvelope,
};
use rule_gameplay_fabric::{
    gameplay_payload_hash, GameplayDecisionMoment, GameplayDecisionStatus,
    GameplayOperationWorkspace, PrimaryFireGameplayDecisionWorkspace, StandardGameplayProposalKind,
    PRIMARY_FIRE_DECISION_OWNER_ID,
};
use svc_combat::CombatFireOutcome;

const PRIMARY_FIRE_DAMAGE_CHANNEL: &str = "value.health";

struct StagedPrimaryFireOwner {
    expected_revision: String,
    initial: PrimaryFireGameplayDecisionWorkspace,
    final_workspace: Option<PrimaryFireGameplayDecisionWorkspace>,
}

struct ComposedPrimaryFireDamage {
    damage_delta: i64,
    evidence_checkpoint: GameplayRuntimeTransactionCheckpoint,
}

impl StagedPrimaryFireOwner {
    fn reject(code: &str) -> GameplayRuntimeDecisionOwnerOutput {
        GameplayRuntimeDecisionOwnerOutput {
            accepted: false,
            diagnostic_codes: vec![code.to_owned()],
            ..GameplayRuntimeDecisionOwnerOutput::default()
        }
    }
}

impl GameplayRuntimeDecisionOwner for StagedPrimaryFireOwner {
    fn revision_hash(&self, owner: &GameplayOwnerRef) -> String {
        if owner.owner_id == PRIMARY_FIRE_DECISION_OWNER_ID {
            self.expected_revision.clone()
        } else {
            "unknown-primary-fire-owner".to_owned()
        }
    }

    fn route_precommit(
        &mut self,
        owner: &GameplayOwnerRef,
        operation: &GameplayProposalEnvelope,
    ) -> GameplayRuntimeDecisionOwnerOutput {
        if owner.owner_id != PRIMARY_FIRE_DECISION_OWNER_ID
            || operation.proposal != StandardGameplayProposalKind::ResolvePrimaryFire.contract()
        {
            return Self::reject("primaryFireOwnerMismatch");
        }
        let Ok(workspace) = serde_json::from_slice::<PrimaryFireGameplayDecisionWorkspace>(
            &operation.canonical_payload,
        ) else {
            return Self::reject("primaryFireWorkspaceDecodeRejected");
        };
        let expected_source = Some(GameplayEntityRef {
            entity: EntityId::new(self.initial.shooter),
        });
        let expected_targets = self
            .initial
            .target
            .map(|target| GameplayEntityRef {
                entity: EntityId::new(target),
            })
            .into_iter()
            .collect::<Vec<_>>();
        let immutable_fields_match = workspace.shooter == self.initial.shooter
            && workspace.shooter_role == self.initial.shooter_role
            && workspace.target == self.initial.target
            && workspace.range_millimeters == self.initial.range_millimeters
            && workspace.base_damage == self.initial.base_damage
            && workspace.channel_id == self.initial.channel_id
            && workspace.tick == self.initial.tick
            && operation.source == expected_source
            && operation.targets == expected_targets;
        if !immutable_fields_match {
            return Self::reject("primaryFireImmutableWorkspaceChanged");
        }
        if workspace.damage == 0 {
            return Self::reject("primaryFireDamageRejected");
        }
        self.final_workspace = Some(workspace);
        GameplayRuntimeDecisionOwnerOutput {
            accepted: true,
            fact_hashes: vec![operation.payload_hash.clone()],
            diagnostic_codes: Vec::new(),
        }
    }
}

impl EngineBridge {
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
        let composed_damage = if self.has_static_gameplay_runtime() {
            Some(self.resolve_composed_primary_fire_damage(
                request.tick,
                shooter_role,
                target_role,
                ray,
                &projection,
                &fps_before,
                &entities_before,
            )?)
        } else {
            None
        };
        let damage_delta = composed_damage
            .as_ref()
            .map(|resolved| resolved.damage_delta)
            .unwrap_or(0);
        let mut fps_next = fps_before.clone();
        let mut entities_next = entities_before.clone();
        let receipt = match fps_next
            .apply_primary_fire_for_roles_with_entities(FpsPrimaryFireAuthorityInput {
                entities: &mut entities_next,
                projection: &projection,
                ray,
                tick: request.tick,
                shooter_role,
                target_role,
                damage_delta,
            })
            .map_err(Self::fps_runtime_error)
        {
            Ok(receipt) => receipt,
            Err(error) => {
                if let Some(resolved) = composed_damage {
                    self.restore_primary_fire_decision_evidence(resolved.evidence_checkpoint)?;
                }
                return Err(error);
            }
        };
        self.gameplay.fps_session = Some(fps_next);
        self.scene.entities = entities_next;
        if let Err(error) =
            self.deliver_static_gameplay_owner_events(receipt.gameplay_events.clone())
        {
            self.gameplay.fps_session = Some(fps_before);
            self.scene.entities = entities_before;
            if let Some(resolved) = composed_damage {
                self.restore_primary_fire_decision_evidence(resolved.evidence_checkpoint)?;
            }
            return Err(error);
        }
        let mut result = Self::primary_fire_result(receipt);
        if composed_damage.is_some() {
            result.workspace_trace = vec![
                "constructed typed primary-fire Workspace from authoritative combat preview"
                    .to_owned(),
                "ran Guard -> Transform -> React inside the composed gameplay Fabric".to_owned(),
                "revalidated the final Workspace and committed through rule-lifecycle + svc-combat"
                    .to_owned(),
            ];
        }
        self.project_primary_fire_feedback(request, &result)?;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_composed_primary_fire_damage(
        &mut self,
        tick: u64,
        shooter_role: FpsRuntimeRole,
        target_role: FpsRuntimeRole,
        ray: Ray,
        projection: &CollisionProjection,
        fps_before: &FpsRuntimeSessionState,
        entities_before: &EntityStore,
    ) -> BridgeResult<ComposedPrimaryFireDamage> {
        let weapon = fps_before
            .primary_fire_weapon_for_roles(shooter_role, target_role)
            .map_err(Self::fps_runtime_error)?;
        let mut preview_session = fps_before.clone();
        let mut preview_entities = entities_before.clone();
        let preview = preview_session
            .apply_primary_fire_for_roles_with_entities(FpsPrimaryFireAuthorityInput {
                entities: &mut preview_entities,
                projection,
                ray,
                tick,
                shooter_role,
                target_role,
                damage_delta: 0,
            })
            .map_err(Self::fps_runtime_error)?;
        let range_millimeters = match preview.combat.outcome {
            CombatFireOutcome::Hit { distance, .. } => {
                let millimeters = (distance * 1_000.0).round();
                Some(millimeters.clamp(0.0, f64::from(u32::MAX)) as u32)
            }
            CombatFireOutcome::Miss { .. } => None,
        };
        let initial = PrimaryFireGameplayDecisionWorkspace {
            shooter: preview.shooter.raw(),
            shooter_role: shooter_role.label().to_owned(),
            target: preview.target.map(EntityId::raw),
            range_millimeters,
            base_damage: weapon.damage,
            damage: weapon.damage,
            channel_id: PRIMARY_FIRE_DAMAGE_CHANNEL.to_owned(),
            tick,
        };
        let canonical_payload = serde_json::to_vec(&initial).map_err(|error| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("primary-fire decision Workspace did not serialize: {error}"),
            )
        })?;
        let contract = StandardGameplayProposalKind::ResolvePrimaryFire.contract();
        let workspace =
            GameplayOperationWorkspace::from_payload(contract.clone(), canonical_payload.clone());
        let decision_id = format!(
            "fps-primary-fire:{tick}:{}:{}:{}",
            initial.shooter,
            initial.target.unwrap_or(0),
            fps_before.replay_records.len(),
        );
        let expected_revision = format!(
            "fps-primary-fire:{}:{}:{}:{}",
            self.gameplay.fps_epoch,
            entities_before.hash().0,
            fps_before.combat.health_hash(),
            fps_before
                .replay_records
                .last()
                .map(|record| record.record_hash)
                .unwrap_or(0)
        );
        let operation = GameplayProposalEnvelope {
            proposal_id: format!("{decision_id}:proposal"),
            proposal: contract,
            tick,
            root_sequence: fps_before.replay_records.len() as u64,
            wave: 0,
            proposal_sequence: 0,
            emitter: GameplayEmitterRef::Owner {
                owner_id: PRIMARY_FIRE_DECISION_OWNER_ID.to_owned(),
            },
            causation: GameplayCausationRef {
                root_id: decision_id.clone(),
                parent_event_id: None,
                decision_id: Some(decision_id.clone()),
            },
            originating_event_id: None,
            source: Some(GameplayEntityRef {
                entity: preview.shooter,
            }),
            targets: preview
                .target
                .map(|entity| GameplayEntityRef { entity })
                .into_iter()
                .collect(),
            payload_hash: gameplay_payload_hash(&canonical_payload),
            canonical_payload,
        };
        let evidence_checkpoint = self
            .with_static_gameplay_runtime("checkpoint_primary_fire_decision", |host| {
                Ok(host.checkpoint_transaction_evidence())
            })?
            .expect("composed gameplay runtime checked above");
        let mut owner = StagedPrimaryFireOwner {
            expected_revision: expected_revision.clone(),
            initial: initial.clone(),
            final_workspace: None,
        };
        let receipt = match self.decide_composed_gameplay(
            GameplayDecisionMoment {
                decision_id,
                operation,
                expected_owner_revision: expected_revision,
                workspace,
                resume_token: None,
            },
            &mut owner,
        ) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.restore_primary_fire_decision_evidence(evidence_checkpoint)?;
                return Err(error);
            }
        };
        if receipt.status != GameplayDecisionStatus::Accepted {
            self.restore_primary_fire_decision_evidence(evidence_checkpoint)?;
            let diagnostic = receipt
                .diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.as_str())
                .unwrap_or("gameplay Fabric rejected primary-fire decision");
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("primary-fire decision was not accepted: {diagnostic}"),
            ));
        }
        let Some(final_workspace) = owner.final_workspace else {
            self.restore_primary_fire_decision_evidence(evidence_checkpoint)?;
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "accepted primary-fire decision did not stage an owner Workspace",
            ));
        };
        Ok(ComposedPrimaryFireDamage {
            damage_delta: i64::from(final_workspace.damage) - i64::from(initial.base_damage),
            evidence_checkpoint,
        })
    }

    fn restore_primary_fire_decision_evidence(
        &mut self,
        checkpoint: GameplayRuntimeTransactionCheckpoint,
    ) -> BridgeResult<()> {
        self.with_static_gameplay_runtime("restore_primary_fire_decision", |host| {
            host.restore_transaction_evidence(checkpoint);
            Ok(())
        })?
        .expect("composed gameplay runtime checked above");
        Ok(())
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
                "restart_fps_runtime_session called before canonical FPS project activation",
            )
        })?;
        let input = seed;
        let mut entities = self
            .gameplay
            .static_gameplay_base_entities
            .clone()
            .unwrap_or_default();
        let loaded = if input
            .definitions
            .iter()
            .all(|definition| entities.contains(definition.entity))
        {
            load_fps_project_bundle_from_existing_entities(&mut entities, input)
        } else {
            load_fps_project_bundle_into(&mut entities, input)
        }
        .map_err(Self::fps_runtime_error)?;
        if let Some(checkpoint) = self.gameplay.static_gameplay_reset_checkpoint.clone() {
            self.with_static_gameplay_runtime("restart_fps_runtime_session", move |host| {
                host.restore_reset_state(checkpoint)
            })?
            .expect("composed gameplay reset checkpoint requires a static gameplay host");
        }
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
                    "apply_fps_encounter_transition called before canonical FPS project activation",
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

#[cfg(test)]
mod semantic_origin_tests {
    use super::*;

    fn operation(workspace: &PrimaryFireGameplayDecisionWorkspace) -> GameplayProposalEnvelope {
        let canonical_payload = serde_json::to_vec(workspace).unwrap();
        GameplayProposalEnvelope {
            proposal_id: "proposal.semantic-origin".to_owned(),
            proposal: StandardGameplayProposalKind::ResolvePrimaryFire.contract(),
            tick: workspace.tick,
            root_sequence: 0,
            wave: 0,
            proposal_sequence: 0,
            emitter: GameplayEmitterRef::Owner {
                owner_id: PRIMARY_FIRE_DECISION_OWNER_ID.to_owned(),
            },
            causation: GameplayCausationRef {
                root_id: "root.semantic-origin".to_owned(),
                parent_event_id: None,
                decision_id: Some("decision.semantic-origin".to_owned()),
            },
            originating_event_id: None,
            source: Some(GameplayEntityRef {
                entity: EntityId::new(workspace.shooter),
            }),
            targets: workspace
                .target
                .map(|entity| GameplayEntityRef {
                    entity: EntityId::new(entity),
                })
                .into_iter()
                .collect(),
            payload_hash: gameplay_payload_hash(&canonical_payload),
            canonical_payload,
        }
    }

    #[test]
    fn primary_fire_owner_rejects_a_transformed_semantic_shooter_role() {
        let initial = PrimaryFireGameplayDecisionWorkspace {
            shooter: 10,
            shooter_role: "player".to_owned(),
            target: Some(20),
            range_millimeters: Some(1_500),
            base_damage: 40,
            damage: 40,
            channel_id: PRIMARY_FIRE_DAMAGE_CHANNEL.to_owned(),
            tick: 7,
        };
        let mut owner = StagedPrimaryFireOwner {
            expected_revision: "revision.semantic-origin".to_owned(),
            initial: initial.clone(),
            final_workspace: None,
        };
        let mut transformed = initial;
        transformed.shooter_role = "enemy".to_owned();

        let output = owner.route_precommit(
            &StandardGameplayProposalKind::ResolvePrimaryFire.owner(),
            &operation(&transformed),
        );

        assert!(!output.accepted);
        assert_eq!(
            output.diagnostic_codes,
            vec!["primaryFireImmutableWorkspaceChanged"]
        );
        assert!(owner.final_workspace.is_none());
    }
}
