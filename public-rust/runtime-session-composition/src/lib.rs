//! Public Rust facade for one statically composed native RuntimeSession cell.
//!
//! Downstream addons register concrete gameplay modules through the gameplay
//! SDK, build one cell here, and expose the returned bounded RuntimeBridge root.

#![forbid(unsafe_code)]

pub use gameplay_runtime_host::{
    BundleArtifacts, EntityId, GameplayBindingEntityTargets, GameplayDecisionMoment,
    GameplayDecisionReceipt, GameplayDecisionStatus, GameplayOperationWorkspace,
    GameplayRuntimeDecisionOwner, GameplayRuntimeDecisionOwnerOutput,
    GameplayRuntimeDeclaredReadPlan, GameplayRuntimePrefabBootstrap, GameplayRuntimeProjectInput,
    GameplayRuntimeSchedulerCommand, GameplayRuntimeSchedulerCommandReceipt,
    GameplayRuntimeSchedulerDefinition, GameplayRuntimeSchedulerReadout,
    GameplayRuntimeSchedulerRoutingReceipt, GameplayRuntimeSpatialEntity,
    GameplayTriggerDefinition, LoadPlan, LoadStep, RuntimeSessionId, SceneId, ScheduledActionId,
    ScheduledActionValidity, TickScheduledActionDraft, TriggerReconcileCause,
    GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
};
pub use runtime_bridge_api::*;
