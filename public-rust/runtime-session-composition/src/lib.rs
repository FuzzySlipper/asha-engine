//! Public Rust facade for distinct statically composed RuntimeSession and
//! pre-runtime project-authoring cells.
//!
//! Downstream addons register concrete gameplay modules through the gameplay
//! SDK, then choose `StaticRuntimeSessionBuilder` for live authority or
//! `StaticProjectAuthoringBuilder` for immutable provider schema/codec
//! authority without ProjectBundle activation.

#![forbid(unsafe_code)]

pub use gameplay_runtime_host::{
    BootstrapResolutionContext, BundleArtifacts, EntityId, GameplayBindingEntityTargets,
    GameplayDecisionMoment, GameplayDecisionReceipt, GameplayDecisionStatus,
    GameplayOperationWorkspace, GameplayRuntimeDecisionOwner, GameplayRuntimeDecisionOwnerOutput,
    GameplayRuntimeDeclaredReadPlan, GameplayRuntimePrefabBootstrap, GameplayRuntimePrefabCatalog,
    GameplayRuntimePrefabOverride, GameplayRuntimePrefabPlacement,
    GameplayRuntimePrefabPlacementOrigin, GameplayRuntimePrefabTransform,
    GameplayRuntimeProjectInput, GameplayRuntimeSchedulerCommand,
    GameplayRuntimeSchedulerCommandReceipt, GameplayRuntimeSchedulerDefinition,
    GameplayRuntimeSchedulerReadout, GameplayRuntimeSchedulerRoutingReceipt,
    GameplayRuntimeSpatialEntity, GameplayTriggerDefinition, LoadPlan, LoadStep, RuntimeSessionId,
    SceneId, ScheduledActionId, ScheduledActionValidity, TickScheduledActionDraft,
    TriggerReconcileCause, GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
};
pub use runtime_bridge_api::*;
