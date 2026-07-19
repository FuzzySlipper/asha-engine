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
    ProjectContentDiagnosticCode, ProjectContentDiagnosticDto, ProjectContentDocumentDto,
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
    declared_reads: Vec<GameplayRuntimeDeclaredReadPlan>,
    triggers: Vec<protocol_project_bundle::GameplayTriggerDefinition>,
    scheduler: GameplayRuntimeSchedulerDefinition,
    prefabs: GameplayRuntimePrefabBootstrap,
    artifacts: BundleArtifacts,
    voxel_assets: BTreeMap<String, VoxelVolumeAsset>,
    admission_hash: String,
}

pub(crate) struct RuntimeProjectActivationParts {
    pub manifest_hash: BundleHash,
    pub project_id: u64,
    pub entry_scene: FlatSceneDocument,
    pub load_plan: LoadPlan,
    pub artifacts: BundleArtifacts,
    pub bootstrap_resolution: core_scene::BootstrapResolutionContext,
    pub composition: GameplayStaticComposition,
    pub bindings: GameplayModuleBindingRegistry,
    pub entity_targets: GameplayBindingEntityTargets,
    pub spatial_entities: Vec<GameplayRuntimeSpatialEntity>,
    pub declared_reads: Vec<GameplayRuntimeDeclaredReadPlan>,
    pub triggers: Vec<protocol_project_bundle::GameplayTriggerDefinition>,
    pub scheduler: GameplayRuntimeSchedulerDefinition,
    pub prefabs: GameplayRuntimePrefabBootstrap,
    pub voxel_assets: BTreeMap<String, VoxelVolumeAsset>,
    pub admission_hash: String,
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
        gameplay_module_payload_hash(
            format!(
                "{:?}|{:?}|{}|{:?}|{:?}|{:?}|{:?}|{}|{}",
                self.load_plan,
                self.bootstrap_resolution,
                self.bindings.registry_hash,
                self.entity_targets,
                self.spatial_entities,
                self.triggers,
                self.scheduler,
                self.prefabs.registry_json,
                self.prefabs.placements.len(),
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
            load_plan: self.load_plan,
            artifacts: self.artifacts,
            bootstrap_resolution: self.bootstrap_resolution,
            composition: self.composition,
            bindings: self.bindings,
            entity_targets: self.entity_targets,
            spatial_entities: self.spatial_entities,
            declared_reads: self.declared_reads,
            triggers: self.triggers,
            scheduler: self.scheduler,
            prefabs: self.prefabs,
            voxel_assets: self.voxel_assets,
            admission_hash: self.admission_hash,
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

    let bindings = compiled_bindings(&content);
    let (entity_targets, spatial_entities) =
        derive_entity_authority(&content, &bootstrap, &mut report);
    let prefabs = derive_prefab_bootstrap(&content, &bootstrap, &mut report);
    if !report.is_valid() {
        report.canonicalize();
        return Err(report);
    }
    let declared_reads = composition.declared_reads().to_vec();
    let scheduler = derive_scheduler(&composition);
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
        declared_reads,
        triggers,
        scheduler,
        prefabs,
        artifacts,
        voxel_assets,
        admission_hash,
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
                    }
                }
            }
            ProjectContentDocumentDto::PresentationCatalog {
                document_id,
                catalog,
            } => {
                for resource in &catalog.resources {
                    require_staged_path(source, report, document_id, &resource.source_path);
                    if let Some(path) = resource.license_path.as_deref() {
                        require_staged_path(source, report, document_id, path);
                    }
                }
            }
            _ => {}
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

fn definitions(content: &ValidatedProjectContentSet) -> BTreeMap<&str, &EntityDefinition> {
    content
        .result()
        .documents
        .iter()
        .filter_map(|document| match document {
            ProjectContentDocumentDto::EntityDefinition { definition, .. } => {
                Some((definition.stable_id.as_str(), definition))
            }
            _ => None,
        })
        .collect()
}

fn derive_entity_authority(
    content: &ValidatedProjectContentSet,
    bootstrap: &BootstrapPlan,
    report: &mut RuntimeProjectAdmissionReport,
) -> (
    GameplayBindingEntityTargets,
    Vec<GameplayRuntimeSpatialEntity>,
) {
    let definitions = definitions(content);
    let mut targets = GameplayBindingEntityTargets::new();
    let mut spatial = Vec::new();
    for instance in bootstrap.resolved_instances() {
        let SceneEntityReference::EntityDefinition { stable_id } = &instance.reference else {
            continue;
        };
        targets.bind(stable_id.clone(), instance.entity);
        let Some(definition) = definitions.get(stable_id.as_str()).copied() else {
            continue;
        };
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
    (targets, spatial)
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

fn derive_scheduler(composition: &GameplayStaticComposition) -> GameplayRuntimeSchedulerDefinition {
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
    use core_ids::{ProjectId, SceneId, SceneNodeId};
    use core_scene::{
        FlatSceneDocument, NodeMetadata, SceneEntityInstance, SceneEntityReference, SceneMetadata,
        SceneNodeKind, SceneNodeRecord, SceneTransform,
    };
    use gameplay_module_sdk::GameplayStaticCompositionBuilder;
    use protocol_entity_authoring::{
        EntityDefinitionCapability, EntityDefinitionMetadataEntry, EntityDefinitionSourceTrace,
    };
    use protocol_project_content::{
        ProjectContentDecodeRequestDto, ProjectContentDocumentKind, ProjectContentSourceDto,
    };
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
        let lock_bytes = b"asset-lock-v1".to_vec();
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
                    document_id: "catalogs/assets.json".to_owned(),
                    kind: ProjectContentDocumentKind::AssetCatalog,
                    source_text: r#"{"entries":[{"id":"mesh/reference-house","version":1,"hash":null,"sourcePath":"assets/reference-house.glb","label":"House","dependencies":[],"material":null}]}"#.to_owned(),
                }],
            },
            ProjectContentValidationContext {
                scenes: &[],
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
}
