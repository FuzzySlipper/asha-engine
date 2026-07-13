//! Bounded compatibility adapter for the legacy weapon-effect hook.
//!
//! Delete this adapter when #5634 replaces the manifest-only hook provider with
//! the real static gameplay-module provider. Until then, legacy module behavior
//! participates in the same Transform transaction and owner routing evidence.

use game_rule_extension::{
    GameExtensionDiagnostic, GameExtensionProposal, GameRuleModule, WeaponEffectHookRequest,
};
use protocol_game_extension::{
    GameplayCausationRef, GameplayContractRef, GameplayEmitterRef, GameplayEntityRef,
    GameplayExecutionBudget, GameplayInvocationDescriptor, GameplayInvocationFamily,
    GameplayModuleManifest, GameplayModuleRef, GameplayOwnerRef, GameplayProposalDeclaration,
    GameplayProposalEnvelope,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use svc_gameplay_fabric::{
    GameplayFabricRegistryBuilder, GameplayLinkedProvider, GameplayProposalOwnerRegistration,
    GameplayRegistryBuildError,
};

use crate::{
    FrozenGameplayViews, GameplayDecisionContinuations, GameplayDecisionMoment,
    GameplayDecisionOutput, GameplayDecisionOwner, GameplayDecisionReceipt,
    GameplayFabricCoordinator, GameplayInvocationCall, GameplayInvocationHost,
    GameplayInvocationOutput, GameplayOperationWorkspace, GameplayOwnerRoutingCall,
    GameplayOwnerRoutingOutput, GameplayRuntimeLimits, GameplayViewSource,
    GameplayWorkspaceTransform,
};

const COMPAT_TRANSFORM_INVOCATION: &str = "compat.weapon-effect.transform";
const COMPAT_COMBAT_OWNER: &str = "rule-lifecycle.combat";
const COMPAT_COMBAT_PROVIDER: &str = "provider.rule-lifecycle.combat";
const COMPAT_OWNER_REVISION: &str = "legacy-weapon-transform-owner.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyWeaponEffectWorkspace {
    request_id: String,
    base_damage: i64,
    proposal: Option<GameExtensionProposal>,
}

#[derive(Debug)]
pub enum LegacyWeaponEffectTransformError {
    ModuleRejected(GameExtensionDiagnostic),
    Registry(GameplayRegistryBuildError),
    Encode(String),
    DecisionRejected(Box<GameplayDecisionReceipt>),
    MissingAcceptedProposal,
}

impl core::fmt::Display for LegacyWeaponEffectTransformError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for LegacyWeaponEffectTransformError {}

#[derive(Debug)]
pub struct LegacyWeaponEffectTransformOutcome {
    pub proposal: GameExtensionProposal,
    pub damage_delta: i64,
    pub decision_receipt: GameplayDecisionReceipt,
}

/// Executes one legacy weapon hook as a common Transform decision. The returned
/// delta is still applied by the existing combat owner path; this adapter grants
/// the legacy module no direct mutation access.
pub fn run_legacy_weapon_effect_transform(
    module: &dyn GameRuleModule,
    request: &WeaponEffectHookRequest,
) -> Result<LegacyWeaponEffectTransformOutcome, LegacyWeaponEffectTransformError> {
    let registry =
        compatibility_registry(module).map_err(LegacyWeaponEffectTransformError::Registry)?;
    let workspace_contract = compatibility_workspace_contract();
    let initial_workspace = LegacyWeaponEffectWorkspace {
        request_id: request.request_id.clone(),
        base_damage: request.base_damage,
        proposal: None,
    };
    let initial_payload = serde_json::to_vec(&initial_workspace)
        .map_err(|error| LegacyWeaponEffectTransformError::Encode(error.to_string()))?;
    let workspace = GameplayOperationWorkspace::from_payload(workspace_contract, initial_payload);
    let operation = GameplayProposalEnvelope {
        proposal_id: format!("{}.compat-transform", request.request_id),
        proposal: compatibility_operation_contract(),
        tick: request.tick,
        root_sequence: 0,
        wave: 0,
        proposal_sequence: 0,
        emitter: GameplayEmitterRef::Module {
            module_id: compatibility_module_id(module),
        },
        causation: GameplayCausationRef {
            root_id: request.request_id.clone(),
            parent_event_id: None,
            decision_id: Some(request.request_id.clone()),
        },
        originating_event_id: None,
        source: Some(GameplayEntityRef {
            entity: request.source,
        }),
        targets: request
            .target
            .into_iter()
            .map(|entity| GameplayEntityRef { entity })
            .collect(),
        canonical_payload: workspace.canonical_payload.clone(),
        payload_hash: workspace.workspace_hash.clone(),
    };
    let moment = GameplayDecisionMoment {
        decision_id: request.request_id.clone(),
        operation,
        expected_owner_revision: COMPAT_OWNER_REVISION.to_owned(),
        workspace,
        resume_token: None,
    };
    let coordinator = GameplayFabricCoordinator::new(
        &registry,
        GameplayRuntimeLimits {
            max_waves: 1,
            max_events_per_root: 1,
            max_proposals_per_root: 1,
            max_invocations_per_root: 1,
            max_payload_bytes_per_root: 16_384,
        },
    );
    let host = LegacyWeaponTransformHost {
        module,
        request,
        module_diagnostic: RefCell::new(None),
    };
    let views = LegacyWeaponTransformViews {
        view_hash: request.input_hash.clone(),
    };
    let mut owner = LegacyWeaponTransformOwner {
        request,
        proposal: None,
        damage_delta: None,
    };
    let mut continuations = GameplayDecisionContinuations::default();
    let decision_receipt =
        coordinator.decide(moment, &mut continuations, &views, &host, &mut owner);
    if !decision_receipt.accepted() {
        if let Some(diagnostic) = host.module_diagnostic.into_inner() {
            return Err(LegacyWeaponEffectTransformError::ModuleRejected(diagnostic));
        }
        return Err(LegacyWeaponEffectTransformError::DecisionRejected(
            Box::new(decision_receipt),
        ));
    }
    let proposal = owner
        .proposal
        .ok_or(LegacyWeaponEffectTransformError::MissingAcceptedProposal)?;
    let damage_delta = owner
        .damage_delta
        .ok_or(LegacyWeaponEffectTransformError::MissingAcceptedProposal)?;
    Ok(LegacyWeaponEffectTransformOutcome {
        proposal,
        damage_delta,
        decision_receipt,
    })
}

struct LegacyWeaponTransformHost<'module> {
    module: &'module dyn GameRuleModule,
    request: &'module WeaponEffectHookRequest,
    module_diagnostic: RefCell<Option<GameExtensionDiagnostic>>,
}

impl GameplayInvocationHost for LegacyWeaponTransformHost<'_> {
    fn invoke(
        &self,
        call: &GameplayInvocationCall,
    ) -> Result<GameplayInvocationOutput, crate::GameplayHostError> {
        if call.family != GameplayInvocationFamily::Transform
            || call.invocation_id != COMPAT_TRANSFORM_INVOCATION
        {
            return Err(crate::GameplayHostError {
                code: "legacyWeaponTransform.unexpectedInvocation".to_owned(),
                message: "compatibility host only accepts its declared Transform invocation"
                    .to_owned(),
            });
        }
        let proposal = self
            .module
            .evaluate_weapon_effect(self.request)
            .map_err(|diagnostic| {
                self.module_diagnostic.replace(Some(diagnostic.clone()));
                crate::GameplayHostError {
                    code: diagnostic.code.as_str().to_owned(),
                    message: diagnostic.message,
                }
            })?;
        let input_workspace = match &call.input {
            crate::GameplayInvocationInput::Decision(moment) => &moment.workspace,
            crate::GameplayInvocationInput::Observe(_) => {
                return Err(crate::GameplayHostError {
                    code: "legacyWeaponTransform.invalidInput".to_owned(),
                    message: "legacy Transform requires a decision Workspace".to_owned(),
                });
            }
        };
        let next_workspace = LegacyWeaponEffectWorkspace {
            request_id: self.request.request_id.clone(),
            base_damage: self.request.base_damage,
            proposal: Some(proposal),
        };
        let canonical_payload =
            serde_json::to_vec(&next_workspace).map_err(|error| crate::GameplayHostError {
                code: "legacyWeaponTransform.encode".to_owned(),
                message: error.to_string(),
            })?;
        Ok(GameplayInvocationOutput {
            decision: Some(GameplayDecisionOutput::Transform(
                GameplayWorkspaceTransform {
                    input_workspace_hash: input_workspace.workspace_hash.clone(),
                    workspace: GameplayOperationWorkspace::from_payload(
                        input_workspace.contract.clone(),
                        canonical_payload,
                    ),
                },
            )),
            ..GameplayInvocationOutput::default()
        })
    }
}

struct LegacyWeaponTransformViews {
    view_hash: String,
}

impl GameplayViewSource for LegacyWeaponTransformViews {
    fn freeze(&self, _root_id: &str, epoch: u32) -> FrozenGameplayViews {
        FrozenGameplayViews {
            epoch: u64::from(epoch),
            view_hash: self.view_hash.clone(),
        }
    }
}

struct LegacyWeaponTransformOwner<'request> {
    request: &'request WeaponEffectHookRequest,
    proposal: Option<GameExtensionProposal>,
    damage_delta: Option<i64>,
}

impl GameplayDecisionOwner for LegacyWeaponTransformOwner<'_> {
    fn revision_hash(&self, _owner: &GameplayOwnerRef) -> String {
        COMPAT_OWNER_REVISION.to_owned()
    }

    fn route_precommit(&mut self, call: &GameplayOwnerRoutingCall) -> GameplayOwnerRoutingOutput {
        let workspace: LegacyWeaponEffectWorkspace =
            match serde_json::from_slice(&call.proposal.canonical_payload) {
                Ok(workspace) => workspace,
                Err(_) => return rejected_owner_output("workspaceDecode"),
            };
        let Some(proposal) = workspace.proposal else {
            return rejected_owner_output("missingProposal");
        };
        let GameExtensionProposal::DamageModifier {
            target,
            channel_id,
            amount_delta,
            proposal_hash,
            ..
        } = &proposal
        else {
            return rejected_owner_output("wrongProposalKind");
        };
        if Some(*target) != self.request.target {
            return rejected_owner_output("targetMismatch");
        }
        if channel_id != "combat.primary_fire.damage" {
            return rejected_owner_output("channelMismatch");
        }
        if !proposal_hash.starts_with("fnv1a64:") {
            return rejected_owner_output("proposalHashInvalid");
        }
        self.damage_delta = Some(*amount_delta);
        self.proposal = Some(proposal);
        GameplayOwnerRoutingOutput {
            accepted: true,
            fact_hashes: Vec::new(),
            events: Vec::new(),
            diagnostic_codes: Vec::new(),
        }
    }
}

fn rejected_owner_output(code: &str) -> GameplayOwnerRoutingOutput {
    GameplayOwnerRoutingOutput {
        accepted: false,
        fact_hashes: Vec::new(),
        events: Vec::new(),
        diagnostic_codes: vec![format!("legacyWeaponTransform.{code}")],
    }
}

fn compatibility_registry(
    module: &dyn GameRuleModule,
) -> Result<svc_gameplay_fabric::GameplayFabricRegistry, GameplayRegistryBuildError> {
    let manifest = compatibility_manifest(module);
    let owner = compatibility_owner();
    let mut builder = GameplayFabricRegistryBuilder::new();
    builder
        .register_linked_provider(GameplayLinkedProvider {
            provider_id: manifest.module_ref.provider_id.clone(),
            module_id: manifest.module_ref.module_id.clone(),
            version: manifest.module_ref.version.clone(),
            contract_hash: manifest.module_ref.contract_hash.clone(),
            artifact_hash: manifest.module_ref.artifact_hash.clone(),
            sdk_hash: manifest.module_ref.sdk_hash.clone(),
            source_hash: manifest.source_hash.clone(),
        })
        .register_proposal_owner(GameplayProposalOwnerRegistration {
            proposal: compatibility_operation_contract(),
            owner,
        })
        .register_module(manifest);
    builder.build()
}

fn compatibility_manifest(module: &dyn GameRuleModule) -> GameplayModuleManifest {
    let legacy = module.manifest();
    let owner = compatibility_owner();
    GameplayModuleManifest {
        module_ref: GameplayModuleRef {
            module_id: compatibility_module_id(module),
            namespace: compatibility_module_id(module),
            version: legacy.module_ref.version.clone(),
            sdk_hash: crate::gameplay_payload_hash(b"legacy-game-rule-extension-sdk-v0"),
            contract_hash: legacy.module_ref.contract_hash.clone(),
            artifact_hash: legacy.source_hash.clone(),
            provider_id: format!("provider.{}", compatibility_module_id(module)),
        },
        published_events: Vec::new(),
        subscriptions: Vec::new(),
        invocations: vec![GameplayInvocationDescriptor {
            invocation_id: COMPAT_TRANSFORM_INVOCATION.to_owned(),
            family: GameplayInvocationFamily::Transform,
            input_contract: compatibility_operation_contract(),
            output_contract: compatibility_workspace_contract(),
            read_requirements: Vec::new(),
            max_outputs: 1,
            max_payload_bytes: 16_384,
        }],
        read_views: Vec::new(),
        proposal_kinds: vec![GameplayProposalDeclaration {
            proposal: compatibility_operation_contract(),
            owner,
        }],
        state_schemas: Vec::new(),
        fact_schemas: Vec::new(),
        ordering: Vec::new(),
        budget: GameplayExecutionBudget {
            max_waves: 1,
            max_events_per_root: 1,
            max_proposals_per_root: 1,
            max_invocations_per_root: 1,
            max_payload_bytes_per_root: 16_384,
        },
        deterministic_requirements: legacy.deterministic_requirements.clone(),
        source_hash: legacy.source_hash.clone(),
    }
}

fn compatibility_module_id(module: &dyn GameRuleModule) -> String {
    format!(
        "compat.{}",
        module
            .manifest()
            .module_ref
            .module_id
            .chars()
            .map(|character| match character {
                'a'..='z' | '0'..='9' | '.' | '-' => character,
                'A'..='Z' => character.to_ascii_lowercase(),
                _ => '-',
            })
            .collect::<String>()
    )
}

fn compatibility_owner() -> GameplayOwnerRef {
    GameplayOwnerRef {
        owner_id: COMPAT_COMBAT_OWNER.to_owned(),
        provider_id: COMPAT_COMBAT_PROVIDER.to_owned(),
    }
}

fn contract(namespace: &str, name: &str, schema: &str) -> GameplayContractRef {
    GameplayContractRef {
        namespace: namespace.to_owned(),
        name: name.to_owned(),
        version: 1,
        schema_hash: crate::gameplay_payload_hash(schema.as_bytes()),
    }
}

fn compatibility_operation_contract() -> GameplayContractRef {
    contract(
        "asha.combat",
        "primary-fire",
        "compat-primary-fire-operation-v1",
    )
}

fn compatibility_workspace_contract() -> GameplayContractRef {
    contract(
        "asha.combat",
        "damage-workspace",
        "compat-damage-workspace-v1",
    )
}
