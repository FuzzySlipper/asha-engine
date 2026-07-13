use super::*;

use gameplay_runtime_host::{
    GameplayDecisionMoment, GameplayDecisionReceipt, GameplayRuntimeDecisionOwner,
    GameplayRuntimeHost, GameplayRuntimeHostError, GameplayRuntimeHostReadout,
    GameplayRuntimePrefabBootstrap, GameplayRuntimeProjectInput, GameplayRuntimeSchedulerCommand,
    GameplayRuntimeSchedulerCommandReceipt, GameplayRuntimeSchedulerRoutingReceipt,
    ScheduledActionId,
};
use serde::{Deserialize, Serialize};

const COMPOSED_RUNTIME_SESSION_SCHEMA_VERSION: u32 = 1;

/// Failure while constructing the closed, statically linked RuntimeSession
/// topology. The builder accepts concrete Rust module compositions only; it
/// has no dynamic loader, callback registry, or mutable authority handle.
#[derive(Debug)]
pub enum StaticRuntimeSessionCompositionError {
    Gameplay(GameplayRuntimeHostError),
    Snapshot(String),
}

impl core::fmt::Display for StaticRuntimeSessionCompositionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Gameplay(error) => write!(formatter, "gameplay composition failed: {error}"),
            Self::Snapshot(message) => write!(formatter, "composition snapshot failed: {message}"),
        }
    }
}

impl std::error::Error for StaticRuntimeSessionCompositionError {}

impl From<GameplayRuntimeHostError> for StaticRuntimeSessionCompositionError {
    fn from(value: GameplayRuntimeHostError) -> Self {
        Self::Gameplay(value)
    }
}

/// Narrow consumer entrypoint for one native authority cell.
///
/// A downstream addon links its concrete gameplay modules into a
/// [`GameplayRuntimeProjectInput`], then consumes this builder to obtain the
/// ordinary [`EngineBridge`] root used by the native transport. Module code,
/// scheduler authority, triggers, prefabs, FPS rules, and bridge operations
/// therefore live in one Rust object graph.
pub struct StaticRuntimeSessionBuilder {
    gameplay_host: GameplayRuntimeHost,
    restored: Option<RestoredCompositionState>,
}

struct RestoredCompositionState {
    fps_session: Option<FpsRuntimeSessionState>,
    fps_seed: Option<FpsRuntimeSessionLoadRequest>,
    fps_epoch: u64,
    base_entities: EntityStore,
}

impl StaticRuntimeSessionBuilder {
    pub fn activate_project(
        input: GameplayRuntimeProjectInput,
    ) -> Result<Self, StaticRuntimeSessionCompositionError> {
        Ok(Self {
            gameplay_host: GameplayRuntimeHost::activate_project(input)?,
            restored: None,
        })
    }

    pub fn activate_project_with_prefabs(
        input: GameplayRuntimeProjectInput,
        prefabs: GameplayRuntimePrefabBootstrap,
    ) -> Result<Self, StaticRuntimeSessionCompositionError> {
        Ok(Self {
            gameplay_host: GameplayRuntimeHost::activate_project_with_prefabs(input, prefabs)?,
            restored: None,
        })
    }

    pub fn restore_project(
        input: GameplayRuntimeProjectInput,
        checkpoint: &ComposedRuntimeSessionCheckpoint,
    ) -> Result<Self, StaticRuntimeSessionCompositionError> {
        checkpoint.validate()?;
        Ok(Self {
            gameplay_host: GameplayRuntimeHost::restore_project(
                input,
                &checkpoint.gameplay_snapshot,
            )?,
            restored: Some(checkpoint.restored_state()),
        })
    }

    pub fn restore_project_with_prefabs(
        input: GameplayRuntimeProjectInput,
        prefabs: GameplayRuntimePrefabBootstrap,
        checkpoint: &ComposedRuntimeSessionCheckpoint,
    ) -> Result<Self, StaticRuntimeSessionCompositionError> {
        checkpoint.validate()?;
        Ok(Self {
            gameplay_host: GameplayRuntimeHost::restore_project_with_prefabs(
                input,
                prefabs,
                &checkpoint.gameplay_snapshot,
            )?,
            restored: Some(checkpoint.restored_state()),
        })
    }

    pub fn build(mut self) -> Result<EngineBridge, StaticRuntimeSessionCompositionError> {
        let entities = self.gameplay_host.take_entity_authority()?;
        let mut bridge = EngineBridge::new();
        bridge.scene.entities = entities;
        bridge.gameplay.static_gameplay_host = Some(self.gameplay_host);
        match self.restored {
            Some(restored) => {
                bridge.gameplay.static_gameplay_base_entities = Some(restored.base_entities);
                bridge.gameplay.fps_session = restored.fps_session;
                bridge.gameplay.fps_seed = restored.fps_seed;
                bridge.gameplay.fps_epoch = restored.fps_epoch;
            }
            None => {
                bridge.gameplay.static_gameplay_base_entities = Some(bridge.scene.entities.clone());
            }
        }
        Ok(bridge)
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

/// In-memory native-provider checkpoint. Durable gameplay state is canonical
/// text; the private FPS rule state and initial entity generation remain
/// opaque so consumers cannot mutate or fabricate authority while rebuilding
/// the statically linked cell.
#[derive(Clone)]
pub struct ComposedRuntimeSessionCheckpoint {
    gameplay_snapshot: String,
    gameplay_snapshot_hash: String,
    fps_session: Option<FpsRuntimeSessionState>,
    fps_seed: Option<FpsRuntimeSessionLoadRequest>,
    fps_epoch: u64,
    base_entities: EntityStore,
    readout: ComposedRuntimeSessionReadout,
}

impl ComposedRuntimeSessionCheckpoint {
    pub fn gameplay_snapshot(&self) -> &str {
        &self.gameplay_snapshot
    }

    pub fn gameplay_snapshot_hash(&self) -> &str {
        &self.gameplay_snapshot_hash
    }

    pub fn readout(&self) -> &ComposedRuntimeSessionReadout {
        &self.readout
    }

    fn validate(&self) -> Result<(), StaticRuntimeSessionCompositionError> {
        let actual =
            rule_gameplay_fabric::gameplay_module_payload_hash(self.gameplay_snapshot.as_bytes());
        if actual != self.gameplay_snapshot_hash
            || self.readout.runtime_session_hash
                != composed_runtime_session_hash(
                    &self.readout.entity_authority_hash,
                    &self.readout.gameplay,
                    self.readout.fps_session_epoch,
                    self.readout.fps_replay_hash,
                )
        {
            return Err(StaticRuntimeSessionCompositionError::Snapshot(
                "checkpoint canonical hash mismatch".to_owned(),
            ));
        }
        Ok(())
    }

    fn restored_state(&self) -> RestoredCompositionState {
        RestoredCompositionState {
            fps_session: self.fps_session.clone(),
            fps_seed: self.fps_seed.clone(),
            fps_epoch: self.fps_epoch,
            base_entities: self.base_entities.clone(),
        }
    }
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
        Ok(self.composed_runtime_session_readout(gameplay))
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

    pub fn checkpoint_composed_runtime_session(
        &mut self,
    ) -> BridgeResult<ComposedRuntimeSessionCheckpoint> {
        let (artifact, gameplay) = self
            .with_static_gameplay_runtime("checkpoint_composed_runtime_session", |host| {
                Ok((host.compose_snapshot()?, host.readout()))
            })?
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    "RuntimeSession was not built with a static gameplay composition",
                )
            })?;
        let readout = self.composed_runtime_session_readout(gameplay);
        let gameplay_snapshot_hash =
            rule_gameplay_fabric::gameplay_module_payload_hash(artifact.text.as_bytes());
        Ok(ComposedRuntimeSessionCheckpoint {
            gameplay_snapshot: artifact.text,
            gameplay_snapshot_hash,
            fps_session: self.gameplay.fps_session.clone(),
            fps_seed: self.gameplay.fps_seed.clone(),
            fps_epoch: self.gameplay.fps_epoch,
            base_entities: self
                .gameplay
                .static_gameplay_base_entities
                .clone()
                .unwrap_or_default(),
            readout,
        })
    }

    fn composed_runtime_session_readout(
        &self,
        gameplay: GameplayRuntimeHostReadout,
    ) -> ComposedRuntimeSessionReadout {
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
        ComposedRuntimeSessionReadout {
            schema_version: COMPOSED_RUNTIME_SESSION_SCHEMA_VERSION,
            entity_authority_hash,
            gameplay,
            fps_session_epoch: self.gameplay.fps_epoch,
            fps_replay_hash,
            runtime_session_hash,
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
