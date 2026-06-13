//! Intentionally-broken fixtures and their diagnostic goldens (scene-capability-06,
//! subtask #2332).
//!
//! Each test builds one deliberately-broken scenario, renders its diagnostics to
//! the deterministic text form, and asserts byte-equality against a committed
//! golden under `harness/fixtures/diagnostics/`. The goldens double as
//! agent-training examples: a stable code + source ref + remedy for each failure
//! class, documented in that directory's `README.md`.
//!
//! Regenerate the goldens after an intended change:
//!
//! ```text
//! BLESS=1 cargo test -p scene-diagnostics --test goldens
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use core_assets::{AssetId, AssetReference, AssetVersionReq};
use core_catalog::{
    material::{MaterialAuthority, MaterialDef, MaterialStyle, Rgba, StructuralClass},
    Catalog, CatalogEntry,
};
use core_events::VoxelEditEvent;
use core_ids::{SceneId, SceneNodeId, WorldId};
use core_scene::document::{FlatSceneDocument, SceneMetadata, SceneNodeKind, SceneNodeRecord};
use core_scene::transform::SceneTransform;
use core_space::{ChunkCoord, ChunkDims, GridId, VoxelCoord, VoxelGridSpec};
use core_voxel::VoxelValue;
use rule_voxel_edit::generate_chunk;
use scene_diagnostics::text::{report_set_to_text, resource_report_to_text, traces_to_text};
use scene_diagnostics::{
    artifact_integrity_diagnostics, build_source_traces, catalog_diagnostics, manifest_diagnostics,
    missing_cache_diagnostics, resource_diagnostics, scene_diagnostics, source_trace_diagnostics,
    voxel_round_trip, ProjectionRecord, RendererResourceReport,
};
use svc_serialization::artifact::ArtifactRole;
use svc_serialization::{
    ArtifactEntry, AssetLockSection, BundleHash, GeneratorMetadata, SceneSection,
    WorldBundleManifest, WorldSection,
};

// ── golden harness ────────────────────────────────────────────────────────────

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../harness/fixtures/diagnostics")
        .join(name)
}

/// Compare `actual` to the committed golden `name`. With `BLESS=1` set, write the
/// golden instead (used to (re)generate after an intended change).
fn check_golden(name: &str, actual: &str) {
    let path = golden_path(name);
    if std::env::var_os("BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "missing golden {}: {e}\n  regenerate with: BLESS=1 cargo test -p scene-diagnostics --test goldens",
            path.display()
        )
    });
    assert_eq!(
        actual, expected,
        "golden {} drifted; if intended, regenerate with BLESS=1",
        name
    );
}

// ── builders ──────────────────────────────────────────────────────────────────

fn aref(s: &str) -> AssetReference {
    AssetReference::new(AssetId::parse(s).unwrap(), AssetVersionReq::Any, None)
}

fn id(s: &str) -> AssetId {
    AssetId::parse(s).unwrap()
}

fn node(idn: u64, parent: Option<u64>, kind: SceneNodeKind) -> SceneNodeRecord {
    SceneNodeRecord {
        id: SceneNodeId::new(idn),
        parent: parent.map(SceneNodeId::new),
        child_order: 0,
        transform: SceneTransform::IDENTITY,
        kind,
        metadata: Default::default(),
    }
}

fn doc(nodes: Vec<SceneNodeRecord>) -> FlatSceneDocument {
    FlatSceneDocument {
        id: SceneId::new(1),
        schema_version: 1,
        metadata: SceneMetadata::default(),
        dependencies: Vec::new(),
        nodes,
    }
}

fn material_with_texture(texture: Option<&str>) -> MaterialDef {
    MaterialDef {
        authority: MaterialAuthority {
            solid: true,
            collidable: true,
            occludes: true,
            structural_class: StructuralClass::Structural,
        },
        style: MaterialStyle {
            texture: texture.map(aref),
            ..MaterialStyle::flat(Rgba::DEBUG_GREY)
        },
    }
}

fn manifest() -> WorldBundleManifest {
    WorldBundleManifest {
        bundle_schema_version: 1,
        protocol_version: 1,
        world: WorldSection {
            id: WorldId::new(1),
            name: None,
        },
        scene: SceneSection {
            id: SceneId::new(1),
            schema_version: 1,
            artifact: "scene/scene.json".to_string(),
        },
        asset_lock: AssetLockSection {
            artifact: "scene/asset-lock.json".to_string(),
            asset_count: 0,
        },
        generator: GeneratorMetadata {
            seed: 7,
            version: 1,
            params: "p".to_string(),
        },
        artifacts: vec![
            ArtifactEntry::durable(
                "scene/scene.json",
                ArtifactRole::SceneDocument,
                b"scene-doc",
            ),
            ArtifactEntry::durable(
                "scene/asset-lock.json",
                ArtifactRole::AssetLock,
                b"asset-lock",
            ),
            ArtifactEntry::generated(
                "voxel/chunks/0_0_0.snap",
                ArtifactRole::VoxelChunkSnapshot,
                b"chunk-snapshot",
            ),
            ArtifactEntry::cache("cache/mesh/0_0_0.bin", ArtifactRole::Cache),
        ],
    }
}

// ── scene fixtures ─────────────────────────────────────────────────────────────

#[test]
fn duplicate_scene_id() {
    let d = doc(vec![
        node(1, None, SceneNodeKind::EmptyGroup),
        node(1, None, SceneNodeKind::EmptyGroup),
    ]);
    check_golden(
        "duplicate-scene-id.txt",
        &report_set_to_text(&scene_diagnostics(&d, None)),
    );
}

#[test]
fn missing_static_mesh() {
    let d = doc(vec![node(
        7,
        None,
        SceneNodeKind::StaticMesh(aref("mesh/belt-straight")),
    )]);
    let empty = Catalog::new();
    check_golden(
        "missing-static-mesh.txt",
        &report_set_to_text(&scene_diagnostics(&d, Some(&empty))),
    );
}

// ── catalog fixtures ───────────────────────────────────────────────────────────

#[test]
fn missing_sprite_texture() {
    // A sprite that draws a material whose texture dependency is absent.
    let sprite_mat = CatalogEntry::new(id("material/hard-hat"), 1)
        .with_material(material_with_texture(None))
        .with_dependencies(vec![aref("texture/hard-hat-atlas")]);
    let catalog = Catalog::from_entries(vec![sprite_mat]);
    check_golden(
        "missing-sprite-texture.txt",
        &report_set_to_text(&catalog_diagnostics(&catalog)),
    );
}

#[test]
fn wrong_kind_asset_ref() {
    // A material whose texture slot points at a non-texture asset.
    let mesh = CatalogEntry::new(id("mesh/surface-a"), 1);
    let mat = CatalogEntry::new(id("material/surface-a"), 1)
        .with_material(material_with_texture(Some("mesh/surface-a")))
        .with_dependencies(vec![aref("mesh/surface-a")]);
    let catalog = Catalog::from_entries(vec![mesh, mat]);
    check_golden(
        "wrong-kind-asset-ref.txt",
        &report_set_to_text(&catalog_diagnostics(&catalog)),
    );
}

#[test]
fn asset_dependency_cycle() {
    // mesh/a -> material/b -> texture/c -> mesh/a
    let a = CatalogEntry::new(id("mesh/a"), 1).with_dependencies(vec![aref("material/b")]);
    let b = CatalogEntry::new(id("material/b"), 1)
        .with_material(material_with_texture(None))
        .with_dependencies(vec![aref("texture/c")]);
    let c = CatalogEntry::new(id("texture/c"), 1).with_dependencies(vec![aref("mesh/a")]);
    let catalog = Catalog::from_entries(vec![a, b, c]);
    check_golden(
        "asset-dependency-cycle.txt",
        &report_set_to_text(&catalog_diagnostics(&catalog)),
    );
}

// ── world-bundle fixtures ──────────────────────────────────────────────────────

#[test]
fn corrupt_bundle_artifact() {
    let m = manifest();
    let mut actual: BTreeMap<String, BundleHash> = BTreeMap::new();
    actual.insert("scene/scene.json".to_string(), BundleHash::of(b"scene-doc"));
    actual.insert(
        "scene/asset-lock.json".to_string(),
        BundleHash::of(b"asset-lock"),
    );
    // The generated chunk snapshot was tampered: its bytes no longer hash to the
    // value the manifest recorded.
    actual.insert(
        "voxel/chunks/0_0_0.snap".to_string(),
        BundleHash::of(b"TAMPERED"),
    );
    check_golden(
        "corrupt-bundle-artifact.txt",
        &report_set_to_text(&artifact_integrity_diagnostics(&m, &actual)),
    );
}

#[test]
fn stale_cache_warning() {
    let m = manifest();
    // The durable/generated files are present, but the optional cache is gone.
    let present: BTreeSet<String> = [
        "scene/scene.json",
        "scene/asset-lock.json",
        "voxel/chunks/0_0_0.snap",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    check_golden(
        "stale-cache-warning.txt",
        &report_set_to_text(&missing_cache_diagnostics(&m, &present)),
    );
}

#[test]
fn unsupported_manifest_version() {
    let mut m = manifest();
    m.bundle_schema_version = 99;
    check_golden(
        "unsupported-manifest-version.txt",
        &report_set_to_text(&manifest_diagnostics(&m)),
    );
}

// ── render projection fixtures ─────────────────────────────────────────────────

fn projection_batch() -> Vec<ProjectionRecord> {
    vec![
        // Healthy: full chain, asset resolved.
        ProjectionRecord::complete(42, 7, 123, "mesh/belt-straight"),
        // Broken: a handle with no scene node at all.
        ProjectionRecord {
            render_handle: 43,
            scene_node_id: None,
            runtime_entity_id: None,
            asset_id: None,
            asset_resolved: false,
            fallback_used: false,
        },
        // Degraded: traced, but the asset did not resolve and a fallback was drawn.
        ProjectionRecord {
            render_handle: 44,
            scene_node_id: Some(8),
            runtime_entity_id: Some(456),
            asset_id: Some("sprite/hard-hat".to_string()),
            asset_resolved: false,
            fallback_used: true,
        },
    ]
}

#[test]
fn missing_render_source_trace() {
    let batch = projection_batch();
    check_golden(
        "missing-render-source-trace.txt",
        &report_set_to_text(&source_trace_diagnostics(&batch)),
    );
}

#[test]
fn source_trace_snapshot() {
    let batch = projection_batch();
    check_golden(
        "source-trace.txt",
        &traces_to_text(&build_source_traces(&batch)),
    );
}

#[test]
fn renderer_resource_snapshot() {
    let report = RendererResourceReport {
        live_handles: 3,
        geometries: 2,
        materials: 2,
        sprite_instances: 1,
        sprites_updated_last_tick: 1,
        resources_created: 12,
        resources_disposed: 9,
        fallback_materials: 1,
    };
    let mut combined = resource_report_to_text(&report);
    combined.push_str(&report_set_to_text(&resource_diagnostics(&report)));
    check_golden("renderer-resources.txt", &combined);
}

// ── full bundle load→edit→save→reload equivalence (subtask #2362) ──────────────

#[test]
fn bundle_round_trip_equivalence_golden() {
    use core_scene::{SceneMetadata as TreeMeta, SceneNode, SceneTree};
    use scene_diagnostics::world_bundle_round_trip;

    // An abstract fixture bundle: a two-node scene + a voxel section.
    let tree = SceneTree {
        id: SceneId::new(100),
        schema_version: 1,
        metadata: TreeMeta {
            name: Some("equiv-fixture".into()),
            authoring_format_version: 1,
        },
        dependencies: vec![],
        roots: vec![
            SceneNode::leaf(SceneNodeId::new(1), core_scene::SceneNodeKind::EmptyGroup)
                .with_children(vec![SceneNode::leaf(
                    SceneNodeId::new(2),
                    core_scene::SceneNodeKind::EmptyGroup,
                )]),
        ],
    };
    let scene_json = core_scene::encode(&tree.to_flat());

    let spec = VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap();
    let g = GridId::new(0);
    let chunk = ChunkCoord::new(0, 0, 0);
    let gen = generate_chunk(&spec, chunk, 7, 1);
    let initial = vec![VoxelEditEvent::ChunkGenerated {
        grid: g,
        chunk,
        seed: 7,
        generator_version: 1,
        hash: gen.content_hash().0,
    }];
    // The "tick/edit": two deterministic authored voxel edits applied after load.
    let ticks = vec![
        VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(0, 3, 0),
            value: VoxelValue::solid_raw(2),
        },
        VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(1, 3, 0),
            value: VoxelValue::solid_raw(3),
        },
    ];

    let report = world_bundle_round_trip(
        &scene_json,
        SceneId::new(100),
        WorldId::new(7),
        spec,
        &initial,
        &ticks,
        1,
    )
    .expect("round trip executes");
    assert!(report.is_equivalent(), "{}", report.to_report_text());
    check_golden("bundle-equivalence.txt", &report.to_report_text());
}

// ── world load/save composition failures (subtask #2364) ───────────────────────

#[test]
fn composition_failures_golden() {
    use rule_world_bundle::LoadExecutionError;
    use scene_diagnostics::composition_failure_diagnostic;
    use scene_diagnostics::DiagnosticReportSet;
    use svc_serialization::LoadStage;

    // A representative spread across the composition failure categories: a missing
    // durable artifact, a too-new version, a voxel replay conflict, and a final
    // consistency mismatch. Each renders a stage + source ref + severity + remedy.
    let errors = [
        LoadExecutionError::MissingArtifact {
            stage: LoadStage::SceneDocument,
            path: "scene/scene.json".into(),
        },
        LoadExecutionError::VersionUnsupported {
            bundle_schema: 99,
            protocol: 1,
        },
        LoadExecutionError::VoxelReplay {
            detail: "generated context changed under the new generator".into(),
        },
        LoadExecutionError::FinalConsistency {
            detail: "source trace count 1 != entity count 2".into(),
        },
    ];
    let mut set = DiagnosticReportSet::new();
    for e in &errors {
        set.push(composition_failure_diagnostic(e));
    }
    check_golden("composition-failures.txt", &report_set_to_text(&set));
}

// ── save/load round-trip (subtask #2333) ───────────────────────────────────────

#[test]
fn save_load_round_trip_equivalence() {
    let spec = VoxelGridSpec::new(GridId::new(0), 1.0, ChunkDims::cubic(4).unwrap()).unwrap();
    let g = GridId::new(0);
    let chunk = ChunkCoord::new(0, 0, 0);
    let gen = generate_chunk(&spec, chunk, 7, 1);
    // State A: a generated chunk plus one authored edit.
    let initial = vec![
        VoxelEditEvent::ChunkGenerated {
            grid: g,
            chunk,
            seed: 7,
            generator_version: 1,
            hash: gen.content_hash().0,
        },
        VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(0, 3, 0),
            value: VoxelValue::solid_raw(2),
        },
    ];
    // N deterministic operations → state B.
    let ticks = vec![
        VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(1, 3, 0),
            value: VoxelValue::solid_raw(2),
        },
        VoxelEditEvent::VoxelSet {
            grid: g,
            coord: VoxelCoord::new(2, 3, 0),
            value: VoxelValue::solid_raw(3),
        },
    ];
    let report = voxel_round_trip(spec, &initial, &ticks, 1).unwrap();
    assert!(report.is_equivalent());
    check_golden("round-trip-equivalence.txt", &report.to_report_text());
}
