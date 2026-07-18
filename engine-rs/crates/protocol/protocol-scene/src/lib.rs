//! Cross-boundary schema for authored scene documents (scene-capability-super,
//! epic #2351, subtask #2365).
//!
//! # Lane
//!
//! `contract-steward` — owns the border shape TypeScript uses to **author and
//! inspect** scene documents, source traces, and bootstrap records. It depends
//! only on `core-ids` and renderer-neutral protocol vocabulary and
//! carries **no authority logic**: validation, flattening, bootstrap allocation,
//! and serialization all stay in `core-scene`. This crate is the single Rust
//! home for the *wire shape* plus the *stable string vocabularies*
//! ([`SCENE_NODE_KIND_TAGS`], [`SCENE_VALIDATION_CODES`]) that
//! `protocol-codegen` mirrors to `@asha/contracts`.
//!
//! # What crosses the border
//!
//! - The canonical **flat** scene document ([`FlatSceneDocumentDto`]) — the form
//!   TS authoring tools read/write and Rust validates. (The ergonomic authoring
//!   *tree* stays a Rust-side convenience; only the flat form is canonical, so
//!   only it gets a border DTO.)
//! - Classified **validation** results ([`SceneValidationReportDto`]) so TS can
//!   render *why* a scene was rejected without parsing prose.
//! - The **source trace** ([`SceneSourceTraceDto`]) `scene node → runtime entity`
//!   and the atomic **bootstrap record** ([`BootstrapRecordDto`]).
//!
//! # Why a DTO layer separate from `core-scene`
//!
//! `core-scene`'s types carry rich authority detail (typed `AssetReference`,
//! `SceneTransform` with validation, tree⇄flat machinery). The border needs only
//! the serialized projection of that — plain integers, fixed tuples, and stable
//! string tags — so a renamed internal field never silently changes the wire.
//! `core-scene` owns the conversion *into* these DTOs (it is the higher layer).

#![forbid(unsafe_code)]

use core_ids::{EntityId, ProjectId, RuntimeSessionId, SceneId, SceneNodeId};

// ── Stable string vocabularies (the contract) ─────────────────────────────────

/// Stable tag for each scene-node kind, identical in Rust and generated
/// TypeScript. The string form is a contract: tags are *added*, never renamed.
/// Mirrors `core_scene::SceneNodeKind::tag`.
pub const SCENE_NODE_KIND_TAGS: &[&str] = &[
    "emptyGroup",
    "staticMesh",
    "sprite",
    "voxelVolume",
    "light",
    "marker",
    "entityInstance",
    "bootstrap",
];

/// Stable classified scene-validation codes. Mirrors
/// `core_scene::SceneValidationError::label`; the string form is a contract.
pub const SCENE_VALIDATION_CODES: &[&str] = &[
    "duplicate-node-id",
    "unknown-parent",
    "cycle",
    "invalid-transform",
    "invalid-voxel-volume-transform",
    "asset-kind-mismatch",
    "invalid-light",
    "duplicate-marker-id",
    "invalid-marker",
    "duplicate-entity-instance-id",
    "invalid-entity-instance",
    "duplicate-bootstrap-node",
    "invalid-bootstrap",
    "duplicate-catalog-binding",
];

/// Stable scene-object command rejection codes. Mirrors
/// `core_scene::SceneObjectCommandRejection::label`; the string form is a contract.
pub const SCENE_OBJECT_COMMAND_REJECTION_CODES: &[&str] = &[
    "stale-scene-object-snapshot",
    "invalid-scene-before-command",
    "invalid-scene-after-command",
    "missing-scene-object",
    "duplicate-scene-object",
    "missing-scene-object-parent",
    "scene-object-self-parent",
    "blank-scene-object-label",
    "invalid-scene-object-kind",
    "invalid-scene-object-transform",
    "readonly-scene-object-transform",
];

/// Stable classifications for stored scene-document codec failures. Structural
/// decode failures are kept separate from semantic [`SceneValidationCode`]
/// entries so authoring tools never need to parse Rust error prose.
pub const SCENE_DOCUMENT_CODEC_DIAGNOSTIC_CODES: &[&str] = &[
    "invalid-json",
    "invalid-field",
    "invalid-asset",
    "unknown-kind",
    "unknown-version-requirement",
    "unsupported-schema",
    "unsupported-authoring-format",
    "invalid-document",
    "legacy-demo-scene",
];

/// Stable classifications for a stored SceneDocument compare-and-swap
/// authoring transaction.
pub const SCENE_DOCUMENT_AUTHORING_REJECTION_CODES: &[&str] = &[
    "stale-scene-document",
    "invalid-current-scene-document",
    "invalid-resulting-scene-document",
    "invalid-scene-document-command",
    "missing-scene-document-target",
    "foreign-scene-document-identity",
];

/// Classified stored-authoring transaction rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneDocumentAuthoringRejectionCode {
    StaleDocument,
    InvalidCurrentDocument,
    InvalidResultingDocument,
    InvalidCommand,
    MissingTarget,
    ForeignDocumentIdentity,
}

impl SceneDocumentAuthoringRejectionCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StaleDocument => "stale-scene-document",
            Self::InvalidCurrentDocument => "invalid-current-scene-document",
            Self::InvalidResultingDocument => "invalid-resulting-scene-document",
            Self::InvalidCommand => "invalid-scene-document-command",
            Self::MissingTarget => "missing-scene-document-target",
            Self::ForeignDocumentIdentity => "foreign-scene-document-identity",
        }
    }
}

/// The scene-node kind tag as a closed enum with a stable string form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneNodeKindTag {
    EmptyGroup,
    StaticMesh,
    Sprite,
    VoxelVolume,
    Light,
    Marker,
    EntityInstance,
    Bootstrap,
}

impl SceneNodeKindTag {
    /// The stable wire string. Must match the corresponding [`SCENE_NODE_KIND_TAGS`] entry.
    pub fn as_str(self) -> &'static str {
        match self {
            SceneNodeKindTag::EmptyGroup => "emptyGroup",
            SceneNodeKindTag::StaticMesh => "staticMesh",
            SceneNodeKindTag::Sprite => "sprite",
            SceneNodeKindTag::VoxelVolume => "voxelVolume",
            SceneNodeKindTag::Light => "light",
            SceneNodeKindTag::Marker => "marker",
            SceneNodeKindTag::EntityInstance => "entityInstance",
            SceneNodeKindTag::Bootstrap => "bootstrap",
        }
    }

    /// Whether this kind must carry an asset reference.
    pub fn requires_asset(self) -> bool {
        matches!(
            self,
            SceneNodeKindTag::StaticMesh | SceneNodeKindTag::Sprite | SceneNodeKindTag::VoxelVolume
        )
    }
}

/// Every [`SceneNodeKindTag`] in declaration order, for table/round-trip tests.
pub const ALL_SCENE_NODE_KIND_TAGS: &[SceneNodeKindTag] = &[
    SceneNodeKindTag::EmptyGroup,
    SceneNodeKindTag::StaticMesh,
    SceneNodeKindTag::Sprite,
    SceneNodeKindTag::VoxelVolume,
    SceneNodeKindTag::Light,
    SceneNodeKindTag::Marker,
    SceneNodeKindTag::EntityInstance,
    SceneNodeKindTag::Bootstrap,
];

/// A classified scene-validation code as a closed enum with a stable string form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneValidationCode {
    DuplicateNodeId,
    UnknownParent,
    Cycle,
    InvalidTransform,
    InvalidVoxelVolumeTransform,
    AssetKindMismatch,
    InvalidLight,
    DuplicateMarkerId,
    InvalidMarker,
    DuplicateEntityInstanceId,
    InvalidEntityInstance,
    DuplicateBootstrapNode,
    InvalidBootstrap,
    DuplicateCatalogBinding,
}

impl SceneValidationCode {
    /// The stable wire string. Must match the corresponding [`SCENE_VALIDATION_CODES`] entry.
    pub fn as_str(self) -> &'static str {
        match self {
            SceneValidationCode::DuplicateNodeId => "duplicate-node-id",
            SceneValidationCode::UnknownParent => "unknown-parent",
            SceneValidationCode::Cycle => "cycle",
            SceneValidationCode::InvalidTransform => "invalid-transform",
            SceneValidationCode::InvalidVoxelVolumeTransform => "invalid-voxel-volume-transform",
            SceneValidationCode::AssetKindMismatch => "asset-kind-mismatch",
            SceneValidationCode::InvalidLight => "invalid-light",
            SceneValidationCode::DuplicateMarkerId => "duplicate-marker-id",
            SceneValidationCode::InvalidMarker => "invalid-marker",
            SceneValidationCode::DuplicateEntityInstanceId => "duplicate-entity-instance-id",
            SceneValidationCode::InvalidEntityInstance => "invalid-entity-instance",
            SceneValidationCode::DuplicateBootstrapNode => "duplicate-bootstrap-node",
            SceneValidationCode::InvalidBootstrap => "invalid-bootstrap",
            SceneValidationCode::DuplicateCatalogBinding => "duplicate-catalog-binding",
        }
    }
}

/// Every [`SceneValidationCode`] in declaration order, for table/round-trip tests.
pub const ALL_SCENE_VALIDATION_CODES: &[SceneValidationCode] = &[
    SceneValidationCode::DuplicateNodeId,
    SceneValidationCode::UnknownParent,
    SceneValidationCode::Cycle,
    SceneValidationCode::InvalidTransform,
    SceneValidationCode::InvalidVoxelVolumeTransform,
    SceneValidationCode::AssetKindMismatch,
    SceneValidationCode::InvalidLight,
    SceneValidationCode::DuplicateMarkerId,
    SceneValidationCode::InvalidMarker,
    SceneValidationCode::DuplicateEntityInstanceId,
    SceneValidationCode::InvalidEntityInstance,
    SceneValidationCode::DuplicateBootstrapNode,
    SceneValidationCode::InvalidBootstrap,
    SceneValidationCode::DuplicateCatalogBinding,
];

/// A classified scene-object command rejection code as a closed enum with a
/// stable string form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneObjectCommandRejectionCode {
    StaleSnapshot,
    InvalidBefore,
    InvalidAfter,
    MissingObject,
    DuplicateObject,
    MissingParent,
    SelfParent,
    BlankLabel,
    WrongObjectKind,
    InvalidTransform,
    ReadonlyTransform,
}

impl SceneObjectCommandRejectionCode {
    /// The stable wire string. Must match
    /// [`SCENE_OBJECT_COMMAND_REJECTION_CODES`].
    pub fn as_str(self) -> &'static str {
        match self {
            SceneObjectCommandRejectionCode::StaleSnapshot => "stale-scene-object-snapshot",
            SceneObjectCommandRejectionCode::InvalidBefore => "invalid-scene-before-command",
            SceneObjectCommandRejectionCode::InvalidAfter => "invalid-scene-after-command",
            SceneObjectCommandRejectionCode::MissingObject => "missing-scene-object",
            SceneObjectCommandRejectionCode::DuplicateObject => "duplicate-scene-object",
            SceneObjectCommandRejectionCode::MissingParent => "missing-scene-object-parent",
            SceneObjectCommandRejectionCode::SelfParent => "scene-object-self-parent",
            SceneObjectCommandRejectionCode::BlankLabel => "blank-scene-object-label",
            SceneObjectCommandRejectionCode::WrongObjectKind => "invalid-scene-object-kind",
            SceneObjectCommandRejectionCode::InvalidTransform => "invalid-scene-object-transform",
            SceneObjectCommandRejectionCode::ReadonlyTransform => "readonly-scene-object-transform",
        }
    }
}

/// Every [`SceneObjectCommandRejectionCode`] in declaration order, for tests.
pub const ALL_SCENE_OBJECT_COMMAND_REJECTION_CODES: &[SceneObjectCommandRejectionCode] = &[
    SceneObjectCommandRejectionCode::StaleSnapshot,
    SceneObjectCommandRejectionCode::InvalidBefore,
    SceneObjectCommandRejectionCode::InvalidAfter,
    SceneObjectCommandRejectionCode::MissingObject,
    SceneObjectCommandRejectionCode::DuplicateObject,
    SceneObjectCommandRejectionCode::MissingParent,
    SceneObjectCommandRejectionCode::SelfParent,
    SceneObjectCommandRejectionCode::BlankLabel,
    SceneObjectCommandRejectionCode::WrongObjectKind,
    SceneObjectCommandRejectionCode::InvalidTransform,
    SceneObjectCommandRejectionCode::ReadonlyTransform,
];

/// Classified structural or compatibility failure from the stored scene codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneDocumentCodecDiagnosticCode {
    InvalidJson,
    InvalidField,
    InvalidAsset,
    UnknownKind,
    UnknownVersionRequirement,
    UnsupportedSchema,
    UnsupportedAuthoringFormat,
    InvalidDocument,
    LegacyDemoScene,
}

impl SceneDocumentCodecDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidJson => "invalid-json",
            Self::InvalidField => "invalid-field",
            Self::InvalidAsset => "invalid-asset",
            Self::UnknownKind => "unknown-kind",
            Self::UnknownVersionRequirement => "unknown-version-requirement",
            Self::UnsupportedSchema => "unsupported-schema",
            Self::UnsupportedAuthoringFormat => "unsupported-authoring-format",
            Self::InvalidDocument => "invalid-document",
            Self::LegacyDemoScene => "legacy-demo-scene",
        }
    }
}

pub const ALL_SCENE_DOCUMENT_CODEC_DIAGNOSTIC_CODES: &[SceneDocumentCodecDiagnosticCode] = &[
    SceneDocumentCodecDiagnosticCode::InvalidJson,
    SceneDocumentCodecDiagnosticCode::InvalidField,
    SceneDocumentCodecDiagnosticCode::InvalidAsset,
    SceneDocumentCodecDiagnosticCode::UnknownKind,
    SceneDocumentCodecDiagnosticCode::UnknownVersionRequirement,
    SceneDocumentCodecDiagnosticCode::UnsupportedSchema,
    SceneDocumentCodecDiagnosticCode::UnsupportedAuthoringFormat,
    SceneDocumentCodecDiagnosticCode::InvalidDocument,
    SceneDocumentCodecDiagnosticCode::LegacyDemoScene,
];

// ── Asset reference border DTO ────────────────────────────────────────────────

/// Border form of an asset version requirement. Mirrors the `{ "req": … }` wire
/// object `core_scene::json` reads/writes.
#[derive(Debug, Clone, PartialEq)]
pub enum AssetVersionReqDto {
    /// Any version satisfies.
    Any,
    /// Exactly this version.
    Exact(u32),
    /// At least this version.
    AtLeast(u32),
}

impl AssetVersionReqDto {
    /// The stable `req` discriminant string.
    pub fn req_tag(&self) -> &'static str {
        match self {
            AssetVersionReqDto::Any => "any",
            AssetVersionReqDto::Exact(_) => "exact",
            AssetVersionReqDto::AtLeast(_) => "atLeast",
        }
    }
}

/// Border form of a kind-erased asset reference.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetReferenceDto {
    /// Kind-prefixed scoped-kebab-case asset id (e.g. `static-mesh:env/crate`).
    pub id: String,
    /// Version requirement.
    pub version: AssetVersionReqDto,
    /// Optional content hash pin.
    pub hash: Option<String>,
}

// ── Scene document border DTOs ────────────────────────────────────────────────

/// Border form of a scene node's initial transform: fixed-width tuples, no
/// validation (Rust validates the authority form).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneTransformDto {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

/// Stored shadow intent. Render backends may expose a classified degradation,
/// but the authored request remains durable scene data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneLightShadowIntentDto {
    #[default]
    Disabled,
    Requested,
}

/// Renderer-neutral authored light. Pose is intentionally absent: translation
/// and orientation come from the containing scene node transform.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneLightDto {
    Ambient {
        color: [f32; 3],
        intensity: f32,
        enabled: bool,
        shadow_intent: SceneLightShadowIntentDto,
    },
    Directional {
        color: [f32; 3],
        intensity: f32,
        enabled: bool,
        shadow_intent: SceneLightShadowIntentDto,
    },
    Point {
        color: [f32; 3],
        intensity: f32,
        enabled: bool,
        range: Option<f32>,
        decay: f32,
        shadow_intent: SceneLightShadowIntentDto,
    },
    Spot {
        color: [f32; 3],
        intensity: f32,
        enabled: bool,
        range: Option<f32>,
        decay: f32,
        outer_angle_radians: f32,
        penumbra: f32,
        shadow_intent: SceneLightShadowIntentDto,
    },
}

/// Stored target resolved by one authored runtime instance placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneEntityReferenceDto {
    EntityDefinition {
        stable_id: String,
    },
    Prefab {
        prefab_id: u64,
        variant_id: Option<String>,
        instantiation_seed: u64,
    },
}

/// Standalone marker metadata retained for consumers that inspect marker
/// fields outside the flattened scene-node discriminated union.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneMarkerDto {
    pub marker_id: String,
}

/// Renderer-neutral stored runtime instance intent. Hierarchy and local pose
/// remain on the containing [`SceneNodeRecordDto`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneEntityInstanceDto {
    pub instance_id: String,
    pub reference: SceneEntityReferenceDto,
    pub spawn_marker_id: Option<String>,
}

/// Generic procedural generator input for one scene bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneGeneratorBindingDto {
    pub provider_id: String,
    pub preset_id: String,
    pub seed: u64,
}

/// One named ProjectBundle catalog input used by scene bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneCatalogBindingDto {
    pub binding_id: String,
    pub catalog_id: String,
    pub source_path: String,
}

/// Explicit non-spatial scene bootstrap inputs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SceneBootstrapBindingsDto {
    pub generator: Option<SceneGeneratorBindingDto>,
    pub catalogs: Vec<SceneCatalogBindingDto>,
}

/// Border form of a scene node's kind. Only asset-backed kinds carry an asset,
/// mirroring the generated TypeScript discriminated union (so an "empty group
/// with an asset" is unrepresentable rather than merely discouraged).
#[derive(Debug, Clone, PartialEq)]
pub enum SceneNodeKindDto {
    EmptyGroup,
    StaticMesh(AssetReferenceDto),
    Sprite(AssetReferenceDto),
    VoxelVolume(AssetReferenceDto),
    Light(SceneLightDto),
    Marker { marker_id: String },
    EntityInstance { instance: SceneEntityInstanceDto },
    Bootstrap { bindings: SceneBootstrapBindingsDto },
}

impl SceneNodeKindDto {
    /// The stable discriminant tag.
    pub fn tag(&self) -> SceneNodeKindTag {
        match self {
            SceneNodeKindDto::EmptyGroup => SceneNodeKindTag::EmptyGroup,
            SceneNodeKindDto::StaticMesh(_) => SceneNodeKindTag::StaticMesh,
            SceneNodeKindDto::Sprite(_) => SceneNodeKindTag::Sprite,
            SceneNodeKindDto::VoxelVolume(_) => SceneNodeKindTag::VoxelVolume,
            SceneNodeKindDto::Light(_) => SceneNodeKindTag::Light,
            SceneNodeKindDto::Marker { .. } => SceneNodeKindTag::Marker,
            SceneNodeKindDto::EntityInstance { .. } => SceneNodeKindTag::EntityInstance,
            SceneNodeKindDto::Bootstrap { .. } => SceneNodeKindTag::Bootstrap,
        }
    }

    /// The asset reference this kind carries, if any.
    pub fn asset(&self) -> Option<&AssetReferenceDto> {
        match self {
            SceneNodeKindDto::EmptyGroup
            | SceneNodeKindDto::Light(_)
            | SceneNodeKindDto::Marker { .. }
            | SceneNodeKindDto::EntityInstance { .. }
            | SceneNodeKindDto::Bootstrap { .. } => None,
            SceneNodeKindDto::StaticMesh(a)
            | SceneNodeKindDto::Sprite(a)
            | SceneNodeKindDto::VoxelVolume(a) => Some(a),
        }
    }
}

/// Border form of one canonical flat scene-node record.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneNodeRecordDto {
    pub id: SceneNodeId,
    pub parent: Option<SceneNodeId>,
    pub child_order: u32,
    pub label: Option<String>,
    pub tags: Vec<String>,
    pub transform: SceneTransformDto,
    pub kind: SceneNodeKindDto,
}

/// Border form of document-level metadata.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneMetadataDto {
    pub name: Option<String>,
    pub authoring_format_version: u32,
}

/// Border form of the canonical flat scene document — the shape TS authors and
/// Rust validates.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatSceneDocumentDto {
    pub schema_version: u32,
    pub id: SceneId,
    pub metadata: SceneMetadataDto,
    pub dependencies: Vec<AssetReferenceDto>,
    pub nodes: Vec<SceneNodeRecordDto>,
}

// ── Stored scene-document codec border DTOs ──────────────────────────────────

/// Authored scene source text to decode, canonicalize, and validate in Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneDocumentDecodeRequestDto {
    pub source_text: String,
}

/// Typed authored scene document to validate and encode canonically in Rust.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneDocumentEncodeRequestDto {
    pub document: FlatSceneDocumentDto,
}

/// One structural or compatibility diagnostic from the stored scene codec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneDocumentCodecDiagnosticDto {
    pub code: SceneDocumentCodecDiagnosticCode,
    pub message: String,
}

/// Shared result for decode and encode. Accepted results always carry the
/// canonical typed document, canonical JSON, and a stable content identity.
/// Rejected results carry structural diagnostics and/or semantic validation
/// errors and never mutate RuntimeSession state.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneDocumentCodecResultDto {
    pub accepted: bool,
    pub document: Option<FlatSceneDocumentDto>,
    pub canonical_json: Option<String>,
    pub content_hash: Option<String>,
    pub diagnostics: Vec<SceneDocumentCodecDiagnosticDto>,
    pub validation: SceneValidationReportDto,
}

/// Durable project/scene identity targeted by one stored authoring command.
/// The command target is checked against the current project identity and the
/// current document's scene identity before Rust applies the edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneDocumentAuthoringTargetDto {
    pub project_id: ProjectId,
    pub scene_id: SceneId,
}

/// Bounded stored SceneDocument commands. Unlike the live scene-object command
/// surface, these commands edit caller-supplied durable scene data and return a
/// canonical replacement without mutating RuntimeSession authority.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneDocumentAuthoringCommandDto {
    RefreshProjection {
        target: SceneDocumentAuthoringTargetDto,
    },
    Create {
        target: SceneDocumentAuthoringTargetDto,
        record: SceneNodeRecordDto,
    },
    Delete {
        target: SceneDocumentAuthoringTargetDto,
        id: SceneNodeId,
    },
    Rename {
        target: SceneDocumentAuthoringTargetDto,
        id: SceneNodeId,
        label: Option<String>,
    },
    Reparent {
        target: SceneDocumentAuthoringTargetDto,
        id: SceneNodeId,
        parent: Option<SceneNodeId>,
        child_order: u32,
    },
    SetTransform {
        target: SceneDocumentAuthoringTargetDto,
        id: SceneNodeId,
        transform: SceneTransformDto,
    },
    UpdateLight {
        target: SceneDocumentAuthoringTargetDto,
        id: SceneNodeId,
        scene_light: SceneLightDto,
    },
    RetargetVoxelAsset {
        target: SceneDocumentAuthoringTargetDto,
        id: SceneNodeId,
        asset: AssetReferenceDto,
        tags: Vec<String>,
    },
}

impl SceneDocumentAuthoringCommandDto {
    pub fn target(&self) -> SceneDocumentAuthoringTargetDto {
        match self {
            Self::RefreshProjection { target }
            | Self::Create { target, .. }
            | Self::Delete { target, .. }
            | Self::Rename { target, .. }
            | Self::Reparent { target, .. }
            | Self::SetTransform { target, .. }
            | Self::UpdateLight { target, .. }
            | Self::RetargetVoxelAsset { target, .. } => *target,
        }
    }
}

/// One compare-and-swap command against durable stored scene data. The current
/// document remains caller-owned input; Rust validates it, applies exactly one
/// bounded command, and only returns the accepted canonical result.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneDocumentAuthoringRequestDto {
    pub current_project_id: ProjectId,
    pub expected_content_hash: String,
    pub current_document: FlatSceneDocumentDto,
    pub command: SceneDocumentAuthoringCommandDto,
}

/// Classified rejection from a stored scene authoring transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneDocumentAuthoringRejectionDto {
    pub code: SceneDocumentAuthoringRejectionCode,
    pub message: String,
    pub expected_hash: Option<String>,
    pub actual_hash: Option<String>,
}

// ── Validation border DTOs ────────────────────────────────────────────────────

/// Border form of one classified validation failure. Optional fields are
/// populated per code (e.g. `parent` for `unknown-parent`, `cycle_path` for
/// `cycle`), so TS can render the failure precisely without parsing prose.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneValidationErrorDto {
    pub code: SceneValidationCode,
    /// The offending node, when the failure is about a single node.
    pub node: Option<SceneNodeId>,
    /// The named-but-absent parent, for `unknown-parent`.
    pub parent: Option<SceneNodeId>,
    /// The asset kind a node should have referenced, for `asset-kind-mismatch`.
    pub expected_kind: Option<String>,
    /// The asset kind actually referenced, for `asset-kind-mismatch`.
    pub actual_kind: Option<String>,
    /// A stable reason string, for `invalid-transform`.
    pub transform_reason: Option<String>,
    /// A stable reason string, for `invalid-light`.
    pub light_reason: Option<String>,
    /// A stable reason for typed entity-instance/bootstrap validation failures.
    pub detail_reason: Option<String>,
    /// Durable instance identity, for duplicate instance diagnostics.
    pub instance_id: Option<String>,
    /// Scene-local catalog binding identity, for duplicate binding diagnostics.
    pub binding_id: Option<String>,
    /// The ids forming the cycle in order, for `cycle`.
    pub cycle_path: Vec<SceneNodeId>,
}

impl SceneValidationErrorDto {
    /// A bare error carrying only its code; callers fill in the relevant locus.
    pub fn of(code: SceneValidationCode) -> Self {
        Self {
            code,
            node: None,
            parent: None,
            expected_kind: None,
            actual_kind: None,
            transform_reason: None,
            light_reason: None,
            detail_reason: None,
            instance_id: None,
            binding_id: None,
            cycle_path: Vec::new(),
        }
    }
}

/// Border form of a full validation report: every classified error.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneValidationReportDto {
    pub errors: Vec<SceneValidationErrorDto>,
}

impl SceneValidationReportDto {
    /// `true` if the document validated with no errors.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

// ── Scene-object hierarchy command border DTOs ───────────────────────────────

/// Border projection of one canonical scene object. Scene objects are authored
/// scene nodes, never runtime entities or render handles.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObjectRecordDto {
    pub id: SceneNodeId,
    pub parent: Option<SceneNodeId>,
    pub child_order: u32,
    pub label: Option<String>,
    pub kind: SceneNodeKindTag,
    pub has_renderable_asset: bool,
}

/// Border projection of the deterministic hierarchy snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObjectSnapshotDto {
    pub document_hash: u64,
    pub objects: Vec<SceneObjectRecordDto>,
}

/// Explicit scene-object hierarchy commands. Selection is included so GUI and
/// agent surfaces share the same command identity.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneObjectCommandDto {
    Create {
        record: SceneNodeRecordDto,
    },
    Delete {
        id: SceneNodeId,
    },
    Rename {
        id: SceneNodeId,
        label: Option<String>,
    },
    Reparent {
        id: SceneNodeId,
        parent: Option<SceneNodeId>,
        child_order: u32,
    },
    UpdateLight {
        id: SceneNodeId,
        scene_light: SceneLightDto,
    },
    Translate {
        id: SceneNodeId,
        delta: [f32; 3],
    },
    Rotate {
        id: SceneNodeId,
        rotation: [f32; 4],
    },
    Select {
        id: Option<SceneNodeId>,
    },
}

/// Border form of a scene-object command rejection.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObjectCommandRejectionDto {
    pub code: SceneObjectCommandRejectionCode,
    pub id: Option<SceneNodeId>,
    pub parent: Option<SceneNodeId>,
    pub expected_hash: Option<u64>,
    pub actual_hash: Option<u64>,
    pub validation_errors: Vec<SceneValidationErrorDto>,
}

/// Border form of a successful scene-object command.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObjectCommandOutcomeDto {
    pub document: FlatSceneDocumentDto,
    pub snapshot: SceneObjectSnapshotDto,
    pub selected: Option<SceneNodeId>,
}

/// One-in request envelope for applying a scene-object command.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObjectCommandRequestDto {
    pub expected_document_hash: u64,
    pub command: SceneObjectCommandDto,
}

/// One-out result envelope for applying a scene-object command.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObjectCommandResultDto {
    pub accepted: bool,
    pub outcome: Option<SceneObjectCommandOutcomeDto>,
    pub rejection: Option<SceneObjectCommandRejectionDto>,
}

// ── Source trace + bootstrap border DTOs ──────────────────────────────────────

/// Border form of one hop in the `scene node → runtime entity` source trace.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneSourceTraceDto {
    pub scene_node_id: SceneNodeId,
    pub runtime_entity_id: EntityId,
}

/// One resolved stored placement retained in atomic bootstrap evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneResolvedEntityInstanceDto {
    pub scene_node_id: SceneNodeId,
    pub runtime_entity_id: EntityId,
    pub instance_id: String,
    pub reference: SceneEntityReferenceDto,
    pub spawn_marker_id: Option<String>,
    pub local_transform: SceneTransformDto,
    pub world_transform: SceneTransformDto,
}

/// Border form of the atomic bootstrap record — the single replay/audit unit a
/// scene→authority initialization produces.
#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapRecordDto {
    pub scene_id: SceneId,
    pub runtime_session_id: RuntimeSessionId,
    pub schema_version: u32,
    pub node_count: u32,
    pub entity_count: u32,
    /// Deterministic fingerprint of the bootstrapped world.
    pub spatial_session_hash: u64,
    /// One entry per scene-sourced entity, in deterministic order.
    pub source_trace: Vec<SceneSourceTraceDto>,
    /// Canonical FNV-1a identity of the exact stored scene document.
    pub scene_content_hash: u64,
    /// Typed resolved instance placements in stable scene-node order.
    pub resolved_instances: Vec<SceneResolvedEntityInstanceDto>,
    /// Explicit generator/catalog inputs retained for replay/audit correlation.
    pub bootstrap_bindings: Option<SceneBootstrapBindingsDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_kind_table_matches_variants() {
        let from_variants: Vec<&str> = ALL_SCENE_NODE_KIND_TAGS
            .iter()
            .map(|t| t.as_str())
            .collect();
        assert_eq!(from_variants, SCENE_NODE_KIND_TAGS);
    }

    #[test]
    fn validation_code_table_matches_variants() {
        let from_variants: Vec<&str> = ALL_SCENE_VALIDATION_CODES
            .iter()
            .map(|c| c.as_str())
            .collect();
        assert_eq!(from_variants, SCENE_VALIDATION_CODES);
    }

    #[test]
    fn scene_object_command_rejection_table_matches_variants() {
        let from_variants: Vec<&str> = ALL_SCENE_OBJECT_COMMAND_REJECTION_CODES
            .iter()
            .map(|c| c.as_str())
            .collect();
        assert_eq!(from_variants, SCENE_OBJECT_COMMAND_REJECTION_CODES);
    }

    #[test]
    fn only_asset_backed_node_kinds_require_assets() {
        for tag in ALL_SCENE_NODE_KIND_TAGS {
            let requires = tag.requires_asset();
            assert_eq!(
                requires,
                matches!(
                    tag,
                    SceneNodeKindTag::StaticMesh
                        | SceneNodeKindTag::Sprite
                        | SceneNodeKindTag::VoxelVolume
                )
            );
        }
    }

    #[test]
    fn version_req_tags_are_stable() {
        assert_eq!(AssetVersionReqDto::Any.req_tag(), "any");
        assert_eq!(AssetVersionReqDto::Exact(2).req_tag(), "exact");
        assert_eq!(AssetVersionReqDto::AtLeast(3).req_tag(), "atLeast");
    }

    #[test]
    fn dto_builders_compose() {
        let doc = FlatSceneDocumentDto {
            schema_version: 1,
            id: SceneId::new(1),
            metadata: SceneMetadataDto {
                name: Some("demo".into()),
                authoring_format_version: 0,
            },
            dependencies: vec![AssetReferenceDto {
                id: "static-mesh:env/crate".into(),
                version: AssetVersionReqDto::Any,
                hash: None,
            }],
            nodes: vec![SceneNodeRecordDto {
                id: SceneNodeId::new(10),
                parent: None,
                child_order: 0,
                label: None,
                tags: vec![],
                transform: SceneTransformDto {
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
                kind: SceneNodeKindDto::StaticMesh(AssetReferenceDto {
                    id: "static-mesh:env/crate".into(),
                    version: AssetVersionReqDto::Exact(1),
                    hash: Some("blake3:abc".into()),
                }),
            }],
        };
        assert_eq!(doc.nodes.len(), 1);
        assert_eq!(doc.nodes[0].kind.tag(), SceneNodeKindTag::StaticMesh);
        assert!(doc.nodes[0].kind.asset().is_some());

        let snapshot = SceneObjectSnapshotDto {
            document_hash: 99,
            objects: vec![SceneObjectRecordDto {
                id: SceneNodeId::new(10),
                parent: None,
                child_order: 0,
                label: Some("crate".into()),
                kind: SceneNodeKindTag::StaticMesh,
                has_renderable_asset: true,
            }],
        };
        let command = SceneObjectCommandDto::Rename {
            id: SceneNodeId::new(10),
            label: Some("renamed".into()),
        };
        let translate = SceneObjectCommandDto::Translate {
            id: SceneNodeId::new(10),
            delta: [0.25, 0.0, 0.0],
        };
        let rotate = SceneObjectCommandDto::Rotate {
            id: SceneNodeId::new(10),
            rotation: [0.0, 0.38268343, 0.0, 0.9238795],
        };
        let outcome = SceneObjectCommandOutcomeDto {
            document: doc,
            snapshot,
            selected: Some(SceneNodeId::new(10)),
        };
        let result = SceneObjectCommandResultDto {
            accepted: true,
            outcome: Some(outcome),
            rejection: None,
        };
        assert!(matches!(command, SceneObjectCommandDto::Rename { .. }));
        assert!(matches!(translate, SceneObjectCommandDto::Translate { .. }));
        assert!(matches!(rotate, SceneObjectCommandDto::Rotate { .. }));
        assert_eq!(
            result.outcome.unwrap().snapshot.objects[0].kind,
            SceneNodeKindTag::StaticMesh
        );
    }
}
