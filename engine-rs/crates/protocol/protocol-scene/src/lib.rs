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

use core_ids::{EntityId, RuntimeSessionId, SceneId, SceneNodeId};

// ── Stable string vocabularies (the contract) ─────────────────────────────────

/// Stable tag for each scene-node kind, identical in Rust and generated
/// TypeScript. The string form is a contract: tags are *added*, never renamed.
/// Mirrors `core_scene::SceneNodeKind::tag`.
pub const SCENE_NODE_KIND_TAGS: &[&str] =
    &["emptyGroup", "staticMesh", "sprite", "voxelVolume", "light"];

/// Stable classified scene-validation codes. Mirrors
/// `core_scene::SceneValidationError::label`; the string form is a contract.
pub const SCENE_VALIDATION_CODES: &[&str] = &[
    "duplicate-node-id",
    "unknown-parent",
    "cycle",
    "invalid-transform",
    "asset-kind-mismatch",
    "invalid-light",
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
];

/// Stable classifications for a stored SceneDocument compare-and-swap
/// authoring transaction.
pub const SCENE_DOCUMENT_AUTHORING_REJECTION_CODES: &[&str] = &[
    "stale-scene-document",
    "invalid-current-scene-document",
    "invalid-candidate-scene-document",
    "foreign-scene-document-identity",
];

/// Classified stored-authoring transaction rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneDocumentAuthoringRejectionCode {
    StaleDocument,
    InvalidCurrentDocument,
    InvalidCandidateDocument,
    ForeignDocumentIdentity,
}

impl SceneDocumentAuthoringRejectionCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StaleDocument => "stale-scene-document",
            Self::InvalidCurrentDocument => "invalid-current-scene-document",
            Self::InvalidCandidateDocument => "invalid-candidate-scene-document",
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
];

/// A classified scene-validation code as a closed enum with a stable string form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneValidationCode {
    DuplicateNodeId,
    UnknownParent,
    Cycle,
    InvalidTransform,
    AssetKindMismatch,
    InvalidLight,
}

impl SceneValidationCode {
    /// The stable wire string. Must match the corresponding [`SCENE_VALIDATION_CODES`] entry.
    pub fn as_str(self) -> &'static str {
        match self {
            SceneValidationCode::DuplicateNodeId => "duplicate-node-id",
            SceneValidationCode::UnknownParent => "unknown-parent",
            SceneValidationCode::Cycle => "cycle",
            SceneValidationCode::InvalidTransform => "invalid-transform",
            SceneValidationCode::AssetKindMismatch => "asset-kind-mismatch",
            SceneValidationCode::InvalidLight => "invalid-light",
        }
    }
}

/// Every [`SceneValidationCode`] in declaration order, for table/round-trip tests.
pub const ALL_SCENE_VALIDATION_CODES: &[SceneValidationCode] = &[
    SceneValidationCode::DuplicateNodeId,
    SceneValidationCode::UnknownParent,
    SceneValidationCode::Cycle,
    SceneValidationCode::InvalidTransform,
    SceneValidationCode::AssetKindMismatch,
    SceneValidationCode::InvalidLight,
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
        }
    }

    /// The asset reference this kind carries, if any.
    pub fn asset(&self) -> Option<&AssetReferenceDto> {
        match self {
            SceneNodeKindDto::EmptyGroup | SceneNodeKindDto::Light(_) => None,
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

/// One compare-and-swap proposal against durable stored scene data. The current
/// document remains caller-owned input; Rust validates both documents and only
/// returns a replacement after accepting the complete candidate atomically.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneDocumentAuthoringRequestDto {
    pub expected_content_hash: String,
    pub current_document: FlatSceneDocumentDto,
    pub candidate_document: FlatSceneDocumentDto,
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
