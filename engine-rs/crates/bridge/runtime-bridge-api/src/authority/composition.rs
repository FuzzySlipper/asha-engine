use super::*;

use gameplay_runtime_host::{
    GameplayDecisionMoment, GameplayDecisionReceipt, GameplayRuntimeDecisionOwner,
    GameplayRuntimeHost, GameplayRuntimeHostError, GameplayRuntimeHostReadout,
    GameplayRuntimePrefabInteractionIntent, GameplayRuntimeSchedulerCommand,
    GameplayRuntimeSchedulerCommandReceipt, GameplayRuntimeSchedulerRoutingReceipt,
    GameplayStaticComposition, ScheduledActionId,
};
use rule_gameplay_fabric::GameplayModuleStateScope;
use serde::{Deserialize, Serialize};

const COMPOSED_RUNTIME_SESSION_SCHEMA_VERSION: u32 = 2;

/// Closed engine-owned domain adapter installed before project admission.
/// The adapter declares both availability and requirement: a project loaded by
/// this RuntimeSession must satisfy the selected domain's typed semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProjectDomainAdapter {
    Fps,
}

/// Pre-runtime composition root for Studio and other trusted project tools.
///
/// Consuming a static gameplay composition here retains only its immutable
/// registry, schema, and typed-codec authority. It never loads a ProjectBundle,
/// creates runtime entities, activates module state, or installs a gameplay
/// RuntimeSession host in the returned bridge.
pub struct StaticProjectAuthoringBuilder {
    project_content_admission: rule_project_bundle::GameplayProjectContentAdmission,
}

impl StaticProjectAuthoringBuilder {
    pub fn from_static_composition(composition: GameplayStaticComposition) -> Self {
        Self {
            project_content_admission: rule_project_bundle::GameplayProjectContentAdmission::new(
                composition.project_configuration_authority(),
            ),
        }
    }

    pub fn build(self) -> EngineBridge {
        let mut bridge = EngineBridge::new();
        bridge.gameplay.static_project_content_admission = Some(self.project_content_admission);
        bridge
    }
}

/// Static provider composition installed before any project is admitted.
/// Project source changes therefore require no native-provider rebuild, while
/// the immutable Rust schemas/codecs/behaviors remain fixed for this bridge.
pub struct DeferredRuntimeSessionBuilder {
    composition: GameplayStaticComposition,
    domain_adapter: Option<RuntimeProjectDomainAdapter>,
}

impl DeferredRuntimeSessionBuilder {
    pub fn from_static_composition(composition: GameplayStaticComposition) -> Self {
        Self {
            composition,
            domain_adapter: None,
        }
    }

    /// Install one typed domain requirement as part of the immutable runtime
    /// composition. Domain selection never depends on inspecting project data.
    pub fn with_project_domain(mut self, domain_adapter: RuntimeProjectDomainAdapter) -> Self {
        self.domain_adapter = Some(domain_adapter);
        self
    }

    pub fn build_unloaded(self) -> EngineBridge {
        let project_content_admission = rule_project_bundle::GameplayProjectContentAdmission::new(
            self.composition.project_configuration_authority(),
        );
        let mut bridge = EngineBridge::new();
        bridge.gameplay.static_project_content_admission = Some(project_content_admission);
        bridge.gameplay.static_gameplay_composition = Some(self.composition);
        bridge.gameplay.static_project_domain_adapter = self.domain_adapter;
        bridge
    }
}

/// Bounded public evidence for one composed cell. The hash binds the sole
/// EntityStore to gameplay registry/module/scheduler/continuation evidence and
/// the current FPS replay epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComposedRuntimeSessionReadout {
    pub schema_version: u32,
    pub entity_authority_hash: String,
    pub gameplay: GameplayRuntimeHostReadout,
    pub fps_session_epoch: u64,
    pub fps_replay_hash: Option<u64>,
    pub runtime_session_hash: String,
}

impl EngineBridge {
    pub(super) fn has_static_gameplay_runtime(&self) -> bool {
        self.gameplay.static_gameplay_host.is_some()
    }

    pub(super) fn with_static_gameplay_runtime<R>(
        &mut self,
        operation: &'static str,
        apply: impl FnOnce(&mut GameplayRuntimeHost) -> Result<R, GameplayRuntimeHostError>,
    ) -> BridgeResult<Option<R>> {
        let Some(host) = self.gameplay.static_gameplay_host.as_mut() else {
            return Ok(None);
        };
        let entities = core::mem::take(&mut self.scene.entities);
        host.install_entity_authority(entities).map_err(|error| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("{operation} could not enter composed entity authority: {error}"),
            )
        })?;
        let result = apply(host);
        self.scene.entities = host.take_entity_authority().map_err(|error| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("{operation} did not return composed entity authority: {error}"),
            )
        })?;
        result.map(Some).map_err(|error| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("{operation} was rejected by composed gameplay authority: {error}"),
            )
        })
    }

    pub(super) fn deliver_static_gameplay_owner_events(
        &mut self,
        events: Vec<protocol_game_extension::GameplayEventEnvelope>,
    ) -> BridgeResult<()> {
        if events.is_empty() || !self.has_static_gameplay_runtime() {
            return Ok(());
        }
        let receipt = self
            .with_static_gameplay_runtime("deliver_static_gameplay_owner_events", |host| {
                host.observe_owner_events(events)
            })?
            .expect("static gameplay host checked above");
        if receipt.observe.accepted() {
            return Ok(());
        }
        let diagnostic = receipt
            .observe
            .diagnostics
            .first()
            .map(|diagnostic| diagnostic.message.as_str())
            .unwrap_or("gameplay reaction rejected without a diagnostic");
        Err(RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::InvalidInput,
            format!("authoritative owner events were rejected by gameplay fabric: {diagnostic}"),
        ))
    }

    pub fn read_composed_runtime_session(&mut self) -> BridgeResult<ComposedRuntimeSessionReadout> {
        let gameplay = self
            .with_static_gameplay_runtime("read_composed_runtime_session", |host| {
                Ok(host.readout())
            })?
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    "RuntimeSession was not built with a static gameplay composition",
                )
            })?;
        self.composed_runtime_session_readout(gameplay)
    }

    pub fn read_gameplay_module_view(
        &mut self,
        request: GameplayModuleViewRequest,
    ) -> BridgeResult<GameplayModuleViewSnapshot> {
        let before = self.read_composed_runtime_session()?;
        if request.expected_runtime_session_hash != before.runtime_session_hash {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "gameplay module view expected RuntimeSession {}, current {}",
                    request.expected_runtime_session_hash, before.runtime_session_hash
                ),
            ));
        }
        let scope = module_state_scope(&request.scope);
        let view = self
            .with_static_gameplay_runtime("read_gameplay_module_view", |host| {
                host.module_named_view(&request.view, &scope)
            })?
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    "RuntimeSession was not built with a static gameplay composition",
                )
            })?;
        let after = self.read_composed_runtime_session()?;
        if after.runtime_session_hash != before.runtime_session_hash {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "read_gameplay_module_view mutated composed RuntimeSession authority",
            ));
        }
        Ok(GameplayModuleViewSnapshot {
            view: view.view,
            provider_id: view.provider_id,
            scope: request.scope,
            revision: view.revision,
            canonical_payload: view.canonical_payload,
            view_hash: view.view_hash,
            runtime_session_hash: after.runtime_session_hash,
        })
    }

    pub fn apply_gameplay_prefab_part_interaction(
        &mut self,
        request: GameplayPrefabPartInteractionRequest,
    ) -> BridgeResult<GameplayPrefabPartInteractionReceipt> {
        let before = self.read_composed_runtime_session()?;
        if request.expected_runtime_session_hash != before.runtime_session_hash {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "prefab interaction expected RuntimeSession {}, current {}",
                    request.expected_runtime_session_hash, before.runtime_session_hash
                ),
            ));
        }
        let interaction = self
            .with_static_gameplay_runtime("apply_gameplay_prefab_part_interaction", |host| {
                host.interact_with_prefab_part(GameplayRuntimePrefabInteractionIntent {
                    actor: EntityId::new(request.actor),
                    instance: request.instance,
                    role: request.role.clone(),
                    expected_target: EntityId::new(request.expected_target),
                    tick: request.tick,
                })
            })?
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    "RuntimeSession was not built with a static gameplay composition",
                )
            })?;
        let event_hash = rule_gameplay_fabric::gameplay_module_payload_hash(
            &serde_json::to_vec(&interaction.event).map_err(|error| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!("prefab interaction event did not serialize: {error}"),
                )
            })?,
        );
        let after = self.read_composed_runtime_session()?;
        Ok(GameplayPrefabPartInteractionReceipt {
            actor: request.actor,
            instance: request.instance,
            role: request.role,
            target: interaction.target.raw(),
            event_hash,
            reaction_frame_hash: interaction.reaction_frame_hash,
            runtime_session_hash: after.runtime_session_hash,
        })
    }

    /// Execute one pre-commit decision entirely inside the composed Rust cell.
    /// The owner port is a statically linked, revisioned authority adapter; no
    /// semantic proposal or owner result crosses TypeScript.
    pub fn decide_composed_gameplay(
        &mut self,
        moment: GameplayDecisionMoment,
        owner: &mut dyn GameplayRuntimeDecisionOwner,
    ) -> BridgeResult<GameplayDecisionReceipt> {
        self.with_static_gameplay_runtime("decide_composed_gameplay", |host| {
            Ok(host.decide(moment, owner))
        })?
        .ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "RuntimeSession was not built with a static gameplay composition",
            )
        })
    }

    pub fn apply_composed_gameplay_scheduler_command(
        &mut self,
        command: GameplayRuntimeSchedulerCommand,
    ) -> BridgeResult<GameplayRuntimeSchedulerCommandReceipt> {
        self.with_static_gameplay_runtime("apply_composed_gameplay_scheduler_command", |host| {
            host.scheduler_port().apply(command)
        })?
        .ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "RuntimeSession was not built with a static gameplay composition",
            )
        })
    }

    pub fn route_composed_gameplay_scheduled_action(
        &mut self,
        action_id: &ScheduledActionId,
    ) -> BridgeResult<GameplayRuntimeSchedulerRoutingReceipt> {
        self.with_static_gameplay_runtime("route_composed_gameplay_scheduled_action", |host| {
            host.scheduler_port().route(action_id)
        })?
        .ok_or_else(|| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::NotInitialized,
                "RuntimeSession was not built with a static gameplay composition",
            )
        })
    }

    fn composed_runtime_session_readout(
        &self,
        gameplay: GameplayRuntimeHostReadout,
    ) -> BridgeResult<ComposedRuntimeSessionReadout> {
        let entity_authority_hash = format!("fnv1a64:{:016x}", self.scene.entities.hash().0);
        let fps_replay_hash = self
            .gameplay
            .fps_session
            .as_ref()
            .and_then(|session| session.replay_records.last())
            .map(|record| record.record_hash);
        let runtime_session_hash = composed_runtime_session_hash(
            &entity_authority_hash,
            &gameplay,
            self.gameplay.fps_epoch,
            fps_replay_hash,
        );
        Ok(ComposedRuntimeSessionReadout {
            schema_version: COMPOSED_RUNTIME_SESSION_SCHEMA_VERSION,
            entity_authority_hash,
            gameplay,
            fps_session_epoch: self.gameplay.fps_epoch,
            fps_replay_hash,
            runtime_session_hash,
        })
    }
}

fn module_state_scope(scope: &GameplayModuleViewScope) -> GameplayModuleStateScope {
    match scope {
        GameplayModuleViewScope::Session => GameplayModuleStateScope::Session,
        GameplayModuleViewScope::Entity { entity } => {
            GameplayModuleStateScope::Entity { entity: *entity }
        }
        GameplayModuleViewScope::PrefabInstance { instance } => {
            GameplayModuleStateScope::PrefabInstance {
                instance: *instance,
            }
        }
    }
}

fn composed_runtime_session_hash(
    entity_authority_hash: &str,
    gameplay: &GameplayRuntimeHostReadout,
    fps_session_epoch: u64,
    fps_replay_hash: Option<u64>,
) -> String {
    rule_gameplay_fabric::gameplay_module_payload_hash(
        format!(
            "{}|{}|{}|{}|{}|{}",
            COMPOSED_RUNTIME_SESSION_SCHEMA_VERSION,
            entity_authority_hash,
            gameplay.gameplay_registry_digest,
            gameplay.runtime_host_hash,
            fps_session_epoch,
            fps_replay_hash
                .map(|hash| format!("{hash:016x}"))
                .unwrap_or_else(|| "none".to_owned()),
        )
        .as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composed_gameplay_operations_fail_closed_without_static_provider() {
        let mut bridge = EngineBridge::new();
        let entity_hash_before = bridge.scene.entities.hash();

        let readout = bridge
            .read_composed_runtime_session()
            .expect_err("an ordinary bridge cannot claim a composed RuntimeSession");
        assert_eq!(readout.kind, RuntimeBridgeErrorKind::NotInitialized);

        let view = bridge
            .read_gameplay_module_view(GameplayModuleViewRequest {
                view: GameplayContractRef {
                    namespace: "fixture.missing".to_owned(),
                    name: "state".to_owned(),
                    version: 1,
                    schema_hash: "fnv1a64:0000000000000001".to_owned(),
                },
                scope: GameplayModuleViewScope::Session,
                expected_runtime_session_hash: "fnv1a64:0000000000000002".to_owned(),
            })
            .expect_err("a module view cannot bypass static composition");
        assert_eq!(view.kind, RuntimeBridgeErrorKind::NotInitialized);

        let interaction = bridge
            .apply_gameplay_prefab_part_interaction(GameplayPrefabPartInteractionRequest {
                actor: 1,
                instance: 1,
                role: "interaction/target".to_owned(),
                expected_target: 2,
                tick: 1,
                expected_runtime_session_hash: "fnv1a64:0000000000000002".to_owned(),
            })
            .expect_err("a prefab interaction cannot bypass static composition");
        assert_eq!(interaction.kind, RuntimeBridgeErrorKind::NotInitialized);
        assert_eq!(bridge.scene.entities.hash(), entity_hash_before);
    }
}
