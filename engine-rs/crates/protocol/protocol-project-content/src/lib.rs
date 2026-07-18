//! Typed border for durable project content edited before RuntimeSession start.
//!
//! The document union is intentionally closed. It reuses the existing entity,
//! catalog, prefab, gameplay-binding, trigger, and scene-reference contracts;
//! it is not a JSON value bus or an arbitrary property-path API.

#![forbid(unsafe_code)]

use protocol_assets::StoredAssetCatalog;
use protocol_entity_authoring::EntityDefinition;
use protocol_game_extension::{
    GameplayContractRef, GameplayModuleBinding, GameplayModuleBindingOverride, GameplayModuleRef,
};
use protocol_project_bundle::{GameplayTriggerDefinition, PrefabRegistry};

pub const PROJECT_CONTENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectContentDocumentKind {
    EntityDefinition,
    AssetCatalog,
    PrefabRegistry,
    GameplayConfiguration,
    PresentationCatalog,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContentSourceDto {
    pub document_id: String,
    pub kind: ProjectContentDocumentKind,
    pub source_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectConfigurationValueKind {
    Boolean,
    Integer,
    Number,
    String,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectContentReferenceKind {
    Asset,
    EntityDefinition,
    SceneInstance,
    Prefab,
    PrefabPart,
    PresentationResource,
}

/// Provider-owned field metadata. Engine validates the shape and references;
/// providers retain the semantic meaning and codec identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectConfigurationFieldDto {
    pub field_id: String,
    pub label: String,
    pub value_kind: ProjectConfigurationValueKind,
    pub required: bool,
    pub reference_kind: Option<ProjectContentReferenceKind>,
    pub integer_min: Option<i64>,
    pub integer_max: Option<i64>,
    pub number_min: Option<f64>,
    pub number_max: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectConfigurationSchemaDto {
    pub schema_id: String,
    pub provider_id: String,
    pub contract: GameplayContractRef,
    pub codec_id: String,
    pub fields: Vec<ProjectConfigurationFieldDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectConfigurationValueDto {
    Boolean {
        value: bool,
    },
    Integer {
        value: i64,
    },
    Number {
        value: f64,
    },
    String {
        value: String,
    },
    Reference {
        reference_kind: ProjectContentReferenceKind,
        target_id: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectConfigurationFieldValueDto {
    pub field_id: String,
    pub value: ProjectConfigurationValueDto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectGameplayConfigurationDto {
    pub configuration_id: String,
    pub module: GameplayModuleRef,
    pub schema_id: String,
    pub values: Vec<ProjectConfigurationFieldValueDto>,
}

/// Human-authored gameplay selection. Canonical provider bytes and hashes are
/// derived by Rust and are deliberately absent from stored source.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectGameplayConfigurationDocumentDto {
    pub schema_version: u32,
    pub configurations: Vec<ProjectGameplayConfigurationDto>,
    pub bindings: Vec<GameplayModuleBinding>,
    pub overrides: Vec<GameplayModuleBindingOverride>,
    pub triggers: Vec<GameplayTriggerDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectPresentationResourceKind {
    AnimatedMesh,
    Audio,
    Particle,
    Font,
    Overlay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPresentationResourceDto {
    pub resource_id: String,
    pub kind: ProjectPresentationResourceKind,
    pub asset_id: String,
    pub source_path: String,
    pub content_hash: String,
    pub license_path: Option<String>,
    pub clip_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectPresentationCueDto {
    Animation {
        cue_id: String,
        resource_id: String,
        clip_id: String,
        looped: bool,
    },
    Audio {
        cue_id: String,
        resource_id: String,
        gain: f32,
    },
    Particle {
        cue_id: String,
        resource_id: String,
        scale: f32,
    },
    Overlay {
        cue_id: String,
        resource_id: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectPresentationCatalogDto {
    pub schema_version: u32,
    pub resources: Vec<ProjectPresentationResourceDto>,
    pub cues: Vec<ProjectPresentationCueDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectContentDocumentDto {
    EntityDefinition {
        document_id: String,
        definition: EntityDefinition,
    },
    AssetCatalog {
        document_id: String,
        catalog: StoredAssetCatalog,
    },
    PrefabRegistry {
        document_id: String,
        registry: PrefabRegistry,
    },
    GameplayConfiguration {
        document_id: String,
        document: ProjectGameplayConfigurationDocumentDto,
    },
    PresentationCatalog {
        document_id: String,
        catalog: ProjectPresentationCatalogDto,
    },
}

impl ProjectContentDocumentDto {
    pub fn document_id(&self) -> &str {
        match self {
            Self::EntityDefinition { document_id, .. }
            | Self::AssetCatalog { document_id, .. }
            | Self::PrefabRegistry { document_id, .. }
            | Self::GameplayConfiguration { document_id, .. }
            | Self::PresentationCatalog { document_id, .. } => document_id,
        }
    }

    pub fn kind(&self) -> ProjectContentDocumentKind {
        match self {
            Self::EntityDefinition { .. } => ProjectContentDocumentKind::EntityDefinition,
            Self::AssetCatalog { .. } => ProjectContentDocumentKind::AssetCatalog,
            Self::PrefabRegistry { .. } => ProjectContentDocumentKind::PrefabRegistry,
            Self::GameplayConfiguration { .. } => ProjectContentDocumentKind::GameplayConfiguration,
            Self::PresentationCatalog { .. } => ProjectContentDocumentKind::PresentationCatalog,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContentDecodeRequestDto {
    pub sources: Vec<ProjectContentSourceDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContentEncodeRequestDto {
    pub documents: Vec<ProjectContentDocumentDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectContentDiagnosticCode {
    InvalidJson,
    UnknownField,
    InvalidField,
    DuplicateDocument,
    InvalidDocument,
    UnknownReference,
    ReferenceKindMismatch,
    StaleRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContentDiagnosticDto {
    pub code: ProjectContentDiagnosticCode,
    pub document_id: Option<String>,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContentCanonicalFileDto {
    pub document_id: String,
    pub kind: ProjectContentDocumentKind,
    pub canonical_json: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContentFieldMetadataDto {
    pub document_id: String,
    pub path: String,
    pub label: String,
    pub value_kind: ProjectConfigurationValueKind,
    pub required: bool,
    pub editable: bool,
    pub reference_kind: Option<ProjectContentReferenceKind>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContentCodecResultDto {
    pub accepted: bool,
    pub documents: Vec<ProjectContentDocumentDto>,
    pub canonical_files: Vec<ProjectContentCanonicalFileDto>,
    pub set_hash: Option<String>,
    pub field_metadata: Vec<ProjectContentFieldMetadataDto>,
    pub diagnostics: Vec<ProjectContentDiagnosticDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectContentAuthoringCommandDto {
    Upsert {
        document: ProjectContentDocumentDto,
    },
    Delete {
        document_id: String,
        document_kind: ProjectContentDocumentKind,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContentAuthoringRequestDto {
    pub expected_workspace_id: String,
    pub expected_generation: u64,
    pub expected_working_revision: u64,
    pub expected_set_hash: String,
    pub command: ProjectContentAuthoringCommandDto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectContentAuthoringResultDto {
    pub accepted: bool,
    pub documents: Vec<ProjectContentDocumentDto>,
    pub canonical_files: Vec<ProjectContentCanonicalFileDto>,
    pub set_hash: Option<String>,
    pub field_metadata: Vec<ProjectContentFieldMetadataDto>,
    pub diagnostics: Vec<ProjectContentDiagnosticDto>,
}
