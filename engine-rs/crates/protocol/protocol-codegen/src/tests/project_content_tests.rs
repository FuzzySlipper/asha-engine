use super::*;

pub(super) fn extend_round_trip_coverage(coverage: &mut BTreeSet<String>) {
    coverage.extend([
        interface_coverage_key("projectContent", "ProjectContentSource"),
        interface_coverage_key("projectContent", "ProjectContentReferenceOption"),
        interface_coverage_key("projectContent", "ProjectConfigurationField"),
        interface_coverage_key("projectContent", "ProjectConfigurationSchema"),
        variant_coverage_key("projectContent", "ProjectConfigurationValue", "boolean"),
        variant_coverage_key("projectContent", "ProjectConfigurationValue", "integer"),
        variant_coverage_key("projectContent", "ProjectConfigurationValue", "number"),
        variant_coverage_key("projectContent", "ProjectConfigurationValue", "string"),
        variant_coverage_key("projectContent", "ProjectConfigurationValue", "reference"),
        interface_coverage_key("projectContent", "AuthoredBehaviorProvenance"),
        interface_coverage_key("projectContent", "AuthoredBehaviorState"),
        interface_coverage_key("projectContent", "AuthoredBehaviorTransition"),
        interface_coverage_key("projectContent", "AuthoredBehaviorStateMachine"),
        interface_coverage_key("projectContent", "AuthoredBehaviorSemanticRef"),
        variant_coverage_key("projectContent", "AuthoredBehaviorValue", "sceneEntity"),
        variant_coverage_key("projectContent", "AuthoredBehaviorValue", "prefabPart"),
        variant_coverage_key("projectContent", "AuthoredBehaviorValue", "stateMachine"),
        variant_coverage_key("projectContent", "AuthoredBehaviorValue", "state"),
        variant_coverage_key("projectContent", "AuthoredBehaviorValue", "text"),
        variant_coverage_key("projectContent", "AuthoredBehaviorValue", "boolean"),
        variant_coverage_key("projectContent", "AuthoredBehaviorValue", "integer"),
        variant_coverage_key("projectContent", "AuthoredBehaviorValue", "number"),
        variant_coverage_key("projectContent", "AuthoredBehaviorValue", "vector3"),
        interface_coverage_key("projectContent", "AuthoredBehaviorArgument"),
        interface_coverage_key("projectContent", "AuthoredBehaviorSignal"),
        interface_coverage_key("projectContent", "AuthoredBehaviorCondition"),
        interface_coverage_key("projectContent", "AuthoredBehaviorOperation"),
        interface_coverage_key("projectContent", "AuthoredBehaviorStep"),
        interface_coverage_key("projectContent", "AuthoredBehaviorDefinition"),
        interface_coverage_key("projectContent", "AuthoredBehaviorPackage"),
        interface_coverage_key("projectContent", "ProjectConfigurationFieldValue"),
        interface_coverage_key("projectContent", "ProjectGameplayConfiguration"),
        interface_coverage_key("projectContent", "ProjectGameplayConfigurationDocument"),
        interface_coverage_key("projectContent", "ProjectAnimationClipDescriptor"),
        interface_coverage_key("projectContent", "ProjectMeshMaterialSlot"),
        interface_coverage_key("projectContent", "ProjectMeshBoundsDescriptor"),
        interface_coverage_key("projectContent", "ProjectAnimatedMeshDescriptor"),
        interface_coverage_key("projectContent", "ProjectPresentationResource"),
        interface_coverage_key("projectContent", "ProjectPresentationSignal"),
        variant_coverage_key("projectContent", "ProjectPresentationCue", "animation"),
        variant_coverage_key("projectContent", "ProjectPresentationCue", "audio"),
        variant_coverage_key("projectContent", "ProjectPresentationCue", "particle"),
        variant_coverage_key("projectContent", "ProjectPresentationCue", "overlay"),
        interface_coverage_key("projectContent", "ProjectPresentationCatalog"),
        variant_coverage_key(
            "projectContent",
            "ProjectContentDocument",
            "entityDefinition",
        ),
        variant_coverage_key("projectContent", "ProjectContentDocument", "assetCatalog"),
        variant_coverage_key("projectContent", "ProjectContentDocument", "prefabRegistry"),
        variant_coverage_key(
            "projectContent",
            "ProjectContentDocument",
            "gameplayConfiguration",
        ),
        variant_coverage_key(
            "projectContent",
            "ProjectContentDocument",
            "presentationCatalog",
        ),
        variant_coverage_key("projectContent", "ProjectContentDocument", "inputCatalog"),
        variant_coverage_key(
            "projectContent",
            "ProjectContentDocument",
            "behaviorPackage",
        ),
        interface_coverage_key("projectContent", "ProjectContentDecodeRequest"),
        interface_coverage_key("projectContent", "ProjectContentEncodeRequest"),
        interface_coverage_key("projectContent", "ProjectContentDiagnostic"),
        interface_coverage_key("projectContent", "ProjectContentCanonicalFile"),
        interface_coverage_key("projectContent", "ProjectContentFieldMetadata"),
        variant_coverage_key(
            "projectContent",
            "ProjectEntityAppearanceUpdate",
            "resource",
        ),
        variant_coverage_key(
            "projectContent",
            "ProjectEntityAppearanceUpdate",
            "initialClip",
        ),
        variant_coverage_key(
            "projectContent",
            "ProjectEntityAppearanceUpdate",
            "modelScale",
        ),
        interface_coverage_key("projectContent", "ProjectContentCodecResult"),
        interface_coverage_key("projectContent", "ActiveRuntimeProjectEntityRoleReadout"),
        interface_coverage_key("projectContent", "ActiveRuntimeProjectDomainReadout"),
        interface_coverage_key("projectContent", "ActiveRuntimeProjectContentReadout"),
        variant_coverage_key("projectContent", "ProjectContentAuthoringCommand", "upsert"),
        variant_coverage_key("projectContent", "ProjectContentAuthoringCommand", "delete"),
        variant_coverage_key(
            "projectContent",
            "ProjectContentAuthoringCommand",
            "updateEntityAppearance",
        ),
        interface_coverage_key("projectContent", "ProjectContentAuthoringRequest"),
        interface_coverage_key("projectContent", "ProjectContentAuthoringResult"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentLimits"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentMarkerTarget"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentTarget"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentPreviewRequest"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentDiagnostic"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentProvenance"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentMarkerReadout"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentSourceReadout"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentCanonicalFile"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentArtifactCandidate"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentPreviewResult"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentApplyRequest"),
        interface_coverage_key("projectContent", "ProceduralEnvironmentApplyResult"),
    ]);
}

#[test]
fn project_content_samples_match_closed_generated_ir_shapes() {
    let project = module("projectContent");
    let source = json!({
        "sourcePath": "content/gameplay.json",
        "documentId": "gameplay/demo",
        "kind": "gameplayConfiguration",
        "sourceText": "{}"
    });
    let field = json!({
        "fieldId": "damage",
        "label": "Damage",
        "valueKind": "integer",
        "required": true,
        "referenceKind": null,
        "integerMin": 0,
        "integerMax": 100,
        "numberMin": null,
        "numberMax": null
    });
    let schema = json!({
        "schemaId": "demo.weapon.v1",
        "moduleId": "demo.weapon.module",
        "providerId": "demo.weapon",
        "contract": { "contractId": "demo.weapon", "version": 1 },
        "codecId": "asha.project-configuration.canonical-json.v1",
        "fields": [field]
    });
    let values = [
        ("boolean", json!({ "kind": "boolean", "value": true })),
        ("integer", json!({ "kind": "integer", "value": 12 })),
        ("number", json!({ "kind": "number", "value": 1.5 })),
        ("string", json!({ "kind": "string", "value": "demo" })),
        (
            "reference",
            json!({ "kind": "reference", "referenceKind": "sceneInstance", "targetId": "demo.target" }),
        ),
    ];
    for (tag, value) in &values {
        compare_object_to_variant(&project, "ProjectConfigurationValue", tag, value).unwrap();
    }
    let field_value = json!({ "fieldId": "damage", "value": values[1].1 });
    let configuration = json!({
        "configurationId": "demo.weapon.primary",
        "module": { "moduleId": "demo.weapon", "version": 1 },
        "schemaId": "demo.weapon.v1",
        "values": [field_value]
    });
    let gameplay = json!({
        "schemaVersion": 1,
        "configurations": [configuration],
        "bindings": [],
        "overrides": [],
        "triggers": []
    });
    let animation_clip = json!({ "id": "fire", "name": null, "durationSeconds": 0.4 });
    let material_slot = json!({ "slot": 0, "material": "material/demo-weapon" });
    let mesh_bounds = json!({ "min": [-0.5, -0.5, -0.5], "max": [0.5, 0.5, 0.5] });
    let animated_mesh = json!({
        "asset": "mesh/demo-weapon",
        "runtimeFormat": "glb",
        "contentHash": "sha256:mesh",
        "clips": [animation_clip],
        "defaultClip": "fire",
        "materialSlots": [material_slot],
        "bounds": mesh_bounds
    });
    let resource = json!({
        "resourceId": "demo.weapon.mesh",
        "kind": "animatedMesh",
        "assetId": "mesh/demo-weapon",
        "sourcePath": "assets/demo-weapon.mesh",
        "contentHash": "sha256:mesh",
        "licensePath": null,
        "animatedMesh": animated_mesh
    });
    let cues = [
        (
            "animation",
            json!({ "kind": "animation", "cueId": "weapon.fire", "resourceId": "demo.weapon.mesh", "clipId": "fire", "looped": false, "atSeconds": 0.05, "signal": { "domain": "particle", "signalId": "weapon.fire.impact" } }),
        ),
        (
            "audio",
            json!({ "kind": "audio", "cueId": "weapon.sound", "signalId": "weapon.fire", "resourceId": "demo.weapon.audio", "gain": 0.8 }),
        ),
        (
            "particle",
            json!({ "kind": "particle", "cueId": "weapon.flash", "signalId": "weapon.fire.impact", "resourceId": "demo.weapon.particle", "scale": 1.0 }),
        ),
        (
            "overlay",
            json!({ "kind": "overlay", "cueId": "hud.crosshair", "resourceId": "demo.hud" }),
        ),
    ];
    for (tag, cue) in &cues {
        compare_object_to_variant(&project, "ProjectPresentationCue", tag, cue).unwrap();
    }
    let presentation = json!({ "schemaVersion": 1, "resources": [resource], "cues": cues.iter().map(|(_, cue)| cue).collect::<Vec<_>>() });
    let input_catalog = json!({
        "schemaVersion": 1,
        "namespace": "demo",
        "actions": [{ "actionId": "demo.interact", "valueKind": "button", "acceptedPhases": ["pressed"] }],
        "contexts": [],
        "bindings": [{
            "bindingId": "demo.interact.primary",
            "actionId": "demo.interact",
            "contextId": "gameplay",
            "platformKind": "keyboardKey",
            "control": "KeyE",
            "scale": 1.0,
            "extension": null
        }]
    });
    let behavior_provenance = json!({
        "sdkId": "@asha/game-workspace",
        "sdkVersion": 1,
        "vocabularyHash": "fnv1a64:authored-behavior-v1",
        "sourceModule": "@demo/content",
        "sourcePath": "src/content/main-door.ts",
        "sourceHash": "fnv1a64:source"
    });
    let closed_state = json!({ "stateId": "closed" });
    let open_state = json!({ "stateId": "open" });
    let open_transition = json!({
        "transitionId": "open-door",
        "fromStateId": "closed",
        "toStateId": "open"
    });
    let close_transition = json!({
        "transitionId": "close-door",
        "fromStateId": "open",
        "toStateId": "closed"
    });
    let behavior_machine = json!({
        "machineId": "main-door",
        "targetSceneInstanceId": "scene.main/door",
        "initialStateId": "closed",
        "states": [closed_state, open_state],
        "transitions": [open_transition, close_transition]
    });
    let semantic_ref = json!({ "semanticId": "asha.signal.prefab-part-interacted", "version": 1 });
    let values = [
        (
            "sceneEntity",
            json!({ "kind": "sceneEntity", "sceneInstanceId": "scene.main/door" }),
        ),
        (
            "prefabPart",
            json!({ "kind": "prefabPart", "sceneInstanceId": "scene.main/switch", "role": "button" }),
        ),
        (
            "stateMachine",
            json!({ "kind": "stateMachine", "machineId": "main-door" }),
        ),
        (
            "state",
            json!({ "kind": "state", "machineId": "main-door", "stateId": "closed" }),
        ),
        ("text", json!({ "kind": "text", "value": "open-door" })),
        ("boolean", json!({ "kind": "boolean", "value": true })),
        ("integer", json!({ "kind": "integer", "value": 12 })),
        ("number", json!({ "kind": "number", "value": 1.5 })),
        (
            "vector3",
            json!({ "kind": "vector3", "value": [0.0, 3.0, 0.0] }),
        ),
    ];
    for (tag, value) in &values {
        compare_object_to_variant(&project, "AuthoredBehaviorValue", tag, value).unwrap();
    }
    let prefab_argument = json!({ "name": "part", "value": values[1].1 });
    let prefab_signal = json!({
        "signal": semantic_ref,
        "arguments": [prefab_argument]
    });
    let behavior_condition = json!({
        "predicate": { "semanticId": "asha.predicate.state-is", "version": 1 },
        "arguments": [{ "name": "state", "value": values[3].1 }]
    });
    let transition_operation = json!({
        "verb": { "semanticId": "asha.verb.transition-state", "version": 1 },
        "arguments": [
            { "name": "machine", "value": values[2].1 },
            { "name": "transition", "value": values[4].1 }
        ]
    });
    let immediate_step = json!({
        "stepId": "open",
        "afterStepIds": [],
        "delayTicks": 0,
        "operations": [transition_operation]
    });
    let delayed_step = json!({
        "stepId": "close",
        "afterStepIds": ["open"],
        "delayTicks": 120,
        "operations": [{
            "verb": { "semanticId": "asha.verb.transition-state", "version": 1 },
            "arguments": [
                { "name": "machine", "value": values[2].1 },
                { "name": "transition", "value": { "kind": "text", "value": "close-door" } }
            ]
        }]
    });
    let behavior_definition = json!({
        "behaviorId": "open-then-close",
        "signal": prefab_signal,
        "conditions": [behavior_condition],
        "steps": [immediate_step, delayed_step]
    });
    let behavior_package = json!({
        "schemaVersion": 1,
        "packageId": "demo.main-door",
        "provenance": behavior_provenance,
        "stateMachines": [behavior_machine],
        "behaviors": [behavior_definition]
    });

    compare_object_to_interface(&project, "ProjectContentSource", &source).unwrap();
    compare_object_to_interface(&project, "ProjectConfigurationField", &field).unwrap();
    compare_object_to_interface(&project, "ProjectConfigurationSchema", &schema).unwrap();
    compare_object_to_interface(&project, "ProjectConfigurationFieldValue", &field_value).unwrap();
    compare_object_to_interface(&project, "ProjectGameplayConfiguration", &configuration).unwrap();
    compare_object_to_interface(&project, "ProjectGameplayConfigurationDocument", &gameplay)
        .unwrap();
    compare_object_to_interface(&project, "ProjectAnimationClipDescriptor", &animation_clip)
        .unwrap();
    compare_object_to_interface(&project, "ProjectMeshMaterialSlot", &material_slot).unwrap();
    compare_object_to_interface(&project, "ProjectMeshBoundsDescriptor", &mesh_bounds).unwrap();
    compare_object_to_interface(&project, "ProjectAnimatedMeshDescriptor", &animated_mesh).unwrap();
    compare_object_to_interface(&project, "ProjectPresentationResource", &resource).unwrap();
    compare_object_to_interface(
        &project,
        "ProjectPresentationSignal",
        &json!({ "domain": "particle", "signalId": "weapon.fire.impact" }),
    )
    .unwrap();
    compare_object_to_interface(&project, "ProjectPresentationCatalog", &presentation).unwrap();
    for (name, value) in [
        ("AuthoredBehaviorProvenance", &behavior_provenance),
        ("AuthoredBehaviorState", &closed_state),
        ("AuthoredBehaviorTransition", &open_transition),
        ("AuthoredBehaviorStateMachine", &behavior_machine),
        ("AuthoredBehaviorSemanticRef", &semantic_ref),
        ("AuthoredBehaviorArgument", &prefab_argument),
        ("AuthoredBehaviorSignal", &prefab_signal),
        ("AuthoredBehaviorCondition", &behavior_condition),
        ("AuthoredBehaviorOperation", &transition_operation),
        ("AuthoredBehaviorStep", &immediate_step),
        ("AuthoredBehaviorDefinition", &behavior_definition),
        ("AuthoredBehaviorPackage", &behavior_package),
    ] {
        compare_object_to_interface(&project, name, value).unwrap();
    }
    let documents = [
        (
            "entityDefinition",
            json!({ "kind": "entityDefinition", "documentId": "entity/demo", "definition": {} }),
        ),
        (
            "assetCatalog",
            json!({ "kind": "assetCatalog", "documentId": "catalog/demo", "catalog": {} }),
        ),
        (
            "prefabRegistry",
            json!({ "kind": "prefabRegistry", "documentId": "prefabs/demo", "registry": {} }),
        ),
        (
            "gameplayConfiguration",
            json!({ "kind": "gameplayConfiguration", "documentId": "gameplay/demo", "document": gameplay }),
        ),
        (
            "presentationCatalog",
            json!({ "kind": "presentationCatalog", "documentId": "presentation/demo", "catalog": presentation }),
        ),
        (
            "inputCatalog",
            json!({ "kind": "inputCatalog", "documentId": "input/demo", "catalog": input_catalog }),
        ),
        (
            "behaviorPackage",
            json!({ "kind": "behaviorPackage", "documentId": "behavior/main-door", "package": behavior_package }),
        ),
    ];
    for (tag, document) in &documents {
        compare_object_to_variant(&project, "ProjectContentDocument", tag, document).unwrap();
    }
    let decode = json!({ "sources": [source] });
    let encode = json!({ "documents": [documents[3].1] });
    let diagnostic = json!({ "code": "invalidField", "documentId": "gameplay/demo", "path": "configurations[0]", "message": "invalid" });
    let canonical = json!({ "sourcePath": "content/gameplay.json", "documentId": "gameplay/demo", "kind": "gameplayConfiguration", "canonicalJson": "{}\n", "contentHash": "fnv1a64:1" });
    let reference_option = json!({ "targetId": "actor/demo-player", "label": "Player" });
    let metadata = json!({
        "documentId": "gameplay/demo",
        "fieldId": "damage",
        "path": "document.configurations[0].values.damage",
        "label": "Damage",
        "valueKind": "integer",
        "required": true,
        "editable": true,
        "referenceKind": null,
        "referenceOptions": [],
        "configurationId": "demo.weapon.primary",
        "schemaId": "demo.weapon.v1",
        "moduleId": "demo.weapon.module",
        "providerId": "demo.weapon",
        "contract": { "contractId": "demo.weapon", "version": 1 },
        "codecId": "asha.project-configuration.canonical-json.v1",
        "integerMin": 0,
        "integerMax": 100,
        "numberMin": null,
        "numberMax": null
    });
    let result = json!({ "accepted": true, "documents": [documents[3].1], "canonicalFiles": [canonical], "setHash": "fnv1a64:set", "providerSchemas": [schema], "fieldMetadata": [metadata], "diagnostics": [] });
    let upsert = json!({ "kind": "upsert", "sourcePath": "content/gameplay.json", "document": documents[3].1 });
    let delete = json!({ "kind": "delete", "documentId": "gameplay/demo", "documentKind": "gameplayConfiguration" });
    let appearance_resource = json!({ "kind": "resource", "resourceId": "presentation/enemy" });
    let appearance_clip = json!({ "kind": "initialClip", "initialClipId": null });
    let appearance_scale = json!({ "kind": "modelScale", "axis": 1, "value": 2.0 });
    let update_appearance = json!({
        "kind": "updateEntityAppearance",
        "documentId": "entity/demo",
        "projectionId": "enemy",
        "update": appearance_resource,
    });
    let authoring = json!({ "expectedWorkspaceId": "workspace-1", "expectedGeneration": 2, "expectedWorkingRevision": 3, "expectedSetHash": "fnv1a64:set", "command": upsert });

    for (name, value) in [
        ("ProjectContentDecodeRequest", &decode),
        ("ProjectContentEncodeRequest", &encode),
        ("ProjectContentDiagnostic", &diagnostic),
        ("ProjectContentCanonicalFile", &canonical),
        ("ProjectContentReferenceOption", &reference_option),
        ("ProjectContentFieldMetadata", &metadata),
        ("ProjectContentCodecResult", &result),
        ("ProjectContentAuthoringRequest", &authoring),
        ("ProjectContentAuthoringResult", &result),
    ] {
        compare_object_to_interface(&project, name, value).unwrap();
    }
    compare_object_to_variant(
        &project,
        "ProjectContentAuthoringCommand",
        "upsert",
        &upsert,
    )
    .unwrap();
    for (tag, update) in [
        ("resource", &appearance_resource),
        ("initialClip", &appearance_clip),
        ("modelScale", &appearance_scale),
    ] {
        compare_object_to_variant(&project, "ProjectEntityAppearanceUpdate", tag, update).unwrap();
    }
    compare_object_to_variant(
        &project,
        "ProjectContentAuthoringCommand",
        "updateEntityAppearance",
        &update_appearance,
    )
    .unwrap();
    compare_object_to_variant(
        &project,
        "ProjectContentAuthoringCommand",
        "delete",
        &delete,
    )
    .unwrap();

    let environment_limits = json!({ "maxVoxels": 10000, "maxSparseRuns": 10000, "maxMarkers": 8 });
    let marker_target = json!({
        "sourceMarkerId": "player_start",
        "nodeId": 11,
        "markerId": "spawn/player",
        "childOrder": 0
    });
    let environment_target = json!({
        "sceneId": 7,
        "scenePath": "scenes/tunnel.scene.json",
        "assetId": "voxel-volume/generated-tunnel",
        "assetPath": "assets/generated-tunnel.avxl.json",
        "voxelNodeId": 10,
        "voxelParentId": null,
        "voxelChildOrder": 1,
        "voxelLabel": "Generated tunnel",
        "voxelTransform": {
            "translation": [-3.5, -1.0, -5.5],
            "rotation": [0.0, 0.0, 0.0, 1.0],
            "scale": [1.0, 1.0, 1.0]
        },
        "markerTargets": [marker_target]
    });
    let palette_binding = json!({
        "voxelMaterial": 1,
        "paletteEntryId": "voxel-material/tunnel",
        "displayName": "Tunnel",
        "materialAssetId": "material/tunnel",
        "materialCatalogBindingId": null
    });
    let asset_authoring = json!({
        "label": "Generated tunnel",
        "createdBy": "protocol-codegen",
        "sourceTool": "svc-environment-authoring"
    });
    let preview_request = json!({
        "expectedWorkspaceId": "workspace-1",
        "expectedGeneration": 2,
        "expectedWorkingRevision": 3,
        "expectedSceneContentHash": "fnv1a64:scene",
        "providerId": "asha.tunnel.enclosed.v2",
        "presetId": "tiny-enclosed",
        "seed": 42,
        "target": environment_target,
        "materialPalette": [palette_binding],
        "authoring": asset_authoring,
        "limits": environment_limits
    });
    let environment_diagnostic =
        json!({ "code": "staleCandidate", "path": "candidateHash", "message": "stale" });
    let provenance = json!({
        "providerId": "asha.tunnel.enclosed.v2",
        "providerVersion": 2,
        "presetId": "tiny-enclosed",
        "seed": 42,
        "configHash": "fnv1a64:config",
        "outputHash": "fnv1a64:output"
    });
    let marker_readout = json!({
        "sourceMarkerId": "player_start",
        "markerId": "spawn/player",
        "nodeId": 11,
        "localPosition": [1.0, 2.0, 3.0],
        "yawDegrees": 90
    });
    let sources = json!({
        "voxelDataHash": "fnv1a64:voxels",
        "collisionSourceHash": "fnv1a64:collision",
        "navigationSourceHash": "fnv1a64:navigation",
        "solidVoxelCount": 120,
        "walkableVoxelCount": 40
    });
    let scene_file = json!({
        "path": "scenes/tunnel.scene.json",
        "mediaType": "application/vnd.asha.scene+json;version=4",
        "canonicalJson": "{}\n",
        "contentHash": "fnv1a64:scene"
    });
    let voxel_file = json!({
        "path": "assets/generated-tunnel.avxl.json",
        "mediaType": "application/vnd.asha.voxel-volume+json;version=1",
        "canonicalJson": "{}\n",
        "contentHash": "fnv1a64:asset"
    });
    let stored_scene = json!({
        "schemaVersion": 4,
        "id": 7,
        "metadata": { "name": "Tunnel", "authoringFormatVersion": 4 },
        "dependencies": [],
        "nodes": []
    });
    let stored_asset = json!({
        "assetId": "voxel-volume/generated-tunnel",
        "schemaVersion": 1,
        "mediaType": "application/vnd.asha.voxel-volume+json;version=1",
        "grid": { "origin": [0.0, 0.0, 0.0], "cellSize": 1.0, "coordinateSystem": "asha-y-up-right-handed" },
        "bounds": { "min": { "x": 0, "y": 0, "z": 0 }, "max": { "x": 1, "y": 1, "z": 1 } },
        "representation": { "kind": "sparse_runs", "sparseRuns": [] },
        "materialPalette": [palette_binding],
        "provenance": [{ "kind": "generated", "uri": "asha-generator://tunnel", "contentHash": "fnv1a64:output" }],
        "authoring": asset_authoring,
        "validationDiagnostics": [],
        "contentHashes": { "canonicalJson": "fnv1a64:asset", "voxelData": "fnv1a64:voxels" }
    });
    let artifact_candidate = json!({
        "candidateHash": "fnv1a64:candidate",
        "sceneFile": scene_file,
        "voxelFile": voxel_file,
        "artifactSetHash": "fnv1a64:set",
        "scene": stored_scene,
        "asset": stored_asset,
        "provenance": provenance,
        "markers": [marker_readout],
        "sources": sources
    });
    let preview_result = json!({
        "accepted": true,
        "candidate": artifact_candidate,
        "previewFrame": { "ops": [] },
        "previewProjectionHash": "fnv1a64:preview",
        "previewDiffCount": 1,
        "diagnostics": []
    });
    let apply_request = json!({
        "expectedWorkspaceId": "workspace-1",
        "expectedGeneration": 2,
        "expectedWorkingRevision": 3,
        "candidateHash": "fnv1a64:candidate"
    });
    let apply_result = json!({
        "accepted": true,
        "workingRevision": 4,
        "saveCandidateHash": "fnv1a64:set",
        "candidate": artifact_candidate,
        "diagnostics": []
    });
    for (name, value) in [
        ("ProceduralEnvironmentLimits", &environment_limits),
        ("ProceduralEnvironmentMarkerTarget", &marker_target),
        ("ProceduralEnvironmentTarget", &environment_target),
        ("ProceduralEnvironmentPreviewRequest", &preview_request),
        ("ProceduralEnvironmentDiagnostic", &environment_diagnostic),
        ("ProceduralEnvironmentProvenance", &provenance),
        ("ProceduralEnvironmentMarkerReadout", &marker_readout),
        ("ProceduralEnvironmentSourceReadout", &sources),
        ("ProceduralEnvironmentCanonicalFile", &scene_file),
        (
            "ProceduralEnvironmentArtifactCandidate",
            &artifact_candidate,
        ),
        ("ProceduralEnvironmentPreviewResult", &preview_result),
        ("ProceduralEnvironmentApplyRequest", &apply_request),
        ("ProceduralEnvironmentApplyResult", &apply_result),
    ] {
        compare_object_to_interface(&project, name, value).unwrap();
    }
}
