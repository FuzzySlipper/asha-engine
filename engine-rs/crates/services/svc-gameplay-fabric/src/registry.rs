use crate::codec::{
    GameplayCodecError, GameplayEventCodecRegistration, RegisteredCodec, TypedGameplayEventCodec,
};
use crate::topology::{build_readout, semantic_topology_dump, stable_digest, topology_dump};
use crate::validation::{
    budget_values, canonicalize_diagnostics, is_hash, is_namespace, is_stable_id, is_version,
    namespace_owns, push_diagnostic, validate_contract,
};
use protocol_game_extension::{
    GameplayCausationRef, GameplayContractRef, GameplayEmitterRef, GameplayEntityRef,
    GameplayEventEnvelope, GameplayEventPhase, GameplayInvocationFamily, GameplayModuleManifest,
    GameplayOwnerRef, GameplayProposalEnvelope, GameplayReadSelectorCapability,
    GameplayReadViewKind, GameplayRegistryDiagnostic, GameplayRegistryDiagnosticCode,
    GameplayRegistryReadout,
};
use std::any::{Any, TypeId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayLinkedProvider {
    pub provider_id: String,
    pub module_id: String,
    pub version: String,
    pub contract_hash: String,
    pub artifact_hash: String,
    pub sdk_hash: String,
    pub source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayProposalOwnerRegistration {
    pub proposal: GameplayContractRef,
    pub owner: GameplayOwnerRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayReadViewProviderRegistration {
    pub view: GameplayContractRef,
    pub provider_id: String,
    pub kind: GameplayReadViewKind,
    pub fields: Vec<String>,
    pub selector_capabilities: Vec<GameplayReadSelectorCapability>,
    pub max_items: u32,
    pub ordering: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayStateOwnerRegistration {
    pub schema: GameplayContractRef,
    pub owner: GameplayOwnerRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayEventMetadata {
    pub event_id: String,
    pub tick: u64,
    pub root_sequence: u64,
    pub wave: u32,
    pub event_sequence: u32,
    pub phase: GameplayEventPhase,
    pub emitter: GameplayEmitterRef,
    pub causation: GameplayCausationRef,
    pub source: Option<GameplayEntityRef>,
    pub subjects: Vec<GameplayEntityRef>,
    pub targets: Vec<GameplayEntityRef>,
    pub scope: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayProposalMetadata {
    pub proposal_id: String,
    pub tick: u64,
    pub root_sequence: u64,
    pub wave: u32,
    pub proposal_sequence: u32,
    pub emitter: GameplayEmitterRef,
    pub causation: GameplayCausationRef,
    pub originating_event_id: Option<String>,
    pub source: Option<GameplayEntityRef>,
    pub targets: Vec<GameplayEntityRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRegistryBuildError {
    pub diagnostics: Vec<GameplayRegistryDiagnostic>,
}

impl core::fmt::Display for GameplayRegistryBuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "gameplay-fabric registry rejected with {} diagnostic(s)",
            self.diagnostics.len()
        )
    }
}

impl std::error::Error for GameplayRegistryBuildError {}

#[derive(Default)]
pub struct GameplayFabricRegistryBuilder {
    manifests: Vec<GameplayModuleManifest>,
    providers: Vec<GameplayLinkedProvider>,
    codecs: Vec<RegisteredCodec>,
    proposal_owners: Vec<GameplayProposalOwnerRegistration>,
    read_view_providers: Vec<GameplayReadViewProviderRegistration>,
    state_owners: Vec<GameplayStateOwnerRegistration>,
}

impl GameplayFabricRegistryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_module(&mut self, manifest: GameplayModuleManifest) -> &mut Self {
        self.manifests.push(manifest);
        self
    }

    pub fn register_linked_provider(&mut self, provider: GameplayLinkedProvider) -> &mut Self {
        self.providers.push(provider);
        self
    }

    pub fn register_event_codec<T: 'static>(
        &mut self,
        codec: TypedGameplayEventCodec<T>,
    ) -> &mut Self {
        self.codecs.push(codec.into());
        self
    }

    pub fn register_event_codec_registration(
        &mut self,
        registration: GameplayEventCodecRegistration,
    ) -> &mut Self {
        self.codecs.push(registration.codec);
        self
    }

    pub fn register_proposal_owner(
        &mut self,
        owner: GameplayProposalOwnerRegistration,
    ) -> &mut Self {
        self.proposal_owners.push(owner);
        self
    }

    pub fn register_read_view_provider(
        &mut self,
        provider: GameplayReadViewProviderRegistration,
    ) -> &mut Self {
        self.read_view_providers.push(provider);
        self
    }

    pub fn register_state_owner(&mut self, owner: GameplayStateOwnerRegistration) -> &mut Self {
        self.state_owners.push(owner);
        self
    }

    pub fn build(mut self) -> Result<GameplayFabricRegistry, GameplayRegistryBuildError> {
        self.manifests
            .sort_by(|a, b| a.module_ref.module_id.cmp(&b.module_ref.module_id));
        self.providers
            .sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
        self.proposal_owners.sort_by(|a, b| {
            (a.proposal.key(), a.owner.owner_id.as_str())
                .cmp(&(b.proposal.key(), b.owner.owner_id.as_str()))
        });
        self.read_view_providers.sort_by(|a, b| {
            (a.view.key(), a.provider_id.as_str()).cmp(&(b.view.key(), b.provider_id.as_str()))
        });
        self.state_owners.sort_by(|a, b| {
            (a.schema.key(), a.owner.owner_id.as_str())
                .cmp(&(b.schema.key(), b.owner.owner_id.as_str()))
        });

        let mut diagnostics = Vec::new();
        let modules = collect_modules(&self.manifests, &mut diagnostics);
        let providers = collect_providers(&self.providers, &mut diagnostics);
        validate_namespaces(&modules, &mut diagnostics);
        validate_provider_links(&modules, &providers, &mut diagnostics);

        let events = collect_events(&modules, &mut diagnostics);
        let codecs = collect_codec_refs(&self.codecs, &mut diagnostics);
        validate_event_codecs(&events, &codecs, &mut diagnostics);
        validate_module_declarations(&modules, &events, &mut diagnostics);
        validate_proposal_codecs(&modules, &codecs, &mut diagnostics);
        validate_proposal_owners(&modules, &self.proposal_owners, &mut diagnostics);
        validate_read_view_providers(&modules, &self.read_view_providers, &mut diagnostics);
        validate_state_owners(&modules, &self.state_owners, &mut diagnostics);
        validate_ordering(&modules, &mut diagnostics);

        canonicalize_diagnostics(&mut diagnostics);
        if !diagnostics.is_empty() {
            return Err(GameplayRegistryBuildError { diagnostics });
        }

        let topology_dump = topology_dump(
            &modules,
            &events,
            &self.proposal_owners,
            &self.read_view_providers,
            &self.state_owners,
        );
        let registry_digest = stable_digest(&topology_dump);
        let semantic_topology_dump = semantic_topology_dump(
            &modules,
            &events,
            &self.proposal_owners,
            &self.read_view_providers,
            &self.state_owners,
        );
        let semantic_compatibility_digest = stable_digest(&semantic_topology_dump);
        let readout = build_readout(
            (&registry_digest, &semantic_compatibility_digest),
            &topology_dump,
            &modules,
            &events,
            &self.proposal_owners,
            &self.read_view_providers,
            &self.state_owners,
        );
        let module_order = ordered_module_ids(&modules);
        let proposal_owners = self
            .proposal_owners
            .iter()
            .map(|registration| (registration.proposal.key(), registration.owner.clone()))
            .collect();
        let registered_codecs = self
            .codecs
            .into_iter()
            .map(|codec| (codec.event.key(), codec))
            .collect();
        let state_owners = self
            .state_owners
            .iter()
            .map(|registration| (registration.schema.key(), registration.owner.clone()))
            .collect();
        let read_view_providers = self
            .read_view_providers
            .iter()
            .map(|registration| (registration.view.key(), registration.clone()))
            .collect();

        Ok(GameplayFabricRegistry {
            modules: self
                .manifests
                .into_iter()
                .map(|manifest| (manifest.module_ref.module_id.clone(), manifest))
                .collect(),
            codecs: registered_codecs,
            proposal_owners,
            state_owners,
            read_view_providers,
            module_order,
            registry_digest,
            semantic_compatibility_digest,
            topology_dump,
            readout,
        })
    }
}

pub struct GameplayFabricRegistry {
    modules: BTreeMap<String, GameplayModuleManifest>,
    codecs: BTreeMap<String, RegisteredCodec>,
    proposal_owners: BTreeMap<String, GameplayOwnerRef>,
    state_owners: BTreeMap<String, GameplayOwnerRef>,
    read_view_providers: BTreeMap<String, GameplayReadViewProviderRegistration>,
    module_order: Vec<String>,
    registry_digest: String,
    semantic_compatibility_digest: String,
    topology_dump: String,
    readout: GameplayRegistryReadout,
}

impl GameplayFabricRegistry {
    pub fn registry_digest(&self) -> &str {
        &self.registry_digest
    }

    pub fn semantic_compatibility_digest(&self) -> &str {
        &self.semantic_compatibility_digest
    }

    pub fn artifact_provenance_digest(&self) -> &str {
        &self.registry_digest
    }

    pub fn topology_dump(&self) -> &str {
        &self.topology_dump
    }

    pub fn readout(&self) -> &GameplayRegistryReadout {
        &self.readout
    }

    pub fn module(&self, module_id: &str) -> Option<&GameplayModuleManifest> {
        self.modules.get(module_id)
    }

    pub fn module_order(&self) -> &[String] {
        &self.module_order
    }

    pub fn proposal_owner(&self, proposal: &GameplayContractRef) -> Option<&GameplayOwnerRef> {
        self.proposal_owners.get(&proposal.key())
    }

    pub fn state_owner(&self, schema: &GameplayContractRef) -> Option<&GameplayOwnerRef> {
        self.state_owners.get(&schema.key())
    }

    pub fn read_view_provider(
        &self,
        view: &GameplayContractRef,
    ) -> Option<&GameplayReadViewProviderRegistration> {
        self.read_view_providers.get(&view.key())
    }

    pub fn module_declares_state(&self, module_id: &str, schema: &GameplayContractRef) -> bool {
        self.module(module_id).is_some_and(|manifest| {
            manifest
                .state_schemas
                .iter()
                .any(|declaration| declaration.schema == *schema)
        })
    }

    pub fn module_declares_fact(&self, module_id: &str, schema: &GameplayContractRef) -> bool {
        self.module(module_id).is_some_and(|manifest| {
            manifest
                .fact_schemas
                .iter()
                .any(|declaration| declaration.schema == *schema)
        })
    }

    pub fn module_declares_named_view(
        &self,
        module_id: &str,
        schema: &GameplayContractRef,
    ) -> bool {
        self.module(module_id).is_some_and(|manifest| {
            manifest.read_views.iter().any(|declaration| {
                declaration.view == *schema && declaration.kind == GameplayReadViewKind::ModuleNamed
            })
        })
    }

    pub fn event_is_declared(&self, event: &GameplayContractRef) -> bool {
        self.modules.values().any(|manifest| {
            manifest
                .published_events
                .iter()
                .any(|declaration| declaration.event == *event)
        })
    }

    pub fn module_publishes_event(&self, module_id: &str, event: &GameplayContractRef) -> bool {
        self.module(module_id).is_some_and(|manifest| {
            manifest
                .published_events
                .iter()
                .any(|declaration| declaration.event == *event)
        })
    }

    pub fn module_declares_proposal(
        &self,
        module_id: &str,
        proposal: &GameplayContractRef,
    ) -> bool {
        self.module(module_id).is_some_and(|manifest| {
            manifest
                .proposal_kinds
                .iter()
                .any(|declaration| declaration.proposal == *proposal)
        })
    }

    pub fn encode_event<T: 'static>(
        &self,
        event: &GameplayContractRef,
        payload: &T,
    ) -> Result<Vec<u8>, GameplayCodecError> {
        let key = event.key();
        let codec = self
            .codecs
            .get(&key)
            .filter(|codec| codec.event == *event)
            .ok_or_else(|| GameplayCodecError::UnknownContract {
                contract: key.clone(),
            })?;
        if codec.codec.payload_type_id() != TypeId::of::<T>() {
            return Err(GameplayCodecError::WrongPayloadType { contract: key });
        }
        codec.codec.encode_any(payload as &dyn Any)
    }

    pub fn event<T: 'static>(
        &self,
        event: &GameplayContractRef,
        payload: &T,
        metadata: GameplayEventMetadata,
    ) -> Result<GameplayEventEnvelope, GameplayCodecError> {
        let canonical_payload = self.encode_event(event, payload)?;
        let envelope = GameplayEventEnvelope {
            event_id: metadata.event_id,
            event: event.clone(),
            tick: metadata.tick,
            root_sequence: metadata.root_sequence,
            wave: metadata.wave,
            event_sequence: metadata.event_sequence,
            phase: metadata.phase,
            emitter: metadata.emitter,
            causation: metadata.causation,
            source: metadata.source,
            subjects: metadata.subjects,
            targets: metadata.targets,
            scope: metadata.scope,
            tags: metadata.tags,
            payload_hash: crate::gameplay_canonical_payload_hash(&canonical_payload),
            canonical_payload,
        };
        self.admit_event(&envelope)?;
        Ok(envelope)
    }

    pub fn proposal<T: 'static>(
        &self,
        proposal: &GameplayContractRef,
        payload: &T,
        metadata: GameplayProposalMetadata,
    ) -> Result<GameplayProposalEnvelope, GameplayCodecError> {
        let canonical_payload = self.encode_event(proposal, payload)?;
        let envelope = GameplayProposalEnvelope {
            proposal_id: metadata.proposal_id,
            proposal: proposal.clone(),
            tick: metadata.tick,
            root_sequence: metadata.root_sequence,
            wave: metadata.wave,
            proposal_sequence: metadata.proposal_sequence,
            emitter: metadata.emitter,
            causation: metadata.causation,
            originating_event_id: metadata.originating_event_id,
            source: metadata.source,
            targets: metadata.targets,
            payload_hash: crate::gameplay_canonical_payload_hash(&canonical_payload),
            canonical_payload,
        };
        self.admit_proposal(&envelope)?;
        Ok(envelope)
    }

    pub fn decode_event<T: 'static>(
        &self,
        event: &GameplayContractRef,
        bytes: &[u8],
    ) -> Result<T, GameplayCodecError> {
        let key = event.key();
        let codec = self
            .codecs
            .get(&key)
            .filter(|codec| codec.event == *event)
            .ok_or_else(|| GameplayCodecError::UnknownContract {
                contract: key.clone(),
            })?;
        if codec.codec.payload_type_id() != TypeId::of::<T>() {
            return Err(GameplayCodecError::WrongPayloadType { contract: key });
        }
        codec
            .codec
            .decode_any(bytes)?
            .downcast::<T>()
            .map(|payload| *payload)
            .map_err(|_| GameplayCodecError::WrongPayloadType { contract: key })
    }

    pub fn admit_event(&self, event: &GameplayEventEnvelope) -> Result<(), GameplayCodecError> {
        self.admit_payload(&event.event, &event.canonical_payload, &event.payload_hash)
    }

    pub fn admit_proposal(
        &self,
        proposal: &GameplayProposalEnvelope,
    ) -> Result<(), GameplayCodecError> {
        self.admit_payload(
            &proposal.proposal,
            &proposal.canonical_payload,
            &proposal.payload_hash,
        )
    }

    pub fn admit_payload(
        &self,
        contract: &GameplayContractRef,
        bytes: &[u8],
        payload_hash: &str,
    ) -> Result<(), GameplayCodecError> {
        let key = contract.key();
        let codec = self
            .codecs
            .get(&key)
            .filter(|codec| codec.event == *contract)
            .ok_or_else(|| GameplayCodecError::UnknownContract {
                contract: key.clone(),
            })?;
        if !codec.codec.descriptor_matches_contract() {
            return Err(GameplayCodecError::SchemaDescriptorMismatch { contract: key });
        }
        let canonical = codec.codec.canonicalize(bytes)?;
        if canonical != bytes {
            return Err(GameplayCodecError::NonCanonical { contract: key });
        }
        if crate::gameplay_canonical_payload_hash(&canonical) != payload_hash {
            return Err(GameplayCodecError::PayloadHashMismatch { contract: key });
        }
        Ok(())
    }
}

fn collect_modules<'a>(
    manifests: &'a [GameplayModuleManifest],
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) -> BTreeMap<String, &'a GameplayModuleManifest> {
    let mut modules = BTreeMap::new();
    for (index, manifest) in manifests.iter().enumerate() {
        let module = &manifest.module_ref;
        let path = format!("modules[{index}]");
        for (field, value) in [
            ("moduleId", module.module_id.as_str()),
            ("providerId", module.provider_id.as_str()),
        ] {
            if !is_stable_id(value) {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::InvalidIdentifier,
                    format!("{path}.{field}"),
                    format!("`{value}` is not a dot-scoped lowercase identifier"),
                );
            }
        }
        if !is_namespace(&module.namespace) {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::InvalidNamespace,
                format!("{path}.namespace"),
                format!("invalid module namespace `{}`", module.namespace),
            );
        }
        if !is_version(&module.version) {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::InvalidIdentifier,
                format!("{path}.version"),
                "module version must be a non-empty whitespace-free value",
            );
        }
        if modules.insert(module.module_id.clone(), manifest).is_some() {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::DuplicateModule,
                format!("{path}.moduleId"),
                format!("duplicate module `{}`", module.module_id),
            );
        }
        if [
            module.sdk_hash.as_str(),
            module.contract_hash.as_str(),
            module.artifact_hash.as_str(),
            manifest.source_hash.as_str(),
        ]
        .into_iter()
        .any(|hash| !is_hash(hash))
        {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::InvalidIdentifier,
                format!("{path}.hashes"),
                "module hashes must be non-empty algorithm-prefixed values",
            );
        }
        if budget_values(manifest).into_iter().any(|value| value == 0) {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::InvalidBudget,
                format!("{path}.budget"),
                "all gameplay execution budgets must be greater than zero",
            );
        }
    }
    modules
}

fn collect_providers<'a>(
    providers: &'a [GameplayLinkedProvider],
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) -> BTreeMap<String, &'a GameplayLinkedProvider> {
    let mut result = BTreeMap::new();
    for (index, provider) in providers.iter().enumerate() {
        if !is_stable_id(&provider.provider_id)
            || !is_stable_id(&provider.module_id)
            || !is_version(&provider.version)
            || !is_hash(&provider.contract_hash)
            || !is_hash(&provider.artifact_hash)
        {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::InvalidIdentifier,
                format!("providers[{index}]"),
                "linked provider identity, version, or hashes are invalid",
            );
        }
        if result
            .insert(provider.provider_id.clone(), provider)
            .is_some()
        {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::DuplicateProvider,
                format!("providers[{index}].providerId"),
                format!("duplicate linked provider `{}`", provider.provider_id),
            );
        }
    }
    result
}

fn validate_namespaces(
    modules: &BTreeMap<String, &GameplayModuleManifest>,
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) {
    let values: Vec<_> = modules.values().collect();
    for (index, left) in values.iter().enumerate() {
        for right in values.iter().skip(index + 1) {
            let a = left.module_ref.namespace.as_str();
            let b = right.module_ref.namespace.as_str();
            if namespace_owns(a, b) || namespace_owns(b, a) {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::OverlappingNamespace,
                    "modules.namespace",
                    format!("module namespaces `{a}` and `{b}` overlap"),
                );
            }
        }
    }
}

fn validate_provider_links(
    modules: &BTreeMap<String, &GameplayModuleManifest>,
    providers: &BTreeMap<String, &GameplayLinkedProvider>,
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) {
    for manifest in modules.values() {
        let module = &manifest.module_ref;
        let path = format!("modules.{}.provider", module.module_id);
        let Some(provider) = providers.get(&module.provider_id) else {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::MissingProvider,
                path,
                format!("linked provider `{}` is missing", module.provider_id),
            );
            continue;
        };
        if provider.module_id != module.module_id
            || provider.version != module.version
            || provider.contract_hash != module.contract_hash
            || provider.artifact_hash != module.artifact_hash
            || provider.sdk_hash != module.sdk_hash
            || provider.source_hash != manifest.source_hash
        {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::ProviderManifestMismatch,
                path,
                "linked provider identity/hash does not match the module manifest",
            );
        }
    }
}

fn collect_events<'a>(
    modules: &BTreeMap<String, &'a GameplayModuleManifest>,
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) -> BTreeMap<
    String,
    (
        &'a str,
        &'a protocol_game_extension::GameplayEventSchemaDeclaration,
    ),
> {
    let mut events = BTreeMap::new();
    for manifest in modules.values() {
        for declaration in &manifest.published_events {
            let key = declaration.event.key();
            validate_contract(&declaration.event, "publishedEvents", diagnostics);
            if !namespace_owns(&manifest.module_ref.namespace, &declaration.event.namespace) {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::ForeignNamespaceWrite,
                    format!("modules.{}.publishedEvents", manifest.module_ref.module_id),
                    format!("module cannot publish foreign event `{key}`"),
                );
            }
            if let Some((_, prior)) = events.insert(
                key.clone(),
                (manifest.module_ref.module_id.as_str(), declaration),
            ) {
                let code = if prior.event.schema_hash == declaration.event.schema_hash {
                    GameplayRegistryDiagnosticCode::DuplicateEventKind
                } else {
                    GameplayRegistryDiagnosticCode::SchemaHashMismatch
                };
                push_diagnostic(
                    diagnostics,
                    code,
                    "publishedEvents",
                    format!("event kind `{key}` is declared more than once"),
                );
            }
        }
    }
    events
}

fn collect_codec_refs<'a>(
    codecs: &'a [RegisteredCodec],
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) -> BTreeMap<String, Vec<&'a RegisteredCodec>> {
    let mut result: BTreeMap<String, Vec<&RegisteredCodec>> = BTreeMap::new();
    for codec in codecs {
        result.entry(codec.event.key()).or_default().push(codec);
    }
    for (key, values) in &result {
        if values.len() > 1 {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::DuplicateCodec,
                "codecs",
                format!("event `{key}` has more than one codec"),
            );
        }
    }
    result
}

fn validate_event_codecs(
    events: &BTreeMap<
        String,
        (
            &str,
            &protocol_game_extension::GameplayEventSchemaDeclaration,
        ),
    >,
    codecs: &BTreeMap<String, Vec<&RegisteredCodec>>,
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) {
    for (key, (_, declaration)) in events {
        let Some(values) = codecs.get(key) else {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::MissingCodec,
                "codecs",
                format!("published event `{key}` has no registered codec"),
            );
            continue;
        };
        if let Some(codec) = values.first() {
            if codec.event != declaration.event || codec.codec_id != declaration.codec_id {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::SchemaHashMismatch,
                    "codecs",
                    format!("codec declaration for `{key}` does not match the manifest"),
                );
            }
            if codec.codec.declaration() != *declaration {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::SchemaHashMismatch,
                    "codecs",
                    format!("typed codec for `{key}` carries different schema metadata"),
                );
            }
            if !codec.codec.descriptor_matches_contract() {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::SchemaHashMismatch,
                    "codecs",
                    format!(
                        "typed codec for `{key}` is not derived from its canonical schema descriptor"
                    ),
                );
            }
        }
    }
}

fn validate_proposal_codecs(
    modules: &BTreeMap<String, &GameplayModuleManifest>,
    codecs: &BTreeMap<String, Vec<&RegisteredCodec>>,
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) {
    for manifest in modules.values() {
        for declaration in &manifest.proposal_kinds {
            let key = declaration.proposal.key();
            let Some(values) = codecs.get(&key) else {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::MissingCodec,
                    format!("modules.{}.proposalKinds", manifest.module_ref.module_id),
                    format!("proposal `{key}` has no registered canonical codec"),
                );
                continue;
            };
            if let Some(codec) = values.first() {
                if codec.event != declaration.proposal || !codec.codec.descriptor_matches_contract()
                {
                    push_diagnostic(
                        diagnostics,
                        GameplayRegistryDiagnosticCode::SchemaHashMismatch,
                        format!("modules.{}.proposalKinds", manifest.module_ref.module_id),
                        format!(
                            "proposal codec for `{key}` does not match its canonical schema descriptor"
                        ),
                    );
                }
            }
        }
    }
}

fn validate_module_declarations(
    modules: &BTreeMap<String, &GameplayModuleManifest>,
    events: &BTreeMap<
        String,
        (
            &str,
            &protocol_game_extension::GameplayEventSchemaDeclaration,
        ),
    >,
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) {
    let mut subscription_ids = BTreeSet::new();
    for manifest in modules.values() {
        let module_id = manifest.module_ref.module_id.as_str();
        let mut invocations = BTreeMap::new();
        for invocation in &manifest.invocations {
            if invocations
                .insert(invocation.invocation_id.as_str(), invocation)
                .is_some()
            {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::DuplicateInvocation,
                    format!("modules.{module_id}.invocations"),
                    format!("duplicate invocation `{}`", invocation.invocation_id),
                );
            }
            validate_contract(
                &invocation.input_contract,
                "invocations.inputContract",
                diagnostics,
            );
            validate_contract(
                &invocation.output_contract,
                "invocations.outputContract",
                diagnostics,
            );
            if invocation.max_outputs == 0 || invocation.max_payload_bytes == 0 {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::InvalidBudget,
                    format!(
                        "modules.{module_id}.invocations.{}",
                        invocation.invocation_id
                    ),
                    "invocation budgets must be greater than zero",
                );
            }
            let mut request_ids = BTreeSet::new();
            for requirement in &invocation.read_requirements {
                validate_contract(
                    &requirement.view,
                    "invocations.readRequirements.view",
                    diagnostics,
                );
                if requirement.request_id.trim().is_empty()
                    || !request_ids.insert(requirement.request_id.as_str())
                {
                    push_diagnostic(
                        diagnostics,
                        GameplayRegistryDiagnosticCode::DuplicateInvocationRead,
                        format!(
                            "modules.{module_id}.invocations.{}.readRequirements",
                            invocation.invocation_id
                        ),
                        format!(
                            "read request `{}` must be nonempty and unique within its invocation",
                            requirement.request_id
                        ),
                    );
                }
                if !manifest
                    .read_views
                    .iter()
                    .any(|declared| declared.view == requirement.view)
                {
                    push_diagnostic(
                        diagnostics,
                        GameplayRegistryDiagnosticCode::MissingInvocationReadView,
                        format!(
                            "modules.{module_id}.invocations.{}.readRequirements.{}",
                            invocation.invocation_id, requirement.request_id
                        ),
                        format!(
                            "invocation read `{}` is not declared by the module",
                            requirement.view.key()
                        ),
                    );
                }
            }
        }
        for subscription in &manifest.subscriptions {
            let key = subscription.event.key();
            if !subscription_ids.insert(subscription.subscription_id.as_str()) {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::DuplicateSubscription,
                    format!("modules.{module_id}.subscriptions"),
                    format!("duplicate subscription `{}`", subscription.subscription_id),
                );
            }
            match events.get(&key) {
                None => push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::UnknownSubscription,
                    format!(
                        "modules.{module_id}.subscriptions.{}",
                        subscription.subscription_id
                    ),
                    format!("subscription references unknown event `{key}`"),
                ),
                Some((_, declaration)) if declaration.event != subscription.event => {
                    push_diagnostic(
                        diagnostics,
                        GameplayRegistryDiagnosticCode::SchemaHashMismatch,
                        format!(
                            "modules.{module_id}.subscriptions.{}",
                            subscription.subscription_id
                        ),
                        format!("subscription schema does not match `{key}`"),
                    )
                }
                Some(_) => {}
            }
            let Some(invocation) = invocations.get(subscription.invocation_id.as_str()) else {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::MissingInvocation,
                    format!(
                        "modules.{module_id}.subscriptions.{}",
                        subscription.subscription_id
                    ),
                    format!("unknown invocation `{}`", subscription.invocation_id),
                );
                continue;
            };
            if invocation.family != GameplayInvocationFamily::Observe {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::InvalidSubscriptionInvocation,
                    format!(
                        "modules.{module_id}.subscriptions.{}",
                        subscription.subscription_id
                    ),
                    "event subscriptions must target an Observe invocation",
                );
            }
            if invocation.input_contract != subscription.event {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::InvalidSubscriptionInvocation,
                    format!(
                        "modules.{module_id}.subscriptions.{}",
                        subscription.subscription_id
                    ),
                    "subscription event must match the Observe invocation input contract",
                );
            }
            if subscription.max_deliveries_per_root == 0 {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::InvalidBudget,
                    format!(
                        "modules.{module_id}.subscriptions.{}",
                        subscription.subscription_id
                    ),
                    "subscription delivery budget must be greater than zero",
                );
            }
        }
        for owned in manifest
            .state_schemas
            .iter()
            .chain(manifest.fact_schemas.iter())
        {
            validate_contract(&owned.schema, "ownedSchemas", diagnostics);
            if !namespace_owns(&manifest.module_ref.namespace, &owned.schema.namespace) {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::ForeignNamespaceWrite,
                    format!("modules.{module_id}.ownedSchemas"),
                    format!("module cannot own foreign schema `{}`", owned.schema.key()),
                );
            }
        }
    }
}

fn validate_proposal_owners(
    modules: &BTreeMap<String, &GameplayModuleManifest>,
    registrations: &[GameplayProposalOwnerRegistration],
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) {
    let indexed = index_proposal_owners(registrations);
    for manifest in modules.values() {
        for declaration in &manifest.proposal_kinds {
            validate_contract(&declaration.proposal, "proposalKinds", diagnostics);
            validate_exact_owner(
                &declaration.proposal,
                &declaration.owner,
                indexed.get(&declaration.proposal.key()),
                OwnerKind::Proposal,
                diagnostics,
            );
        }
    }
}

fn validate_read_view_providers(
    modules: &BTreeMap<String, &GameplayModuleManifest>,
    registrations: &[GameplayReadViewProviderRegistration],
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) {
    let mut indexed: BTreeMap<String, Vec<&GameplayReadViewProviderRegistration>> = BTreeMap::new();
    for registration in registrations {
        validate_contract(&registration.view, "readViewProviders", diagnostics);
        if !is_stable_id(&registration.provider_id) {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::InvalidIdentifier,
                "readViewProviders",
                format!(
                    "read view `{}` has invalid provider id `{}`",
                    registration.view.key(),
                    registration.provider_id
                ),
            );
        }
        let unique_fields = registration.fields.iter().collect::<BTreeSet<_>>();
        let unique_selectors = registration
            .selector_capabilities
            .iter()
            .collect::<BTreeSet<_>>();
        if unique_fields.len() != registration.fields.len()
            || unique_selectors.len() != registration.selector_capabilities.len()
        {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::ReadViewProviderMismatch,
                "readViewProviders",
                format!(
                    "read view `{}` provider metadata contains duplicates",
                    registration.view.key()
                ),
            );
        }
        if registration.max_items == 0 || registration.ordering.trim().is_empty() {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::InvalidBudget,
                "readViewProviders",
                format!(
                    "read view `{}` provider has invalid bounds or ordering",
                    registration.view.key()
                ),
            );
        }
        indexed
            .entry(registration.view.key())
            .or_default()
            .push(registration);
    }
    for (key, providers) in &indexed {
        if providers.len() != 1 {
            push_diagnostic(
                diagnostics,
                GameplayRegistryDiagnosticCode::MultipleReadViewProviders,
                "readViewProviders",
                format!("read view `{key}` must have exactly one provider"),
            );
        }
    }
    for manifest in modules.values() {
        for requirement in &manifest.read_views {
            let key = requirement.view.key();
            validate_contract(&requirement.view, "readViews", diagnostics);
            let Some(values) = indexed.get(&key) else {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::MissingReadViewProvider,
                    "readViews",
                    format!("read view `{key}` has no provider"),
                );
                continue;
            };
            if values.len() != 1 {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::MultipleReadViewProviders,
                    "readViews",
                    format!("read view `{key}` must have exactly one provider"),
                );
                continue;
            }
            let provider = values[0];
            if provider.view != requirement.view || provider.provider_id != requirement.provider_id
            {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::ReadViewProviderMismatch,
                    "readViews",
                    format!("read view provider does not match `{key}`"),
                );
            }
            if provider.kind != requirement.kind {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::ReadViewKindMismatch,
                    "readViews",
                    format!(
                        "provider for `{key}` supplies `{}` rather than `{}`",
                        provider.kind.as_str(),
                        requirement.kind.as_str()
                    ),
                );
            }
            let fields: BTreeSet<&str> = provider.fields.iter().map(String::as_str).collect();
            for field in &requirement.fields {
                if !fields.contains(field.as_str()) {
                    push_diagnostic(
                        diagnostics,
                        GameplayRegistryDiagnosticCode::MissingReadViewField,
                        "readViews",
                        format!("provider for `{key}` does not supply field `{field}`"),
                    );
                }
            }
            let selectors: BTreeSet<_> = provider.selector_capabilities.iter().copied().collect();
            for selector in &requirement.selector_capabilities {
                if !selectors.contains(selector) {
                    push_diagnostic(
                        diagnostics,
                        GameplayRegistryDiagnosticCode::MissingReadViewSelector,
                        "readViews",
                        format!(
                            "provider for `{key}` does not support selector `{}`",
                            selector.as_str()
                        ),
                    );
                }
            }
            if requirement.max_items == 0
                || provider.max_items == 0
                || requirement.max_items > provider.max_items
                || provider.ordering.trim().is_empty()
            {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::InvalidBudget,
                    "readViews",
                    format!("read view `{key}` has invalid provider or consumer bounds"),
                );
            }
        }
    }
}

fn validate_state_owners(
    modules: &BTreeMap<String, &GameplayModuleManifest>,
    registrations: &[GameplayStateOwnerRegistration],
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) {
    let mut indexed: BTreeMap<String, Vec<&GameplayStateOwnerRegistration>> = BTreeMap::new();
    for registration in registrations {
        indexed
            .entry(registration.schema.key())
            .or_default()
            .push(registration);
    }
    for manifest in modules.values() {
        for declaration in manifest
            .state_schemas
            .iter()
            .chain(manifest.fact_schemas.iter())
        {
            validate_exact_owner(
                &declaration.schema,
                &declaration.owner,
                indexed.get(&declaration.schema.key()),
                OwnerKind::State,
                diagnostics,
            );
        }
    }
}

#[derive(Clone, Copy)]
enum OwnerKind {
    Proposal,
    State,
}

fn validate_exact_owner<T: OwnerRegistration>(
    contract: &GameplayContractRef,
    expected: &GameplayOwnerRef,
    registrations: Option<&Vec<&T>>,
    kind: OwnerKind,
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) {
    let key = contract.key();
    let (missing, multiple, mismatch, path) = match kind {
        OwnerKind::Proposal => (
            GameplayRegistryDiagnosticCode::MissingProposalOwner,
            GameplayRegistryDiagnosticCode::MultipleProposalOwners,
            GameplayRegistryDiagnosticCode::ProposalOwnerMismatch,
            "proposalOwners",
        ),
        OwnerKind::State => (
            GameplayRegistryDiagnosticCode::MissingStateOwner,
            GameplayRegistryDiagnosticCode::MultipleStateOwners,
            GameplayRegistryDiagnosticCode::StateOwnerMismatch,
            "stateOwners",
        ),
    };
    let Some(values) = registrations else {
        push_diagnostic(diagnostics, missing, path, format!("`{key}` has no owner"));
        return;
    };
    if values.len() != 1 {
        push_diagnostic(
            diagnostics,
            multiple,
            path,
            format!("`{key}` must have exactly one owner"),
        );
        return;
    }
    let registration = values[0];
    if registration.contract() != contract || registration.owner() != expected {
        push_diagnostic(
            diagnostics,
            mismatch,
            path,
            format!("registered owner does not match declaration for `{key}`"),
        );
    }
}

trait OwnerRegistration {
    fn contract(&self) -> &GameplayContractRef;
    fn owner(&self) -> &GameplayOwnerRef;
}

impl OwnerRegistration for GameplayProposalOwnerRegistration {
    fn contract(&self) -> &GameplayContractRef {
        &self.proposal
    }
    fn owner(&self) -> &GameplayOwnerRef {
        &self.owner
    }
}

impl OwnerRegistration for GameplayStateOwnerRegistration {
    fn contract(&self) -> &GameplayContractRef {
        &self.schema
    }
    fn owner(&self) -> &GameplayOwnerRef {
        &self.owner
    }
}

fn index_proposal_owners(
    registrations: &[GameplayProposalOwnerRegistration],
) -> BTreeMap<String, Vec<&GameplayProposalOwnerRegistration>> {
    let mut indexed: BTreeMap<String, Vec<&GameplayProposalOwnerRegistration>> = BTreeMap::new();
    for registration in registrations {
        indexed
            .entry(registration.proposal.key())
            .or_default()
            .push(registration);
    }
    indexed
}

fn validate_ordering(
    modules: &BTreeMap<String, &GameplayModuleManifest>,
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) {
    let mut edges = BTreeSet::new();
    for manifest in modules.values() {
        for constraint in &manifest.ordering {
            if !modules.contains_key(&constraint.before_module)
                || !modules.contains_key(&constraint.after_module)
            {
                push_diagnostic(
                    diagnostics,
                    GameplayRegistryDiagnosticCode::UnknownOrderingTarget,
                    format!("modules.{}.ordering", manifest.module_ref.module_id),
                    format!(
                        "ordering edge `{}` -> `{}` names an unknown module",
                        constraint.before_module, constraint.after_module
                    ),
                );
                continue;
            }
            edges.insert((
                constraint.before_module.clone(),
                constraint.after_module.clone(),
            ));
        }
    }
    let mut indegree: BTreeMap<String, usize> =
        modules.keys().map(|module| (module.clone(), 0)).collect();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (before, after) in edges {
        outgoing.entry(before).or_default().push(after.clone());
        *indegree.entry(after).or_default() += 1;
    }
    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(module, _)| module.clone())
        .collect();
    let mut visited = 0;
    while let Some(module) = ready.pop_first() {
        visited += 1;
        for after in outgoing.get(&module).into_iter().flatten() {
            let count = indegree.get_mut(after).expect("known ordering node");
            *count -= 1;
            if *count == 0 {
                ready.insert(after.clone());
            }
        }
    }
    if visited != modules.len() {
        push_diagnostic(
            diagnostics,
            GameplayRegistryDiagnosticCode::OrderingCycle,
            "ordering",
            "module ordering constraints contain a cycle",
        );
    }
}

fn ordered_module_ids(modules: &BTreeMap<String, &GameplayModuleManifest>) -> Vec<String> {
    let mut indegree: BTreeMap<String, usize> =
        modules.keys().map(|module| (module.clone(), 0)).collect();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let edges: BTreeSet<(String, String)> = modules
        .values()
        .flat_map(|manifest| manifest.ordering.iter())
        .map(|edge| (edge.before_module.clone(), edge.after_module.clone()))
        .collect();
    for (before, after) in edges {
        outgoing.entry(before).or_default().push(after.clone());
        *indegree.entry(after).or_default() += 1;
    }
    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(module, _)| module.clone())
        .collect();
    let mut ordered = Vec::with_capacity(modules.len());
    while let Some(module) = ready.pop_first() {
        if let Some(after_modules) = outgoing.get(&module) {
            for after in after_modules {
                let count = indegree.get_mut(after).expect("validated ordering target");
                *count -= 1;
                if *count == 0 {
                    ready.insert(after.clone());
                }
            }
        }
        ordered.push(module);
    }
    debug_assert_eq!(
        ordered.len(),
        modules.len(),
        "ordering validated before use"
    );
    ordered
}
