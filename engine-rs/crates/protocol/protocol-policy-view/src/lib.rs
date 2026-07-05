//! World-layer border shapes for the constrained policy boundary (#2391, #2392).
//!
//! # Lane
//!
//! `contract-steward` — owns the border between the Rust authority core and the
//! constrained TypeScript policy host for the *world layer* (entities, transforms,
//! scene source, asset status). Depends on `core-ids` only; it is pure data with
//! no behavior.
//!
//! # Border ownership
//!
//! A policy lives in TypeScript. For the world layer it is handed a read-only
//! [`PolicyWorldView`] and may hand back only a proposed [`PolicyWorldCommand`].
//! The authority core (`svc-policy-view`) validates that proposal into an accepted
//! [`PolicyWorldEvent`] or a classified [`PolicyWorldRejection`]. Those shapes —
//! view, command, event, rejection, outcome — are this crate's whole job, and they
//! are what `protocol-codegen` turns into TypeScript.
//!
//! # Deliberate redactions (design gate)
//!
//! - No renderer handles, no collider geometry, no asset bytes — a policy gets
//!   identity, lifecycle, transform, source, labels, and asset *status* only.
//! - Tombstoned entities and `DiagnosticTooling`-sourced entities are omitted by
//!   the projector: they are never policy truth.
//!
//! # Forbidden convenience logic
//!
//! No projection, no validation, no apply. The projector and validator live in
//! `svc-policy-view`; these types are inert so the TS and Rust sides cannot
//! disagree about shape.

#![forbid(unsafe_code)]

use core_ids::{EntityId, TagId};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

fn serialize_entity_id<S>(id: &EntityId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(id.raw())
}

fn deserialize_entity_id<'de, D>(deserializer: D) -> Result<EntityId, D::Error>
where
    D: Deserializer<'de>,
{
    u64::deserialize(deserializer).map(EntityId::new)
}

fn serialize_tag_id<S>(id: &TagId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(id.raw())
}

fn deserialize_tag_id<'de, D>(deserializer: D) -> Result<TagId, D::Error>
where
    D: Deserializer<'de>,
{
    u64::deserialize(deserializer).map(TagId::new)
}

fn serialize_tag_ids<S>(ids: &[TagId], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ids.iter()
        .map(|id| id.raw())
        .collect::<Vec<_>>()
        .serialize(serializer)
}

fn deserialize_tag_ids<'de, D>(deserializer: D) -> Result<Vec<TagId>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<u64>::deserialize(deserializer).map(|ids| ids.into_iter().map(TagId::new).collect())
}

// ── Read-only world view ────────────────────────────────────────────────────────

/// A runtime transform as a policy sees it. Mirrors the render border's tuple
/// order (`translation`, `rotation` xyzw, `scale`) so the projection is a copy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyTransform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

/// The lifecycle states a policy may observe. `Tombstoned` is intentionally absent:
/// retired entities are omitted from the view, not shown as a state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PolicyEntityLifecycle {
    Active,
    Disabled,
}

impl PolicyEntityLifecycle {
    pub fn label(self) -> &'static str {
        match self {
            PolicyEntityLifecycle::Active => "active",
            PolicyEntityLifecycle::Disabled => "disabled",
        }
    }
}

/// Where an entity came from, as a policy sees it. `DiagnosticTooling` has no
/// variant here: those entities are redacted entirely by the projector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PolicyEntitySource {
    /// Bootstrapped from an authored scene node (carries the node id).
    SceneNode { node: u64 },
    /// Created at runtime by an authority command.
    Runtime,
    /// Instantiated from a catalog asset (carries the asset id only).
    Imported { asset: String },
    /// Proposed by a policy and accepted by authority.
    Policy,
}

impl PolicyEntitySource {
    pub fn label(&self) -> &'static str {
        match self {
            PolicyEntitySource::SceneNode { .. } => "sceneNode",
            PolicyEntitySource::Runtime => "runtime",
            PolicyEntitySource::Imported { .. } => "imported",
            PolicyEntitySource::Policy => "policy",
        }
    }
}

/// The resolution status of an asset a policy might reference. Cached/renderer
/// state is never the source of truth here — this is the catalog's classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PolicyAssetStatus {
    Resolved,
    Missing,
    Stale,
}

impl PolicyAssetStatus {
    pub fn label(self) -> &'static str {
        match self {
            PolicyAssetStatus::Resolved => "resolved",
            PolicyAssetStatus::Missing => "missing",
            PolicyAssetStatus::Stale => "stale",
        }
    }
}

/// One asset a policy may reason about: its id, kind, and resolution status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyAssetView {
    pub id: String,
    pub kind: String,
    pub status: PolicyAssetStatus,
}

/// One entity as a policy sees it: identity, lifecycle, optional transform,
/// source, labels, and whether it occupies space (has a transform capability).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyEntityView {
    #[serde(
        serialize_with = "serialize_entity_id",
        deserialize_with = "deserialize_entity_id"
    )]
    pub id: EntityId,
    pub lifecycle: PolicyEntityLifecycle,
    pub transform: Option<PolicyTransform>,
    pub source: PolicyEntitySource,
    #[serde(
        serialize_with = "serialize_tag_ids",
        deserialize_with = "deserialize_tag_ids"
    )]
    pub labels: Vec<TagId>,
    pub spatial: bool,
}

/// Cheap aggregate counts so a policy can branch without scanning the whole view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyWorldSummary {
    pub tick: u64,
    pub active_entities: u32,
    pub spatial_entities: u32,
    pub asset_count: u32,
    pub missing_assets: u32,
}

/// The complete read-only world projection handed to a policy for one tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyWorldView {
    pub tick: u64,
    pub entities: Vec<PolicyEntityView>,
    pub assets: Vec<PolicyAssetView>,
    pub summary: PolicyWorldSummary,
}

impl PolicyWorldView {
    /// The projection of an empty world at a given tick.
    pub fn empty(tick: u64) -> Self {
        PolicyWorldView {
            tick,
            entities: Vec::new(),
            assets: Vec::new(),
            summary: PolicyWorldSummary {
                tick,
                ..PolicyWorldSummary::default()
            },
        }
    }
}

// ── Proposed world commands (#2392) ───────────────────────────────────────────────

/// The narrow, safe set of world/entity actions a policy may propose. Each is a
/// *request*: authority validates and applies, or rejects. Nothing here mutates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PolicyWorldCommand {
    /// Request a new transform for a spatial, active entity.
    RequestSetTransform {
        #[serde(
            serialize_with = "serialize_entity_id",
            deserialize_with = "deserialize_entity_id"
        )]
        entity: EntityId,
        transform: PolicyTransform,
    },
    /// Request a label be added to an entity.
    RequestAddLabel {
        #[serde(
            serialize_with = "serialize_entity_id",
            deserialize_with = "deserialize_entity_id"
        )]
        entity: EntityId,
        #[serde(
            serialize_with = "serialize_tag_id",
            deserialize_with = "deserialize_tag_id"
        )]
        label: TagId,
    },
    /// Request an active entity be disabled (reversible; never a destroy).
    RequestDisable {
        #[serde(
            serialize_with = "serialize_entity_id",
            deserialize_with = "deserialize_entity_id"
        )]
        entity: EntityId,
    },
    /// A no-op diagnostic marker — proposes no state change, only an audit note.
    NoopMarker { note: String },
}

impl PolicyWorldCommand {
    /// Stable discriminant label for diagnostics and replay.
    pub fn label(&self) -> &'static str {
        match self {
            PolicyWorldCommand::RequestSetTransform { .. } => "requestSetTransform",
            PolicyWorldCommand::RequestAddLabel { .. } => "requestAddLabel",
            PolicyWorldCommand::RequestDisable { .. } => "requestDisable",
            PolicyWorldCommand::NoopMarker { .. } => "noopMarker",
        }
    }
}

/// The accepted domain event a validated command becomes. Distinct from the
/// command (proposal) and from the rejection — the three never share a type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PolicyWorldEvent {
    TransformSet {
        #[serde(
            serialize_with = "serialize_entity_id",
            deserialize_with = "deserialize_entity_id"
        )]
        entity: EntityId,
        transform: PolicyTransform,
    },
    LabelAdded {
        #[serde(
            serialize_with = "serialize_entity_id",
            deserialize_with = "deserialize_entity_id"
        )]
        entity: EntityId,
        #[serde(
            serialize_with = "serialize_tag_id",
            deserialize_with = "deserialize_tag_id"
        )]
        label: TagId,
    },
    Disabled {
        #[serde(
            serialize_with = "serialize_entity_id",
            deserialize_with = "deserialize_entity_id"
        )]
        entity: EntityId,
    },
    /// A recorded no-op marker (accepted, changes no authority state).
    NoopRecorded { note: String },
}

impl PolicyWorldEvent {
    pub fn label(&self) -> &'static str {
        match self {
            PolicyWorldEvent::TransformSet { .. } => "transformSet",
            PolicyWorldEvent::LabelAdded { .. } => "labelAdded",
            PolicyWorldEvent::Disabled { .. } => "disabled",
            PolicyWorldEvent::NoopRecorded { .. } => "noopRecorded",
        }
    }
}

/// The classified reason authority refused a proposed command. Stable string form
/// is a contract; a policy never decides acceptance, it reflects this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PolicyWorldRejection {
    /// The target entity is not present (or was tombstoned/redacted).
    UnknownEntity,
    /// The entity exists but is disabled, so it may not be acted on.
    EntityDisabled,
    /// The entity has no transform capability; it cannot be moved.
    NotSpatial,
    /// The entity is spatial but immovable (a static collider); it may not be moved.
    Immovable,
    /// The proposed transform has a non-finite or zero-scale component.
    InvalidTransform,
    /// The label is already present on the entity.
    LabelAlreadyPresent,
    /// The entity is already disabled.
    AlreadyDisabled,
}

impl PolicyWorldRejection {
    pub fn label(self) -> &'static str {
        match self {
            PolicyWorldRejection::UnknownEntity => "unknownEntity",
            PolicyWorldRejection::EntityDisabled => "entityDisabled",
            PolicyWorldRejection::NotSpatial => "notSpatial",
            PolicyWorldRejection::Immovable => "immovable",
            PolicyWorldRejection::InvalidTransform => "invalidTransform",
            PolicyWorldRejection::LabelAlreadyPresent => "labelAlreadyPresent",
            PolicyWorldRejection::AlreadyDisabled => "alreadyDisabled",
        }
    }
}

/// The outcome authority reports for one proposed command: accepted (with its
/// event) or rejected (with the classified reason).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PolicyWorldOutcome {
    Accepted { event: PolicyWorldEvent },
    Rejected { rejection: PolicyWorldRejection },
}

impl PolicyWorldOutcome {
    pub fn is_accepted(&self) -> bool {
        matches!(self, PolicyWorldOutcome::Accepted { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transform() -> PolicyTransform {
        PolicyTransform {
            translation: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    #[test]
    fn world_view_serializes_with_numeric_ids_and_round_trips() {
        let view = PolicyWorldView {
            tick: 12,
            entities: vec![PolicyEntityView {
                id: EntityId::new(42),
                lifecycle: PolicyEntityLifecycle::Active,
                transform: Some(transform()),
                source: PolicyEntitySource::SceneNode { node: 7 },
                labels: vec![TagId::new(9), TagId::new(10)],
                spatial: true,
            }],
            assets: vec![PolicyAssetView {
                id: "mesh.wall".to_string(),
                kind: "staticMesh".to_string(),
                status: PolicyAssetStatus::Stale,
            }],
            summary: PolicyWorldSummary {
                tick: 12,
                active_entities: 1,
                spatial_entities: 1,
                asset_count: 1,
                missing_assets: 0,
            },
        };

        let json = serde_json::to_string_pretty(&view).expect("policy view serializes");
        assert!(json.contains(r#""id": 42"#));
        assert!(json.contains(r#""labels": ["#));
        assert!(json.contains(r#""kind": "sceneNode""#));
        assert!(json.contains(r#""status": "stale""#));

        let decoded: PolicyWorldView =
            serde_json::from_str(&json).expect("policy view deserializes");
        assert_eq!(decoded, view);
        assert_eq!(decoded.entities[0].id.raw(), 42);
        assert_eq!(decoded.entities[0].labels[1].raw(), 10);
    }

    #[test]
    fn command_event_and_outcome_round_trip_with_stable_tags() {
        let command = PolicyWorldCommand::RequestAddLabel {
            entity: EntityId::new(5),
            label: TagId::new(8),
        };
        let command_json = serde_json::to_string(&command).expect("command serializes");
        assert_eq!(
            command_json,
            r#"{"kind":"requestAddLabel","entity":5,"label":8}"#
        );
        let decoded_command: PolicyWorldCommand =
            serde_json::from_str(&command_json).expect("command deserializes");
        assert_eq!(decoded_command, command);
        assert_eq!(decoded_command.label(), "requestAddLabel");

        let outcome = PolicyWorldOutcome::Accepted {
            event: PolicyWorldEvent::TransformSet {
                entity: EntityId::new(5),
                transform: transform(),
            },
        };
        let outcome_json = serde_json::to_string(&outcome).expect("outcome serializes");
        assert!(outcome_json.contains(r#""status":"accepted""#));
        assert!(outcome_json.contains(r#""kind":"transformSet""#));
        let decoded_outcome: PolicyWorldOutcome =
            serde_json::from_str(&outcome_json).expect("outcome deserializes");
        assert_eq!(decoded_outcome, outcome);
        assert!(decoded_outcome.is_accepted());
    }

    #[test]
    fn rejects_unknown_wire_vocabulary() {
        let err =
            serde_json::from_str::<PolicyWorldCommand>(r#"{"kind":"deleteEntity","entity":5}"#)
                .expect_err("unknown policy command kind is rejected");
        assert!(err.to_string().contains("unknown variant"));

        let err = serde_json::from_str::<PolicyWorldRejection>(r#""rendererHandleMissing""#)
            .expect_err("unknown rejection vocabulary is rejected");
        assert!(err.to_string().contains("unknown variant"));
    }
}
