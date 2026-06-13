//! Voxel material → catalog material render projection golden (material-wiring
//! super, epic #2353; subtask #2375).
//!
//! Projects the render material descriptors for a chunk that uses two voxel
//! materials (stone, dirt) through the `VoxelMaterialTable` + catalog and asserts
//! the encoded `DefineMaterial` diffs match the committed fixture the renderer
//! golden applies (two distinct catalog styles). Compact voxel storage is
//! untouched — the table maps the `u16` ids to catalog material assets.
//!
//! Regenerate: `BLESS=1 cargo test -p render-bridge --test voxel_materials_golden`.

use std::path::PathBuf;

use core_assets::AssetId;
use core_catalog::entry::CatalogEntry;
use core_catalog::material::{
    MaterialAuthority, MaterialDef, MaterialStyle, Rgba, StructuralClass,
};
use core_catalog::{Catalog, VoxelMaterialTable};
use core_voxel::VoxelMaterialId;
use protocol_render::RenderFrameDiff;
use render_bridge::json;
use render_bridge::presentation::project_voxel_materials;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../harness/fixtures/render-diffs")
        .join(name)
}

fn check_fixture(name: &str, actual: &str) {
    let path = fixture_path(name);
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e} (BLESS=1 to create)", path.display()));
    assert_eq!(actual, expected, "voxel material projection drifted");
}

fn material_entry(id: &str, color: Rgba, structural: StructuralClass) -> CatalogEntry {
    CatalogEntry::new(AssetId::parse(id).unwrap(), 1).with_material(MaterialDef {
        authority: MaterialAuthority {
            solid: true,
            collidable: true,
            occludes: true,
            structural_class: structural,
        },
        style: MaterialStyle::flat(color),
    })
}

fn stone_dirt_catalog() -> Catalog {
    Catalog {
        entries: vec![
            material_entry(
                "material/stone",
                Rgba {
                    r: 0.5,
                    g: 0.5,
                    b: 0.55,
                    a: 1.0,
                },
                StructuralClass::Structural,
            ),
            material_entry(
                "material/dirt",
                Rgba {
                    r: 0.4,
                    g: 0.25,
                    b: 0.1,
                    a: 1.0,
                },
                StructuralClass::Solid,
            ),
        ],
    }
}

fn table() -> VoxelMaterialTable {
    VoxelMaterialTable::from_pairs([
        (
            VoxelMaterialId::new(1),
            AssetId::parse("material/stone").unwrap(),
        ),
        (
            VoxelMaterialId::new(2),
            AssetId::parse("material/dirt").unwrap(),
        ),
    ])
}

#[test]
fn projects_two_voxel_materials_to_distinct_catalog_styles() {
    let catalog = stone_dirt_catalog();
    let table = table();
    let used = [VoxelMaterialId::new(1), VoxelMaterialId::new(2)];

    let (diffs, fallbacks) = project_voxel_materials(&table, &catalog, &used);
    assert!(
        fallbacks.is_empty(),
        "both materials resolve from the catalog"
    );

    let mut frame = RenderFrameDiff::new();
    for d in diffs {
        frame.push(d);
    }
    check_fixture("voxel-materials.json", &json::encode_frame(&frame));
}
