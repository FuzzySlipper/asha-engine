use crate::registry::{
    GameplayProposalOwnerRegistration, GameplayReadViewProviderRegistration,
    GameplayStateOwnerRegistration,
};
use protocol_game_extension::{
    GameplayEventSchemaDeclaration, GameplayModuleManifest, GameplayOrderingConstraint,
    GameplayReadViewProviderReadout, GameplayRegistryReadout, GameplayTopologyEdge,
};
use std::collections::{BTreeMap, BTreeSet};

type ModuleIndex<'a> = BTreeMap<String, &'a GameplayModuleManifest>;
type EventIndex<'a> = BTreeMap<String, (&'a str, &'a GameplayEventSchemaDeclaration)>;

pub(crate) fn topology_dump(
    modules: &ModuleIndex<'_>,
    events: &EventIndex<'_>,
    proposal_owners: &[GameplayProposalOwnerRegistration],
    view_providers: &[GameplayReadViewProviderRegistration],
    state_owners: &[GameplayStateOwnerRegistration],
) -> String {
    topology_dump_with_provenance(
        modules,
        events,
        proposal_owners,
        view_providers,
        state_owners,
        true,
    )
}

pub(crate) fn semantic_topology_dump(
    modules: &ModuleIndex<'_>,
    events: &EventIndex<'_>,
    proposal_owners: &[GameplayProposalOwnerRegistration],
    view_providers: &[GameplayReadViewProviderRegistration],
    state_owners: &[GameplayStateOwnerRegistration],
) -> String {
    topology_dump_with_provenance(
        modules,
        events,
        proposal_owners,
        view_providers,
        state_owners,
        false,
    )
}

fn topology_dump_with_provenance(
    modules: &ModuleIndex<'_>,
    events: &EventIndex<'_>,
    proposal_owners: &[GameplayProposalOwnerRegistration],
    view_providers: &[GameplayReadViewProviderRegistration],
    state_owners: &[GameplayStateOwnerRegistration],
    include_provenance: bool,
) -> String {
    let mut lines = Vec::new();
    for manifest in modules.values() {
        let module = &manifest.module_ref;
        if include_provenance {
            lines.push(format!(
                "module {} namespace={} version={} provider={} sdk={} contract={} artifact={} source={}",
                module.module_id,
                module.namespace,
                module.version,
                module.provider_id,
                module.sdk_hash,
                module.contract_hash,
                module.artifact_hash,
                manifest.source_hash,
            ));
        } else {
            lines.push(format!(
                "module {} namespace={} version={} provider={} sdk={} contract={}",
                module.module_id,
                module.namespace,
                module.version,
                module.provider_id,
                module.sdk_hash,
                module.contract_hash,
            ));
        }
        lines.push(format!(
            "budget module={} waves={} events={} proposals={} invocations={} payloadBytes={}",
            module.module_id,
            manifest.budget.max_waves,
            manifest.budget.max_events_per_root,
            manifest.budget.max_proposals_per_root,
            manifest.budget.max_invocations_per_root,
            manifest.budget.max_payload_bytes_per_root,
        ));
        let mut requirements = manifest.deterministic_requirements.clone();
        requirements.sort();
        lines.push(format!(
            "determinism module={} requirements={}",
            module.module_id,
            requirements.join(",")
        ));
        for invocation in &manifest.invocations {
            let mut declared_reads = invocation
                .read_requirements
                .iter()
                .map(|requirement| {
                    format!(
                        "{}={}@{}",
                        requirement.request_id,
                        requirement.view.key(),
                        requirement.view.schema_hash
                    )
                })
                .collect::<Vec<_>>();
            declared_reads.sort();
            lines.push(format!(
                "invocation {} module={} family={} input={}@{} output={}@{} reads={} maxOutputs={} maxPayloadBytes={}",
                invocation.invocation_id,
                module.module_id,
                invocation.family.as_str(),
                invocation.input_contract.key(),
                invocation.input_contract.schema_hash,
                invocation.output_contract.key(),
                invocation.output_contract.schema_hash,
                declared_reads.join(","),
                invocation.max_outputs,
                invocation.max_payload_bytes,
            ));
        }
        for subscription in &manifest.subscriptions {
            let mut tags = subscription.selector.required_tags.clone();
            tags.sort();
            lines.push(format!(
                "subscription {} module={} event={} invocation={} maxDeliveries={} source={} target={} scope={} tags={}",
                subscription.subscription_id,
                module.module_id,
                subscription.event.key(),
                subscription.invocation_id,
                subscription.max_deliveries_per_root,
                entity_ref(subscription.selector.source.as_ref()),
                entity_ref(subscription.selector.target.as_ref()),
                subscription.selector.scope.as_deref().unwrap_or("-"),
                tags.join(","),
            ));
        }
        for requirement in &manifest.read_views {
            let mut fields = requirement.fields.clone();
            fields.sort();
            let mut selectors = requirement
                .selector_capabilities
                .iter()
                .map(|selector| selector.as_str())
                .collect::<Vec<_>>();
            selectors.sort();
            lines.push(format!(
                "viewRequirement module={} view={} provider={} kind={} maxItems={} fields={} selectors={}",
                module.module_id,
                requirement.view.key(),
                requirement.provider_id,
                requirement.kind.as_str(),
                requirement.max_items,
                fields.join(","),
                selectors.join(","),
            ));
        }
        for declaration in &manifest.state_schemas {
            lines.push(format!(
                "stateDeclaration module={} schema={} owner={} provider={}",
                module.module_id,
                declaration.schema.key(),
                declaration.owner.owner_id,
                declaration.owner.provider_id,
            ));
        }
        for declaration in &manifest.fact_schemas {
            lines.push(format!(
                "factDeclaration module={} schema={} owner={} provider={}",
                module.module_id,
                declaration.schema.key(),
                declaration.owner.owner_id,
                declaration.owner.provider_id,
            ));
        }
        for declaration in &manifest.proposal_kinds {
            lines.push(format!(
                "proposalDeclaration module={} proposal={} schema={} owner={} provider={}",
                module.module_id,
                declaration.proposal.key(),
                declaration.proposal.schema_hash,
                declaration.owner.owner_id,
                declaration.owner.provider_id,
            ));
        }
    }
    for (key, (publisher, declaration)) in events {
        lines.push(format!(
            "event {key} publisher={publisher} schema={} codec={}",
            declaration.event.schema_hash, declaration.codec_id
        ));
    }
    for owner in proposal_owners {
        lines.push(format!(
            "proposal {} owner={} provider={}",
            owner.proposal.key(),
            owner.owner.owner_id,
            owner.owner.provider_id
        ));
    }
    for provider in view_providers {
        let mut fields = provider.fields.clone();
        fields.sort();
        let mut selectors = provider
            .selector_capabilities
            .iter()
            .map(|selector| selector.as_str())
            .collect::<Vec<_>>();
        selectors.sort();
        lines.push(format!(
            "view {} provider={} kind={} maxItems={} ordering={} fields={} selectors={}",
            provider.view.key(),
            provider.provider_id,
            provider.kind.as_str(),
            provider.max_items,
            provider.ordering,
            fields.join(","),
            selectors.join(",")
        ));
    }
    for owner in state_owners {
        lines.push(format!(
            "state {} owner={} provider={}",
            owner.schema.key(),
            owner.owner.owner_id,
            owner.owner.provider_id
        ));
    }
    let mut ordering: BTreeSet<(String, String)> = BTreeSet::new();
    for manifest in modules.values() {
        for edge in &manifest.ordering {
            ordering.insert((edge.before_module.clone(), edge.after_module.clone()));
        }
    }
    for (before, after) in ordering {
        lines.push(format!("order {before} -> {after}"));
    }
    lines.sort();
    let mut dump = lines.join("\n");
    dump.push('\n');
    dump
}

pub(crate) fn build_readout(
    digests: (&str, &str),
    dump: &str,
    modules: &ModuleIndex<'_>,
    events: &EventIndex<'_>,
    proposal_owners: &[GameplayProposalOwnerRegistration],
    view_providers: &[GameplayReadViewProviderRegistration],
    state_owners: &[GameplayStateOwnerRegistration],
) -> GameplayRegistryReadout {
    let (digest, semantic_compatibility_digest) = digests;
    let mut subscription_ids = Vec::new();
    let mut ordering = BTreeSet::new();
    let mut topology = Vec::new();
    for manifest in modules.values() {
        for declaration in &manifest.published_events {
            topology.push(GameplayTopologyEdge {
                kind: "publishes".into(),
                from: manifest.module_ref.module_id.clone(),
                to: declaration.event.key(),
                contract: Some(declaration.codec_id.clone()),
            });
        }
        for subscription in &manifest.subscriptions {
            subscription_ids.push(subscription.subscription_id.clone());
            topology.push(GameplayTopologyEdge {
                kind: "subscription".into(),
                from: manifest.module_ref.module_id.clone(),
                to: subscription.event.key(),
                contract: Some(subscription.invocation_id.clone()),
            });
        }
        for edge in &manifest.ordering {
            ordering.insert((edge.before_module.clone(), edge.after_module.clone()));
        }
        for requirement in &manifest.read_views {
            topology.push(GameplayTopologyEdge {
                kind: "reads".into(),
                from: manifest.module_ref.module_id.clone(),
                to: requirement.view.key(),
                contract: Some(requirement.provider_id.clone()),
            });
        }
    }
    subscription_ids.sort();
    for owner in proposal_owners {
        topology.push(GameplayTopologyEdge {
            kind: "proposalOwner".into(),
            from: owner.proposal.key(),
            to: owner.owner.owner_id.clone(),
            contract: Some(owner.owner.provider_id.clone()),
        });
    }
    for provider in view_providers {
        topology.push(GameplayTopologyEdge {
            kind: "viewProvider".into(),
            from: provider.view.key(),
            to: provider.provider_id.clone(),
            contract: None,
        });
    }
    for owner in state_owners {
        topology.push(GameplayTopologyEdge {
            kind: "stateOwner".into(),
            from: owner.schema.key(),
            to: owner.owner.owner_id.clone(),
            contract: Some(owner.owner.provider_id.clone()),
        });
    }
    for (before, after) in &ordering {
        topology.push(GameplayTopologyEdge {
            kind: "order".into(),
            from: before.clone(),
            to: after.clone(),
            contract: None,
        });
    }
    topology.sort_by(|a, b| {
        (&a.kind, &a.from, &a.to, &a.contract).cmp(&(&b.kind, &b.from, &b.to, &b.contract))
    });
    GameplayRegistryReadout {
        registry_digest: digest.to_string(),
        semantic_compatibility_digest: semantic_compatibility_digest.to_string(),
        artifact_provenance_digest: digest.to_string(),
        module_ids: modules.keys().cloned().collect(),
        event_kinds: events.keys().cloned().collect(),
        subscription_ids,
        proposal_owners: proposal_owners
            .iter()
            .map(|owner| format!("{}={}", owner.proposal.key(), owner.owner.owner_id))
            .collect(),
        read_view_providers: view_providers
            .iter()
            .map(|provider| format!("{}={}", provider.view.key(), provider.provider_id))
            .collect(),
        read_view_provider_details: view_providers
            .iter()
            .map(|provider| {
                let mut fields = provider.fields.clone();
                fields.sort();
                let mut selector_capabilities = provider.selector_capabilities.clone();
                selector_capabilities.sort();
                let provider_key = format!(
                    "{}|{}|{}|{}|{}|{}|{}",
                    provider.view.key(),
                    provider.provider_id,
                    provider.kind.as_str(),
                    provider.max_items,
                    provider.ordering,
                    fields.join(","),
                    selector_capabilities
                        .iter()
                        .map(|selector| selector.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                );
                GameplayReadViewProviderReadout {
                    view: provider.view.key(),
                    provider_id: provider.provider_id.clone(),
                    kind: provider.kind,
                    fields,
                    selector_capabilities,
                    max_items: provider.max_items,
                    ordering: provider.ordering.clone(),
                    provider_hash: stable_digest(&provider_key),
                }
            })
            .collect(),
        state_owners: state_owners
            .iter()
            .map(|owner| format!("{}={}", owner.schema.key(), owner.owner.owner_id))
            .collect(),
        ordering: ordering
            .into_iter()
            .map(|(before_module, after_module)| GameplayOrderingConstraint {
                before_module,
                after_module,
            })
            .collect(),
        topology,
        topology_dump: dump.to_string(),
    }
}

pub(crate) fn stable_digest(text: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn entity_ref(entity: Option<&protocol_game_extension::GameplayEntityRef>) -> String {
    entity
        .map(|entity| entity.entity.raw().to_string())
        .unwrap_or_else(|| "-".into())
}
