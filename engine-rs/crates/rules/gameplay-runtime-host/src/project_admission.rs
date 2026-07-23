//! Rust-owned linking of a manifest source closure into one opaque runtime admission.

use std::collections::{BTreeMap, BTreeSet};

use core_ids::RuntimeSessionId;
use core_scene::{BootstrapPlan, FlatSceneDocument, SceneEntityReference, SceneNodeKind};
use gameplay_module_sdk::{
    gameplay_module_payload_hash, GameplayCapabilityReadKind, GameplayEventEntityBinding,
    GameplayModuleBindingRegistryBuilder, GameplayModuleStateScope, GameplayOwnerQuery,
    GameplayReadRequest, GameplayReadSelector, GameplayRelationshipReadKind,
    GameplayRuntimeDeclaredReadPlan, GameplayStaticComposition,
};
use protocol_entity_authoring::{EntityDefinition, EntityDefinitionCapability};
use protocol_game_extension::{
    GameplayContractRef, GameplayModuleBindingRegistry, GameplayOwnerRef,
};
use protocol_project_content::{
    ProjectContentCodecResultDto, ProjectContentDiagnosticCode, ProjectContentDiagnosticDto,
    ProjectContentDocumentDto,
};
use protocol_voxel_asset::VoxelVolumeAsset;
use rule_project_bundle::{
    BundleArtifacts, GameplayBindingEntityTargets, GameplayProjectContentAdmission,
};
use svc_project_content::{
    decode_project_content_artifact, project_scene_document_dto,
    validate_project_content_documents, ProjectContentValidationContext,
    ValidatedProjectContentSet,
};
use svc_serialization::{
    encode_prefab_registry, AdmittedRuntimeProjectSourceBatch, ArtifactRole, BundleHash, LoadPlan,
};

use crate::{
    authored_behavior::{compile_authored_program, CompiledAuthoredProgram},
    GameplayRuntimePrefabBootstrap, GameplayRuntimePrefabCatalog, GameplayRuntimePrefabPlacement,
    GameplayRuntimePrefabPlacementOrigin, GameplayRuntimePrefabTransform,
    GameplayRuntimeSchedulerDefinition, GameplayRuntimeSpatialEntity,
};

#[path = "project_admission_read_identity.rs"]
mod read_identity;
#[path = "project_admission_voxel.rs"]
mod voxel;

/// Stable diagnostic categories for failures at the project linker boundary.
/// Paths and messages remain detailed, but callers never need to parse them to
/// distinguish a broken stored reference from provider or resource failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeProjectAdmissionDiagnosticCode {
    SceneDecode,
    SceneInvalid,
    SceneIdentityMismatch,
    ProjectContentDecode,
    ProjectContentInvalid,
    ArtifactRoleMismatch,
    AmbiguousReference,
    CrossSceneReference,
    DanglingReference,
    DuplicateTrigger,
    ProviderSchemaMismatch,
    ResourceNotStaged,
    ResourceHashMismatch,
    ResourceDecode,
    RuntimeGeneratorDependency,
    BootstrapLink,
}

impl RuntimeProjectAdmissionDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SceneDecode => "sceneDecode",
            Self::SceneInvalid => "sceneInvalid",
            Self::SceneIdentityMismatch => "sceneIdentityMismatch",
            Self::ProjectContentDecode => "projectContentDecode",
            Self::ProjectContentInvalid => "projectContentInvalid",
            Self::ArtifactRoleMismatch => "artifactRoleMismatch",
            Self::AmbiguousReference => "ambiguousReference",
            Self::CrossSceneReference => "crossSceneReference",
            Self::DanglingReference => "danglingReference",
            Self::DuplicateTrigger => "duplicateTrigger",
            Self::ProviderSchemaMismatch => "providerSchemaMismatch",
            Self::ResourceNotStaged => "resourceNotStaged",
            Self::ResourceHashMismatch => "resourceHashMismatch",
            Self::ResourceDecode => "resourceDecode",
            Self::RuntimeGeneratorDependency => "runtimeGeneratorDependency",
            Self::BootstrapLink => "bootstrapLink",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProjectAdmissionDiagnostic {
    pub code: RuntimeProjectAdmissionDiagnosticCode,
    pub document_id: Option<String>,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeProjectAdmissionReport {
    pub diagnostics: Vec<RuntimeProjectAdmissionDiagnostic>,
}

impl RuntimeProjectAdmissionReport {
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }

    fn push(
        &mut self,
        code: RuntimeProjectAdmissionDiagnosticCode,
        document_id: Option<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.diagnostics.push(RuntimeProjectAdmissionDiagnostic {
            code,
            document_id,
            path: path.into(),
            message: message.into(),
        });
    }

    fn canonicalize(&mut self) {
        self.diagnostics.sort_by(|left, right| {
            (
                left.code,
                left.document_id.as_deref().unwrap_or_default(),
                left.path.as_str(),
                left.message.as_str(),
            )
                .cmp(&(
                    right.code,
                    right.document_id.as_deref().unwrap_or_default(),
                    right.path.as_str(),
                    right.message.as_str(),
                ))
        });
        self.diagnostics.dedup();
    }
}

/// Private plan obtainable only through [`compile_runtime_project_admission`].
pub struct ValidatedRuntimeProjectAdmission {
    source: AdmittedRuntimeProjectSourceBatch,
    scenes: BTreeMap<u64, FlatSceneDocument>,
    content: ValidatedProjectContentSet,
    composition: GameplayStaticComposition,
    load_plan: LoadPlan,
    bootstrap_resolution: core_scene::BootstrapResolutionContext,
    bindings: GameplayModuleBindingRegistry,
    entity_targets: GameplayBindingEntityTargets,
    spatial_entities: Vec<GameplayRuntimeSpatialEntity>,
    runtime_entity_seeds: Vec<RuntimeProjectEntitySeed>,
    declared_reads: Vec<GameplayRuntimeDeclaredReadPlan>,
    triggers: Vec<protocol_project_bundle::GameplayTriggerDefinition>,
    scheduler: GameplayRuntimeSchedulerDefinition,
    prefabs: GameplayRuntimePrefabBootstrap,
    artifacts: BundleArtifacts,
    voxel_assets: BTreeMap<String, VoxelVolumeAsset>,
    admission_hash: String,
    authored_program: Option<CompiledAuthoredProgram>,
}

pub(crate) struct RuntimeProjectActivationParts {
    pub manifest_hash: BundleHash,
    pub project_id: u64,
    pub entry_scene: FlatSceneDocument,
    pub content_readout: ProjectContentCodecResultDto,
    pub load_plan: LoadPlan,
    pub artifacts: BundleArtifacts,
    pub bootstrap_resolution: core_scene::BootstrapResolutionContext,
    pub composition: GameplayStaticComposition,
    pub bindings: GameplayModuleBindingRegistry,
    pub entity_targets: GameplayBindingEntityTargets,
    pub spatial_entities: Vec<GameplayRuntimeSpatialEntity>,
    pub runtime_entity_seeds: Vec<RuntimeProjectEntitySeed>,
    pub declared_reads: Vec<GameplayRuntimeDeclaredReadPlan>,
    pub triggers: Vec<protocol_project_bundle::GameplayTriggerDefinition>,
    pub scheduler: GameplayRuntimeSchedulerDefinition,
    pub prefabs: GameplayRuntimePrefabBootstrap,
    pub voxel_assets: BTreeMap<String, VoxelVolumeAsset>,
    pub admission_hash: String,
    pub authored_program: Option<CompiledAuthoredProgram>,
}

/// Internal typed handoff from canonical scene/bootstrap admission to domain
/// authority. It carries only validated stored meaning and resolved instance
/// identity; no downstream runtime IDs, hashes, or registries can be supplied.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeProjectEntitySeed {
    pub entity: core_ids::EntityId,
    pub instance_id: String,
    pub document_id: String,
    pub source_path: String,
    pub spawn_marker_id: Option<String>,
    pub world_translation: [f32; 3],
    pub world_rotation: [f32; 4],
    pub world_scale: [f32; 3],
    pub definition: EntityDefinition,
}

impl ValidatedRuntimeProjectAdmission {
    pub fn admission_hash(&self) -> &str {
        &self.admission_hash
    }

    pub fn manifest_hash(&self) -> BundleHash {
        self.source.manifest_hash()
    }

    pub fn project_content_set_hash(&self) -> &str {
        self.content.set_hash()
    }

    pub fn composition_registry_digest(&self) -> &str {
        self.composition.registry().registry_digest()
    }

    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }

    pub fn declared_read_plan_count(&self) -> usize {
        self.declared_reads.len()
    }

    /// Inspectable identity of the private activation plan. This is evidence,
    /// not an editable serialization surface; callers cannot recover or mutate
    /// any compiled field from it.
    pub fn compiled_plan_hash(&self) -> String {
        let authored_program = self
            .authored_program
            .as_ref()
            .map(|plan| serde_json::to_vec(plan).expect("validated authored behavior serializes"))
            .unwrap_or_default();
        gameplay_module_payload_hash(
            format!(
                "{:?}|{:?}|{}|{:?}|{:?}|{:?}|{:?}|{}|{}|{}",
                self.load_plan,
                self.bootstrap_resolution,
                self.bindings.registry_hash,
                self.entity_targets,
                self.spatial_entities,
                self.triggers,
                self.scheduler,
                self.prefabs.registry_json,
                self.prefabs.placements.len(),
                gameplay_module_payload_hash(&authored_program),
            )
            .as_bytes(),
        )
    }

    pub(crate) fn into_activation_parts(self) -> RuntimeProjectActivationParts {
        let entry_scene_id = self.source.manifest().entry_scene.raw();
        let entry_scene = self
            .scenes
            .get(&entry_scene_id)
            .expect("validated admission retains its entry scene")
            .clone();
        RuntimeProjectActivationParts {
            manifest_hash: self.source.manifest_hash(),
            project_id: self.source.manifest().project.id.raw(),
            entry_scene,
            content_readout: self.content.result().clone(),
            load_plan: self.load_plan,
            artifacts: self.artifacts,
            bootstrap_resolution: self.bootstrap_resolution,
            composition: self.composition,
            bindings: self.bindings,
            entity_targets: self.entity_targets,
            spatial_entities: self.spatial_entities,
            runtime_entity_seeds: self.runtime_entity_seeds,
            declared_reads: self.declared_reads,
            triggers: self.triggers,
            scheduler: self.scheduler,
            prefabs: self.prefabs,
            voxel_assets: self.voxel_assets,
            admission_hash: self.admission_hash,
            authored_program: self.authored_program,
        }
    }
}

/// Strictly decode and link one already hash/closure-admitted source batch
/// against the immutable static gameplay provider composition.
pub fn compile_runtime_project_admission(
    source: AdmittedRuntimeProjectSourceBatch,
    composition: GameplayStaticComposition,
) -> Result<ValidatedRuntimeProjectAdmission, RuntimeProjectAdmissionReport> {
    let mut report = RuntimeProjectAdmissionReport::default();
    check_declared_read_topology(&composition, &mut report);
    let scenes = decode_scenes(&source, &mut report);
    check_cross_scene_markers(&scenes, &mut report);
    let voxel_assets = voxel::decode_voxel_assets(&source, &mut report);

    let mut documents = Vec::new();
    let mut entity_definition_source_paths = BTreeMap::new();
    for artifact in source.manifest().artifacts.iter().filter(|artifact| {
        matches!(
            artifact.role,
            ArtifactRole::ProjectContent
                | ArtifactRole::PrefabRegistry
                | ArtifactRole::EntityDefinitionCatalog
                | ArtifactRole::MaterialCatalog
        )
    }) {
        let Some(body) = source.body(&artifact.path) else {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::ProjectContentDecode,
                None,
                &artifact.path,
                "manifest project-content body was not retained in the admitted closure",
            );
            continue;
        };
        match decode_project_content_artifact(&artifact.path, body) {
            Ok(document) if artifact_role_accepts_document(&artifact.role, &document) => {
                if matches!(
                    &document,
                    ProjectContentDocumentDto::EntityDefinition { .. }
                ) {
                    entity_definition_source_paths
                        .insert(document.document_id().to_owned(), artifact.path.clone());
                }
                documents.push(document);
            }
            Ok(document) => report.push(
                RuntimeProjectAdmissionDiagnosticCode::ArtifactRoleMismatch,
                Some(document.document_id().to_owned()),
                &artifact.path,
                format!(
                    "manifest role `{}` does not accept project-content kind {:?}",
                    artifact.role.tag(),
                    document.kind()
                ),
            ),
            Err(diagnostic) => report.diagnostics.push(map_project_diagnostic(diagnostic)),
        }
    }

    check_runtime_resource_closure(&source, &scenes, &documents, &mut report);
    voxel::check_voxel_asset_links(&source, &scenes, &documents, &voxel_assets, &mut report);
    check_scene_references(&scenes, &documents, &mut report);
    check_entry_scene_targets(&source, &scenes, &documents, &mut report);
    if !report.is_valid() {
        report.canonicalize();
        return Err(report);
    }

    let scene_dtos = scenes
        .values()
        .map(project_scene_document_dto)
        .collect::<Vec<_>>();
    let gameplay =
        GameplayProjectContentAdmission::new(composition.project_configuration_authority());
    let outcome = validate_project_content_documents(
        documents,
        ProjectContentValidationContext {
            scenes: &scene_dtos,
            entry_scene_id: Some(source.manifest().entry_scene),
            gameplay: &gameplay,
            reference_revision: 0,
        },
    );
    let Some(content) = outcome.validated else {
        report.diagnostics.extend(
            outcome
                .result
                .diagnostics
                .into_iter()
                .map(map_project_diagnostic),
        );
        report.canonicalize();
        return Err(report);
    };

    let bootstrap_resolution = derive_bootstrap_resolution(&content);
    let entry_scene_id = source.manifest().entry_scene.raw();
    let entry_scene = scenes
        .get(&entry_scene_id)
        .expect("admitted manifest entry scene was decoded above");
    if entry_scene.nodes.iter().any(|node| {
        matches!(&node.kind, SceneNodeKind::Bootstrap(bindings) if bindings.generator.is_some())
    }) {
        report.push(
            RuntimeProjectAdmissionDiagnosticCode::RuntimeGeneratorDependency,
            None,
            format!("scenes.{entry_scene_id}.bootstrap.generator"),
            "materialized runtime scenes may retain authoring provenance in the manifest but may not require a generator provider",
        );
    }
    let bootstrap = BootstrapPlan::prepare_resolved(
        entry_scene,
        RuntimeSessionId::new(source.manifest().project.id.raw()),
        &bootstrap_resolution,
    );
    let bootstrap = match bootstrap {
        Ok(bootstrap) => bootstrap,
        Err(error) => {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::BootstrapLink,
                None,
                format!("scenes.{entry_scene_id}.bootstrap"),
                format!("entry-scene bootstrap identities did not link: {error:?}"),
            );
            report.canonicalize();
            return Err(report);
        }
    };

    let (entity_targets, spatial_entities, runtime_entity_seeds) = derive_entity_authority(
        &content,
        &entity_definition_source_paths,
        &bootstrap,
        &mut report,
    );
    let prefabs = derive_prefab_bootstrap(&content, &bootstrap, &mut report);
    let authored_program = match compile_authored_program(&content, &prefabs, &runtime_entity_seeds)
    {
        Ok(program) => program,
        Err(message) => {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::ProviderSchemaMismatch,
                None,
                "projectContent.authoredBehavior",
                message,
            );
            None
        }
    };
    let bindings = compiled_bindings(&content);
    if !report.is_valid() {
        report.canonicalize();
        return Err(report);
    }
    let declared_reads = composition.declared_reads().to_vec();
    let scheduler = derive_scheduler(&composition, authored_program.is_some());
    let triggers = content.compiled_gameplay().triggers().to_vec();
    let load_plan = match LoadPlan::build(source.manifest()) {
        Ok(plan) => plan,
        Err(error) => {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::BootstrapLink,
                None,
                "manifest.loadPlan",
                format!("canonical load plan could not be derived: {error}"),
            );
            report.canonicalize();
            return Err(report);
        }
    };
    let artifacts = voxel::bundle_artifacts(&source, &mut report);
    if !report.is_valid() {
        report.canonicalize();
        return Err(report);
    }
    let admission_hash = runtime_admission_hash(
        &source,
        &scenes,
        &content,
        composition.registry().registry_digest(),
        &bindings,
        &declared_reads,
    );

    Ok(ValidatedRuntimeProjectAdmission {
        source,
        scenes,
        content,
        composition,
        load_plan,
        bootstrap_resolution,
        bindings,
        entity_targets,
        spatial_entities,
        runtime_entity_seeds,
        declared_reads,
        triggers,
        scheduler,
        prefabs,
        artifacts,
        voxel_assets,
        admission_hash,
        authored_program,
    })
}

fn artifact_role_accepts_document(
    role: &ArtifactRole,
    document: &ProjectContentDocumentDto,
) -> bool {
    match role {
        ArtifactRole::ProjectContent => true,
        ArtifactRole::PrefabRegistry => {
            matches!(document, ProjectContentDocumentDto::PrefabRegistry { .. })
        }
        ArtifactRole::EntityDefinitionCatalog => {
            matches!(document, ProjectContentDocumentDto::EntityDefinition { .. })
        }
        ArtifactRole::MaterialCatalog => {
            matches!(document, ProjectContentDocumentDto::AssetCatalog { .. })
        }
        _ => false,
    }
}

fn decode_scenes(
    source: &AdmittedRuntimeProjectSourceBatch,
    report: &mut RuntimeProjectAdmissionReport,
) -> BTreeMap<u64, FlatSceneDocument> {
    let mut scenes = BTreeMap::new();
    for scene in &source.manifest().scenes {
        let Some(body) = source.body(&scene.artifact) else {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::SceneDecode,
                None,
                &scene.artifact,
                "manifest scene body was not retained in the admitted closure",
            );
            continue;
        };
        let text = match std::str::from_utf8(body) {
            Ok(text) => text,
            Err(error) => {
                report.push(
                    RuntimeProjectAdmissionDiagnosticCode::SceneDecode,
                    None,
                    &scene.artifact,
                    format!("scene body is not UTF-8: {error}"),
                );
                continue;
            }
        };
        let document = match core_scene::decode(text) {
            Ok(document) => document.canonical(),
            Err(error) => {
                report.push(
                    RuntimeProjectAdmissionDiagnosticCode::SceneDecode,
                    None,
                    &scene.artifact,
                    format!("scene could not be decoded: {error:?}"),
                );
                continue;
            }
        };
        if document.id != scene.id {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::SceneIdentityMismatch,
                None,
                &scene.artifact,
                format!(
                    "manifest scene id {} does not match decoded id {}",
                    scene.id.raw(),
                    document.id.raw()
                ),
            );
            continue;
        }
        let validation = core_scene::validate(&document);
        if !validation.is_ok() {
            for error in validation.errors {
                report.push(
                    RuntimeProjectAdmissionDiagnosticCode::SceneInvalid,
                    None,
                    &scene.artifact,
                    format!("{}: {error:?}", error.label()),
                );
            }
            continue;
        }
        scenes.insert(scene.id.raw(), document);
    }
    scenes
}

fn check_declared_read_topology(
    composition: &GameplayStaticComposition,
    report: &mut RuntimeProjectAdmissionReport,
) {
    let plans = composition
        .declared_reads()
        .iter()
        .map(|plan| ((plan.module_id.as_str(), plan.invocation_id.as_str()), plan))
        .collect::<BTreeMap<_, _>>();
    for module_id in composition.registry().module_order() {
        let module = composition
            .registry()
            .module(module_id)
            .expect("closed registry module order");
        for invocation in &module.invocations {
            let key = (module_id.as_str(), invocation.invocation_id.as_str());
            let expected = invocation
                .read_requirements
                .iter()
                .map(|requirement| (requirement.request_id.as_str(), requirement.view.key()))
                .collect::<BTreeSet<_>>();
            let actual = plans
                .get(&key)
                .map(|plan| {
                    plan.requests
                        .iter()
                        .map(|request| (request.request_id.as_str(), request.view.key()))
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            if actual != expected {
                report.push(
                    RuntimeProjectAdmissionDiagnosticCode::ProviderSchemaMismatch,
                    None,
                    format!("composition.modules.{module_id}.invocations.{}.reads", invocation.invocation_id),
                    "static provider declared-read plan does not exactly match its invocation contract",
                );
            }
        }
    }
    for ((module_id, invocation_id), _) in plans {
        let known = composition
            .registry()
            .module(module_id)
            .is_some_and(|module| {
                module
                    .invocations
                    .iter()
                    .any(|invocation| invocation.invocation_id == invocation_id)
            });
        if !known {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::ProviderSchemaMismatch,
                None,
                format!("composition.modules.{module_id}.declaredReads.{invocation_id}"),
                "static provider supplied a declared-read plan for an unknown invocation",
            );
        }
    }
}

fn check_cross_scene_markers(
    scenes: &BTreeMap<u64, FlatSceneDocument>,
    report: &mut RuntimeProjectAdmissionReport,
) {
    let markers = scenes
        .iter()
        .map(|(scene_id, scene)| {
            let values = scene
                .nodes
                .iter()
                .filter_map(|node| match &node.kind {
                    SceneNodeKind::Marker(marker) => Some(marker.marker_id.as_str()),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            (*scene_id, values)
        })
        .collect::<BTreeMap<_, _>>();
    for (scene_id, scene) in scenes {
        for instance in scene.nodes.iter().filter_map(|node| match &node.kind {
            SceneNodeKind::EntityInstance(instance) => Some(instance),
            _ => None,
        }) {
            let Some(marker) = instance.spawn_marker_id.as_deref() else {
                continue;
            };
            if markers
                .get(scene_id)
                .is_some_and(|local| local.contains(marker))
            {
                continue;
            }
            if markers
                .iter()
                .any(|(other, values)| other != scene_id && values.contains(marker))
            {
                report.push(
                    RuntimeProjectAdmissionDiagnosticCode::CrossSceneReference,
                    None,
                    format!(
                        "scenes.{scene_id}.instances.{}.spawnMarkerId",
                        instance.instance_id
                    ),
                    format!("spawn marker `{marker}` exists only in another scene"),
                );
            }
        }
    }
}

fn check_runtime_resource_closure(
    source: &AdmittedRuntimeProjectSourceBatch,
    scenes: &BTreeMap<u64, FlatSceneDocument>,
    documents: &[ProjectContentDocumentDto],
    report: &mut RuntimeProjectAdmissionReport,
) {
    let asset_ids = documents
        .iter()
        .filter_map(|document| match document {
            ProjectContentDocumentDto::AssetCatalog { catalog, .. } => Some(catalog),
            _ => None,
        })
        .flat_map(|catalog| catalog.entries.iter().map(|entry| entry.id.as_str()))
        .collect::<BTreeSet<_>>();
    for (scene_id, scene) in scenes {
        for asset in scene
            .dependencies
            .iter()
            .chain(scene.nodes.iter().filter_map(|node| node.kind.asset()))
        {
            if !asset_ids.contains(asset.id().as_str()) {
                report.push(
                    RuntimeProjectAdmissionDiagnosticCode::DanglingReference,
                    None,
                    format!("scenes.{scene_id}.assets.{}", asset.id().as_str()),
                    "scene asset is absent from the admitted asset catalogs",
                );
            }
        }
    }
    for document in documents {
        match document {
            ProjectContentDocumentDto::AssetCatalog {
                document_id,
                catalog,
            } => {
                for entry in &catalog.entries {
                    if let Some(path) = entry.source_path.as_deref() {
                        require_staged_path(source, report, document_id, path);
                        require_catalog_resource_identity(
                            source,
                            report,
                            document_id,
                            path,
                            entry.hash.as_deref(),
                            None,
                        );
                    }
                }
            }
            ProjectContentDocumentDto::PresentationCatalog {
                document_id,
                catalog,
            } => {
                for resource in &catalog.resources {
                    require_staged_path(source, report, document_id, &resource.source_path);
                    require_catalog_resource_identity(
                        source,
                        report,
                        document_id,
                        &resource.source_path,
                        Some(&resource.content_hash),
                        Some(presentation_resource_role(resource.kind)),
                    );
                    if let Some(path) = resource.license_path.as_deref() {
                        require_staged_path(source, report, document_id, path);
                    }
                }
            }
            _ => {}
        }
    }
}

fn presentation_resource_role(
    kind: protocol_project_content::ProjectPresentationResourceKind,
) -> &'static str {
    use protocol_project_content::ProjectPresentationResourceKind;
    match kind {
        ProjectPresentationResourceKind::AnimatedMesh => "resource:animatedMesh",
        ProjectPresentationResourceKind::Audio => "resource:audio",
        ProjectPresentationResourceKind::Particle => "resource:particle",
        ProjectPresentationResourceKind::Font => "resource:font",
        ProjectPresentationResourceKind::Overlay => "resource:overlay",
    }
}

fn require_catalog_resource_identity(
    source: &AdmittedRuntimeProjectSourceBatch,
    report: &mut RuntimeProjectAdmissionReport,
    document_id: &str,
    path: &str,
    expected_hash: Option<&str>,
    expected_role: Option<&str>,
) {
    let Some(artifact) = source
        .manifest()
        .artifacts
        .iter()
        .find(|artifact| artifact.path == path)
    else {
        return;
    };
    if let Some(expected_hash) = expected_hash {
        let actual = artifact.content_hash.map(BundleHash::to_hex);
        if actual.as_deref() != Some(expected_hash) {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::ResourceHashMismatch,
                Some(document_id.to_owned()),
                path,
                "stored resource content hash does not match its manifest-authorized body",
            );
        }
    }
    if let Some(expected_role) = expected_role {
        if artifact.role.tag() != expected_role {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::ArtifactRoleMismatch,
                Some(document_id.to_owned()),
                path,
                format!(
                    "stored resource expects artifact role `{expected_role}`, found `{}`",
                    artifact.role.tag()
                ),
            );
        }
    }
}

fn check_scene_references(
    scenes: &BTreeMap<u64, FlatSceneDocument>,
    documents: &[ProjectContentDocumentDto],
    report: &mut RuntimeProjectAdmissionReport,
) {
    let entities = documents
        .iter()
        .filter_map(|document| match document {
            ProjectContentDocumentDto::EntityDefinition { definition, .. } => {
                Some(definition.stable_id.as_str())
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let base_prefabs = documents
        .iter()
        .filter_map(|document| match document {
            ProjectContentDocumentDto::PrefabRegistry { registry, .. } => Some(registry),
            _ => None,
        })
        .flat_map(|registry| registry.definitions.iter())
        .filter(|definition| definition.variant.is_none())
        .map(|definition| definition.id.raw())
        .collect::<BTreeSet<_>>();
    for (scene_id, scene) in scenes {
        for instance in scene.nodes.iter().filter_map(|node| match &node.kind {
            SceneNodeKind::EntityInstance(instance) => Some(instance),
            _ => None,
        }) {
            let (known, target) = match &instance.reference {
                SceneEntityReference::EntityDefinition { stable_id } => {
                    (entities.contains(stable_id.as_str()), stable_id.clone())
                }
                SceneEntityReference::Prefab { prefab_id, .. } => {
                    (base_prefabs.contains(prefab_id), prefab_id.to_string())
                }
            };
            if !known {
                report.push(
                    RuntimeProjectAdmissionDiagnosticCode::DanglingReference,
                    None,
                    format!(
                        "scenes.{scene_id}.instances.{}.reference",
                        instance.instance_id
                    ),
                    format!("stored scene reference `{target}` has no admitted definition"),
                );
            }
        }
    }
}

fn require_staged_path(
    source: &AdmittedRuntimeProjectSourceBatch,
    report: &mut RuntimeProjectAdmissionReport,
    document_id: &str,
    path: &str,
) {
    if source.body(path).is_none() {
        report.push(
            RuntimeProjectAdmissionDiagnosticCode::ResourceNotStaged,
            Some(document_id.to_owned()),
            path,
            "stored resource source path is not present in the manifest-admitted closure",
        );
    }
}

fn check_entry_scene_targets(
    source: &AdmittedRuntimeProjectSourceBatch,
    scenes: &BTreeMap<u64, FlatSceneDocument>,
    documents: &[ProjectContentDocumentDto],
    report: &mut RuntimeProjectAdmissionReport,
) {
    let entry_id = source.manifest().entry_scene.raw();
    let entry_instances = scenes
        .get(&entry_id)
        .into_iter()
        .flat_map(|scene| scene.nodes.iter())
        .filter_map(|node| match &node.kind {
            SceneNodeKind::EntityInstance(instance) => Some(instance.instance_id.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    for document in documents {
        let ProjectContentDocumentDto::GameplayConfiguration {
            document_id,
            document,
        } = document
        else {
            continue;
        };
        for trigger in &document.triggers {
            if !entry_instances.contains(trigger.scene_instance_id.as_str()) {
                report.push(
                    RuntimeProjectAdmissionDiagnosticCode::CrossSceneReference,
                    Some(document_id.clone()),
                    "document.triggers.sceneInstanceId",
                    format!(
                        "trigger target `{}` is not part of entry scene {entry_id}",
                        trigger.scene_instance_id
                    ),
                );
            }
        }
        for layer in &document.overrides {
            if !entry_instances.contains(layer.scene_instance_id.as_str()) {
                report.push(
                    RuntimeProjectAdmissionDiagnosticCode::CrossSceneReference,
                    Some(document_id.clone()),
                    "document.overrides.sceneInstanceId",
                    format!(
                        "binding override target `{}` is not part of entry scene {entry_id}",
                        layer.scene_instance_id
                    ),
                );
            }
        }
    }
}

fn derive_bootstrap_resolution(
    content: &ValidatedProjectContentSet,
) -> core_scene::BootstrapResolutionContext {
    let mut resolution = core_scene::BootstrapResolutionContext::default();
    for document in &content.result().documents {
        match document {
            ProjectContentDocumentDto::EntityDefinition { definition, .. } => {
                resolution
                    .entity_definition_ids
                    .insert(definition.stable_id.clone());
            }
            ProjectContentDocumentDto::AssetCatalog { document_id, .. } => {
                resolution.catalog_ids.insert(document_id.clone());
            }
            _ => {}
        }
    }
    resolution.prefab_ids.extend(
        content
            .prefab_registry()
            .as_registry()
            .definitions
            .iter()
            .filter(|definition| definition.variant.is_none())
            .map(|definition| definition.id.raw()),
    );
    resolution
}

fn compiled_bindings(content: &ValidatedProjectContentSet) -> GameplayModuleBindingRegistry {
    let mut builder = GameplayModuleBindingRegistryBuilder::new();
    for configuration in content.compiled_gameplay().configurations().iter().cloned() {
        builder.configuration(configuration);
    }
    for binding in content.compiled_gameplay().bindings().iter().cloned() {
        builder.binding(binding);
    }
    for layer in content.compiled_gameplay().overrides().iter().cloned() {
        builder.instance_override(layer);
    }
    builder.build()
}

fn definitions<'a>(
    content: &'a ValidatedProjectContentSet,
    source_paths: &'a BTreeMap<String, String>,
) -> BTreeMap<&'a str, (&'a str, &'a str, &'a EntityDefinition)> {
    content
        .result()
        .documents
        .iter()
        .filter_map(|document| match document {
            ProjectContentDocumentDto::EntityDefinition {
                document_id,
                definition,
            } => Some((
                definition.stable_id.as_str(),
                (
                    document_id.as_str(),
                    source_paths
                        .get(document_id)
                        .map(String::as_str)
                        .expect("validated runtime content retains every manifest source path"),
                    definition,
                ),
            )),
            _ => None,
        })
        .collect()
}

fn derive_entity_authority(
    content: &ValidatedProjectContentSet,
    source_paths: &BTreeMap<String, String>,
    bootstrap: &BootstrapPlan,
    report: &mut RuntimeProjectAdmissionReport,
) -> (
    GameplayBindingEntityTargets,
    Vec<GameplayRuntimeSpatialEntity>,
    Vec<RuntimeProjectEntitySeed>,
) {
    let definitions = definitions(content, source_paths);
    let mut targets = GameplayBindingEntityTargets::new();
    let mut spatial = Vec::new();
    let mut runtime_entity_seeds = Vec::new();
    for instance in bootstrap.resolved_instances() {
        let SceneEntityReference::EntityDefinition { stable_id } = &instance.reference else {
            continue;
        };
        targets.bind(stable_id.clone(), instance.entity);
        let Some((document_id, source_path, definition)) =
            definitions.get(stable_id.as_str()).copied()
        else {
            continue;
        };
        runtime_entity_seeds.push(RuntimeProjectEntitySeed {
            entity: instance.entity,
            instance_id: instance.instance_id.clone(),
            document_id: document_id.to_owned(),
            source_path: source_path.to_owned(),
            spawn_marker_id: instance.spawn_marker_id.clone(),
            world_translation: instance.world_transform.translation.to_array(),
            world_rotation: [
                instance.world_transform.rotation.x,
                instance.world_transform.rotation.y,
                instance.world_transform.rotation.z,
                instance.world_transform.rotation.w,
            ],
            world_scale: instance.world_transform.scale.to_array(),
            definition: definition.clone(),
        });
        let bounds = definition
            .capabilities
            .iter()
            .find_map(|capability| match capability {
                EntityDefinitionCapability::Bounds { min, max } => Some((*min, *max)),
                _ => None,
            });
        let collision = definition
            .capabilities
            .iter()
            .find_map(|capability| match capability {
                EntityDefinitionCapability::Collision { static_collider } => Some(*static_collider),
                _ => None,
            });
        let (Some((min, max)), Some(static_collider)) = (bounds, collision) else {
            continue;
        };
        let transform = instance.world_transform;
        if [
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
            transform.rotation.w,
        ] != [0.0, 0.0, 0.0, 1.0]
        {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::BootstrapLink,
                None,
                format!("entryScene.instances.{}.transform", instance.instance_id),
                "spatial EntityDefinition placement currently requires identity world rotation",
            );
            continue;
        }
        let scale = transform.scale.to_array();
        let translation = transform.translation.to_array();
        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        spatial.push(GameplayRuntimeSpatialEntity {
            entity: instance.entity,
            translation: [
                translation[0] + center[0] * scale[0],
                translation[1] + center[1] * scale[1],
                translation[2] + center[2] * scale[2],
            ],
            half_extents: [
                (max[0] - min[0]) * 0.5 * scale[0],
                (max[1] - min[1]) * 0.5 * scale[1],
                (max[2] - min[2]) * 0.5 * scale[2],
            ],
            static_collider,
        });
    }
    (targets, spatial, runtime_entity_seeds)
}

fn derive_prefab_bootstrap(
    content: &ValidatedProjectContentSet,
    bootstrap: &BootstrapPlan,
    report: &mut RuntimeProjectAdmissionReport,
) -> GameplayRuntimePrefabBootstrap {
    let registry = content.prefab_registry();
    let catalog = prefab_catalog(content);
    let mut placements = Vec::new();
    for instance in bootstrap.resolved_instances() {
        let SceneEntityReference::Prefab {
            prefab_id,
            variant_id,
            instantiation_seed,
        } = &instance.reference
        else {
            continue;
        };
        let selected = match variant_id {
            None => Some(*prefab_id),
            Some(variant_id) => registry
                .as_registry()
                .definitions
                .iter()
                .find(|definition| {
                    definition.variant.as_ref().is_some_and(|variant| {
                        variant.base.raw() == *prefab_id && variant.variant_id == *variant_id
                    })
                })
                .map(|definition| definition.id.raw()),
        };
        let Some(selected) = selected else {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::DanglingReference,
                None,
                format!("entryScene.instances.{}.variant", instance.instance_id),
                format!(
                    "prefab {} has no variant `{}`",
                    prefab_id,
                    variant_id.as_deref().unwrap_or_default()
                ),
            );
            continue;
        };
        placements.push(GameplayRuntimePrefabPlacement {
            command_id: format!("project-admission:prefab:{}", instance.instance_id),
            scene_instance_id: instance.instance_id.clone(),
            origin: GameplayRuntimePrefabPlacementOrigin::Authored,
            instance: instance.node.raw(),
            authored_prefab: *prefab_id,
            prefab: selected,
            seed: *instantiation_seed,
            transform: GameplayRuntimePrefabTransform {
                translation: instance.world_transform.translation.to_array(),
                rotation: [
                    instance.world_transform.rotation.x,
                    instance.world_transform.rotation.y,
                    instance.world_transform.rotation.z,
                    instance.world_transform.rotation.w,
                ],
                scale: instance.world_transform.scale.to_array(),
            },
            overrides: Vec::new(),
        });
    }
    GameplayRuntimePrefabBootstrap {
        registry_json: encode_prefab_registry(registry),
        catalog,
        placements,
    }
}

fn prefab_catalog(content: &ValidatedProjectContentSet) -> GameplayRuntimePrefabCatalog {
    let mut asset_ids = BTreeSet::new();
    let mut entity_definition_ids = BTreeSet::new();
    for document in &content.result().documents {
        match document {
            ProjectContentDocumentDto::AssetCatalog { catalog, .. } => {
                asset_ids.extend(catalog.entries.iter().map(|entry| entry.id.clone()));
            }
            ProjectContentDocumentDto::EntityDefinition { definition, .. } => {
                entity_definition_ids.insert(definition.stable_id.clone());
            }
            _ => {}
        }
    }
    GameplayRuntimePrefabCatalog {
        asset_ids: asset_ids.into_iter().collect(),
        entity_definition_ids: entity_definition_ids.into_iter().collect(),
    }
}

fn derive_scheduler(
    composition: &GameplayStaticComposition,
    has_authored_program: bool,
) -> GameplayRuntimeSchedulerDefinition {
    let mut events = BTreeMap::<String, GameplayContractRef>::new();
    let mut proposals = BTreeMap::<String, GameplayContractRef>::new();
    for module_id in composition.registry().module_order() {
        let module = composition
            .registry()
            .module(module_id)
            .expect("closed registry module order");
        for declaration in &module.published_events {
            events.insert(declaration.event.key(), declaration.event.clone());
        }
        for declaration in &module.proposal_kinds {
            proposals.insert(declaration.proposal.key(), declaration.proposal.clone());
        }
    }
    if has_authored_program {
        let contract = crate::authored_behavior::authored_program_step_contract();
        proposals.insert(contract.key(), contract);
    }
    GameplayRuntimeSchedulerDefinition::new(
        GameplayOwnerRef {
            owner_id: "authority.asha.gameplay-scheduler".to_owned(),
            provider_id: "provider.asha.gameplay-scheduler".to_owned(),
        },
        events.into_values().collect(),
        proposals.into_values().collect(),
    )
}

fn runtime_admission_hash(
    source: &AdmittedRuntimeProjectSourceBatch,
    scenes: &BTreeMap<u64, FlatSceneDocument>,
    content: &ValidatedProjectContentSet,
    registry_digest: &str,
    bindings: &GameplayModuleBindingRegistry,
    declared_reads: &[GameplayRuntimeDeclaredReadPlan],
) -> String {
    let mut identity = format!(
        "runtime-project-admission-v1|{}|{}|{}|{}",
        source.manifest_hash().to_hex(),
        source.manifest().project.id.raw(),
        content.set_hash(),
        registry_digest,
    );
    for (scene_id, scene) in scenes {
        identity.push('|');
        identity.push_str(&scene_id.to_string());
        identity.push(':');
        identity.push_str(&gameplay_module_payload_hash(
            core_scene::encode(scene).as_bytes(),
        ));
    }
    identity.push('|');
    identity.push_str(&bindings.registry_hash);
    for plan in declared_reads {
        identity.push('|');
        identity.push_str(&plan.module_id);
        identity.push(':');
        identity.push_str(&plan.invocation_id);
        identity.push(':');
        identity.push_str(&declared_read_plan_hash(plan));
    }
    gameplay_module_payload_hash(identity.as_bytes())
}

fn declared_read_plan_hash(plan: &GameplayRuntimeDeclaredReadPlan) -> String {
    let mut bytes = Vec::new();
    append_identity_text(&mut bytes, &plan.module_id);
    append_identity_text(&mut bytes, &plan.invocation_id);
    append_identity_u64(&mut bytes, plan.requests.len() as u64);
    for request in &plan.requests {
        append_read_request_identity(&mut bytes, request);
    }
    gameplay_module_payload_hash(&bytes)
}

fn append_read_request_identity(bytes: &mut Vec<u8>, request: &GameplayReadRequest) {
    append_identity_text(bytes, &request.request_id);
    append_identity_text(bytes, &request.view.namespace);
    append_identity_text(bytes, &request.view.name);
    append_identity_u64(bytes, u64::from(request.view.version));
    append_identity_text(bytes, &request.view.schema_hash);
    append_identity_u64(bytes, request.fields.len() as u64);
    for field in &request.fields {
        append_identity_text(bytes, field);
    }
    read_identity::append_read_selector_identity(bytes, &request.selector);
}

fn append_event_binding_identity(bytes: &mut Vec<u8>, binding: &GameplayEventEntityBinding) {
    match binding {
        GameplayEventEntityBinding::Source => append_identity_text(bytes, "source"),
        GameplayEventEntityBinding::Subject { index } => {
            append_identity_text(bytes, "subject");
            append_identity_u64(bytes, u64::from(*index));
        }
        GameplayEventEntityBinding::Target { index } => {
            append_identity_text(bytes, "target");
            append_identity_u64(bytes, u64::from(*index));
        }
        GameplayEventEntityBinding::Known(entity) => {
            append_identity_text(bytes, "known");
            append_identity_u64(bytes, entity.raw());
        }
    }
}

fn append_module_scope_identity(bytes: &mut Vec<u8>, scope: &GameplayModuleStateScope) {
    match scope {
        GameplayModuleStateScope::Session => append_identity_text(bytes, "session"),
        GameplayModuleStateScope::Entity { entity } => {
            append_identity_text(bytes, "entity");
            append_identity_u64(bytes, *entity);
        }
        GameplayModuleStateScope::PrefabInstance { instance } => {
            append_identity_text(bytes, "prefabInstance");
            append_identity_u64(bytes, *instance);
        }
    }
}

fn append_owner_query_identity(bytes: &mut Vec<u8>, query: &GameplayOwnerQuery) {
    match query {
        GameplayOwnerQuery::NearbyEntities {
            anchor,
            radius_millimeters,
            required_tags,
            max_items,
        } => {
            append_identity_text(bytes, "nearbyEntities");
            append_event_binding_identity(bytes, anchor);
            append_identity_u64(bytes, *radius_millimeters);
            append_identity_u64(bytes, required_tags.len() as u64);
            for tag in required_tags {
                append_identity_u64(bytes, tag.raw());
            }
            append_identity_u64(bytes, u64::from(*max_items));
        }
        GameplayOwnerQuery::LineOfSight { source, target } => {
            append_identity_text(bytes, "lineOfSight");
            append_event_binding_identity(bytes, source);
            append_event_binding_identity(bytes, target);
        }
        GameplayOwnerQuery::PathBetween {
            source,
            target,
            max_steps,
        } => {
            append_identity_text(bytes, "pathBetween");
            append_event_binding_identity(bytes, source);
            append_event_binding_identity(bytes, target);
            append_identity_u64(bytes, u64::from(*max_steps));
        }
        GameplayOwnerQuery::CurrentTriggerOverlaps { trigger, max_items } => {
            append_identity_text(bytes, "currentTriggerOverlaps");
            append_event_binding_identity(bytes, trigger);
            append_identity_u64(bytes, u64::from(*max_items));
        }
    }
}

fn append_identity_text(bytes: &mut Vec<u8>, value: &str) {
    append_identity_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn append_identity_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn map_project_diagnostic(
    diagnostic: ProjectContentDiagnosticDto,
) -> RuntimeProjectAdmissionDiagnostic {
    let lowercase = diagnostic.message.to_ascii_lowercase();
    let code = if lowercase.contains("only one trigger") {
        RuntimeProjectAdmissionDiagnosticCode::DuplicateTrigger
    } else if lowercase.contains("unique") || lowercase.contains("duplicate") {
        RuntimeProjectAdmissionDiagnosticCode::AmbiguousReference
    } else if diagnostic.path.contains("schema")
        || diagnostic.path.contains("module")
        || diagnostic.path.contains("runtimeAdmission")
        || lowercase.contains("provider")
        || lowercase.contains("codec")
    {
        RuntimeProjectAdmissionDiagnosticCode::ProviderSchemaMismatch
    } else if matches!(
        diagnostic.code,
        ProjectContentDiagnosticCode::UnknownReference
            | ProjectContentDiagnosticCode::ReferenceKindMismatch
    ) {
        RuntimeProjectAdmissionDiagnosticCode::DanglingReference
    } else if matches!(
        diagnostic.code,
        ProjectContentDiagnosticCode::InvalidJson | ProjectContentDiagnosticCode::UnknownField
    ) {
        RuntimeProjectAdmissionDiagnosticCode::ProjectContentDecode
    } else {
        RuntimeProjectAdmissionDiagnosticCode::ProjectContentInvalid
    };
    RuntimeProjectAdmissionDiagnostic {
        code,
        document_id: diagnostic.document_id,
        path: diagnostic.path,
        message: diagnostic.message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GameplayRuntimeHost, GameplayRuntimePrefabInteractionIntent};
    use core_ids::{PrefabId, PrefabPartId, ProjectId, SceneId, SceneNodeId};
    use core_scene::{
        FlatSceneDocument, NodeMetadata, SceneEntityInstance, SceneEntityReference, SceneMetadata,
        SceneNodeKind, SceneNodeRecord, SceneTransform,
    };
    use gameplay_module_sdk::GameplayStaticCompositionBuilder;
    use protocol_entity_authoring::{
        AuthoringTransform, EntityDefinitionCapability, EntityDefinitionMetadataEntry,
        EntityDefinitionSourceTrace,
    };
    use protocol_project_bundle::{
        PrefabDefinition, PrefabPart, PrefabPartRoleBinding, PrefabPartSource, PrefabRegistry,
        PrefabTransform, PREFAB_DEFINITION_SCHEMA_VERSION, PREFAB_REGISTRY_SCHEMA_VERSION,
    };
    use protocol_project_content::{
        AuthoredBehaviorArgumentDto, AuthoredBehaviorConditionDto, AuthoredBehaviorDefinitionDto,
        AuthoredBehaviorOperationDto, AuthoredBehaviorPackageDto, AuthoredBehaviorProvenanceDto,
        AuthoredBehaviorSemanticRefDto, AuthoredBehaviorSignalDto, AuthoredBehaviorStateDto,
        AuthoredBehaviorStateMachineDto, AuthoredBehaviorStepDto, AuthoredBehaviorTransitionDto,
        AuthoredBehaviorValueDto, ProjectContentDecodeRequestDto, ProjectContentDocumentKind,
        ProjectContentSourceDto, AUTHORED_BEHAVIOR_PACKAGE_SCHEMA_VERSION,
        AUTHORED_BEHAVIOR_VOCABULARY_HASH, AUTHORED_BEHAVIOR_VOCABULARY_VERSION,
        AUTHORED_PREDICATE_STATE_IS, AUTHORED_SIGNAL_PREFAB_PART_INTERACTED,
        AUTHORED_VERB_SET_CAPABILITY_ACTIVE, AUTHORED_VERB_SET_RELATIVE_TRANSLATION,
        AUTHORED_VERB_TRANSITION_STATE,
    };
    use rule_gameplay_fabric::StandardGameplayEventKind;
    use svc_project_content::{decode_project_content, EmptyProjectContentGameplayAdmission};
    use svc_serialization::{
        encode, validate_runtime_project_source_batch, ArtifactEntry, AssetLockSection,
        ProjectResourceStaging, ProjectSection, ProjectSourceBody, RuntimeProjectSourceBatch,
        SceneSection, BUNDLE_SCHEMA_VERSION, SUPPORTED_PROTOCOL_VERSION,
    };

    fn empty_scene(id: u64) -> FlatSceneDocument {
        FlatSceneDocument {
            id: SceneId::new(id),
            schema_version: 4,
            metadata: SceneMetadata {
                name: Some("Admission room".to_owned()),
                authoring_format_version: 4,
            },
            dependencies: Vec::new(),
            nodes: Vec::new(),
        }
    }

    fn scene_with_unknown_entity(id: u64) -> FlatSceneDocument {
        let mut scene = empty_scene(id);
        scene.nodes.push(SceneNodeRecord {
            id: SceneNodeId::new(1),
            parent: None,
            child_order: 0,
            transform: SceneTransform::IDENTITY,
            kind: SceneNodeKind::EntityInstance(SceneEntityInstance {
                instance_id: "missing.entity.instance".to_owned(),
                reference: SceneEntityReference::EntityDefinition {
                    stable_id: "missing.entity".to_owned(),
                },
                spawn_marker_id: None,
            }),
            metadata: NodeMetadata::default(),
        });
        scene
    }

    fn authored_behavior_scene(id: u64) -> FlatSceneDocument {
        let mut scene = empty_scene(id);
        let instance =
            |node, instance_id: &str, stable_id: &str, translation: [f32; 3]| SceneNodeRecord {
                id: SceneNodeId::new(node),
                parent: None,
                child_order: u32::try_from(node).expect("fixture node order"),
                transform: SceneTransform {
                    translation: core_math::Vec3::new(
                        translation[0],
                        translation[1],
                        translation[2],
                    ),
                    ..SceneTransform::IDENTITY
                },
                kind: SceneNodeKind::EntityInstance(SceneEntityInstance {
                    instance_id: instance_id.to_owned(),
                    reference: SceneEntityReference::EntityDefinition {
                        stable_id: stable_id.to_owned(),
                    },
                    spawn_marker_id: None,
                }),
                metadata: NodeMetadata::default(),
            };
        scene.nodes = vec![
            instance(1, "fixture.door.instance", "fixture.door", [0.0, 0.0, 5.0]),
            SceneNodeRecord {
                id: SceneNodeId::new(2),
                parent: None,
                child_order: 2,
                transform: SceneTransform {
                    translation: core_math::Vec3::new(2.0, 0.0, 0.0),
                    ..SceneTransform::IDENTITY
                },
                kind: SceneNodeKind::EntityInstance(SceneEntityInstance {
                    instance_id: "fixture.switch.instance".to_owned(),
                    reference: SceneEntityReference::Prefab {
                        prefab_id: 70,
                        variant_id: None,
                        instantiation_seed: 6088,
                    },
                    spawn_marker_id: None,
                }),
                metadata: NodeMetadata::default(),
            },
            instance(
                3,
                "fixture.actor.instance",
                "fixture.actor",
                [0.0, 0.0, 0.0],
            ),
        ];
        scene
    }

    fn spatial_entity_document(stable_id: &str) -> ProjectContentDocumentDto {
        ProjectContentDocumentDto::EntityDefinition {
            document_id: format!("entities/{stable_id}.json"),
            definition: EntityDefinition {
                stable_id: stable_id.to_owned(),
                display_name: stable_id.to_owned(),
                source: EntityDefinitionSourceTrace {
                    project_bundle: "authored-behavior-fixture".to_owned(),
                    relative_path: format!("entities/{stable_id}.json"),
                },
                tags: Vec::new(),
                metadata: Vec::new(),
                capabilities: vec![
                    EntityDefinitionCapability::Transform {
                        transform: AuthoringTransform {
                            translation: [0.0, 0.0, 0.0],
                            rotation: [0.0, 0.0, 0.0, 1.0],
                            scale: [1.0, 1.0, 1.0],
                        },
                    },
                    EntityDefinitionCapability::Bounds {
                        min: [-0.5, -0.5, -0.5],
                        max: [0.5, 0.5, 0.5],
                    },
                    EntityDefinitionCapability::Collision {
                        static_collider: false,
                    },
                    EntityDefinitionCapability::Render { visible: true },
                ],
            },
        }
    }

    fn authored_behavior_documents() -> Vec<ProjectContentDocumentDto> {
        let semantic = |semantic_id: &str| AuthoredBehaviorSemanticRefDto {
            semantic_id: semantic_id.to_owned(),
            version: AUTHORED_BEHAVIOR_VOCABULARY_VERSION,
        };
        let argument = |name: &str, value| AuthoredBehaviorArgumentDto {
            name: name.to_owned(),
            value,
        };
        let door_entity = || AuthoredBehaviorValueDto::SceneEntity {
            scene_instance_id: "fixture.door.instance".to_owned(),
        };
        let transition = |transition_id: &str| AuthoredBehaviorOperationDto {
            verb: semantic(AUTHORED_VERB_TRANSITION_STATE),
            arguments: vec![
                argument(
                    "machine",
                    AuthoredBehaviorValueDto::StateMachine {
                        machine_id: "door".to_owned(),
                    },
                ),
                argument(
                    "transition",
                    AuthoredBehaviorValueDto::Text {
                        value: transition_id.to_owned(),
                    },
                ),
            ],
        };
        let realize = |offset: [f32; 3], active: bool| {
            vec![
                AuthoredBehaviorOperationDto {
                    verb: semantic(AUTHORED_VERB_SET_RELATIVE_TRANSLATION),
                    arguments: vec![
                        argument("entity", door_entity()),
                        argument("value", AuthoredBehaviorValueDto::Vector3 { value: offset }),
                    ],
                },
                AuthoredBehaviorOperationDto {
                    verb: semantic(AUTHORED_VERB_SET_CAPABILITY_ACTIVE),
                    arguments: vec![
                        argument("entity", door_entity()),
                        argument(
                            "capability",
                            AuthoredBehaviorValueDto::Text {
                                value: "collision".to_owned(),
                            },
                        ),
                        argument(
                            "active",
                            AuthoredBehaviorValueDto::Boolean { value: active },
                        ),
                    ],
                },
            ]
        };
        vec![
            spatial_entity_document("fixture.door"),
            spatial_entity_document("fixture.switch"),
            spatial_entity_document("fixture.actor"),
            ProjectContentDocumentDto::PrefabRegistry {
                document_id: "prefabs/switch.json".to_owned(),
                registry: PrefabRegistry {
                    schema_version: PREFAB_REGISTRY_SCHEMA_VERSION,
                    definitions: vec![PrefabDefinition {
                        id: PrefabId::new(70),
                        schema_version: PREFAB_DEFINITION_SCHEMA_VERSION,
                        display_name: "Fixture switch".to_owned(),
                        parts: vec![PrefabPart {
                            id: PrefabPartId::new(1),
                            namespace: "switch".to_owned(),
                            display_name: "Switch".to_owned(),
                            parent: None,
                            transform: PrefabTransform {
                                translation: [0.0, 0.0, 0.0],
                                rotation: [0.0, 0.0, 0.0, 1.0],
                                scale: [1.0, 1.0, 1.0],
                            },
                            source: PrefabPartSource::EntityDefinition {
                                stable_id: "fixture.switch".to_owned(),
                            },
                        }],
                        part_roles: vec![PrefabPartRoleBinding {
                            role: "interaction/switch".to_owned(),
                            part: PrefabPartId::new(1),
                        }],
                        variant: None,
                    }],
                },
            },
            ProjectContentDocumentDto::BehaviorPackage {
                document_id: "behaviors/doors.json".to_owned(),
                package: AuthoredBehaviorPackageDto {
                    schema_version: AUTHORED_BEHAVIOR_PACKAGE_SCHEMA_VERSION,
                    package_id: "fixture.doors".to_owned(),
                    provenance: AuthoredBehaviorProvenanceDto {
                        sdk_id: "@asha/game-workspace".to_owned(),
                        sdk_version: AUTHORED_BEHAVIOR_VOCABULARY_VERSION,
                        vocabulary_hash: AUTHORED_BEHAVIOR_VOCABULARY_HASH.to_owned(),
                        source_module: "@fixture/gameplay".to_owned(),
                        source_path: "src/doors.ts".to_owned(),
                        source_hash: "fnv1a64:fixture-door-source".to_owned(),
                    },
                    state_machines: vec![AuthoredBehaviorStateMachineDto {
                        machine_id: "door".to_owned(),
                        target_scene_instance_id: "fixture.door.instance".to_owned(),
                        initial_state_id: "closed".to_owned(),
                        states: vec![
                            AuthoredBehaviorStateDto {
                                state_id: "closed".to_owned(),
                            },
                            AuthoredBehaviorStateDto {
                                state_id: "open".to_owned(),
                            },
                        ],
                        transitions: vec![
                            AuthoredBehaviorTransitionDto {
                                transition_id: "open".to_owned(),
                                from_state_id: "closed".to_owned(),
                                to_state_id: "open".to_owned(),
                            },
                            AuthoredBehaviorTransitionDto {
                                transition_id: "close".to_owned(),
                                from_state_id: "open".to_owned(),
                                to_state_id: "closed".to_owned(),
                            },
                        ],
                    }],
                    behaviors: vec![AuthoredBehaviorDefinitionDto {
                        behavior_id: "switch-opens-door".to_owned(),
                        signal: AuthoredBehaviorSignalDto {
                            signal: semantic(AUTHORED_SIGNAL_PREFAB_PART_INTERACTED),
                            arguments: vec![argument(
                                "part",
                                AuthoredBehaviorValueDto::PrefabPart {
                                    scene_instance_id: "fixture.switch.instance".to_owned(),
                                    role: "interaction/switch".to_owned(),
                                },
                            )],
                        },
                        conditions: vec![AuthoredBehaviorConditionDto {
                            predicate: semantic(AUTHORED_PREDICATE_STATE_IS),
                            arguments: vec![argument(
                                "state",
                                AuthoredBehaviorValueDto::State {
                                    machine_id: "door".to_owned(),
                                    state_id: "closed".to_owned(),
                                },
                            )],
                        }],
                        steps: vec![
                            AuthoredBehaviorStepDto {
                                step_id: "open-now".to_owned(),
                                after_step_ids: Vec::new(),
                                delay_ticks: 0,
                                operations: {
                                    let mut operations = vec![transition("open")];
                                    operations.extend(realize([0.0, 3.0, 0.0], false));
                                    operations
                                },
                            },
                            AuthoredBehaviorStepDto {
                                step_id: "close-later".to_owned(),
                                after_step_ids: vec!["open-now".to_owned()],
                                delay_ticks: 120,
                                operations: {
                                    let mut operations = vec![transition("close")];
                                    operations.extend(realize([0.0, 0.0, 0.0], true));
                                    operations
                                },
                            },
                        ],
                    }],
                },
            },
        ]
    }

    fn authored_behavior_composition() -> GameplayStaticComposition {
        let mut builder = GameplayStaticCompositionBuilder::new();
        builder.include_standard_owner_events();
        builder.build().expect("authored behavior composition")
    }

    fn authored_behavior_batch(
        scene: &FlatSceneDocument,
        composition: &GameplayStaticComposition,
    ) -> AdmittedRuntimeProjectSourceBatch {
        authored_behavior_batch_with_documents(scene, composition, authored_behavior_documents())
    }

    fn authored_behavior_batch_with_documents(
        scene: &FlatSceneDocument,
        composition: &GameplayStaticComposition,
        documents: Vec<ProjectContentDocumentDto>,
    ) -> AdmittedRuntimeProjectSourceBatch {
        let scene_dto = project_scene_document_dto(scene);
        let gameplay =
            GameplayProjectContentAdmission::new(composition.project_configuration_authority());
        let outcome = validate_project_content_documents(
            documents,
            ProjectContentValidationContext {
                scenes: std::slice::from_ref(&scene_dto),
                entry_scene_id: Some(scene_dto.id),
                gameplay: &gameplay,
                reference_revision: 0,
            },
        );
        assert!(outcome.result.accepted, "{:?}", outcome.result.diagnostics);
        let files = outcome
            .result
            .canonical_files
            .into_iter()
            .map(|file| (file.document_id, file.canonical_json.into_bytes()))
            .collect::<Vec<_>>();
        admitted_batch(
            scene,
            files
                .iter()
                .map(|(path, bytes)| (path.as_str(), bytes.clone()))
                .collect(),
        )
    }

    fn entity_document() -> ProjectContentDocumentDto {
        ProjectContentDocumentDto::EntityDefinition {
            document_id: "entities/reference-console.json".to_owned(),
            definition: EntityDefinition {
                stable_id: "reference.console".to_owned(),
                display_name: "Reference Console".to_owned(),
                source: EntityDefinitionSourceTrace {
                    project_bundle: "admission-project".to_owned(),
                    relative_path: "entities/reference-console.json".to_owned(),
                },
                tags: Vec::new(),
                metadata: vec![EntityDefinitionMetadataEntry {
                    key: "purpose".to_owned(),
                    value: "admission-test".to_owned(),
                }],
                capabilities: vec![EntityDefinitionCapability::Render { visible: true }],
            },
        }
    }

    fn canonical_typed_document(document: ProjectContentDocumentDto) -> Vec<u8> {
        let gameplay = EmptyProjectContentGameplayAdmission::default();
        let outcome = validate_project_content_documents(
            vec![document],
            ProjectContentValidationContext {
                scenes: &[],
                entry_scene_id: None,
                gameplay: &gameplay,
                reference_revision: 0,
            },
        );
        assert!(outcome.result.accepted, "{:?}", outcome.result.diagnostics);
        outcome.result.canonical_files[0]
            .canonical_json
            .as_bytes()
            .to_vec()
    }

    fn canonical_source_document() -> Vec<u8> {
        let gameplay = EmptyProjectContentGameplayAdmission::default();
        let outcome = decode_project_content(
            ProjectContentDecodeRequestDto {
                sources: vec![ProjectContentSourceDto {
                    source_path: "entities/reference-console.json".to_owned(),
                    document_id: "entities/reference-console.json".to_owned(),
                    kind: ProjectContentDocumentKind::EntityDefinition,
                    source_text: r#"{
                      "kind":"EntityDefinition",
                      "stableId":"reference.console",
                      "displayName":"Reference Console",
                      "source":{"projectBundle":"admission-project","relativePath":"entities/reference-console.json"},
                      "tags":[],
                      "metadata":[{"key":"purpose","value":"admission-test"}],
                      "capabilities":[{"kind":"render","visible":true}]
                    }"#
                    .to_owned(),
                }],
            },
            ProjectContentValidationContext {
                scenes: &[],
                entry_scene_id: None,
                gameplay: &gameplay,
                reference_revision: 0,
            },
        );
        assert!(outcome.result.accepted, "{:?}", outcome.result.diagnostics);
        outcome.result.canonical_files[0]
            .canonical_json
            .as_bytes()
            .to_vec()
    }

    fn admitted_batch(
        scene: &FlatSceneDocument,
        contents: Vec<(&str, Vec<u8>)>,
    ) -> AdmittedRuntimeProjectSourceBatch {
        admitted_batch_with_roles(
            scene,
            contents
                .into_iter()
                .map(|(path, bytes)| (path, ArtifactRole::ProjectContent, bytes))
                .collect(),
        )
    }

    fn admitted_batch_with_roles(
        scene: &FlatSceneDocument,
        contents: Vec<(&str, ArtifactRole, Vec<u8>)>,
    ) -> AdmittedRuntimeProjectSourceBatch {
        let scene_bytes = core_scene::encode(scene).into_bytes();
        let lock_bytes = br#"{"entries":[]}"#.to_vec();
        let mut artifacts = vec![
            ArtifactEntry::durable("assets/lock.json", ArtifactRole::AssetLock, &lock_bytes),
            ArtifactEntry::durable(
                "scenes/entry.scene.json",
                ArtifactRole::SceneDocument,
                &scene_bytes,
            ),
        ];
        let mut bodies = vec![
            ProjectSourceBody::inline("assets/lock.json", lock_bytes),
            ProjectSourceBody::inline("scenes/entry.scene.json", scene_bytes),
        ];
        for (path, role, bytes) in contents {
            artifacts.push(ArtifactEntry::durable(path, role, &bytes));
            bodies.push(ProjectSourceBody::inline(path, bytes));
        }
        let manifest = svc_serialization::ProjectBundleManifest {
            bundle_schema_version: BUNDLE_SCHEMA_VERSION,
            protocol_version: SUPPORTED_PROTOCOL_VERSION,
            project: ProjectSection {
                id: ProjectId::new(91),
                name: Some("admission-project".to_owned()),
            },
            entry_scene: scene.id,
            scenes: vec![SceneSection {
                id: scene.id,
                schema_version: scene.schema_version,
                artifact: "scenes/entry.scene.json".to_owned(),
            }],
            asset_lock: AssetLockSection {
                artifact: "assets/lock.json".to_owned(),
                asset_count: 0,
            },
            generation_provenance: None,
            artifacts,
        };
        let batch = RuntimeProjectSourceBatch {
            manifest_json: encode(&manifest),
            resource_generation: None,
            bodies,
        };
        let mut staging = ProjectResourceStaging::new();
        validate_runtime_project_source_batch(&batch, &mut staging)
            .expect("source batch")
            .commit(&mut staging)
            .expect("commit source batch")
    }

    fn composition() -> GameplayStaticComposition {
        GameplayStaticCompositionBuilder::new()
            .build()
            .expect("empty static composition")
    }

    #[test]
    fn typed_ts_and_studio_candidates_converge_on_one_admission_hash() {
        let typed = canonical_typed_document(entity_document());
        let decoded = canonical_source_document();
        assert_eq!(
            typed, decoded,
            "both authoring paths must persist one artifact"
        );

        let typed_admission = compile_runtime_project_admission(
            admitted_batch(
                &empty_scene(7),
                vec![("content/reference-console.json", typed)],
            ),
            composition(),
        )
        .expect("typed admission");
        let studio_admission = compile_runtime_project_admission(
            admitted_batch(
                &empty_scene(7),
                vec![("content/reference-console.json", decoded)],
            ),
            composition(),
        )
        .expect("studio admission");

        assert_eq!(
            typed_admission.admission_hash(),
            studio_admission.admission_hash()
        );
        assert_eq!(
            typed_admission.compiled_plan_hash(),
            studio_admission.compiled_plan_hash()
        );
    }

    #[test]
    fn dedicated_catalog_roles_are_typed_and_cannot_be_silently_ignored() {
        let entity = canonical_typed_document(entity_document());
        let admitted = compile_runtime_project_admission(
            admitted_batch_with_roles(
                &empty_scene(10),
                vec![(
                    "content/reference-console.json",
                    ArtifactRole::EntityDefinitionCatalog,
                    entity.clone(),
                )],
            ),
            composition(),
        )
        .expect("entity-definition catalog role");
        assert!(!admitted.project_content_set_hash().is_empty());

        let report = compile_runtime_project_admission(
            admitted_batch_with_roles(
                &empty_scene(10),
                vec![(
                    "content/reference-console.json",
                    ArtifactRole::MaterialCatalog,
                    entity,
                )],
            ),
            composition(),
        )
        .err()
        .expect("role-kind mismatch must reject");
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RuntimeProjectAdmissionDiagnosticCode::ArtifactRoleMismatch
                && diagnostic.path == "content/reference-console.json"
        }));
    }

    #[test]
    fn missing_catalog_resource_is_classified_before_activation() {
        let gameplay = EmptyProjectContentGameplayAdmission::default();
        let decoded = decode_project_content(
            ProjectContentDecodeRequestDto {
                sources: vec![ProjectContentSourceDto {
                    source_path: "catalogs/assets.json".to_owned(),
                    document_id: "catalogs/assets.json".to_owned(),
                    kind: ProjectContentDocumentKind::AssetCatalog,
                    source_text: r#"{"entries":[{"id":"mesh/reference-house","version":1,"hash":null,"sourcePath":"assets/reference-house.glb","label":"House","dependencies":[],"material":null}]}"#.to_owned(),
                }],
            },
            ProjectContentValidationContext {
                scenes: &[],
                entry_scene_id: None,
                gameplay: &gameplay,
                reference_revision: 0,
            },
        );
        assert!(decoded.result.accepted, "{:?}", decoded.result.diagnostics);
        let artifact = decoded.result.canonical_files[0]
            .canonical_json
            .as_bytes()
            .to_vec();
        let report = compile_runtime_project_admission(
            admitted_batch(&empty_scene(8), vec![("content/assets.json", artifact)]),
            composition(),
        )
        .err()
        .expect("missing resource must reject");
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RuntimeProjectAdmissionDiagnosticCode::ResourceNotStaged
                && diagnostic.path == "assets/reference-house.glb"
        }));
    }

    #[test]
    fn presentation_resources_reject_hash_and_role_drift_before_activation() {
        fn canonical_documents(declared_hash: &str) -> Vec<(String, Vec<u8>)> {
            let gameplay = EmptyProjectContentGameplayAdmission::default();
            let decoded = decode_project_content(
                ProjectContentDecodeRequestDto {
                    sources: vec![
                        ProjectContentSourceDto {
                            source_path: "catalogs/assets.json".to_owned(),
                            document_id: "catalogs/assets.json".to_owned(),
                            kind: ProjectContentDocumentKind::AssetCatalog,
                            source_text: format!(
                                r#"{{"entries":[{{"id":"audio/reference-fire","version":1,"hash":"{declared_hash}","sourcePath":"assets/reference-fire.wav","label":"Fire","dependencies":[],"material":null}}]}}"#
                            ),
                        },
                        ProjectContentSourceDto {
                            source_path: "catalogs/presentation.json".to_owned(),
                            document_id: "catalogs/presentation.json".to_owned(),
                            kind: ProjectContentDocumentKind::PresentationCatalog,
                            source_text: format!(
                                r#"{{"schemaVersion":1,"resources":[{{"resourceId":"reference.fire","kind":"audio","assetId":"audio/reference-fire","sourcePath":"assets/reference-fire.wav","contentHash":"{declared_hash}","licensePath":null,"animatedMesh":null}}],"cues":[{{"kind":"audio","cueId":"reference.fire","signalId":"reference.fire","resourceId":"reference.fire","gain":0.5}}]}}"#
                            ),
                        },
                    ],
                },
                ProjectContentValidationContext {
                    scenes: &[],
                    entry_scene_id: None,
                    gameplay: &gameplay,
                    reference_revision: 0,
                },
            );
            assert!(decoded.result.accepted, "{:?}", decoded.result.diagnostics);
            decoded
                .result
                .canonical_files
                .into_iter()
                .map(|file| (file.document_id, file.canonical_json.into_bytes()))
                .collect()
        }

        let resource = b"canonical-reference-audio";
        let actual_hash = svc_serialization::BundleHash::of(resource).to_hex();
        let stale_documents = canonical_documents("0000000000000000");
        let mut stale_contents = stale_documents
            .iter()
            .map(|(path, bytes)| (path.as_str(), ArtifactRole::ProjectContent, bytes.clone()))
            .collect::<Vec<_>>();
        stale_contents.push((
            "assets/reference-fire.wav",
            ArtifactRole::GeneratedMetadata,
            resource.to_vec(),
        ));
        let hash_report = compile_runtime_project_admission(
            admitted_batch_with_roles(&empty_scene(8), stale_contents),
            composition(),
        )
        .err()
        .expect("stale presentation hash must reject");
        assert!(hash_report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RuntimeProjectAdmissionDiagnosticCode::ResourceHashMismatch
                && diagnostic.path == "assets/reference-fire.wav"
        }));

        let valid_documents = canonical_documents(&actual_hash);
        let mut wrong_role_contents = valid_documents
            .iter()
            .map(|(path, bytes)| (path.as_str(), ArtifactRole::ProjectContent, bytes.clone()))
            .collect::<Vec<_>>();
        wrong_role_contents.push((
            "assets/reference-fire.wav",
            ArtifactRole::GeneratedMetadata,
            resource.to_vec(),
        ));
        let role_report = compile_runtime_project_admission(
            admitted_batch_with_roles(&empty_scene(8), wrong_role_contents),
            composition(),
        )
        .err()
        .expect("presentation resource role mismatch must reject");
        assert!(role_report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RuntimeProjectAdmissionDiagnosticCode::ArtifactRoleMismatch
                && diagnostic.path == "assets/reference-fire.wav"
        }));
    }

    #[test]
    fn dangling_scene_reference_is_classified_before_bootstrap() {
        let report = compile_runtime_project_admission(
            admitted_batch(&scene_with_unknown_entity(9), Vec::new()),
            composition(),
        )
        .err()
        .expect("dangling scene reference must reject");
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RuntimeProjectAdmissionDiagnosticCode::DanglingReference
                && diagnostic.path == "scenes.9.instances.missing.entity.instance.reference"
        }));
    }

    #[test]
    fn provider_and_reference_failures_have_stable_categories() {
        for (diagnostic, expected) in [
            (
                ProjectContentDiagnosticDto {
                    code: ProjectContentDiagnosticCode::UnknownReference,
                    document_id: Some("gameplay.json".to_owned()),
                    path: "document.configurations[0].schemaId".to_owned(),
                    message: "configuration references an unknown provider schema".to_owned(),
                },
                RuntimeProjectAdmissionDiagnosticCode::ProviderSchemaMismatch,
            ),
            (
                ProjectContentDiagnosticDto {
                    code: ProjectContentDiagnosticCode::UnknownReference,
                    document_id: Some("gameplay.json".to_owned()),
                    path: "document.bindings[0].target".to_owned(),
                    message: "binding references an unknown prefab role".to_owned(),
                },
                RuntimeProjectAdmissionDiagnosticCode::DanglingReference,
            ),
            (
                ProjectContentDiagnosticDto {
                    code: ProjectContentDiagnosticCode::InvalidDocument,
                    document_id: Some("gameplay.json".to_owned()),
                    path: "document.triggers[1].sceneInstanceId".to_owned(),
                    message: "only one trigger definition may target a stored scene entity"
                        .to_owned(),
                },
                RuntimeProjectAdmissionDiagnosticCode::DuplicateTrigger,
            ),
            (
                ProjectContentDiagnosticDto {
                    code: ProjectContentDiagnosticCode::InvalidDocument,
                    document_id: Some("entities.json".to_owned()),
                    path: "definition.stableId".to_owned(),
                    message: "entity definition stable ids must be unique across the project"
                        .to_owned(),
                },
                RuntimeProjectAdmissionDiagnosticCode::AmbiguousReference,
            ),
        ] {
            assert_eq!(map_project_diagnostic(diagnostic).code, expected);
        }
    }

    #[test]
    fn authored_program_identity_ignores_provenance_but_rejects_semantic_restore_drift() {
        let scene = authored_behavior_scene(11);
        let baseline_composition = authored_behavior_composition();
        let baseline_admission = compile_runtime_project_admission(
            authored_behavior_batch(&scene, &baseline_composition),
            baseline_composition,
        )
        .expect("baseline authored admission");
        let baseline_hash = baseline_admission
            .authored_program
            .as_ref()
            .expect("baseline authored program")
            .program_hash
            .clone();

        let mut provenance_documents = authored_behavior_documents();
        let provenance_package = provenance_documents
            .iter_mut()
            .find_map(|document| match document {
                ProjectContentDocumentDto::BehaviorPackage { package, .. } => Some(package),
                _ => None,
            })
            .expect("provenance package");
        provenance_package.provenance.source_path = "src/renamed-doors.ts".to_owned();
        provenance_package.provenance.source_hash = "fnv1a64:renamed-source".to_owned();
        let provenance_composition = authored_behavior_composition();
        let provenance_admission = compile_runtime_project_admission(
            authored_behavior_batch_with_documents(
                &scene,
                &provenance_composition,
                provenance_documents,
            ),
            provenance_composition,
        )
        .expect("provenance-only authored admission");
        assert_eq!(
            provenance_admission
                .authored_program
                .as_ref()
                .expect("provenance authored program")
                .program_hash,
            baseline_hash,
            "source location and formatting identity must not alter executable semantics"
        );

        let mut semantic_documents = authored_behavior_documents();
        let semantic_package = semantic_documents
            .iter_mut()
            .find_map(|document| match document {
                ProjectContentDocumentDto::BehaviorPackage { package, .. } => Some(package),
                _ => None,
            })
            .expect("semantic package");
        let open_offset = semantic_package
            .behaviors
            .iter_mut()
            .flat_map(|behavior| behavior.steps.iter_mut())
            .flat_map(|step| step.operations.iter_mut())
            .flat_map(|operation| operation.arguments.iter_mut())
            .find_map(|argument| match &mut argument.value {
                AuthoredBehaviorValueDto::Vector3 { value } if *value == [0.0, 3.0, 0.0] => {
                    Some(value)
                }
                _ => None,
            })
            .expect("open offset");
        *open_offset = [0.0, 4.0, 0.0];
        let semantic_composition = authored_behavior_composition();
        let semantic_admission = compile_runtime_project_admission(
            authored_behavior_batch_with_documents(
                &scene,
                &semantic_composition,
                semantic_documents,
            ),
            semantic_composition,
        )
        .expect("semantic-drift authored admission");
        assert_ne!(
            semantic_admission
                .authored_program
                .as_ref()
                .expect("semantic authored program")
                .program_hash,
            baseline_hash
        );

        let actor = baseline_admission
            .runtime_entity_seeds
            .iter()
            .find(|seed| seed.instance_id == "fixture.actor.instance")
            .expect("actor seed")
            .entity;
        let mut baseline_host = GameplayRuntimeHost::activate_validated_project(baseline_admission)
            .expect("baseline activation");
        baseline_host
            .set_actor_translation_and_reconcile(actor, [10.0, 0.0, 0.0], 1)
            .expect("baseline schedules a delayed continuation");
        let snapshot = baseline_host.compose_snapshot().expect("baseline snapshot");
        let restore_error = match GameplayRuntimeHost::restore_validated_project(
            semantic_admission,
            &snapshot.text,
        ) {
            Ok(_) => panic!("semantic drift must reject the saved authored continuation"),
            Err(error) => error,
        };
        assert!(restore_error
            .to_string()
            .contains("saved authored program identity does not match admitted content"));
    }

    #[test]
    fn authored_behavior_runs_through_project_admission_scheduler_and_fresh_restore() {
        let scene = authored_behavior_scene(12);
        let composition = authored_behavior_composition();
        let admission = compile_runtime_project_admission(
            authored_behavior_batch(&scene, &composition),
            composition,
        )
        .expect("authored behavior project admission");
        let door = admission
            .runtime_entity_seeds
            .iter()
            .find(|seed| seed.instance_id == "fixture.door.instance")
            .expect("door seed")
            .entity;
        let actor = admission
            .runtime_entity_seeds
            .iter()
            .find(|seed| seed.instance_id == "fixture.actor.instance")
            .expect("actor seed")
            .entity;
        assert!(admission.authored_program.is_some());

        let mut host = GameplayRuntimeHost::activate_validated_project(admission)
            .expect("validated behavior project activation");
        let interaction = GameplayRuntimePrefabInteractionIntent {
            actor,
            role: "interaction/switch".to_owned(),
            max_distance_millimeters: 3_000,
            tick: 1,
        };
        assert!(host
            .resolve_prefab_part_interaction_target(&interaction)
            .expect("closed door interaction query")
            .is_some());
        let receipt = host
            .interact_with_prefab_part(interaction.clone())
            .expect("eligible authored switch interaction executes the package");
        assert_eq!(
            receipt.event.event,
            StandardGameplayEventKind::PrefabPartInteracted.contract()
        );
        assert!(host
            .resolve_prefab_part_interaction_target(&interaction)
            .expect("open door interaction query")
            .is_none());
        let open_hash = host.readout().runtime_host_hash;
        assert!(host
            .interact_with_prefab_part(interaction.clone())
            .expect_err("open-state predicate must reject a repeat interaction")
            .to_string()
            .contains("no eligible authored prefab role"));
        assert_eq!(host.readout().runtime_host_hash, open_hash);
        assert_eq!(
            host.authored_program
                .as_ref()
                .expect("active authored program")
                .accepted_facts()
                .len(),
            3,
            "state, transform, and collision must each record an accepted owner fact"
        );
        assert_eq!(host.scheduler_readout().pending_action_count, 1);

        let entities = host.take_entity_authority().expect("entity authority");
        assert_eq!(
            entities
                .transform(door)
                .expect("door transform")
                .transform
                .translation
                .to_array(),
            [0.0, 3.0, 5.0]
        );
        assert_eq!(
            entities
                .capability_activation(door, core_entity::ActivatableCapabilityKind::Collision,)
                .expect("door collision activation")
                .presence,
            core_entity::CapabilityActivationPresence::Inactive
        );
        host.install_entity_authority(entities)
            .expect("return entity authority");

        let snapshot = host.compose_snapshot().expect("behavior snapshot");
        let before_restore = host.readout();
        drop(host);

        let restore_composition = authored_behavior_composition();
        let restore_admission = compile_runtime_project_admission(
            authored_behavior_batch(&scene, &restore_composition),
            restore_composition,
        )
        .expect("fresh-process project admission");
        let mut restored =
            GameplayRuntimeHost::restore_validated_project(restore_admission, &snapshot.text)
                .expect("fresh-process behavior restore");
        assert_eq!(
            restored.readout().runtime_host_hash,
            before_restore.runtime_host_hash
        );
        assert_eq!(restored.scheduler_readout().pending_actions.len(), 1);
        restored
            .set_actor_translation_and_reconcile(actor, [0.0, 0.0, 5.0], 120)
            .expect("actor can occupy the closing doorway");
        restored
            .tick(121)
            .expect("unsafe persisted close remains retryable");
        assert_eq!(
            restored.scheduler_readout().outstanding_dispatch_count,
            1,
            "an occupied door must retain its Rust-owned continuation"
        );
        let triggered_snapshot = restored
            .compose_snapshot()
            .expect("snapshot with triggered authored continuation");
        let triggered_restore_composition = authored_behavior_composition();
        let triggered_restore_admission = compile_runtime_project_admission(
            authored_behavior_batch(&scene, &triggered_restore_composition),
            triggered_restore_composition,
        )
        .expect("fresh admission for triggered continuation");
        let mut restored = GameplayRuntimeHost::restore_validated_project(
            triggered_restore_admission,
            &triggered_snapshot.text,
        )
        .expect("fresh-process restore retains a triggered authored continuation");
        assert_eq!(restored.scheduler_readout().outstanding_dispatch_count, 1);
        let occupied_entities = restored
            .take_entity_authority()
            .expect("occupied authority");
        assert_eq!(
            occupied_entities
                .transform(door)
                .expect("occupied door transform")
                .transform
                .translation
                .to_array(),
            [0.0, 3.0, 5.0]
        );
        assert_eq!(
            occupied_entities
                .capability_activation(door, core_entity::ActivatableCapabilityKind::Collision,)
                .expect("occupied door collision activation")
                .presence,
            core_entity::CapabilityActivationPresence::Inactive
        );
        restored
            .install_entity_authority(occupied_entities)
            .expect("return occupied authority");
        restored
            .set_actor_translation_and_reconcile(actor, [15.0, 0.0, 0.0], 122)
            .expect("actor clears the closing doorway");
        restored
            .tick(122)
            .expect("the retained authored continuation closes once safe");
        assert_eq!(restored.scheduler_readout().pending_action_count, 0);
        let entities = restored
            .take_entity_authority()
            .expect("restored authority");
        assert_eq!(
            entities
                .transform(door)
                .expect("restored door transform")
                .transform
                .translation
                .to_array(),
            [0.0, 0.0, 5.0]
        );
        assert_eq!(
            entities
                .capability_activation(door, core_entity::ActivatableCapabilityKind::Collision,)
                .expect("restored door collision activation")
                .presence,
            core_entity::CapabilityActivationPresence::Active
        );
    }
}
