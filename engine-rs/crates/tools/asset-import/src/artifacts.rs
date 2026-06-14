//! Deterministic JSON emission of imported ASHA-native artifacts (#2384).
//!
//! Each import produces a stable, diffable set of `(relative path, contents)`
//! artifacts: the static-mesh descriptor and the catalog fragment. The bytes depend
//! only on the imported assets, so re-importing unchanged source yields identical
//! output. The core never writes files — the CLI (#2386) owns the filesystem.

use core_catalog::{Catalog, CatalogEntry, MaterialDef};
use protocol_render::{MeshCollisionPolicy, MeshPayloadSource, StaticMeshAsset};

use crate::import::ImportedAssets;
use crate::json::JsonWriter;

/// One generated artifact: a repo/output-relative path and its full contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedArtifact {
    pub rel_path: String,
    pub contents: String,
}

/// Render the static-mesh descriptor (`<name>.staticmesh.json`).
pub fn render_static_mesh(asset: &StaticMeshAsset) -> String {
    let mut w = JsonWriter::new();
    w.begin_object();
    w.field_str("asset", &asset.asset, false);
    w.field_str("provenance", asset.payload.provenance.label(), false);

    // layout
    w.indent_field_object("layout");
    w.field_num(
        "vertexCount",
        asset.payload.layout.vertex_count as f64,
        false,
    );
    w.field_num("indexCount", asset.payload.layout.index_count as f64, false);
    // Derived from the protocol enum (not a hardcoded literal) so the artifact
    // cannot silently drift from the contract's index-width vocabulary (#2429).
    w.field_str("indexWidth", asset.payload.layout.index_width.label(), true);
    w.end_object(true);

    // bounds
    let b = &asset.payload.bounds;
    w.indent_field_object("bounds");
    w.field_f32_array("min", &b.min, false);
    w.field_f32_array("max", &b.max, true);
    w.end_object(true);

    // groups
    w.begin_array_field("groups");
    for (i, g) in asset.payload.groups.iter().enumerate() {
        let last = i + 1 == asset.payload.groups.len();
        w.array_element_indent();
        w.begin_object();
        w.field_num("materialSlot", g.material_slot as f64, false);
        w.field_num("start", g.start as f64, false);
        w.field_num("count", g.count as f64, true);
        w.end_object(!last);
    }
    w.end_array(false);

    // material slots
    w.begin_array_field("materialSlots");
    for (i, s) in asset.material_slots.iter().enumerate() {
        let last = i + 1 == asset.material_slots.len();
        w.array_element_indent();
        w.begin_object();
        w.field_num("slot", s.slot as f64, false);
        w.field_str("material", &s.material, true);
        w.end_object(!last);
    }
    w.end_array(false);

    // collision
    let (kind, proxy) = match &asset.collision {
        MeshCollisionPolicy::VisualOnly => ("visualOnly", None),
        MeshCollisionPolicy::AabbFallback => ("aabbFallback", None),
        MeshCollisionPolicy::Proxy { proxy_asset } => ("proxy", Some(proxy_asset.as_str())),
    };
    w.indent_field_object("collision");
    w.field_str("kind", kind, proxy.is_none());
    if let Some(p) = proxy {
        w.field_str("proxyAsset", p, true);
    }
    w.end_object(true);

    // inline geometry streams (offline artifact carries the bytes inline)
    if let MeshPayloadSource::Inline {
        positions,
        normals,
        indices,
    } = &asset.payload.source
    {
        w.indent_field_object("geometry");
        w.field_f32_array("positions", positions, false);
        w.field_f32_array("normals", normals, false);
        w.field_num_array(
            "indices",
            &indices.iter().map(|&i| i as f64).collect::<Vec<_>>(),
            true,
        );
        w.end_object(false);
    } else {
        w.field_opt_str("geometry", None, true);
    }

    w.end_object(false);
    w.finish()
}

/// Render the catalog fragment (`<name>.catalog.json`): all imported entries.
pub fn render_catalog(catalog: &Catalog) -> String {
    let mut w = JsonWriter::new();
    w.begin_object();
    w.begin_array_field("entries");
    for (i, entry) in catalog.entries.iter().enumerate() {
        let last = i + 1 == catalog.entries.len();
        w.array_element_indent();
        render_entry(&mut w, entry, last);
    }
    w.end_array(true);
    w.end_object(false);
    w.finish()
}

fn render_entry(w: &mut JsonWriter, entry: &CatalogEntry, last: bool) {
    w.begin_object();
    w.field_str("id", entry.id.as_str(), false);
    w.field_str("kind", entry.kind().prefix(), false);
    w.field_num("version", entry.version as f64, false);
    w.field_opt_str("hash", entry.hash.as_ref().map(|h| h.as_str()), false);
    w.field_opt_str("label", entry.label.as_deref(), false);
    let deps: Vec<String> = entry
        .dependencies
        .iter()
        .map(|d| d.id().as_str().to_string())
        .collect();
    let has_material = entry.material.is_some();
    w.field_str_array("dependencies", &deps, !has_material);
    if let Some(material) = &entry.material {
        render_material(w, material);
    }
    w.end_object(!last);
}

fn render_material(w: &mut JsonWriter, material: &MaterialDef) {
    w.indent_field_object("material");
    // authority projection
    w.indent_field_object("authority");
    w.field_bool("solid", material.authority.solid, false);
    w.field_bool("collidable", material.authority.collidable, false);
    w.field_bool("occludes", material.authority.occludes, false);
    w.field_str(
        "structuralClass",
        structural_label(material.authority.structural_class),
        true,
    );
    w.end_object(true);
    // style projection
    let c = material.style.color;
    w.indent_field_object("style");
    w.field_f32_array("color", &[c.r, c.g, c.b, c.a], false);
    w.field_opt_str(
        "texture",
        material.style.texture.as_ref().map(|t| t.id().as_str()),
        false,
    );
    w.field_f32("roughness", material.style.roughness, false);
    w.field_f32("emissive", material.style.emissive, false);
    w.field_str("uvStrategy", uv_label(material.style.uv_strategy), true);
    w.end_object(false);
    w.end_object(false);
}

fn structural_label(c: core_catalog::StructuralClass) -> &'static str {
    match c {
        core_catalog::StructuralClass::Decorative => "decorative",
        core_catalog::StructuralClass::Solid => "solid",
        core_catalog::StructuralClass::Structural => "structural",
    }
}

fn uv_label(s: core_catalog::UvStrategy) -> &'static str {
    match s {
        core_catalog::UvStrategy::Flat => "flat",
        core_catalog::UvStrategy::Planar => "planar",
        core_catalog::UvStrategy::Atlas => "atlas",
    }
}

/// Render the full artifact set for an import, in deterministic path order.
pub fn render_artifacts(name: &str, assets: &ImportedAssets) -> Vec<GeneratedArtifact> {
    vec![
        GeneratedArtifact {
            rel_path: format!("{name}.catalog.json"),
            contents: render_catalog(&assets.catalog),
        },
        GeneratedArtifact {
            rel_path: format!("{name}.staticmesh.json"),
            contents: render_static_mesh(&assets.static_mesh),
        },
    ]
}
