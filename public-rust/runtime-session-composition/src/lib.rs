//! Public Rust facade for distinct statically composed RuntimeSession and
//! pre-runtime project-authoring cells.
//!
//! Downstream addons register concrete gameplay modules through the gameplay
//! SDK, then choose `DeferredRuntimeSessionBuilder` for manifest-driven atomic
//! runtime activation, or `StaticProjectAuthoringBuilder` for immutable
//! provider schema/codec authority without ProjectBundle activation. The older
//! `StaticRuntimeSessionBuilder` remains a compatibility surface while direct
//! consumer assembly is retired.

#![forbid(unsafe_code)]

pub use runtime_bridge_api::*;

/// Compatibility-only compiled-plan vocabulary for Engine fixtures and games
/// migrating to canonical `loadProject({ source })` admission. New downstream
/// boot code must not construct these values; store the typed meaning in the
/// ProjectBundle and let Rust admission derive the runtime plan.
pub mod compatibility {
    pub use gameplay_runtime_host::{
        BootstrapResolutionContext, BundleArtifacts, EntityId, GameplayBindingEntityTargets,
        GameplayDecisionMoment, GameplayDecisionReceipt, GameplayDecisionStatus,
        GameplayOperationWorkspace, GameplayRuntimeDecisionOwner,
        GameplayRuntimeDecisionOwnerOutput, GameplayRuntimeDeclaredReadPlan,
        GameplayRuntimePrefabBootstrap, GameplayRuntimePrefabCatalog,
        GameplayRuntimePrefabOverride, GameplayRuntimePrefabPlacement,
        GameplayRuntimePrefabPlacementOrigin, GameplayRuntimePrefabTransform,
        GameplayRuntimeProjectInput, GameplayRuntimeSchedulerCommand,
        GameplayRuntimeSchedulerCommandReceipt, GameplayRuntimeSchedulerDefinition,
        GameplayRuntimeSchedulerReadout, GameplayRuntimeSchedulerRoutingReceipt,
        GameplayRuntimeSpatialEntity, GameplayTriggerDefinition, LoadPlan, LoadStep,
        RuntimeSessionId, SceneId, ScheduledActionId, ScheduledActionValidity,
        TickScheduledActionDraft, TriggerReconcileCause,
        GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION,
    };
    pub use runtime_bridge_api::{
        StaticRuntimeSessionBuilder, StaticRuntimeSessionCompositionError,
    };
}
