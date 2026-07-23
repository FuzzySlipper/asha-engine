//! Closed-registry gameplay reads and validated prefab interaction ingress.

use crate::{EntityId, GameplayRuntimeHost, GameplayRuntimeHostError};
use protocol_game_extension::{GameplayContractRef, GameplayEventEnvelope};
use rule_gameplay_fabric::{
    adapt_prefab_part_interaction, GameplayOwnerEventContext, PrefabPartInteractionGameplayPayload,
};
pub use rule_gameplay_fabric::{GameplayModuleNamedView, GameplayModuleStateScope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRuntimePrefabInteractionIntent {
    pub actor: EntityId,
    pub role: String,
    pub max_distance_millimeters: u64,
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRuntimePrefabInteractionReceipt {
    pub instance: u64,
    pub target: EntityId,
    pub distance_millimeters: u64,
    pub event: GameplayEventEnvelope,
    pub reaction_frame_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRuntimePrefabInteractionTarget {
    pub instance: u64,
    pub target: EntityId,
    pub distance_millimeters: u64,
}

const MAX_INTERACTION_DISTANCE_MILLIMETERS: u64 = 16_000;

impl GameplayRuntimeHost {
    /// Project one closed-registry module view without revealing or copying its
    /// backing state. The returned canonical bytes remain identified by the
    /// registered contract, provider, revision, and view hash.
    pub fn module_named_view(
        &self,
        view: &GameplayContractRef,
        scope: &GameplayModuleStateScope,
    ) -> Result<GameplayModuleNamedView, GameplayRuntimeHostError> {
        self.session
            .module_state
            .named_view_by_contract(view, scope)
            .map_err(GameplayRuntimeHostError::State)
    }

    /// Resolve the nearest stable role against the active prefab generation,
    /// validate proximity from authoritative transforms, then publish the
    /// standard owner event. TypeScript never selects the target identity.
    pub fn interact_with_prefab_part(
        &mut self,
        intent: GameplayRuntimePrefabInteractionIntent,
    ) -> Result<GameplayRuntimePrefabInteractionReceipt, GameplayRuntimeHostError> {
        let selected = self
            .resolve_prefab_part_interaction_target(&intent)?
            .ok_or_else(|| {
                GameplayRuntimeHostError::Prefab(format!(
                    "no eligible authored prefab role {} is within {} millimeters of actor {}",
                    intent.role,
                    intent.max_distance_millimeters,
                    intent.actor.raw()
                ))
            })?;
        let prefabs = self.prefab_readout();
        let instance = prefabs
            .instances
            .iter()
            .find(|candidate| candidate.instance == selected.instance)
            .expect("resolved prefab instance remains active");
        let target = selected.target;
        let payload = PrefabPartInteractionGameplayPayload {
            actor: intent.actor.raw(),
            instance: instance.instance,
            prefab: instance.prefab,
            role: intent.role.clone(),
            target: target.raw(),
            tick: intent.tick,
        };
        let context = GameplayOwnerEventContext {
            owner_id: "rule-project-bundle.prefab-interaction".to_owned(),
            tick: intent.tick,
            root_id: format!(
                "prefab-interaction:{}:{}:{}:{}",
                intent.tick,
                intent.actor.raw(),
                instance.instance,
                payload.role
            ),
            root_sequence: intent.tick,
            first_event_sequence: 0,
            parent_event_id: None,
        };
        let event = adapt_prefab_part_interaction(&context, &payload)
            .map_err(|error| GameplayRuntimeHostError::Codec(error.to_string()))?;
        let reaction = self.observe(event.clone())?;
        if !reaction.observe.accepted() {
            let diagnostic = reaction
                .observe
                .diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone())
                .unwrap_or_else(|| "gameplay reaction rejected without a diagnostic".to_owned());
            return Err(GameplayRuntimeHostError::AuthoredProgram(format!(
                "prefab interaction transaction rejected: {diagnostic}"
            )));
        }
        Ok(GameplayRuntimePrefabInteractionReceipt {
            instance: instance.instance,
            target,
            distance_millimeters: selected.distance_millimeters,
            event,
            reaction_frame_hash: reaction.frame.frame_hash,
        })
    }

    /// Read the nearest eligible role from Rust authority without publishing an
    /// event. This is the contextual prompt seam used by downstream shells.
    pub fn resolve_prefab_part_interaction_target(
        &self,
        intent: &GameplayRuntimePrefabInteractionIntent,
    ) -> Result<Option<GameplayRuntimePrefabInteractionTarget>, GameplayRuntimeHostError> {
        if intent.role.trim().is_empty()
            || intent.max_distance_millimeters == 0
            || intent.max_distance_millimeters > MAX_INTERACTION_DISTANCE_MILLIMETERS
        {
            return Err(GameplayRuntimeHostError::Prefab(format!(
                "interaction role and distance must name a role within 1..={MAX_INTERACTION_DISTANCE_MILLIMETERS} millimeters"
            )));
        }
        let entities = self
            .session
            .bundle
            .runtime_entities
            .as_ref()
            .ok_or(GameplayRuntimeHostError::MissingEntityAuthority)?;
        if !entities.is_alive(intent.actor) {
            return Err(GameplayRuntimeHostError::Prefab(format!(
                "interaction actor {} is not active",
                intent.actor.raw()
            )));
        }
        let actor_translation = entities
            .transform(intent.actor)
            .ok_or_else(|| {
                GameplayRuntimeHostError::Prefab(format!(
                    "interaction actor {} has no authoritative transform",
                    intent.actor.raw()
                ))
            })?
            .transform
            .translation;
        let max_distance = intent.max_distance_millimeters as f32 / 1_000.0;
        let max_distance_squared = max_distance * max_distance;
        let prefabs = self.prefab_readout();
        let mut selected: Option<(f32, u64, EntityId)> = None;
        for instance in &prefabs.instances {
            for role in instance
                .roles
                .iter()
                .filter(|candidate| candidate.role == intent.role)
            {
                let target = EntityId::new(role.entity);
                if !entities.is_alive(target) {
                    continue;
                }
                if !self.authored_program.as_ref().is_some_and(|program| {
                    program.prefab_interaction_is_eligible(instance.instance, &intent.role)
                }) {
                    continue;
                }
                let Some(target_transform) = entities.transform(target) else {
                    continue;
                };
                let distance_squared =
                    (target_transform.transform.translation - actor_translation).length_squared();
                if !distance_squared.is_finite() || distance_squared > max_distance_squared {
                    continue;
                }
                let candidate = (distance_squared, instance.instance, target);
                let replaces = selected.as_ref().is_none_or(|current| {
                    distance_squared.total_cmp(&current.0).is_lt()
                        || (distance_squared == current.0
                            && (instance.instance, target.raw()) < (current.1, current.2.raw()))
                });
                if replaces {
                    selected = Some(candidate);
                }
            }
        }
        Ok(selected.map(|(distance_squared, instance, target)| {
            GameplayRuntimePrefabInteractionTarget {
                instance,
                target,
                distance_millimeters: (distance_squared.sqrt() * 1_000.0).round() as u64,
            }
        }))
    }
}
