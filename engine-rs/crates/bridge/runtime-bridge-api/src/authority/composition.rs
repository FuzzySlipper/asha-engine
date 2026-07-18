use super::*;

use gameplay_runtime_host::{
    GameplayDecisionMoment, GameplayDecisionReceipt, GameplayRuntimeDecisionOwner,
    GameplayRuntimeHost, GameplayRuntimeHostError, GameplayRuntimeHostReadout,
    GameplayRuntimePrefabBootstrap, GameplayRuntimePrefabInteractionIntent,
    GameplayRuntimeProjectInput, GameplayRuntimeResetCheckpoint, GameplayRuntimeSchedulerCommand,
    GameplayRuntimeSchedulerCommandReceipt, GameplayRuntimeSchedulerRoutingReceipt,
    GameplayStaticComposition, ScheduledActionId,
};
use protocol_game_extension::{GameplayEventEnvelope, GameplayOwnerRef, GameplayProposalEnvelope};
use rule_gameplay_fabric::GameplayModuleStateScope;
use serde::{Deserialize, Serialize};

const COMPOSED_RUNTIME_SESSION_SCHEMA_VERSION: u32 = 2;

/// Failure while constructing the closed, statically linked RuntimeSession
/// topology. The builder accepts concrete Rust module compositions only; it
/// has no dynamic loader, callback registry, or mutable authority handle.
#[derive(Debug)]
pub enum StaticRuntimeSessionCompositionError {
    Gameplay(GameplayRuntimeHostError),
    Owner(String),
    Snapshot(String),
}

impl core::fmt::Display for StaticRuntimeSessionCompositionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Gameplay(error) => write!(formatter, "gameplay composition failed: {error}"),
            Self::Owner(message) => {
                write!(formatter, "gameplay owner composition failed: {message}")
            }
            Self::Snapshot(message) => write!(formatter, "composition snapshot failed: {message}"),
        }
    }
}

/// Canonical downstream-owner state retained inside the composed authority
/// cell. Consumers construct this from their typed state codec; ASHA binds the
/// bytes, owner identity, and replay evidence into every composed readout and
/// checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComposedGameplayOwnerCheckpoint {
    owner: GameplayOwnerRef,
    canonical_state: Vec<u8>,
    state_hash: String,
    replay_hash: String,
}

impl ComposedGameplayOwnerCheckpoint {
    pub fn new(
        owner: GameplayOwnerRef,
        canonical_state: Vec<u8>,
        replay_hash: impl Into<String>,
    ) -> Result<Self, String> {
        let replay_hash = replay_hash.into();
        if owner.owner_id.is_empty() || owner.provider_id.is_empty() {
            return Err("owner identity must be non-empty".to_owned());
        }
        if replay_hash.is_empty() {
            return Err("owner replay hash must be non-empty".to_owned());
        }
        let state_hash = rule_gameplay_fabric::gameplay_module_payload_hash(&canonical_state);
        Ok(Self {
            owner,
            canonical_state,
            state_hash,
            replay_hash,
        })
    }

    pub fn owner(&self) -> &GameplayOwnerRef {
        &self.owner
    }

    pub fn canonical_state(&self) -> &[u8] {
        &self.canonical_state
    }

    pub fn state_hash(&self) -> &str {
        &self.state_hash
    }

    pub fn replay_hash(&self) -> &str {
        &self.replay_hash
    }

    fn validate(&self) -> Result<(), String> {
        let actual = rule_gameplay_fabric::gameplay_module_payload_hash(&self.canonical_state);
        if actual != self.state_hash {
            return Err("owner canonical state hash mismatch".to_owned());
        }
        if self.owner.owner_id.is_empty()
            || self.owner.provider_id.is_empty()
            || self.replay_hash.is_empty()
        {
            return Err("owner checkpoint identity or replay hash is empty".to_owned());
        }
        Ok(())
    }

    fn readout(&self) -> ComposedGameplayOwnerReadout {
        ComposedGameplayOwnerReadout {
            owner: self.owner.clone(),
            state_hash: self.state_hash.clone(),
            replay_hash: self.replay_hash.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComposedGameplayOwnerReadout {
    pub owner: GameplayOwnerRef,
    pub state_hash: String,
    pub replay_hash: String,
}

/// Typed result of one statically linked owner commit. Events are admitted by
/// the closed gameplay registry before they enter the wave-frozen cascade.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComposedGameplayOwnerOutput {
    pub accepted: bool,
    pub fact_hashes: Vec<String>,
    pub diagnostic_codes: Vec<String>,
    pub events: Vec<GameplayEventEnvelope>,
}

/// Narrow static owner installed while composing the RuntimeSession. It has no
/// dynamic registration or transport edge: one concrete Rust value owns its
/// typed state codec, pre-commit route, emitted facts, and replay evidence.
pub trait ComposedGameplayOwner: Send {
    fn owner(&self) -> &GameplayOwnerRef;
    fn revision_hash(&self) -> String;
    fn checkpoint(&self) -> Result<ComposedGameplayOwnerCheckpoint, String>;
    fn restore(&mut self, checkpoint: &ComposedGameplayOwnerCheckpoint) -> Result<(), String>;
    fn route_precommit(
        &mut self,
        operation: &GameplayProposalEnvelope,
    ) -> ComposedGameplayOwnerOutput;
}

impl std::error::Error for StaticRuntimeSessionCompositionError {}

impl From<GameplayRuntimeHostError> for StaticRuntimeSessionCompositionError {
    fn from(value: GameplayRuntimeHostError) -> Self {
        Self::Gameplay(value)
    }
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
    gameplay_reset_checkpoint: GameplayRuntimeResetCheckpoint,
    gameplay_owner_checkpoint: Option<ComposedGameplayOwnerCheckpoint>,
    gameplay_owner_reset_checkpoint: Option<ComposedGameplayOwnerCheckpoint>,
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

    pub fn with_gameplay_owner<O>(
        mut self,
        mut owner: O,
    ) -> Result<ComposedGameplayRuntimeBuilder<O>, StaticRuntimeSessionCompositionError>
    where
        O: ComposedGameplayOwner,
    {
        let current = owner
            .checkpoint()
            .map_err(StaticRuntimeSessionCompositionError::Owner)?;
        current
            .validate()
            .map_err(StaticRuntimeSessionCompositionError::Owner)?;
        if current.owner() != owner.owner() {
            return Err(StaticRuntimeSessionCompositionError::Owner(
                "owner checkpoint identity does not match the installed owner".to_owned(),
            ));
        }
        let owner_reset_checkpoint = if let Some(restored) = &mut self.restored {
            let expected = restored.gameplay_owner_checkpoint.as_ref().ok_or_else(|| {
                StaticRuntimeSessionCompositionError::Owner(
                    "checkpoint did not contain a composed gameplay owner".to_owned(),
                )
            })?;
            if expected.owner() != owner.owner() {
                return Err(StaticRuntimeSessionCompositionError::Owner(
                    "installed owner identity does not match the checkpoint".to_owned(),
                ));
            }
            owner
                .restore(expected)
                .map_err(StaticRuntimeSessionCompositionError::Owner)?;
            if owner
                .checkpoint()
                .map_err(StaticRuntimeSessionCompositionError::Owner)?
                != *expected
            {
                return Err(StaticRuntimeSessionCompositionError::Owner(
                    "owner restore did not reproduce the checkpoint".to_owned(),
                ));
            }
            restored.gameplay_owner_checkpoint = None;
            restored
                .gameplay_owner_reset_checkpoint
                .take()
                .ok_or_else(|| {
                    StaticRuntimeSessionCompositionError::Owner(
                        "checkpoint omitted the composed gameplay owner reset state".to_owned(),
                    )
                })?
        } else {
            current
        };
        Ok(ComposedGameplayRuntimeBuilder {
            runtime: self,
            owner,
            owner_reset_checkpoint,
        })
    }

    pub fn build(mut self) -> Result<EngineBridge, StaticRuntimeSessionCompositionError> {
        if self.restored.as_ref().is_some_and(|restored| {
            restored.gameplay_owner_checkpoint.is_some()
                || restored.gameplay_owner_reset_checkpoint.is_some()
        }) {
            return Err(StaticRuntimeSessionCompositionError::Owner(
                "checkpoint requires its statically linked gameplay owner".to_owned(),
            ));
        }
        let fresh_reset_checkpoint = self.gameplay_host.checkpoint_reset_state();
        let entities = self.gameplay_host.take_entity_authority()?;
        let mut bridge = EngineBridge::new();
        bridge.scene.entities = entities;
        bridge.gameplay.static_gameplay_host = Some(self.gameplay_host);
        match self.restored {
            Some(restored) => {
                bridge.gameplay.static_gameplay_base_entities = Some(restored.base_entities);
                bridge.gameplay.static_gameplay_reset_checkpoint =
                    Some(restored.gameplay_reset_checkpoint);
                bridge.gameplay.fps_session = restored.fps_session;
                bridge.gameplay.fps_seed = restored.fps_seed;
                bridge.gameplay.fps_epoch = restored.fps_epoch;
            }
            None => {
                bridge.gameplay.static_gameplay_base_entities = Some(bridge.scene.entities.clone());
                bridge.gameplay.static_gameplay_reset_checkpoint = Some(fresh_reset_checkpoint);
            }
        }
        Ok(bridge)
    }
}

pub struct ComposedGameplayRuntimeBuilder<O: ComposedGameplayOwner> {
    runtime: StaticRuntimeSessionBuilder,
    owner: O,
    owner_reset_checkpoint: ComposedGameplayOwnerCheckpoint,
}

impl<O: ComposedGameplayOwner> ComposedGameplayRuntimeBuilder<O> {
    pub fn build(self) -> Result<ComposedGameplayRuntime<O>, StaticRuntimeSessionCompositionError> {
        Ok(ComposedGameplayRuntime {
            bridge: self.runtime.build()?,
            owner: self.owner,
            owner_reset_checkpoint: self.owner_reset_checkpoint,
        })
    }
}

/// Statically typed RuntimeSession cell containing one concrete downstream
/// owner. The generic owner remains visible to Rust's type system and never
/// enters a boxed callback or mutable registry.
pub struct ComposedGameplayRuntime<O: ComposedGameplayOwner> {
    bridge: EngineBridge,
    owner: O,
    owner_reset_checkpoint: ComposedGameplayOwnerCheckpoint,
}

impl<O: ComposedGameplayOwner> ComposedGameplayRuntime<O> {
    pub fn read_composed_runtime_session(&mut self) -> BridgeResult<ComposedRuntimeSessionReadout> {
        let owner = self.owner_checkpoint()?;
        self.bridge.read_composed_runtime_session_with_owner(&owner)
    }

    pub fn transact_composed_gameplay_owner(
        &mut self,
        moment: GameplayDecisionMoment,
    ) -> BridgeResult<ComposedGameplayOwnerTransactionReceipt> {
        self.bridge
            .transact_composed_gameplay_owner(&mut self.owner, moment)
    }

    pub fn checkpoint_composed_runtime_session(
        &mut self,
    ) -> BridgeResult<ComposedRuntimeSessionCheckpoint> {
        let owner = self.owner_checkpoint()?;
        let mut checkpoint = self.bridge.checkpoint_composed_runtime_session()?;
        checkpoint.readout = self
            .bridge
            .composed_runtime_session_readout(checkpoint.readout.gameplay.clone(), Some(&owner))?;
        checkpoint.gameplay_owner_checkpoint = Some(owner);
        checkpoint.gameplay_owner_reset_checkpoint = Some(self.owner_reset_checkpoint.clone());
        Ok(checkpoint)
    }

    pub fn read_gameplay_module_view(
        &mut self,
        mut request: GameplayModuleViewRequest,
    ) -> BridgeResult<GameplayModuleViewSnapshot> {
        let composed = self.read_composed_runtime_session()?;
        if request.expected_runtime_session_hash != composed.runtime_session_hash {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                "gameplay module view expected a stale composed owner generation",
            ));
        }
        let base = self.bridge.read_composed_runtime_session()?;
        request.expected_runtime_session_hash = base.runtime_session_hash;
        let mut view = self.bridge.read_gameplay_module_view(request)?;
        view.runtime_session_hash = composed.runtime_session_hash;
        Ok(view)
    }

    fn owner_checkpoint(&self) -> BridgeResult<ComposedGameplayOwnerCheckpoint> {
        let checkpoint = self.owner.checkpoint().map_err(|message| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("composed gameplay owner checkpoint failed: {message}"),
            )
        })?;
        checkpoint.validate().map_err(|message| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("composed gameplay owner checkpoint was invalid: {message}"),
            )
        })?;
        if checkpoint.owner() != self.owner.owner() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "composed gameplay owner checkpoint identity drifted",
            ));
        }
        Ok(checkpoint)
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
    pub gameplay_owner: Option<ComposedGameplayOwnerReadout>,
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
    gameplay_reset_checkpoint: GameplayRuntimeResetCheckpoint,
    gameplay_owner_checkpoint: Option<ComposedGameplayOwnerCheckpoint>,
    gameplay_owner_reset_checkpoint: Option<ComposedGameplayOwnerCheckpoint>,
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

    pub fn gameplay_owner_checkpoint(&self) -> Option<&ComposedGameplayOwnerCheckpoint> {
        self.gameplay_owner_checkpoint.as_ref()
    }

    fn validate(&self) -> Result<(), StaticRuntimeSessionCompositionError> {
        if self.gameplay_owner_checkpoint.is_some()
            != self.gameplay_owner_reset_checkpoint.is_some()
        {
            return Err(StaticRuntimeSessionCompositionError::Snapshot(
                "checkpoint gameplay owner topology is incomplete".to_owned(),
            ));
        }
        if let Some(owner) = &self.gameplay_owner_checkpoint {
            owner
                .validate()
                .map_err(StaticRuntimeSessionCompositionError::Snapshot)?;
        }
        if let Some(owner) = &self.gameplay_owner_reset_checkpoint {
            owner
                .validate()
                .map_err(StaticRuntimeSessionCompositionError::Snapshot)?;
        }
        if self
            .gameplay_owner_checkpoint
            .as_ref()
            .zip(self.gameplay_owner_reset_checkpoint.as_ref())
            .is_some_and(|(current, reset)| current.owner() != reset.owner())
        {
            return Err(StaticRuntimeSessionCompositionError::Snapshot(
                "checkpoint gameplay owner reset identity mismatch".to_owned(),
            ));
        }
        if self
            .gameplay_owner_checkpoint
            .as_ref()
            .map(ComposedGameplayOwnerCheckpoint::readout)
            != self.readout.gameplay_owner
        {
            return Err(StaticRuntimeSessionCompositionError::Snapshot(
                "checkpoint gameplay owner readout mismatch".to_owned(),
            ));
        }
        let actual =
            rule_gameplay_fabric::gameplay_module_payload_hash(self.gameplay_snapshot.as_bytes());
        if actual != self.gameplay_snapshot_hash
            || self.readout.runtime_session_hash
                != composed_runtime_session_hash(
                    &self.readout.entity_authority_hash,
                    &self.readout.gameplay,
                    self.readout.gameplay_owner.as_ref(),
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
            gameplay_reset_checkpoint: self.gameplay_reset_checkpoint.clone(),
            gameplay_owner_checkpoint: self.gameplay_owner_checkpoint.clone(),
            gameplay_owner_reset_checkpoint: self.gameplay_owner_reset_checkpoint.clone(),
        }
    }
}

struct InstalledGameplayOwnerDecisionAdapter<'a> {
    owner: &'a mut dyn ComposedGameplayOwner,
    output: Option<ComposedGameplayOwnerOutput>,
}

impl GameplayRuntimeDecisionOwner for InstalledGameplayOwnerDecisionAdapter<'_> {
    fn revision_hash(&self, owner: &GameplayOwnerRef) -> String {
        if owner == self.owner.owner() {
            self.owner.revision_hash()
        } else {
            "composed-owner-identity-mismatch".to_owned()
        }
    }

    fn route_precommit(
        &mut self,
        owner: &GameplayOwnerRef,
        operation: &GameplayProposalEnvelope,
    ) -> gameplay_runtime_host::GameplayRuntimeDecisionOwnerOutput {
        if owner != self.owner.owner() {
            return gameplay_runtime_host::GameplayRuntimeDecisionOwnerOutput {
                accepted: false,
                diagnostic_codes: vec!["composedOwnerIdentityMismatch".to_owned()],
                ..gameplay_runtime_host::GameplayRuntimeDecisionOwnerOutput::default()
            };
        }
        let output = self.owner.route_precommit(operation);
        let decision_output = gameplay_runtime_host::GameplayRuntimeDecisionOwnerOutput {
            accepted: output.accepted,
            fact_hashes: output.fact_hashes.clone(),
            diagnostic_codes: output.diagnostic_codes.clone(),
        };
        self.output = Some(output);
        decision_output
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComposedGameplayOwnerTransactionReceipt {
    pub decision: GameplayDecisionReceipt,
    pub reaction_event_keys: Vec<String>,
    pub reaction_frame_hashes: Vec<String>,
    pub gameplay_owner: ComposedGameplayOwnerReadout,
    pub runtime_session_hash: String,
}

impl EngineBridge {
    fn restore_composed_gameplay_transaction(
        &mut self,
        owner: &mut dyn ComposedGameplayOwner,
        owner_checkpoint: &ComposedGameplayOwnerCheckpoint,
        gameplay_checkpoint: gameplay_runtime_host::GameplayRuntimeTransactionCheckpoint,
        entities: EntityStore,
    ) -> BridgeResult<()> {
        owner.restore(owner_checkpoint).map_err(|message| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("composed gameplay owner rollback failed: {message}"),
            )
        })?;
        if owner.checkpoint().map_err(|message| {
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("composed gameplay owner rollback checkpoint failed: {message}"),
            )
        })? != *owner_checkpoint
        {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "composed gameplay owner rollback did not reproduce its checkpoint",
            ));
        }
        self.with_static_gameplay_runtime("rollback_composed_gameplay_owner", |host| {
            host.restore_transaction_evidence(gameplay_checkpoint);
            Ok(())
        })?
        .expect("installed gameplay owner requires a static gameplay host");
        self.scene.entities = entities;
        Ok(())
    }

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
        self.composed_runtime_session_readout(gameplay, None)
    }

    fn read_composed_runtime_session_with_owner(
        &mut self,
        owner: &ComposedGameplayOwnerCheckpoint,
    ) -> BridgeResult<ComposedRuntimeSessionReadout> {
        let gameplay = self
            .with_static_gameplay_runtime("read_composed_runtime_session_with_owner", |host| {
                Ok(host.readout())
            })?
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    "RuntimeSession was not built with a static gameplay composition",
                )
            })?;
        self.composed_runtime_session_readout(gameplay, Some(owner))
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

    fn transact_composed_gameplay_owner<O: ComposedGameplayOwner>(
        &mut self,
        owner: &mut O,
        moment: GameplayDecisionMoment,
    ) -> BridgeResult<ComposedGameplayOwnerTransactionReceipt> {
        let owner_before = match owner.checkpoint() {
            Ok(checkpoint) => checkpoint,
            Err(message) => {
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!("composed gameplay owner checkpoint failed: {message}"),
                ));
            }
        };
        if let Err(message) = owner_before.validate() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("composed gameplay owner checkpoint was invalid: {message}"),
            ));
        }
        if owner_before.owner() != owner.owner() {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "composed gameplay owner checkpoint identity drifted",
            ));
        }
        let gameplay_before = self
            .with_static_gameplay_runtime("checkpoint_composed_gameplay_owner", |host| {
                Ok(host.checkpoint_transaction_evidence())
            })?
            .ok_or_else(|| {
                RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::NotInitialized,
                    "RuntimeSession was not built with a static gameplay composition",
                )
            })?;
        let entities_before = self.scene.entities.clone();
        let mut adapter = InstalledGameplayOwnerDecisionAdapter {
            owner,
            output: None,
        };
        let decision_result = self
            .with_static_gameplay_runtime("transact_composed_gameplay_owner", |host| {
                Ok(host.decide(moment, &mut adapter))
            });
        let owner_output = adapter.output.take();
        let decision = match decision_result {
            Ok(Some(decision)) => decision,
            Ok(None) => unreachable!("static gameplay host checkpoint succeeded above"),
            Err(error) => {
                let rollback = self.restore_composed_gameplay_transaction(
                    owner,
                    &owner_before,
                    gameplay_before,
                    entities_before,
                );
                rollback?;
                return Err(error);
            }
        };

        if decision.status == rule_gameplay_fabric::GameplayDecisionStatus::Suspended {
            let owner_after = match owner.checkpoint() {
                Ok(checkpoint) => checkpoint,
                Err(message) => {
                    self.restore_composed_gameplay_transaction(
                        owner,
                        &owner_before,
                        gameplay_before,
                        entities_before,
                    )?;
                    return Err(RuntimeBridgeError::new(
                        RuntimeBridgeErrorKind::Internal,
                        format!(
                            "composed gameplay owner post-decision checkpoint failed: {message}"
                        ),
                    ));
                }
            };
            if owner_output.is_some() || owner_after != owner_before {
                self.restore_composed_gameplay_transaction(
                    owner,
                    &owner_before,
                    gameplay_before,
                    entities_before,
                )?;
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    "suspended composed decision mutated its owner",
                ));
            }
            let readout = self.read_composed_runtime_session_with_owner(&owner_before)?;
            let gameplay_owner = owner_before.readout();
            return Ok(ComposedGameplayOwnerTransactionReceipt {
                decision,
                reaction_event_keys: Vec::new(),
                reaction_frame_hashes: Vec::new(),
                gameplay_owner,
                runtime_session_hash: readout.runtime_session_hash,
            });
        }

        if decision.status != rule_gameplay_fabric::GameplayDecisionStatus::Accepted {
            self.restore_composed_gameplay_transaction(
                owner,
                &owner_before,
                gameplay_before,
                entities_before,
            )?;
            let readout = self.read_composed_runtime_session_with_owner(&owner_before)?;
            return Ok(ComposedGameplayOwnerTransactionReceipt {
                decision,
                reaction_event_keys: Vec::new(),
                reaction_frame_hashes: Vec::new(),
                gameplay_owner: owner_before.readout(),
                runtime_session_hash: readout.runtime_session_hash,
            });
        }

        let Some(output) = owner_output else {
            self.restore_composed_gameplay_transaction(
                owner,
                &owner_before,
                gameplay_before,
                entities_before,
            )?;
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "accepted composed decision did not route its installed owner",
            ));
        };
        if !output.accepted {
            self.restore_composed_gameplay_transaction(
                owner,
                &owner_before,
                gameplay_before,
                entities_before,
            )?;
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "accepted decision carried a rejected composed owner output",
            ));
        }

        let reaction_result = if output.events.is_empty() {
            Ok(None)
        } else {
            self.with_static_gameplay_runtime("observe_composed_gameplay_owner_facts", |host| {
                host.observe_owner_events(output.events)
            })
        };
        let reaction = match reaction_result {
            Ok(reaction)
                if reaction
                    .as_ref()
                    .is_none_or(|receipt| receipt.observe.accepted()) =>
            {
                reaction
            }
            Ok(reaction) => {
                let diagnostic = reaction
                    .as_ref()
                    .and_then(|receipt| receipt.observe.diagnostics.first())
                    .map(|diagnostic| diagnostic.message.clone())
                    .unwrap_or_else(|| "gameplay Fabric rejected owner facts".to_owned());
                self.restore_composed_gameplay_transaction(
                    owner,
                    &owner_before,
                    gameplay_before,
                    entities_before,
                )?;
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::InvalidInput,
                    format!(
                        "composed gameplay owner facts were rejected by the gameplay Fabric: {diagnostic}"
                    ),
                ));
            }
            Err(error) => {
                self.restore_composed_gameplay_transaction(
                    owner,
                    &owner_before,
                    gameplay_before,
                    entities_before,
                )?;
                return Err(error);
            }
        };
        let owner_after = match owner.checkpoint() {
            Ok(checkpoint) => checkpoint,
            Err(message) => {
                self.restore_composed_gameplay_transaction(
                    owner,
                    &owner_before,
                    gameplay_before,
                    entities_before,
                )?;
                return Err(RuntimeBridgeError::new(
                    RuntimeBridgeErrorKind::Internal,
                    format!("composed gameplay owner post-commit checkpoint failed: {message}"),
                ));
            }
        };
        if let Err(message) = owner_after.validate() {
            self.restore_composed_gameplay_transaction(
                owner,
                &owner_before,
                gameplay_before,
                entities_before,
            )?;
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                format!("composed gameplay owner post-commit checkpoint was invalid: {message}"),
            ));
        }
        if owner_after.owner() != owner_before.owner() {
            self.restore_composed_gameplay_transaction(
                owner,
                &owner_before,
                gameplay_before,
                entities_before,
            )?;
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::Internal,
                "composed gameplay owner post-commit checkpoint identity drifted",
            ));
        }
        let readout = self.read_composed_runtime_session_with_owner(&owner_after)?;
        let reaction_event_keys = reaction
            .iter()
            .flat_map(|receipt| receipt.frame.root_events.iter())
            .map(|event| event.event.key())
            .collect();
        Ok(ComposedGameplayOwnerTransactionReceipt {
            decision,
            reaction_event_keys,
            reaction_frame_hashes: reaction
                .into_iter()
                .map(|receipt| receipt.frame.frame_hash)
                .collect(),
            gameplay_owner: owner_after.readout(),
            runtime_session_hash: readout.runtime_session_hash,
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
        let readout = self.composed_runtime_session_readout(gameplay, None)?;
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
            gameplay_reset_checkpoint: self
                .gameplay
                .static_gameplay_reset_checkpoint
                .clone()
                .expect("composed RuntimeSession retains its activation reset checkpoint"),
            gameplay_owner_checkpoint: None,
            gameplay_owner_reset_checkpoint: None,
            readout,
        })
    }

    fn composed_runtime_session_readout(
        &self,
        gameplay: GameplayRuntimeHostReadout,
        gameplay_owner_checkpoint: Option<&ComposedGameplayOwnerCheckpoint>,
    ) -> BridgeResult<ComposedRuntimeSessionReadout> {
        let entity_authority_hash = format!("fnv1a64:{:016x}", self.scene.entities.hash().0);
        let gameplay_owner =
            gameplay_owner_checkpoint.map(ComposedGameplayOwnerCheckpoint::readout);
        let fps_replay_hash = self
            .gameplay
            .fps_session
            .as_ref()
            .and_then(|session| session.replay_records.last())
            .map(|record| record.record_hash);
        let runtime_session_hash = composed_runtime_session_hash(
            &entity_authority_hash,
            &gameplay,
            gameplay_owner.as_ref(),
            self.gameplay.fps_epoch,
            fps_replay_hash,
        );
        Ok(ComposedRuntimeSessionReadout {
            schema_version: COMPOSED_RUNTIME_SESSION_SCHEMA_VERSION,
            entity_authority_hash,
            gameplay,
            gameplay_owner,
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
    gameplay_owner: Option<&ComposedGameplayOwnerReadout>,
    fps_session_epoch: u64,
    fps_replay_hash: Option<u64>,
) -> String {
    rule_gameplay_fabric::gameplay_module_payload_hash(
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            COMPOSED_RUNTIME_SESSION_SCHEMA_VERSION,
            entity_authority_hash,
            gameplay.gameplay_registry_digest,
            gameplay.runtime_host_hash,
            gameplay_owner
                .map(|owner| owner.owner.owner_id.as_str())
                .unwrap_or("none"),
            gameplay_owner
                .map(|owner| owner.owner.provider_id.as_str())
                .unwrap_or("none"),
            gameplay_owner
                .map(|owner| owner.state_hash.as_str())
                .unwrap_or("none"),
            gameplay_owner
                .map(|owner| owner.replay_hash.as_str())
                .unwrap_or("none"),
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
