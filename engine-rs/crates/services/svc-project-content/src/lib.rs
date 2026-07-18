//! Rust-owned codec, validation, and compare-and-swap authoring for durable
//! non-scene project content.

#![forbid(unsafe_code)]

mod codec;
mod validate;

use std::collections::BTreeMap;

use protocol_project_content::{
    ProjectContentAuthoringCommandDto, ProjectContentAuthoringRequestDto,
    ProjectContentAuthoringResultDto, ProjectContentCodecResultDto, ProjectContentDecodeRequestDto,
    ProjectContentDiagnosticCode, ProjectContentDiagnosticDto, ProjectContentDocumentDto,
    ProjectContentEncodeRequestDto,
};
use protocol_scene::FlatSceneDocumentDto;

/// Statically composed provider authority used during project-content
/// admission. Public wire requests never implement or populate this port.
pub trait ProjectContentGameplayAdmission: Send + Sync {
    fn configuration_schemas(&self) -> &[protocol_project_content::ProjectConfigurationSchemaDto];

    fn validate_gameplay(
        &self,
        documents: &[ProjectContentDocumentDto],
    ) -> Vec<ProjectContentDiagnosticDto>;
}

#[derive(Debug, Default)]
pub struct EmptyProjectContentGameplayAdmission {
    schemas: Vec<protocol_project_content::ProjectConfigurationSchemaDto>,
}

impl ProjectContentGameplayAdmission for EmptyProjectContentGameplayAdmission {
    fn configuration_schemas(&self) -> &[protocol_project_content::ProjectConfigurationSchemaDto] {
        &self.schemas
    }

    fn validate_gameplay(
        &self,
        documents: &[ProjectContentDocumentDto],
    ) -> Vec<ProjectContentDiagnosticDto> {
        documents
            .iter()
            .filter_map(|document| match document {
                ProjectContentDocumentDto::GameplayConfiguration { document_id, .. } => {
                    Some(ProjectContentDiagnosticDto {
                        code: ProjectContentDiagnosticCode::UnknownReference,
                        document_id: Some(document_id.clone()),
                        path: "document.configurations".to_owned(),
                        message:
                            "gameplay configuration requires a statically composed Rust provider"
                                .to_owned(),
                    })
                }
                _ => None,
            })
            .collect()
    }
}

pub struct ProjectContentValidationContext<'a> {
    pub scenes: &'a [FlatSceneDocumentDto],
    pub gameplay: &'a dyn ProjectContentGameplayAdmission,
    pub reference_revision: u64,
}

/// The only set accepted as current authoring state. Its fields are private so
/// callers cannot manufacture a current set from arbitrary documents/hashes.
#[derive(Debug, Clone)]
pub struct ValidatedProjectContentSet {
    result: ProjectContentCodecResultDto,
    reference_revision: u64,
}

impl ValidatedProjectContentSet {
    pub fn result(&self) -> &ProjectContentCodecResultDto {
        &self.result
    }

    pub fn set_hash(&self) -> &str {
        self.result
            .set_hash
            .as_deref()
            .expect("validated project-content set has an identity")
    }
}

pub struct ProjectContentValidationOutcome {
    pub result: ProjectContentCodecResultDto,
    pub validated: Option<ValidatedProjectContentSet>,
}

pub fn decode_project_content_sources(
    sources: &[protocol_project_content::ProjectContentSourceDto],
) -> Result<Vec<ProjectContentDocumentDto>, Box<ProjectContentCodecResultDto>> {
    codec::decode_sources(sources).map_err(|diagnostics| Box::new(rejected_codec(diagnostics)))
}

pub fn decode_project_content(
    request: ProjectContentDecodeRequestDto,
    context: ProjectContentValidationContext<'_>,
) -> ProjectContentValidationOutcome {
    match codec::decode_sources(&request.sources) {
        Ok(documents) => encode_documents(documents, context),
        Err(diagnostics) => outcome(rejected_codec(diagnostics)),
    }
}

pub fn encode_project_content(
    request: ProjectContentEncodeRequestDto,
    context: ProjectContentValidationContext<'_>,
) -> ProjectContentCodecResultDto {
    encode_documents(request.documents, context).result
}

pub fn apply_project_content_authoring(
    current: &ValidatedProjectContentSet,
    request: ProjectContentAuthoringRequestDto,
    context: ProjectContentValidationContext<'_>,
) -> (
    ProjectContentAuthoringResultDto,
    Option<ValidatedProjectContentSet>,
) {
    if current.set_hash() != request.expected_set_hash
        || current.reference_revision != context.reference_revision
    {
        let result = ProjectContentAuthoringResultDto {
            accepted: false,
            documents: Vec::new(),
            canonical_files: Vec::new(),
            set_hash: Some(current.set_hash().to_owned()),
            field_metadata: Vec::new(),
            diagnostics: vec![ProjectContentDiagnosticDto {
                code: ProjectContentDiagnosticCode::StaleRevision,
                document_id: None,
                path: "expectedSetHash".to_owned(),
                message: "project-content authoring targeted a stale document set".to_owned(),
            }],
        };
        return (result, None);
    }

    let mut documents = current.result.documents.clone();
    match request.command {
        ProjectContentAuthoringCommandDto::Upsert { document } => {
            let key = (document.kind(), document.document_id().to_owned());
            documents.retain(|current| (current.kind(), current.document_id().to_owned()) != key);
            documents.push(document);
        }
        ProjectContentAuthoringCommandDto::Delete {
            document_id,
            document_kind,
        } => {
            let before = documents.len();
            documents.retain(|document| {
                document.kind() != document_kind || document.document_id() != document_id
            });
            if documents.len() == before {
                let result = ProjectContentAuthoringResultDto {
                    accepted: false,
                    documents: Vec::new(),
                    canonical_files: Vec::new(),
                    set_hash: Some(current.set_hash().to_owned()),
                    field_metadata: Vec::new(),
                    diagnostics: vec![ProjectContentDiagnosticDto {
                        code: ProjectContentDiagnosticCode::UnknownReference,
                        document_id: Some(document_id),
                        path: "command.documentId".to_owned(),
                        message: "delete targeted an unknown project-content document".to_owned(),
                    }],
                };
                return (result, None);
            }
        }
    }
    let encoded = encode_documents(documents, context);
    let result = authoring_from_codec(encoded.result.clone());
    (result, encoded.validated)
}

fn encode_documents(
    mut documents: Vec<ProjectContentDocumentDto>,
    context: ProjectContentValidationContext<'_>,
) -> ProjectContentValidationOutcome {
    documents.sort_by(|left, right| {
        (left.kind() as u8, left.document_id()).cmp(&(right.kind() as u8, right.document_id()))
    });
    let mut identities = BTreeMap::new();
    let mut duplicate_diagnostics = Vec::new();
    for document in &documents {
        let key = (document.kind() as u8, document.document_id().to_owned());
        if document.document_id().trim().is_empty() || identities.insert(key, ()).is_some() {
            duplicate_diagnostics.push(ProjectContentDiagnosticDto {
                code: ProjectContentDiagnosticCode::DuplicateDocument,
                document_id: Some(document.document_id().to_owned()),
                path: "documents".to_owned(),
                message: "document ids must be non-empty and unique within each kind".to_owned(),
            });
        }
    }
    if !duplicate_diagnostics.is_empty() {
        return outcome(rejected_codec(duplicate_diagnostics));
    }

    let mut diagnostics = validate::validate_document_set(
        &documents,
        context.scenes,
        context.gameplay.configuration_schemas(),
    );
    diagnostics.extend(context.gameplay.validate_gameplay(&documents));
    if !diagnostics.is_empty() {
        return outcome(rejected_codec(diagnostics));
    }
    match codec::canonical_files(&documents) {
        Ok(canonical_files) => {
            let set_hash = Some(codec::document_set_hash(&canonical_files));
            let field_metadata =
                validate::field_metadata(&documents, context.gameplay.configuration_schemas());
            let result = ProjectContentCodecResultDto {
                accepted: true,
                documents,
                canonical_files,
                set_hash,
                field_metadata,
                diagnostics: Vec::new(),
            };
            ProjectContentValidationOutcome {
                validated: Some(ValidatedProjectContentSet {
                    result: result.clone(),
                    reference_revision: context.reference_revision,
                }),
                result,
            }
        }
        Err(diagnostics) => outcome(rejected_codec(diagnostics)),
    }
}

fn outcome(result: ProjectContentCodecResultDto) -> ProjectContentValidationOutcome {
    ProjectContentValidationOutcome {
        result,
        validated: None,
    }
}

fn rejected_codec(diagnostics: Vec<ProjectContentDiagnosticDto>) -> ProjectContentCodecResultDto {
    ProjectContentCodecResultDto {
        accepted: false,
        documents: Vec::new(),
        canonical_files: Vec::new(),
        set_hash: None,
        field_metadata: Vec::new(),
        diagnostics,
    }
}

fn authoring_from_codec(result: ProjectContentCodecResultDto) -> ProjectContentAuthoringResultDto {
    ProjectContentAuthoringResultDto {
        accepted: result.accepted,
        documents: result.documents,
        canonical_files: result.canonical_files,
        set_hash: result.set_hash,
        field_metadata: result.field_metadata,
        diagnostics: result.diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_project_content_authoring, decode_project_content, ProjectContentGameplayAdmission,
        ProjectContentValidationContext, ProjectContentValidationOutcome,
    };
    use core_ids::{SceneId, SceneNodeId};
    use protocol_project_content::*;
    use protocol_scene::{
        FlatSceneDocumentDto, SceneEntityInstanceDto, SceneEntityReferenceDto, SceneMetadataDto,
        SceneNodeKindDto, SceneNodeRecordDto, SceneTransformDto,
    };

    fn source(
        document_id: &str,
        kind: ProjectContentDocumentKind,
        source_text: &str,
    ) -> ProjectContentSourceDto {
        ProjectContentSourceDto {
            document_id: document_id.to_owned(),
            kind,
            source_text: source_text.to_owned(),
        }
    }

    fn scene() -> FlatSceneDocumentDto {
        let node = |id, instance_id: &str, reference| SceneNodeRecordDto {
            id: SceneNodeId::new(id),
            parent: None,
            child_order: id as u32,
            label: None,
            tags: Vec::new(),
            transform: SceneTransformDto {
                translation: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            },
            kind: SceneNodeKindDto::EntityInstance {
                instance: SceneEntityInstanceDto {
                    instance_id: instance_id.to_owned(),
                    reference,
                    spawn_marker_id: None,
                },
            },
        };
        FlatSceneDocumentDto {
            schema_version: 4,
            id: SceneId::new(41),
            metadata: SceneMetadataDto {
                name: Some("Reference room".to_owned()),
                authoring_format_version: 4,
            },
            dependencies: Vec::new(),
            nodes: vec![
                node(
                    1,
                    "reference.trigger.instance",
                    SceneEntityReferenceDto::EntityDefinition {
                        stable_id: "reference.trigger".to_owned(),
                    },
                ),
                node(
                    2,
                    "reference.console.blue",
                    SceneEntityReferenceDto::Prefab {
                        prefab_id: 70,
                        variant_id: Some("blue".to_owned()),
                        instantiation_seed: 11,
                    },
                ),
            ],
        }
    }

    fn marker_scene(
        scene_id: u64,
        marker_id: Option<&str>,
        spawn_marker_id: Option<&str>,
    ) -> FlatSceneDocumentDto {
        let mut nodes = Vec::new();
        if let Some(marker_id) = marker_id {
            nodes.push(SceneNodeRecordDto {
                id: SceneNodeId::new(1),
                parent: None,
                child_order: 0,
                label: None,
                tags: Vec::new(),
                transform: SceneTransformDto {
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
                kind: SceneNodeKindDto::Marker {
                    marker_id: marker_id.to_owned(),
                },
            });
        }
        if let Some(spawn_marker_id) = spawn_marker_id {
            nodes.push(SceneNodeRecordDto {
                id: SceneNodeId::new(2),
                parent: None,
                child_order: 1,
                label: None,
                tags: Vec::new(),
                transform: SceneTransformDto {
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
                kind: SceneNodeKindDto::EntityInstance {
                    instance: SceneEntityInstanceDto {
                        instance_id: format!("scene.{scene_id}.instance"),
                        reference: SceneEntityReferenceDto::EntityDefinition {
                            stable_id: "reference.console".to_owned(),
                        },
                        spawn_marker_id: Some(spawn_marker_id.to_owned()),
                    },
                },
            });
        }
        FlatSceneDocumentDto {
            schema_version: 4,
            id: SceneId::new(scene_id),
            metadata: SceneMetadataDto {
                name: Some(format!("Marker scene {scene_id}")),
                authoring_format_version: 4,
            },
            dependencies: Vec::new(),
            nodes,
        }
    }

    struct FixtureAdmission {
        schemas: Vec<ProjectConfigurationSchemaDto>,
    }

    impl ProjectContentGameplayAdmission for FixtureAdmission {
        fn configuration_schemas(&self) -> &[ProjectConfigurationSchemaDto] {
            &self.schemas
        }

        fn validate_gameplay(
            &self,
            _documents: &[ProjectContentDocumentDto],
        ) -> Vec<ProjectContentDiagnosticDto> {
            Vec::new()
        }
    }

    fn admission() -> FixtureAdmission {
        FixtureAdmission {
            schemas: vec![ProjectConfigurationSchemaDto {
                schema_id: "reference.primary-action.v1".to_owned(),
                provider_id: "provider.reference.primary-action".to_owned(),
                contract: protocol_game_extension::GameplayContractRef {
                    namespace: "reference.primary-action".to_owned(),
                    name: "configuration".to_owned(),
                    version: 1,
                    schema_hash: "fnv1a64:config".to_owned(),
                },
                codec_id: "asha.project-configuration.canonical-json.v1".to_owned(),
                fields: vec![ProjectConfigurationFieldDto {
                    field_id: "cooldownTicks".to_owned(),
                    label: "Cooldown ticks".to_owned(),
                    value_kind: ProjectConfigurationValueKind::Integer,
                    required: true,
                    reference_kind: None,
                    integer_min: Some(0),
                    integer_max: Some(120),
                    number_min: None,
                    number_max: None,
                }],
            }],
        }
    }

    fn decode(request: ProjectContentDecodeRequestDto) -> ProjectContentValidationOutcome {
        let scenes = vec![scene()];
        let admission = admission();
        decode_project_content(
            request,
            ProjectContentValidationContext {
                scenes: &scenes,
                gameplay: &admission,
                reference_revision: 0,
            },
        )
    }

    fn request() -> ProjectContentDecodeRequestDto {
        ProjectContentDecodeRequestDto {
            sources: vec![
                source(
                    "entities/reference-trigger.json",
                    ProjectContentDocumentKind::EntityDefinition,
                    r#"{
                      "kind":"EntityDefinition",
                      "stableId":"reference.trigger",
                      "displayName":"Reference Trigger",
                      "source":{"projectBundle":"reference-project","relativePath":"entities/reference-trigger.json"},
                      "tags":[],
                      "metadata":[],
                      "capabilities":[
                        {"kind":"bounds","min":[-1,-1,-1],"max":[1,1,1]},
                        {"kind":"collision","staticCollider":false}
                      ]
                    }"#,
                ),
                source(
                    "entities/reference-console.json",
                    ProjectContentDocumentKind::EntityDefinition,
                    r#"{
                      "kind":"EntityDefinition",
                      "stableId":"reference.console",
                      "displayName":"Reference Console",
                      "source":{"projectBundle":"reference-project","relativePath":"entities/reference-console.json"},
                      "tags":[],"metadata":[],
                      "capabilities":[{"kind":"render","visible":true}]
                    }"#,
                ),
                source(
                    "catalogs/reference-assets.json",
                    ProjectContentDocumentKind::AssetCatalog,
                    r#"{
                      "entries":[
                        {"id":"audio/reference-confirm","version":1,"hash":null,"sourcePath":"assets/confirm.wav","label":"Confirm","dependencies":[],"material":null},
                        {"id":"mesh/reference-character","version":1,"hash":null,"sourcePath":"assets/character.glb","label":"Character","dependencies":[],"material":null}
                      ]
                    }"#,
                ),
                source(
                    "prefabs/reference-registry.json",
                    ProjectContentDocumentKind::PrefabRegistry,
                    r#"{
                      "schemaVersion":1,
                      "definitions":[{
                        "id":70,"schemaVersion":1,"displayName":"Reference Console",
                        "parts":[{"id":1,"namespace":"body","displayName":"Body","parent":null,"transform":{"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1]},"source":{"kind":"entityDefinition","stableId":"reference.console"}}],
                        "partRoles":[{"role":"interaction/body","part":1}],"variant":null
                      },{
                        "id":71,"schemaVersion":1,"displayName":"Reference Console Blue",
                        "parts":[],"partRoles":[],
                        "variant":{"variantId":"blue","base":70,"removedRoles":[],"overrides":[]}
                      }]
                    }"#,
                ),
                source(
                    "gameplay/reference-config.json",
                    ProjectContentDocumentKind::GameplayConfiguration,
                    r#"{
                      "schemaVersion":1,
                      "configurations":[{
                        "configurationId":"reference.primary-action.default",
                        "module":{"moduleId":"reference.primary-action","namespace":"reference.primary-action","version":"0.1.0","sdkHash":"fnv1a64:sdk","contractHash":"fnv1a64:contract","artifactHash":"fnv1a64:artifact","providerId":"provider.reference.primary-action"},
                        "schemaId":"reference.primary-action.v1",
                        "values":[{"fieldId":"cooldownTicks","value":{"kind":"integer","value":4}}]
                      }],
                      "bindings":[{
                        "bindingId":"reference.console.binding","moduleId":"reference.primary-action","configurationId":"reference.primary-action.default",
                        "stateSchema":{"namespace":"reference.primary-action","name":"state","version":1,"schemaHash":"fnv1a64:state"},
                        "target":{"kind":"prefabPart","part":{"prefab":70,"role":"interaction/body"}},
                        "requiredReads":[],"outputContracts":[],"enabled":true
                      }],
                      "overrides":[{"bindingId":"reference.console.binding","sceneInstanceId":"reference.console.blue","configurationId":null,"enabled":null}],
                      "triggers":[{"schemaVersion":2,"sceneInstanceId":"reference.trigger.instance","scope":"reference.nearby","tags":["reference"]}]
                    }"#,
                ),
                source(
                    "presentation/reference-cues.json",
                    ProjectContentDocumentKind::PresentationCatalog,
                    r#"{
                      "schemaVersion":1,
                      "resources":[{"resourceId":"reference.confirm.audio","kind":"audio","assetId":"audio/reference-confirm","sourcePath":"assets/confirm.wav","contentHash":"sha256:reference","licensePath":null,"clipIds":[]}],
                      "cues":[{"kind":"audio","cueId":"reference.confirm","resourceId":"reference.confirm.audio","gain":0.8}]
                    }"#,
                ),
            ],
        }
    }

    #[test]
    fn demo_shaped_documents_decode_validate_and_reopen_as_a_canonical_set() {
        let decoded = decode(request());
        assert!(decoded.result.accepted, "{:?}", decoded.result.diagnostics);
        assert_eq!(decoded.result.documents.len(), 6);
        assert_eq!(decoded.result.canonical_files.len(), 6);
        assert!(decoded
            .result
            .field_metadata
            .iter()
            .any(|field| field.path == "configurationValues.cooldownTicks"));

        let reopened = decode(ProjectContentDecodeRequestDto {
            sources: decoded
                .result
                .canonical_files
                .iter()
                .map(|file| ProjectContentSourceDto {
                    document_id: file.document_id.clone(),
                    kind: file.kind,
                    source_text: file.canonical_json.clone(),
                })
                .collect(),
        });
        assert!(
            reopened.result.accepted,
            "{:?}",
            reopened.result.diagnostics
        );
        assert_eq!(reopened.result.set_hash, decoded.result.set_hash);
    }

    #[test]
    fn strict_decode_rejects_unknown_nested_fields() {
        let result = decode(ProjectContentDecodeRequestDto {
            sources: vec![source(
                "entities/invalid.json",
                ProjectContentDocumentKind::EntityDefinition,
                r#"{"kind":"EntityDefinition","stableId":"reference.invalid","displayName":"Invalid","source":{"projectBundle":"reference","relativePath":"invalid.json","browserAccepted":true},"tags":[],"metadata":[],"capabilities":[]}"#,
            )],
        });
        assert!(!result.result.accepted);
        assert_eq!(
            result.result.diagnostics[0].code,
            ProjectContentDiagnosticCode::UnknownField
        );
    }

    #[test]
    fn project_files_cannot_redefine_provider_schemas() {
        let result = decode(ProjectContentDecodeRequestDto {
            sources: vec![source(
                "gameplay/invalid-provider-schema.json",
                ProjectContentDocumentKind::GameplayConfiguration,
                r#"{
                  "schemaVersion":1,"schemas":[],"configurations":[],
                  "bindings":[],"overrides":[],"triggers":[]
                }"#,
            )],
        });
        assert!(!result.result.accepted);
        assert_eq!(
            result.result.diagnostics[0].code,
            ProjectContentDiagnosticCode::UnknownField
        );
    }

    #[test]
    fn scene_variant_ids_resolve_against_named_registry_variants() {
        let request = request();
        let mut scenes = vec![scene()];
        let SceneNodeKindDto::EntityInstance { instance } = &mut scenes[0].nodes[1].kind else {
            panic!("fixture node is not an entity instance");
        };
        let SceneEntityReferenceDto::Prefab { variant_id, .. } = &mut instance.reference else {
            panic!("fixture node is not a prefab instance");
        };
        *variant_id = Some("missing".to_owned());

        let admission = admission();
        let result = decode_project_content(
            request,
            ProjectContentValidationContext {
                scenes: &scenes,
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(!result.result.accepted);
        assert!(result.result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == ProjectContentDiagnosticCode::UnknownReference
                && diagnostic.path.ends_with("variantId")
        }));
    }

    #[test]
    fn marker_ids_are_scene_local_and_never_resolve_across_scenes() {
        let admission = admission();
        let valid_scenes = vec![
            scene(),
            marker_scene(42, Some("shared.spawn"), None),
            marker_scene(43, Some("shared.spawn"), None),
        ];
        let valid = decode_project_content(
            request(),
            ProjectContentValidationContext {
                scenes: &valid_scenes,
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(valid.result.accepted, "{:?}", valid.result.diagnostics);

        let cross_scene = vec![
            scene(),
            marker_scene(42, Some("only.in.scene.42"), None),
            marker_scene(43, None, Some("only.in.scene.42")),
        ];
        let rejected = decode_project_content(
            request(),
            ProjectContentValidationContext {
                scenes: &cross_scene,
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(!rejected.result.accepted);
        assert!(rejected.result.diagnostics.iter().any(|diagnostic| {
            diagnostic.path.ends_with("spawnMarkerId")
                && diagnostic.code == ProjectContentDiagnosticCode::UnknownReference
        }));
    }

    #[test]
    fn stale_authoring_rejects_before_returning_a_save_candidate() {
        let decoded = decode(request());
        let scenes = vec![scene()];
        let admission = admission();
        let (result, next) = apply_project_content_authoring(
            decoded.validated.as_ref().unwrap(),
            ProjectContentAuthoringRequestDto {
                expected_workspace_id: "workspace/reference".to_owned(),
                expected_generation: 1,
                expected_working_revision: 0,
                expected_set_hash: "fnv1a64:stale".to_owned(),
                command: ProjectContentAuthoringCommandDto::Delete {
                    document_id: "presentation/reference-cues.json".to_owned(),
                    document_kind: ProjectContentDocumentKind::PresentationCatalog,
                },
            },
            ProjectContentValidationContext {
                scenes: &scenes,
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(!result.accepted);
        assert!(next.is_none());
        assert!(result.canonical_files.is_empty());
        assert_eq!(
            result.diagnostics[0].code,
            ProjectContentDiagnosticCode::StaleRevision
        );
    }

    #[test]
    fn typed_authoring_returns_a_canonical_reopenable_candidate() {
        let decoded = decode(request());
        assert!(decoded.result.accepted, "{:?}", decoded.result.diagnostics);
        let expected_set_hash = decoded.result.set_hash.clone().expect("accepted set hash");
        let mut changed = decoded
            .result
            .documents
            .iter()
            .find_map(|document| match document {
                ProjectContentDocumentDto::PresentationCatalog {
                    document_id,
                    catalog,
                } => Some((document_id.clone(), catalog.clone())),
                _ => None,
            })
            .expect("presentation catalog");
        changed.1.cues[0] = ProjectPresentationCueDto::Audio {
            cue_id: "reference.confirm".to_owned(),
            resource_id: "reference.confirm.audio".to_owned(),
            gain: 0.65,
        };

        let scenes = vec![scene()];
        let admission = admission();
        let (authored, next) = apply_project_content_authoring(
            decoded.validated.as_ref().unwrap(),
            ProjectContentAuthoringRequestDto {
                expected_workspace_id: "workspace/reference".to_owned(),
                expected_generation: 1,
                expected_working_revision: 0,
                expected_set_hash,
                command: ProjectContentAuthoringCommandDto::Upsert {
                    document: ProjectContentDocumentDto::PresentationCatalog {
                        document_id: changed.0,
                        catalog: changed.1,
                    },
                },
            },
            ProjectContentValidationContext {
                scenes: &scenes,
                gameplay: &admission,
                reference_revision: 0,
            },
        );
        assert!(authored.accepted, "{:?}", authored.diagnostics);
        assert!(next.is_some());
        assert_ne!(authored.set_hash, decoded.result.set_hash);

        let reopened = decode(ProjectContentDecodeRequestDto {
            sources: authored
                .canonical_files
                .iter()
                .map(|file| ProjectContentSourceDto {
                    document_id: file.document_id.clone(),
                    kind: file.kind,
                    source_text: file.canonical_json.clone(),
                })
                .collect(),
        });
        assert!(
            reopened.result.accepted,
            "{:?}",
            reopened.result.diagnostics
        );
        assert_eq!(reopened.result.set_hash, authored.set_hash);
    }
}
