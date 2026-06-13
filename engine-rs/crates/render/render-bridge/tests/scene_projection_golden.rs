//! Authority → render-bridge projection golden harness (render-projection super,
//! epic #2352; subtask #2372).
//!
//! Builds a representative scene (two static-mesh instances sharing one catalog
//! asset, plus a sprite) from **authority** state — a flat scene document, a
//! bootstrapped world, and a catalog — projects it through
//! [`ScenePresentationProjector`], and asserts the encoded render diffs match the
//! committed fixtures the `renderer-three` golden test consumes. The fixtures are
//! generated *from the projector*, not hand-authored, so a projection drift fails
//! here (Rust) and the renderer golden fails downstream (TS).
//!
//! Regenerate the fixtures after an intended projection change:
//!
//! ```text
//! BLESS=1 cargo test -p render-bridge --test scene_projection_golden
//! ```
//! then regenerate the renderer snapshot golden via `harness/ci/check-render-goldens.sh`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use core_assets::{AssetId, AssetReference, AssetVersionReq};
use core_catalog::entry::CatalogEntry;
use core_catalog::material::{
    MaterialAuthority, MaterialDef, MaterialStyle, Rgba, StructuralClass,
};
use core_catalog::Catalog;
use core_ids::{SceneId, SceneNodeId, WorldId};
use core_math::Vec3;
use core_scene::bootstrap::BootstrapPlan;
use core_scene::document::{NodeMetadata, SceneMetadata};
use core_scene::transform::{Quat, SceneTransform};
use core_scene::{FlatSceneDocument, SceneNode, SceneNodeKind, SceneTree, WorldState};
use render_bridge::json;
use render_bridge::presentation::{
    NodePresentation, RenderProjectionDiagnostic, ScenePresentation, ScenePresentationProjector,
    SpriteRuntime,
};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../harness/fixtures/render-diffs")
        .join(name)
}

/// Compare `actual` to the committed fixture `name`; with `BLESS=1`, (re)write it.
fn check_fixture(name: &str, actual: &str) {
    let path = fixture_path(name);
    if std::env::var_os("BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e} (run with BLESS=1 to create)", path.display()));
    assert_eq!(
        actual,
        expected,
        "projected render diffs drifted from {} — regenerate with BLESS=1 and \
         re-bless the renderer snapshot golden",
        path.display()
    );
}

fn asset_ref(id: &str) -> AssetReference {
    AssetReference::new(AssetId::parse(id).unwrap(), AssetVersionReq::Any, None)
}

fn mesh_node(id: u64, asset: &str, label: &str, x: f32) -> SceneNode {
    let mut node = SceneNode::leaf(
        SceneNodeId::new(id),
        SceneNodeKind::StaticMesh(asset_ref(asset)),
    );
    node.transform = SceneTransform::new(Vec3::new(x, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
    node.metadata = NodeMetadata {
        label: Some(label.to_string()),
        tags: Vec::new(),
    };
    node
}

fn sprite_node(id: u64, asset: &str, label: &str, x: f32) -> SceneNode {
    let mut node = SceneNode::leaf(
        SceneNodeId::new(id),
        SceneNodeKind::Sprite(asset_ref(asset)),
    );
    node.transform = SceneTransform::new(Vec3::new(x, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
    node.metadata = NodeMetadata {
        label: Some(label.to_string()),
        tags: Vec::new(),
    };
    node
}

fn showcase_scene() -> FlatSceneDocument {
    SceneTree {
        id: SceneId::new(1),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("projection-showcase".into()),
            authoring_format_version: 0,
        },
        dependencies: Vec::new(),
        roots: vec![
            mesh_node(10, "mesh/crate", "crate-a", 0.0),
            mesh_node(20, "mesh/crate", "crate-b", 3.0),
            sprite_node(30, "sprite/spark-sheet", "spark", 6.0),
        ],
    }
    .to_flat()
}

fn material_entry(id: &str, color: Rgba) -> CatalogEntry {
    CatalogEntry::new(AssetId::parse(id).unwrap(), 1).with_material(MaterialDef {
        authority: MaterialAuthority {
            solid: true,
            collidable: true,
            occludes: true,
            structural_class: StructuralClass::Solid,
        },
        style: MaterialStyle::flat(color),
    })
}

/// `mesh/crate` depends on `material/wood`; both materials carry a visual
/// definition so the projector resolves real `RenderMaterialDescriptor`s. The
/// sprite asset needs no catalog material (atlas wiring lands in #2374).
fn showcase_catalog() -> Catalog {
    Catalog {
        entries: vec![
            material_entry(
                "material/wood",
                Rgba {
                    r: 0.6,
                    g: 0.4,
                    b: 0.2,
                    a: 1.0,
                },
            ),
            material_entry(
                "material/wood-painted",
                Rgba {
                    r: 0.2,
                    g: 0.5,
                    b: 0.7,
                    a: 1.0,
                },
            ),
            CatalogEntry::new(AssetId::parse("mesh/crate").unwrap(), 1)
                .with_dependencies(vec![asset_ref("material/wood")]),
        ],
    }
}

fn bootstrap(doc: &FlatSceneDocument) -> WorldState {
    BootstrapPlan::prepare(doc, WorldId::new(1))
        .expect("valid scene")
        .apply()
        .0
}

#[test]
fn projects_scene_showcase_setup_frame_to_committed_fixture() {
    let doc = showcase_scene();
    let world = bootstrap(&doc);
    let catalog = showcase_catalog();
    let overrides = BTreeMap::new();

    let mut proj = ScenePresentationProjector::new();
    let frame = proj.project(&ScenePresentation {
        scene: &doc,
        world: &world,
        catalog: &catalog,
        overrides: &overrides,
    });
    assert!(
        proj.diagnostics().is_empty(),
        "showcase must project cleanly"
    );

    check_fixture("scene-projection.json", &json::encode_frame(&frame));
}

#[test]
fn projects_setup_then_authority_change_sequence_to_committed_fixture() {
    let doc = showcase_scene();
    let mut world = bootstrap(&doc);
    let catalog = showcase_catalog();

    let mut proj = ScenePresentationProjector::new();
    let setup = proj.project(&ScenePresentation {
        scene: &doc,
        world: &world,
        catalog: &catalog,
        overrides: &BTreeMap::new(),
    });

    // Frame 2: authority moves crate-a, rebinds crate-b's material, and advances
    // the sprite frame — exercising transform-update, instance-recreate, and the
    // deterministic sprite-frame update in one projection.
    let crate_a = world.entity_for_node(SceneNodeId::new(10)).unwrap();
    world.set_transform(
        crate_a,
        SceneTransform::new(Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY, Vec3::ONE),
    );
    let mut overrides = BTreeMap::new();
    overrides.insert(
        SceneNodeId::new(20),
        NodePresentation {
            material_overrides: vec![(0, "material/wood-painted".into())],
            sprite: None,
        },
    );
    overrides.insert(
        SceneNodeId::new(30),
        NodePresentation {
            material_overrides: Vec::new(),
            sprite: Some(SpriteRuntime {
                frame: 3,
                ..SpriteRuntime::default()
            }),
        },
    );
    let changed = proj.project(&ScenePresentation {
        scene: &doc,
        world: &world,
        catalog: &catalog,
        overrides: &overrides,
    });

    check_fixture(
        "scene-projection-sequence.json",
        &json::encode_sequence(&[setup, changed]),
    );
}

#[test]
fn invalid_projection_data_fails_with_a_classified_diagnostic() {
    // A scene referencing a mesh absent from the catalog still projects a visible
    // fallback, but flags the gap — invalid input is classified, not swallowed.
    let doc = SceneTree {
        id: SceneId::new(2),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("broken".into()),
            authoring_format_version: 0,
        },
        dependencies: Vec::new(),
        roots: vec![mesh_node(10, "mesh/ghost", "ghost", 0.0)],
    }
    .to_flat();
    let world = bootstrap(&doc);
    let catalog = Catalog::default();

    let mut proj = ScenePresentationProjector::new();
    let frame = proj.project(&ScenePresentation {
        scene: &doc,
        world: &world,
        catalog: &catalog,
        overrides: &BTreeMap::new(),
    });
    assert!(!frame.is_empty(), "fallback geometry still renders");
    assert!(proj
        .diagnostics()
        .contains(&RenderProjectionDiagnostic::MissingMeshAsset {
            node: SceneNodeId::new(10),
            asset: "mesh/ghost".into()
        }));
}
