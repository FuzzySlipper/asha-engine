//! Sprite atlas / texture projection golden (material-wiring super, epic #2353;
//! subtask #2374).
//!
//! Projects a sprite scene with a registered atlas source and asserts the encoded
//! `DefineTexture` + `DefineSpriteAtlas` + `CreateSprite` diffs match the committed
//! fixture the `renderer-three` golden applies (where the sprite frame resolves to
//! its atlas UV sub-rectangle). Generated from the projector, not hand-authored.
//!
//! Regenerate: `BLESS=1 cargo test -p render-bridge --test sprite_atlas_golden`,
//! then re-bless the renderer snapshot via `harness/ci/check-render-goldens.sh`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use core_assets::{AssetId, AssetReference, AssetVersionReq};
use core_catalog::Catalog;
use core_ids::{RuntimeSessionId, SceneId, SceneNodeId};
use core_scene::bootstrap::BootstrapPlan;
use core_scene::document::{NodeMetadata, SceneMetadata};
use core_scene::{FlatSceneDocument, SceneNode, SceneNodeKind, SceneTree, SpatialSessionState};
use protocol_render::{
    SpriteAtlasDescriptor, SpriteFrameRect, TextureDescriptor, TextureFilter, TextureWrap,
};
use render_bridge::json;
use render_bridge::presentation::{
    ScenePresentation, ScenePresentationProjector, SpriteAtlasSource,
};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../harness/fixtures/render-diffs")
        .join(name)
}

fn check_fixture(name: &str, actual: &str) {
    let path = fixture_path(name);
    if std::env::var_os("BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e} (BLESS=1 to create)", path.display()));
    assert_eq!(
        actual,
        expected,
        "atlas projection drifted from {}",
        path.display()
    );
}

fn sprite_scene() -> FlatSceneDocument {
    let mut node = SceneNode::leaf(
        SceneNodeId::new(10),
        SceneNodeKind::Sprite(AssetReference::new(
            AssetId::parse("sprite/spark-sheet").unwrap(),
            AssetVersionReq::Any,
            None,
        )),
    );
    node.metadata = NodeMetadata {
        label: Some("spark".into()),
        tags: Vec::new(),
    };
    SceneTree {
        id: SceneId::new(1),
        schema_version: 1,
        metadata: SceneMetadata {
            name: Some("atlas-showcase".into()),
            authoring_format_version: 0,
        },
        dependencies: Vec::new(),
        roots: vec![node],
    }
    .to_flat()
}

fn spark_atlas_source() -> SpriteAtlasSource {
    SpriteAtlasSource {
        texture: TextureDescriptor {
            id: "texture/spark".into(),
            width: 64,
            height: 32,
            filter: TextureFilter::Nearest,
            wrap: TextureWrap::Clamp,
            content_hash: Some("blake3:abc".into()),
            version: 1,
        },
        atlas: SpriteAtlasDescriptor {
            id: "sprite/spark-sheet".into(),
            texture: "texture/spark".into(),
            frames: vec![
                SpriteFrameRect {
                    frame: 0,
                    uv_min: [0.0, 0.0],
                    uv_max: [0.5, 1.0],
                },
                SpriteFrameRect {
                    frame: 3,
                    uv_min: [0.5, 0.0],
                    uv_max: [1.0, 1.0],
                },
            ],
        },
    }
}

fn bootstrap(doc: &FlatSceneDocument) -> SpatialSessionState {
    BootstrapPlan::prepare(doc, RuntimeSessionId::new(1))
        .expect("valid scene")
        .apply()
        .0
}

#[test]
fn projects_sprite_atlas_setup_frame_to_committed_fixture() {
    let doc = sprite_scene();
    let world = bootstrap(&doc);
    let catalog = Catalog::default();
    let overrides = BTreeMap::new();

    let mut proj = ScenePresentationProjector::new();
    proj.register_atlas_source(spark_atlas_source());
    let frame = proj.project(&ScenePresentation {
        scene: &doc,
        world: &world,
        catalog: &catalog,
        overrides: &overrides,
    });

    // Texture + atlas define before the sprite that needs them; frame 0 is valid.
    assert!(
        proj.diagnostics().is_empty(),
        "frame 0 must resolve cleanly"
    );
    check_fixture("sprite-atlas.json", &json::encode_frame(&frame));
}

#[test]
fn an_unknown_sprite_frame_is_classified() {
    use render_bridge::presentation::{
        NodePresentation, RenderProjectionDiagnostic, SpriteRuntime,
    };

    let doc = sprite_scene();
    let world = bootstrap(&doc);
    let catalog = Catalog::default();

    let mut proj = ScenePresentationProjector::new();
    proj.register_atlas_source(spark_atlas_source());

    // Project the sprite at frame 9, which the atlas does not define.
    let mut overrides = BTreeMap::new();
    overrides.insert(
        SceneNodeId::new(10),
        NodePresentation {
            material_overrides: Vec::new(),
            sprite: Some(SpriteRuntime {
                frame: 9,
                ..SpriteRuntime::default()
            }),
        },
    );
    let _ = proj.project(&ScenePresentation {
        scene: &doc,
        world: &world,
        catalog: &catalog,
        overrides: &overrides,
    });
    assert!(proj
        .diagnostics()
        .contains(&RenderProjectionDiagnostic::InvalidSpriteFrame {
            node: SceneNodeId::new(10),
            atlas: "sprite/spark-sheet".into(),
            frame: 9,
        }));
}
