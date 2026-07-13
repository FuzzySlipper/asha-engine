//! Federated, replayable gameplay-module state owned through the closed fabric registry.

use protocol_game_extension::{GameplayContractRef, GameplayEventEnvelope, GameplayOwnerRef};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use svc_gameplay_fabric::GameplayFabricRegistry;

use crate::{observe::stable_hash, GameplayObserveReceipt};

const MODULE_STATE_SNAPSHOT_VERSION: u32 = 1;
const GAMEPLAY_SESSION_SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameplayModuleStateScope {
    Session,
    Entity { entity: u64 },
    PrefabInstance { instance: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleInitialization {
    pub initialization_id: String,
    pub module_id: String,
    pub state_schema: GameplayContractRef,
    pub scope: GameplayModuleStateScope,
    pub canonical_config: Vec<u8>,
    pub config_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleStateMigration {
    pub migration_id: String,
    pub module_id: String,
    pub from_state_schema: GameplayContractRef,
    pub to_state_schema: GameplayContractRef,
    pub scope: GameplayModuleStateScope,
    pub source_revision: u64,
    pub canonical_state: Vec<u8>,
    pub state_hash: String,
    pub initialized_from: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleFact {
    pub fact_id: String,
    pub module_id: String,
    pub fact_schema: GameplayContractRef,
    pub state_schema: GameplayContractRef,
    pub scope: GameplayModuleStateScope,
    pub expected_revision: u64,
    pub canonical_payload: Vec<u8>,
    pub payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayModuleStateRecord {
    pub module_id: String,
    pub state_schema: GameplayContractRef,
    pub owner: GameplayOwnerRef,
    pub scope: GameplayModuleStateScope,
    pub revision: u64,
    canonical_state: Vec<u8>,
    pub state_hash: String,
    pub initialized_from: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayModuleStateReceipt {
    pub fact: GameplayModuleFact,
    pub before_hash: String,
    pub after_hash: String,
    pub record_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayModuleStateReadout {
    pub module_id: String,
    pub state_contract: String,
    pub scope: GameplayModuleStateScope,
    pub revision: u64,
    pub state_hash: String,
    pub initialized_from: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayModuleNamedView {
    pub view: GameplayContractRef,
    pub provider_id: String,
    pub scope: GameplayModuleStateScope,
    pub revision: u64,
    pub canonical_payload: Vec<u8>,
    pub view_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayReactionSourceFact {
    pub owner_id: String,
    pub fact_kind: String,
    pub canonical_payload: Vec<u8>,
    pub fact_hash: String,
}

impl GameplayReactionSourceFact {
    pub fn new(owner_id: String, fact_kind: String, canonical_payload: Vec<u8>) -> Self {
        let fact_hash = gameplay_module_payload_hash(&canonical_payload);
        Self {
            owner_id,
            fact_kind,
            canonical_payload,
            fact_hash,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayReactionViewEvidence {
    pub epoch: u64,
    pub view_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayReactionInvocationEvidence {
    pub module_id: String,
    pub subscription_id: String,
    pub invocation_id: String,
    pub event_id: String,
    pub wave: u32,
    pub frozen_view_hash: String,
    pub declared_read_set_hash: Option<String>,
    #[serde(default)]
    pub declared_reads: Option<crate::GameplayFrozenReadSet>,
    #[serde(default)]
    pub configuration: Option<crate::GameplayInvocationConfiguration>,
    pub delivery_hash: String,
    pub output_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayReactionRoutingEvidence {
    pub proposal_id: String,
    pub proposal_kind: String,
    pub proposal_hash: String,
    pub owner_id: String,
    pub accepted: bool,
    pub fact_hashes: Vec<String>,
    pub diagnostic_codes: Vec<String>,
    pub routing_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayReactionDiagnostic {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayReactionFrame {
    pub registry_digest: String,
    #[serde(default)]
    pub root_id: String,
    pub module_order: Vec<String>,
    pub module_artifacts: Vec<String>,
    pub source_facts: Vec<GameplayReactionSourceFact>,
    pub source_fact_hashes: Vec<String>,
    #[serde(default)]
    pub root_events: Vec<GameplayEventEnvelope>,
    pub delivered_events: Vec<GameplayEventEnvelope>,
    pub delivered_event_hashes: Vec<String>,
    pub frozen_views: Vec<GameplayReactionViewEvidence>,
    pub frozen_view_hashes: Vec<String>,
    pub invocations: Vec<GameplayReactionInvocationEvidence>,
    pub invocation_output_hashes: Vec<String>,
    pub routing_receipts: Vec<GameplayReactionRoutingEvidence>,
    pub routed_proposal_hashes: Vec<String>,
    pub routing_hashes: Vec<String>,
    pub accepted_module_facts: Vec<GameplayModuleFact>,
    pub accepted_module_fact_hashes: Vec<String>,
    pub state_hash_before: String,
    pub state_hash_after: String,
    pub diagnostic_codes: Vec<String>,
    pub diagnostics: Vec<GameplayReactionDiagnostic>,
    pub final_session_hash: String,
    pub frame_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GameplayReactionDivergence {
    RegistryOrCode,
    SourceFacts,
    Events,
    Views,
    InvocationOutputs,
    ProposalsOrRouting,
    ModuleFacts,
    State,
    Diagnostics,
    FinalSession,
    FrameHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayVerificationReplayReceipt {
    pub expected_frame_hash: String,
    pub actual_frame_hash: String,
    pub divergences: Vec<GameplayReactionDivergence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameplayVerificationReplayInput {
    pub registry_digest: String,
    pub root_id: String,
    pub module_order: Vec<String>,
    pub module_artifacts: Vec<String>,
    pub source_facts: Vec<GameplayReactionSourceFact>,
    pub root_events: Vec<GameplayEventEnvelope>,
    pub frozen_views: Vec<GameplayReactionViewEvidence>,
    pub frozen_read_sets: Vec<crate::GameplayFrozenReadSet>,
    #[serde(default)]
    pub configurations: Vec<crate::GameplayInvocationConfiguration>,
    pub state_hash_before: String,
}

pub trait GameplayVerificationReplayRunner {
    /// Reruns the fabric using the statically linked module set identified by
    /// the expected frame. Registry and artifact drift is reported by the
    /// resulting frame comparison before the result can be accepted.
    fn rerun(
        &self,
        recorded: &GameplayVerificationReplayInput,
    ) -> Result<GameplayReactionFrame, GameplayModuleStateError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayModuleStateError {
    DuplicateAdapter,
    MissingAdapter,
    MissingOwner,
    OwnerMismatch,
    UndeclaredState,
    UndeclaredFact,
    UndeclaredView,
    ForeignModule,
    DuplicateInitialization,
    InvalidMigration,
    UnknownState,
    StaleRevision,
    PayloadHashMismatch,
    AdapterRejected(String),
    DuplicateFact,
    InvalidSnapshot(String),
}

impl core::fmt::Display for GameplayModuleStateError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GameplayModuleStateError {}

/// Stateless, statically composed typed edge. Type erasure is kept inside the
/// Session coordinator and persistence border; gameplay modules implement this
/// trait using concrete Rust configuration, state, fact, and view types.
pub trait GameplayTypedModuleStateAdapter {
    type Config;
    type State;
    type Fact;
    type View;

    fn module_id(&self) -> &str;
    fn state_schema(&self) -> &GameplayContractRef;
    fn fact_schema(&self) -> &GameplayContractRef;
    fn owner(&self) -> &GameplayOwnerRef;
    fn decode_config(&self, canonical_config: &[u8]) -> Result<Self::Config, String>;
    fn decode_state(&self, canonical_state: &[u8]) -> Result<Self::State, String>;
    fn decode_fact(&self, canonical_fact: &[u8]) -> Result<Self::Fact, String>;
    fn encode_state(&self, state: &Self::State) -> Result<Vec<u8>, String>;
    fn initialize(&self, config: &Self::Config) -> Result<Self::State, String>;
    fn apply_fact(&self, state: &Self::State, fact: &Self::Fact) -> Result<Self::State, String>;
    fn migrate(&self, from_version: u32, state: &Self::State) -> Result<Self::State, String>;
    fn view_schema(&self) -> Option<&GameplayContractRef> {
        None
    }
    fn project_view(&self, _state: &Self::State) -> Result<Self::View, String> {
        Err("adapter publishes no named view".to_owned())
    }
    fn encode_view(&self, _view: &Self::View) -> Result<Vec<u8>, String> {
        Err("adapter publishes no named view".to_owned())
    }
}

trait ErasedGameplayModuleStateAdapter {
    fn module_id(&self) -> &str;
    fn state_schema(&self) -> &GameplayContractRef;
    fn fact_schema(&self) -> &GameplayContractRef;
    fn owner(&self) -> &GameplayOwnerRef;
    fn initialize(&self, canonical_config: &[u8]) -> Result<Vec<u8>, String>;
    fn apply_fact(&self, canonical_state: &[u8], canonical_fact: &[u8]) -> Result<Vec<u8>, String>;
    fn migrate(&self, from_version: u32, canonical_state: &[u8]) -> Result<Vec<u8>, String>;
    fn view_schema(&self) -> Option<&GameplayContractRef>;
    fn project_view(&self, canonical_state: &[u8]) -> Result<Vec<u8>, String>;
}

struct TypedAdapter<T>(T);

impl<T> ErasedGameplayModuleStateAdapter for TypedAdapter<T>
where
    T: GameplayTypedModuleStateAdapter,
{
    fn module_id(&self) -> &str {
        self.0.module_id()
    }

    fn state_schema(&self) -> &GameplayContractRef {
        self.0.state_schema()
    }

    fn fact_schema(&self) -> &GameplayContractRef {
        self.0.fact_schema()
    }

    fn owner(&self) -> &GameplayOwnerRef {
        self.0.owner()
    }

    fn initialize(&self, canonical_config: &[u8]) -> Result<Vec<u8>, String> {
        let config = self.0.decode_config(canonical_config)?;
        let state = self.0.initialize(&config)?;
        self.0.encode_state(&state)
    }

    fn apply_fact(&self, canonical_state: &[u8], canonical_fact: &[u8]) -> Result<Vec<u8>, String> {
        let state = self.0.decode_state(canonical_state)?;
        let fact = self.0.decode_fact(canonical_fact)?;
        let next = self.0.apply_fact(&state, &fact)?;
        self.0.encode_state(&next)
    }

    fn migrate(&self, from_version: u32, canonical_state: &[u8]) -> Result<Vec<u8>, String> {
        let state = self.0.decode_state(canonical_state)?;
        let migrated = self.0.migrate(from_version, &state)?;
        self.0.encode_state(&migrated)
    }

    fn view_schema(&self) -> Option<&GameplayContractRef> {
        self.0.view_schema()
    }

    fn project_view(&self, canonical_state: &[u8]) -> Result<Vec<u8>, String> {
        let state = self.0.decode_state(canonical_state)?;
        let view = self.0.project_view(&state)?;
        self.0.encode_view(&view)
    }
}

/// Opaque registration token accepted by the Session state coordinator.
/// Callers cannot construct an erased adapter or inspect its byte-level state.
pub struct GameplayModuleStateRegistration {
    adapter: Box<dyn ErasedGameplayModuleStateAdapter>,
}

impl GameplayModuleStateRegistration {
    pub fn typed<T>(adapter: T) -> Self
    where
        T: GameplayTypedModuleStateAdapter + 'static,
    {
        Self {
            adapter: Box::new(TypedAdapter(adapter)),
        }
    }

    pub fn validate_against_registry(
        &self,
        registry: &GameplayFabricRegistry,
    ) -> Result<(), GameplayModuleStateError> {
        validate_adapter(registry, self.adapter.as_ref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RecordKey {
    state_key: String,
    scope: GameplayModuleStateScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GameplayModuleStateSnapshot {
    schema_version: u32,
    registry_digest: String,
    records: Vec<StoredGameplayModuleStateRecord>,
    applied_fact_ids: BTreeSet<String>,
    accepted_facts: Vec<GameplayModuleFact>,
    state_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GameplaySessionSnapshot {
    schema_version: u32,
    registry_digest: String,
    authority_snapshot: Vec<u8>,
    authority_snapshot_hash: String,
    authority_state_hash: String,
    module_state_snapshot: Vec<u8>,
    module_state_hash: String,
    final_session_hash: String,
}

/// Validated result of restoring the gameplay portion of a RuntimeSession.
/// The owning Session restores its typed authority snapshot from the opaque
/// bytes only after this envelope and its composed hashes have been checked.
pub struct GameplaySessionRestore {
    pub authority_snapshot: Vec<u8>,
    pub authority_state_hash: String,
    pub module_state: GameplayModuleStateStore,
    pub final_session_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredGameplayModuleStateRecord {
    module_id: String,
    state_schema: GameplayContractRef,
    owner: GameplayOwnerRef,
    scope: GameplayModuleStateScope,
    revision: u64,
    canonical_state: Vec<u8>,
    state_hash: String,
    initialized_from: String,
}

impl From<&GameplayModuleStateRecord> for StoredGameplayModuleStateRecord {
    fn from(record: &GameplayModuleStateRecord) -> Self {
        Self {
            module_id: record.module_id.clone(),
            state_schema: record.state_schema.clone(),
            owner: record.owner.clone(),
            scope: record.scope.clone(),
            revision: record.revision,
            canonical_state: record.canonical_state.clone(),
            state_hash: record.state_hash.clone(),
            initialized_from: record.initialized_from.clone(),
        }
    }
}

impl From<StoredGameplayModuleStateRecord> for GameplayModuleStateRecord {
    fn from(record: StoredGameplayModuleStateRecord) -> Self {
        Self {
            module_id: record.module_id,
            state_schema: record.state_schema,
            owner: record.owner,
            scope: record.scope,
            revision: record.revision,
            canonical_state: record.canonical_state,
            state_hash: record.state_hash,
            initialized_from: record.initialized_from,
        }
    }
}

pub struct GameplayModuleStateStore {
    registry: Rc<GameplayFabricRegistry>,
    adapters: BTreeMap<String, Box<dyn ErasedGameplayModuleStateAdapter>>,
    records: BTreeMap<RecordKey, GameplayModuleStateRecord>,
    applied_fact_ids: BTreeSet<String>,
    accepted_facts: Vec<GameplayModuleFact>,
}

impl GameplayModuleStateStore {
    pub fn new(
        registry: Rc<GameplayFabricRegistry>,
        registrations: Vec<GameplayModuleStateRegistration>,
    ) -> Result<Self, GameplayModuleStateError> {
        let mut indexed = BTreeMap::new();
        for registration in registrations {
            let adapter = registration.adapter;
            let key = adapter.state_schema().key();
            if indexed.contains_key(&key) {
                return Err(GameplayModuleStateError::DuplicateAdapter);
            }
            validate_adapter(registry.as_ref(), adapter.as_ref())?;
            indexed.insert(key, adapter);
        }
        Ok(Self {
            registry,
            adapters: indexed,
            records: BTreeMap::new(),
            applied_fact_ids: BTreeSet::new(),
            accepted_facts: Vec::new(),
        })
    }

    pub fn initialize_atomic(
        &mut self,
        initializations: Vec<GameplayModuleInitialization>,
    ) -> Result<(), GameplayModuleStateError> {
        let mut staged = Vec::new();
        let mut staged_keys = BTreeSet::new();
        for initialization in initializations {
            verify_hash(
                &initialization.canonical_config,
                &initialization.config_hash,
            )?;
            let key = RecordKey {
                state_key: initialization.state_schema.key(),
                scope: initialization.scope.clone(),
            };
            if self.records.contains_key(&key) || !staged_keys.insert(key.clone()) {
                return Err(GameplayModuleStateError::DuplicateInitialization);
            }
            let adapter = self.adapter_for(&initialization.state_schema)?;
            if adapter.module_id() != initialization.module_id {
                return Err(GameplayModuleStateError::ForeignModule);
            }
            let canonical_state = adapter
                .initialize(&initialization.canonical_config)
                .map_err(GameplayModuleStateError::AdapterRejected)?;
            staged.push((
                key,
                GameplayModuleStateRecord {
                    module_id: initialization.module_id,
                    state_schema: initialization.state_schema,
                    owner: adapter.owner().clone(),
                    scope: initialization.scope,
                    revision: 0,
                    state_hash: gameplay_module_payload_hash(&canonical_state),
                    canonical_state,
                    initialized_from: initialization.initialization_id,
                },
            ));
        }
        for (key, record) in staged {
            self.records.insert(key, record);
        }
        Ok(())
    }

    /// Atomically upgrades durable state from an explicit older schema into
    /// the current closed registry. No old-schema owner is installed into the
    /// live registry and no target record appears unless every migration in
    /// the batch validates and succeeds.
    pub fn migrate_atomic(
        &mut self,
        migrations: Vec<GameplayModuleStateMigration>,
    ) -> Result<(), GameplayModuleStateError> {
        let mut staged = Vec::new();
        let mut staged_keys = BTreeSet::new();
        for migration in migrations {
            verify_hash(&migration.canonical_state, &migration.state_hash)?;
            if migration.from_state_schema.namespace != migration.to_state_schema.namespace
                || migration.from_state_schema.name != migration.to_state_schema.name
                || migration.from_state_schema.version >= migration.to_state_schema.version
            {
                return Err(GameplayModuleStateError::InvalidMigration);
            }
            let key = RecordKey {
                state_key: migration.to_state_schema.key(),
                scope: migration.scope.clone(),
            };
            if self.records.contains_key(&key) || !staged_keys.insert(key.clone()) {
                return Err(GameplayModuleStateError::DuplicateInitialization);
            }
            let adapter = self.adapter_for(&migration.to_state_schema)?;
            if adapter.module_id() != migration.module_id {
                return Err(GameplayModuleStateError::ForeignModule);
            }
            let canonical_state = adapter
                .migrate(
                    migration.from_state_schema.version,
                    &migration.canonical_state,
                )
                .map_err(GameplayModuleStateError::AdapterRejected)?;
            staged.push((
                key,
                GameplayModuleStateRecord {
                    module_id: migration.module_id,
                    state_schema: migration.to_state_schema,
                    owner: adapter.owner().clone(),
                    scope: migration.scope,
                    revision: migration.source_revision.saturating_add(1),
                    state_hash: gameplay_module_payload_hash(&canonical_state),
                    canonical_state,
                    initialized_from: format!(
                        "{}; migrated-by:{}",
                        migration.initialized_from, migration.migration_id
                    ),
                },
            ));
        }
        for (key, record) in staged {
            self.records.insert(key, record);
        }
        Ok(())
    }

    pub fn apply_fact(
        &mut self,
        fact: GameplayModuleFact,
    ) -> Result<GameplayModuleStateReceipt, GameplayModuleStateError> {
        if self.applied_fact_ids.contains(&fact.fact_id) {
            return Err(GameplayModuleStateError::DuplicateFact);
        }
        verify_hash(&fact.canonical_payload, &fact.payload_hash)?;
        let key = RecordKey {
            state_key: fact.state_schema.key(),
            scope: fact.scope.clone(),
        };
        let adapter = self.adapter_for(&fact.state_schema)?;
        if adapter.module_id() != fact.module_id {
            return Err(GameplayModuleStateError::ForeignModule);
        }
        if adapter.fact_schema() != &fact.fact_schema
            || !self
                .registry
                .module_declares_fact(&fact.module_id, &fact.fact_schema)
        {
            return Err(GameplayModuleStateError::UndeclaredFact);
        }
        let record = self
            .records
            .get(&key)
            .ok_or(GameplayModuleStateError::UnknownState)?;
        if record.revision != fact.expected_revision {
            return Err(GameplayModuleStateError::StaleRevision);
        }
        let next_state = adapter
            .apply_fact(&record.canonical_state, &fact.canonical_payload)
            .map_err(GameplayModuleStateError::AdapterRejected)?;
        let before_hash = self.state_hash();
        let record = self.records.get_mut(&key).expect("record was checked");
        record.canonical_state = next_state;
        record.state_hash = gameplay_module_payload_hash(&record.canonical_state);
        record.revision = record.revision.saturating_add(1);
        let record_revision = record.revision;
        self.applied_fact_ids.insert(fact.fact_id.clone());
        self.accepted_facts.push(fact.clone());
        Ok(GameplayModuleStateReceipt {
            fact,
            before_hash,
            after_hash: self.state_hash(),
            record_revision,
        })
    }

    /// Validate and apply one invocation's accepted fact batch atomically. The
    /// adapters run against a staged record map, so a bad later fact cannot
    /// leave earlier state mutations behind.
    pub fn apply_facts_atomic(
        &mut self,
        facts: &[GameplayModuleFact],
    ) -> Result<(), GameplayModuleStateError> {
        let mut staged_records = self.records.clone();
        let mut staged_fact_ids = self.applied_fact_ids.clone();
        for fact in facts {
            if !staged_fact_ids.insert(fact.fact_id.clone()) {
                return Err(GameplayModuleStateError::DuplicateFact);
            }
            verify_hash(&fact.canonical_payload, &fact.payload_hash)?;
            let key = RecordKey {
                state_key: fact.state_schema.key(),
                scope: fact.scope.clone(),
            };
            let adapter = self.adapter_for(&fact.state_schema)?;
            if adapter.module_id() != fact.module_id {
                return Err(GameplayModuleStateError::ForeignModule);
            }
            if adapter.fact_schema() != &fact.fact_schema
                || !self
                    .registry
                    .module_declares_fact(&fact.module_id, &fact.fact_schema)
            {
                return Err(GameplayModuleStateError::UndeclaredFact);
            }
            let record = staged_records
                .get_mut(&key)
                .ok_or(GameplayModuleStateError::UnknownState)?;
            if record.revision != fact.expected_revision {
                return Err(GameplayModuleStateError::StaleRevision);
            }
            record.canonical_state = adapter
                .apply_fact(&record.canonical_state, &fact.canonical_payload)
                .map_err(GameplayModuleStateError::AdapterRejected)?;
            record.state_hash = gameplay_module_payload_hash(&record.canonical_state);
            record.revision = record.revision.saturating_add(1);
        }
        self.records = staged_records;
        self.applied_fact_ids = staged_fact_ids;
        self.accepted_facts.extend(facts.iter().cloned());
        Ok(())
    }

    pub fn record(
        &self,
        state_schema: &GameplayContractRef,
        scope: &GameplayModuleStateScope,
    ) -> Option<&GameplayModuleStateRecord> {
        self.records.get(&RecordKey {
            state_key: state_schema.key(),
            scope: scope.clone(),
        })
    }

    pub fn migrate_record(
        &mut self,
        state_schema: &GameplayContractRef,
        scope: &GameplayModuleStateScope,
        from_version: u32,
        expected_revision: u64,
    ) -> Result<(), GameplayModuleStateError> {
        let key = RecordKey {
            state_key: state_schema.key(),
            scope: scope.clone(),
        };
        let adapter = self.adapter_for(state_schema)?;
        let record = self
            .records
            .get(&key)
            .ok_or(GameplayModuleStateError::UnknownState)?;
        if record.revision != expected_revision {
            return Err(GameplayModuleStateError::StaleRevision);
        }
        let migrated = adapter
            .migrate(from_version, &record.canonical_state)
            .map_err(GameplayModuleStateError::AdapterRejected)?;
        let record = self.records.get_mut(&key).expect("record was checked");
        record.canonical_state = migrated;
        record.state_hash = gameplay_module_payload_hash(&record.canonical_state);
        record.revision = record.revision.saturating_add(1);
        Ok(())
    }

    pub fn accepted_facts(&self) -> &[GameplayModuleFact] {
        &self.accepted_facts
    }

    pub fn readouts(&self) -> Vec<GameplayModuleStateReadout> {
        self.records
            .values()
            .map(|record| GameplayModuleStateReadout {
                module_id: record.module_id.clone(),
                state_contract: record.state_schema.key(),
                scope: record.scope.clone(),
                revision: record.revision,
                state_hash: record.state_hash.clone(),
                initialized_from: record.initialized_from.clone(),
            })
            .collect()
    }

    pub fn named_view(
        &self,
        state_schema: &GameplayContractRef,
        scope: &GameplayModuleStateScope,
    ) -> Result<GameplayModuleNamedView, GameplayModuleStateError> {
        let adapter = self.adapter_for(state_schema)?;
        let view = adapter
            .view_schema()
            .ok_or(GameplayModuleStateError::MissingAdapter)?;
        let provider = self
            .registry
            .read_view_provider(view)
            .ok_or(GameplayModuleStateError::MissingOwner)?;
        if provider.provider_id != adapter.owner().provider_id {
            return Err(GameplayModuleStateError::OwnerMismatch);
        }
        let record = self
            .record(state_schema, scope)
            .ok_or(GameplayModuleStateError::UnknownState)?;
        let canonical_payload = adapter
            .project_view(&record.canonical_state)
            .map_err(GameplayModuleStateError::AdapterRejected)?;
        Ok(GameplayModuleNamedView {
            view: view.clone(),
            provider_id: provider.provider_id.clone(),
            scope: scope.clone(),
            revision: record.revision,
            view_hash: gameplay_module_payload_hash(&canonical_payload),
            canonical_payload,
        })
    }

    /// Projects a registered module-owned view without revealing the backing
    /// state schema to the consumer. The closed registry remains the authority
    /// for whether the view exists and which provider owns it.
    pub fn named_view_by_contract(
        &self,
        view: &GameplayContractRef,
        scope: &GameplayModuleStateScope,
    ) -> Result<GameplayModuleNamedView, GameplayModuleStateError> {
        let state_schema = self
            .adapters
            .values()
            .find_map(|adapter| {
                (adapter.view_schema() == Some(view)).then(|| adapter.state_schema().clone())
            })
            .ok_or(GameplayModuleStateError::MissingAdapter)?;
        self.named_view(&state_schema, scope)
    }

    pub fn state_hash(&self) -> String {
        let records = self
            .records
            .values()
            .map(StoredGameplayModuleStateRecord::from)
            .collect::<Vec<_>>();
        let encoded = serde_json::to_vec(&(
            self.registry.registry_digest(),
            records,
            &self.applied_fact_ids,
        ))
        .expect("module state values serialize");
        gameplay_module_payload_hash(&encoded)
    }

    /// Composes the owning RuntimeSession's authority hash with the closed
    /// registry and all federated module state. A module state change therefore
    /// necessarily changes the overall Session hash.
    pub fn final_session_hash(&self, authority_state_hash: &str) -> String {
        stable_hash([
            self.registry.registry_digest(),
            authority_state_hash,
            self.state_hash().as_str(),
        ])
    }

    pub fn encode_snapshot(&self) -> Result<Vec<u8>, GameplayModuleStateError> {
        serde_json::to_vec(&GameplayModuleStateSnapshot {
            schema_version: MODULE_STATE_SNAPSHOT_VERSION,
            registry_digest: self.registry.registry_digest().to_owned(),
            records: self.records.values().map(Into::into).collect(),
            applied_fact_ids: self.applied_fact_ids.clone(),
            accepted_facts: self.accepted_facts.clone(),
            state_hash: self.state_hash(),
        })
        .map_err(|error| GameplayModuleStateError::InvalidSnapshot(error.to_string()))
    }

    /// Produces the save/load envelope owned by RuntimeSession composition.
    /// The base authority snapshot remains typed and owned by its Session lane;
    /// this coordinator binds it to module state without interpreting it.
    pub fn encode_session_snapshot(
        &self,
        authority_snapshot: &[u8],
        authority_state_hash: &str,
    ) -> Result<Vec<u8>, GameplayModuleStateError> {
        let module_state_snapshot = self.encode_snapshot()?;
        serde_json::to_vec(&GameplaySessionSnapshot {
            schema_version: GAMEPLAY_SESSION_SNAPSHOT_VERSION,
            registry_digest: self.registry.registry_digest().to_owned(),
            authority_snapshot: authority_snapshot.to_vec(),
            authority_snapshot_hash: gameplay_module_payload_hash(authority_snapshot),
            authority_state_hash: authority_state_hash.to_owned(),
            module_state_snapshot,
            module_state_hash: self.state_hash(),
            final_session_hash: self.final_session_hash(authority_state_hash),
        })
        .map_err(|error| GameplayModuleStateError::InvalidSnapshot(error.to_string()))
    }

    pub fn decode_snapshot(
        registry: Rc<GameplayFabricRegistry>,
        adapters: Vec<GameplayModuleStateRegistration>,
        bytes: &[u8],
    ) -> Result<Self, GameplayModuleStateError> {
        let snapshot: GameplayModuleStateSnapshot = serde_json::from_slice(bytes)
            .map_err(|error| GameplayModuleStateError::InvalidSnapshot(error.to_string()))?;
        if snapshot.schema_version != MODULE_STATE_SNAPSHOT_VERSION
            || snapshot.registry_digest != registry.registry_digest()
        {
            return Err(GameplayModuleStateError::InvalidSnapshot(
                "snapshot version or registry digest mismatch".to_owned(),
            ));
        }
        let mut store = Self::new(registry, adapters)?;
        for stored_record in snapshot.records {
            let record = GameplayModuleStateRecord::from(stored_record);
            let key = RecordKey {
                state_key: record.state_schema.key(),
                scope: record.scope.clone(),
            };
            if store.records.insert(key, record).is_some() {
                return Err(GameplayModuleStateError::InvalidSnapshot(
                    "duplicate module state record".to_owned(),
                ));
            }
        }
        store.applied_fact_ids = snapshot.applied_fact_ids;
        store.accepted_facts = snapshot.accepted_facts;
        store.validate_records()?;
        if store.state_hash() != snapshot.state_hash {
            return Err(GameplayModuleStateError::InvalidSnapshot(
                "module state hash mismatch".to_owned(),
            ));
        }
        Ok(store)
    }

    pub fn decode_session_snapshot(
        registry: Rc<GameplayFabricRegistry>,
        adapters: Vec<GameplayModuleStateRegistration>,
        bytes: &[u8],
    ) -> Result<GameplaySessionRestore, GameplayModuleStateError> {
        let snapshot: GameplaySessionSnapshot = serde_json::from_slice(bytes)
            .map_err(|error| GameplayModuleStateError::InvalidSnapshot(error.to_string()))?;
        if snapshot.schema_version != GAMEPLAY_SESSION_SNAPSHOT_VERSION
            || snapshot.registry_digest != registry.registry_digest()
            || gameplay_module_payload_hash(&snapshot.authority_snapshot)
                != snapshot.authority_snapshot_hash
        {
            return Err(GameplayModuleStateError::InvalidSnapshot(
                "Session snapshot version, registry, or authority artifact mismatch".to_owned(),
            ));
        }
        let module_state =
            Self::decode_snapshot(registry, adapters, &snapshot.module_state_snapshot)?;
        if module_state.state_hash() != snapshot.module_state_hash
            || module_state.final_session_hash(&snapshot.authority_state_hash)
                != snapshot.final_session_hash
        {
            return Err(GameplayModuleStateError::InvalidSnapshot(
                "Session module-state or composed hash mismatch".to_owned(),
            ));
        }
        Ok(GameplaySessionRestore {
            authority_snapshot: snapshot.authority_snapshot,
            authority_state_hash: snapshot.authority_state_hash,
            module_state,
            final_session_hash: snapshot.final_session_hash,
        })
    }

    pub fn playback(
        registry: Rc<GameplayFabricRegistry>,
        adapters: Vec<GameplayModuleStateRegistration>,
        initializations: Vec<GameplayModuleInitialization>,
        facts: &[GameplayModuleFact],
    ) -> Result<Self, GameplayModuleStateError> {
        let mut store = Self::new(registry, adapters)?;
        store.initialize_atomic(initializations)?;
        for fact in facts {
            store.apply_fact(fact.clone())?;
        }
        Ok(store)
    }

    /// Applies the accepted module facts captured in a reaction frame without
    /// invoking gameplay module behavior or dispatching its recorded events.
    pub fn playback_frame(
        registry: Rc<GameplayFabricRegistry>,
        adapters: Vec<GameplayModuleStateRegistration>,
        initializations: Vec<GameplayModuleInitialization>,
        frame: &GameplayReactionFrame,
    ) -> Result<Self, GameplayModuleStateError> {
        Self::playback(
            registry,
            adapters,
            initializations,
            &frame.accepted_module_facts,
        )
    }

    fn adapter_for(
        &self,
        schema: &GameplayContractRef,
    ) -> Result<&dyn ErasedGameplayModuleStateAdapter, GameplayModuleStateError> {
        self.adapters
            .get(&schema.key())
            .map(Box::as_ref)
            .ok_or(GameplayModuleStateError::MissingAdapter)
    }

    fn validate_records(&self) -> Result<(), GameplayModuleStateError> {
        for (key, record) in &self.records {
            let adapter = self.adapter_for(&record.state_schema)?;
            if key.state_key != record.state_schema.key()
                || adapter.module_id() != record.module_id
                || adapter.owner() != &record.owner
                || gameplay_module_payload_hash(&record.canonical_state) != record.state_hash
            {
                return Err(GameplayModuleStateError::InvalidSnapshot(
                    "record ownership or hash mismatch".to_owned(),
                ));
            }
        }
        let mut accepted_ids = BTreeSet::new();
        for fact in &self.accepted_facts {
            if !accepted_ids.insert(fact.fact_id.clone())
                || gameplay_module_payload_hash(&fact.canonical_payload) != fact.payload_hash
            {
                return Err(GameplayModuleStateError::InvalidSnapshot(
                    "duplicate accepted fact or payload hash mismatch".to_owned(),
                ));
            }
            let adapter = self.adapter_for(&fact.state_schema)?;
            let key = RecordKey {
                state_key: fact.state_schema.key(),
                scope: fact.scope.clone(),
            };
            if adapter.module_id() != fact.module_id
                || adapter.fact_schema() != &fact.fact_schema
                || !self
                    .registry
                    .module_declares_fact(&fact.module_id, &fact.fact_schema)
                || !self.records.contains_key(&key)
            {
                return Err(GameplayModuleStateError::InvalidSnapshot(
                    "accepted fact ownership or target mismatch".to_owned(),
                ));
            }
        }
        if accepted_ids != self.applied_fact_ids {
            return Err(GameplayModuleStateError::InvalidSnapshot(
                "accepted fact evidence does not match applied fact ids".to_owned(),
            ));
        }
        Ok(())
    }
}

fn validate_adapter(
    registry: &GameplayFabricRegistry,
    adapter: &dyn ErasedGameplayModuleStateAdapter,
) -> Result<(), GameplayModuleStateError> {
    if !registry.module_declares_state(adapter.module_id(), adapter.state_schema()) {
        return Err(GameplayModuleStateError::UndeclaredState);
    }
    if !registry.module_declares_fact(adapter.module_id(), adapter.fact_schema()) {
        return Err(GameplayModuleStateError::UndeclaredFact);
    }
    if adapter
        .view_schema()
        .is_some_and(|view| !registry.module_declares_named_view(adapter.module_id(), view))
    {
        return Err(GameplayModuleStateError::UndeclaredView);
    }
    let owner = registry
        .state_owner(adapter.state_schema())
        .ok_or(GameplayModuleStateError::MissingOwner)?;
    if owner != adapter.owner() {
        return Err(GameplayModuleStateError::OwnerMismatch);
    }
    Ok(())
}

fn verify_hash(payload: &[u8], expected: &str) -> Result<(), GameplayModuleStateError> {
    if gameplay_module_payload_hash(payload) == expected {
        Ok(())
    } else {
        Err(GameplayModuleStateError::PayloadHashMismatch)
    }
}

pub fn gameplay_module_payload_hash(payload: &[u8]) -> String {
    let length = payload.len().to_string();
    let hex = payload
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    stable_hash([length.as_str(), hex.as_str()])
}

impl GameplayReactionFrame {
    pub fn from_observe(
        registry: &GameplayFabricRegistry,
        observe: &GameplayObserveReceipt,
        source_facts: Vec<GameplayReactionSourceFact>,
        accepted_module_facts: &[GameplayModuleFact],
        state_hash_before: String,
        state_hash_after: String,
        final_session_hash: String,
    ) -> Self {
        let mut source_fact_hashes = source_facts
            .iter()
            .map(|fact| fact.fact_hash.clone())
            .collect::<Vec<_>>();
        source_fact_hashes.sort();
        let mut module_artifacts = registry
            .module_order()
            .iter()
            .filter_map(|module_id| registry.module(module_id))
            .map(|manifest| {
                format!(
                    "{}:{}:{}:{}",
                    manifest.module_ref.module_id,
                    manifest.module_ref.artifact_hash,
                    manifest.module_ref.contract_hash,
                    manifest.source_hash
                )
            })
            .collect::<Vec<_>>();
        module_artifacts.sort();
        let mut accepted_module_fact_hashes = accepted_module_facts
            .iter()
            .map(|fact| gameplay_module_payload_hash(&fact.canonical_payload))
            .collect::<Vec<_>>();
        accepted_module_fact_hashes.sort();
        let mut frame = Self {
            registry_digest: registry.registry_digest().to_owned(),
            root_id: observe.root_id.clone(),
            module_order: registry.module_order().to_vec(),
            module_artifacts,
            source_facts,
            source_fact_hashes,
            root_events: observe.events.first().cloned().into_iter().collect(),
            delivered_events: observe.events.clone(),
            delivered_event_hashes: observe
                .event_evidence
                .iter()
                .map(|evidence| evidence.event_hash.clone())
                .collect(),
            frozen_views: observe
                .wave_views
                .iter()
                .map(|views| GameplayReactionViewEvidence {
                    epoch: views.epoch,
                    view_hash: views.view_hash.clone(),
                })
                .collect(),
            frozen_view_hashes: observe
                .wave_views
                .iter()
                .map(|views| views.view_hash.clone())
                .collect(),
            invocations: observe
                .invocations
                .iter()
                .map(|invocation| GameplayReactionInvocationEvidence {
                    module_id: invocation.module_id.clone(),
                    subscription_id: invocation.subscription_id.clone(),
                    invocation_id: invocation.invocation_id.clone(),
                    event_id: invocation.event_id.clone(),
                    wave: invocation.wave,
                    frozen_view_hash: invocation.frozen_view_hash.clone(),
                    declared_read_set_hash: invocation.declared_read_set_hash.clone(),
                    declared_reads: invocation.declared_reads.clone(),
                    configuration: invocation.configuration.clone(),
                    delivery_hash: invocation.delivery_hash.clone(),
                    output_hash: invocation.output_hash.clone(),
                })
                .collect(),
            invocation_output_hashes: observe
                .invocations
                .iter()
                .map(|invocation| invocation.output_hash.clone())
                .collect(),
            routing_receipts: observe
                .routing
                .iter()
                .map(|routing| GameplayReactionRoutingEvidence {
                    proposal_id: routing.proposal_id.clone(),
                    proposal_kind: routing.proposal_kind.clone(),
                    proposal_hash: routing.proposal_hash.clone(),
                    owner_id: routing.owner_id.clone(),
                    accepted: routing.accepted,
                    fact_hashes: routing.fact_hashes.clone(),
                    diagnostic_codes: routing.diagnostic_codes.clone(),
                    routing_hash: routing.routing_hash.clone(),
                })
                .collect(),
            routed_proposal_hashes: observe
                .routing
                .iter()
                .map(|routing| routing.proposal_hash.clone())
                .collect(),
            routing_hashes: observe
                .routing
                .iter()
                .map(|routing| routing.routing_hash.clone())
                .collect(),
            accepted_module_facts: accepted_module_facts.to_vec(),
            accepted_module_fact_hashes,
            state_hash_before,
            state_hash_after,
            diagnostic_codes: observe
                .diagnostics
                .iter()
                .map(|diagnostic| format!("{:?}", diagnostic.code))
                .collect(),
            diagnostics: observe
                .diagnostics
                .iter()
                .map(|diagnostic| GameplayReactionDiagnostic {
                    code: format!("{:?}", diagnostic.code),
                    path: diagnostic.path.clone(),
                    message: diagnostic.message.clone(),
                })
                .collect(),
            final_session_hash,
            frame_hash: String::new(),
        };
        frame.frame_hash = frame.canonical_hash();
        frame
    }

    pub fn canonical_hash(&self) -> String {
        let mut copy = self.clone();
        copy.frame_hash.clear();
        let bytes = serde_json::to_vec(&copy).expect("reaction frame serializes");
        gameplay_module_payload_hash(&bytes)
    }

    pub fn verification_replay_input(&self) -> GameplayVerificationReplayInput {
        GameplayVerificationReplayInput {
            registry_digest: self.registry_digest.clone(),
            root_id: self.root_id.clone(),
            module_order: self.module_order.clone(),
            module_artifacts: self.module_artifacts.clone(),
            source_facts: self.source_facts.clone(),
            root_events: self.root_events.clone(),
            frozen_views: self.frozen_views.clone(),
            frozen_read_sets: self
                .invocations
                .iter()
                .filter_map(|invocation| invocation.declared_reads.clone())
                .collect(),
            configurations: self
                .invocations
                .iter()
                .filter_map(|invocation| invocation.configuration.clone())
                .collect(),
            state_hash_before: self.state_hash_before.clone(),
        }
    }
}

pub fn verify_reaction_frame(
    expected: &GameplayReactionFrame,
    actual: &GameplayReactionFrame,
) -> Vec<GameplayReactionDivergence> {
    let mut divergences = BTreeSet::new();
    if expected.registry_digest != actual.registry_digest
        || expected.module_order != actual.module_order
        || expected.module_artifacts != actual.module_artifacts
    {
        divergences.insert(GameplayReactionDivergence::RegistryOrCode);
    }
    if !source_fact_evidence_is_valid(expected)
        || !source_fact_evidence_is_valid(actual)
        || expected.source_facts != actual.source_facts
        || expected.source_fact_hashes != actual.source_fact_hashes
    {
        divergences.insert(GameplayReactionDivergence::SourceFacts);
    }
    if !event_evidence_is_valid(expected)
        || !event_evidence_is_valid(actual)
        || expected.root_id != actual.root_id
        || expected.root_events != actual.root_events
        || expected.delivered_events != actual.delivered_events
        || expected.delivered_event_hashes != actual.delivered_event_hashes
    {
        divergences.insert(GameplayReactionDivergence::Events);
    }
    if !view_evidence_is_valid(expected)
        || !view_evidence_is_valid(actual)
        || expected.frozen_views != actual.frozen_views
        || expected.frozen_view_hashes != actual.frozen_view_hashes
    {
        divergences.insert(GameplayReactionDivergence::Views);
    }
    if !invocation_evidence_is_valid(expected)
        || !invocation_evidence_is_valid(actual)
        || expected.invocations != actual.invocations
        || expected.invocation_output_hashes != actual.invocation_output_hashes
    {
        divergences.insert(GameplayReactionDivergence::InvocationOutputs);
    }
    if !routing_evidence_is_valid(expected)
        || !routing_evidence_is_valid(actual)
        || expected.routing_receipts != actual.routing_receipts
        || expected.routed_proposal_hashes != actual.routed_proposal_hashes
        || expected.routing_hashes != actual.routing_hashes
    {
        divergences.insert(GameplayReactionDivergence::ProposalsOrRouting);
    }
    if !module_fact_evidence_is_valid(expected)
        || !module_fact_evidence_is_valid(actual)
        || expected.accepted_module_facts != actual.accepted_module_facts
        || expected.accepted_module_fact_hashes != actual.accepted_module_fact_hashes
    {
        divergences.insert(GameplayReactionDivergence::ModuleFacts);
    }
    if expected.state_hash_before != actual.state_hash_before
        || expected.state_hash_after != actual.state_hash_after
    {
        divergences.insert(GameplayReactionDivergence::State);
    }
    if !diagnostic_evidence_is_valid(expected)
        || !diagnostic_evidence_is_valid(actual)
        || expected.diagnostics != actual.diagnostics
        || expected.diagnostic_codes != actual.diagnostic_codes
    {
        divergences.insert(GameplayReactionDivergence::Diagnostics);
    }
    if expected.final_session_hash != actual.final_session_hash {
        divergences.insert(GameplayReactionDivergence::FinalSession);
    }
    if expected.frame_hash != expected.canonical_hash()
        || actual.frame_hash != actual.canonical_hash()
    {
        divergences.insert(GameplayReactionDivergence::FrameHash);
    }
    divergences.into_iter().collect()
}

fn source_fact_evidence_is_valid(frame: &GameplayReactionFrame) -> bool {
    let mut hashes = frame
        .source_facts
        .iter()
        .map(|fact| {
            (
                gameplay_module_payload_hash(&fact.canonical_payload),
                fact.fact_hash.as_str(),
            )
        })
        .collect::<Vec<_>>();
    if hashes.iter().any(|(computed, stored)| computed != stored) {
        return false;
    }
    hashes.sort_by(|left, right| left.0.cmp(&right.0));
    hashes
        .into_iter()
        .map(|(computed, _)| computed)
        .collect::<Vec<_>>()
        == frame.source_fact_hashes
}

fn event_evidence_is_valid(frame: &GameplayReactionFrame) -> bool {
    (frame.root_id.is_empty()
        == (frame.root_events.is_empty() && frame.delivered_events.is_empty()))
        && frame
            .root_events
            .iter()
            .all(|event| event.causation.root_id == frame.root_id)
        && frame
            .delivered_events
            .iter()
            .all(|event| event.causation.root_id == frame.root_id)
        && frame
            .delivered_events
            .iter()
            .map(crate::observe::event_hash)
            .eq(frame.delivered_event_hashes.iter().cloned())
}

fn view_evidence_is_valid(frame: &GameplayReactionFrame) -> bool {
    let wave_hashes_valid = frame
        .frozen_views
        .iter()
        .map(|view| view.view_hash.as_str())
        .eq(frame.frozen_view_hashes.iter().map(String::as_str));
    wave_hashes_valid
        && frame.invocations.iter().all(|invocation| {
            match (
                &invocation.declared_read_set_hash,
                &invocation.declared_reads,
            ) {
                (None, None) => true,
                (Some(stored_hash), Some(reads)) => {
                    reads.registry_digest == frame.registry_digest
                        && reads.module_id == invocation.module_id
                        && reads.invocation_id == invocation.invocation_id
                        && reads.event_id == invocation.event_id
                        && reads.wave == invocation.wave
                        && reads.read_set_hash == *stored_hash
                        && reads.nested_hashes_are_valid()
                }
                _ => false,
            }
        })
}

fn invocation_evidence_is_valid(frame: &GameplayReactionFrame) -> bool {
    frame.invocations.iter().all(|invocation| {
        invocation
            .configuration
            .as_ref()
            .is_none_or(|configuration| {
                gameplay_module_payload_hash(&configuration.canonical_config)
                    == configuration.config_hash
            })
    }) && frame
        .invocations
        .iter()
        .map(|invocation| invocation.output_hash.as_str())
        .eq(frame.invocation_output_hashes.iter().map(String::as_str))
}

fn routing_evidence_is_valid(frame: &GameplayReactionFrame) -> bool {
    frame
        .routing_receipts
        .iter()
        .map(|routing| routing.proposal_hash.as_str())
        .eq(frame.routed_proposal_hashes.iter().map(String::as_str))
        && frame
            .routing_receipts
            .iter()
            .map(|routing| routing.routing_hash.as_str())
            .eq(frame.routing_hashes.iter().map(String::as_str))
}

fn module_fact_evidence_is_valid(frame: &GameplayReactionFrame) -> bool {
    let mut hashes = frame
        .accepted_module_facts
        .iter()
        .map(|fact| gameplay_module_payload_hash(&fact.canonical_payload))
        .collect::<Vec<_>>();
    if frame
        .accepted_module_facts
        .iter()
        .zip(&hashes)
        .any(|(fact, computed)| &fact.payload_hash != computed)
    {
        return false;
    }
    hashes.sort();
    hashes == frame.accepted_module_fact_hashes
}

fn diagnostic_evidence_is_valid(frame: &GameplayReactionFrame) -> bool {
    frame
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .eq(frame.diagnostic_codes.iter().map(String::as_str))
}

pub fn run_verification_replay(
    expected: &GameplayReactionFrame,
    runner: &dyn GameplayVerificationReplayRunner,
) -> Result<GameplayVerificationReplayReceipt, GameplayModuleStateError> {
    let encoded = serde_json::to_vec(&expected.verification_replay_input())
        .map_err(|error| GameplayModuleStateError::InvalidSnapshot(error.to_string()))?;
    let recorded: GameplayVerificationReplayInput = serde_json::from_slice(&encoded)
        .map_err(|error| GameplayModuleStateError::InvalidSnapshot(error.to_string()))?;
    let actual = runner.rerun(&recorded)?;
    Ok(GameplayVerificationReplayReceipt {
        expected_frame_hash: expected.frame_hash.clone(),
        actual_frame_hash: actual.frame_hash.clone(),
        divergences: verify_reaction_frame(expected, &actual),
    })
}
