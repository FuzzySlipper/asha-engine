//! Convert a parsed [`SourceMesh`] into ASHA-native descriptors (#2384).
//!
//! The importer produces a [`StaticMeshAsset`] (provenance `StaticAsset`) plus the
//! catalog entries it implies — the mesh, its materials, and any referenced
//! textures — each carrying a deterministic content fingerprint for asset locks.
//! Unsupported topology, mismatched streams, unbound groups, and malformed
//! descriptors are reported as classified diagnostics; on any error the import is
//! refused (no partial asset is returned).

use std::collections::BTreeSet;

use core_assets::{AssetHash, AssetId, AssetReference, AssetVersionReq};
use core_catalog::{
    Catalog, CatalogEntry, MaterialAuthority, MaterialDef, MaterialStyle, Rgba, UvStrategy,
};
use protocol_render::{
    MeshAttribute, MeshAttributeKind, MeshAttributeName, MeshBoundsDescriptor, MeshBufferLayout,
    MeshCollisionPolicy, MeshGroupDescriptor, MeshIndexWidth, MeshMaterialSlot,
    MeshPayloadDescriptor, MeshPayloadSource, MeshProvenance, StaticMeshAsset,
};

use crate::diagnostic::{ImportCode, ImportDiagnostic};
use crate::fingerprint::fingerprint_hash;
use crate::source::{SourceCollision, SourceMaterial, SourceMesh};

/// Context for resolving a source's external resources during import (#2385).
#[derive(Debug, Clone, Default)]
pub struct ImportContext {
    /// The set of external texture resource names the importer can resolve. When
    /// `None`, texture references are assumed present (no external-resource check);
    /// when `Some`, a referenced texture not in the set is reported as missing.
    pub available_textures: Option<BTreeSet<String>>,
}

impl ImportContext {
    /// A context that resolves the given texture names (and reports others missing).
    pub fn with_textures<I: IntoIterator<Item = String>>(textures: I) -> Self {
        ImportContext {
            available_textures: Some(textures.into_iter().collect()),
        }
    }

    fn texture_is_missing(&self, name: &str) -> bool {
        match &self.available_textures {
            Some(set) => !set.contains(name),
            None => false,
        }
    }
}

/// The ASHA-native output of importing one source mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedAssets {
    pub static_mesh: StaticMeshAsset,
    pub catalog: Catalog,
}

/// The full outcome of an import attempt: the assets (when error-free) and every
/// classified diagnostic encountered.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportOutcome {
    pub assets: Option<ImportedAssets>,
    pub diagnostics: Vec<ImportDiagnostic>,
}

impl ImportOutcome {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(ImportDiagnostic::is_error)
    }
}

fn mesh_asset_id(name: &str) -> String {
    format!("mesh/{name}")
}

fn material_asset_id(name: &str) -> String {
    format!("material/{name}")
}

fn texture_asset_id(name: &str) -> String {
    format!("texture/{name}")
}

fn bounds_of(positions: &[f32]) -> MeshBoundsDescriptor {
    if positions.len() < 3 {
        return MeshBoundsDescriptor {
            min: [0.0; 3],
            max: [0.0; 3],
        };
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in positions.chunks_exact(3) {
        for axis in 0..3 {
            min[axis] = min[axis].min(v[axis]);
            max[axis] = max[axis].max(v[axis]);
        }
    }
    MeshBoundsDescriptor { min, max }
}

/// A canonical, deterministic content string for a material's fingerprint.
fn material_content(material: &SourceMaterial) -> String {
    format!(
        "material:{}:color={:?}:texture={:?}",
        material.name, material.color, material.texture
    )
}

/// A canonical, deterministic content string for the mesh's geometry fingerprint.
fn mesh_content(source: &SourceMesh) -> String {
    format!(
        "mesh:{}:pos={:?}:nrm={:?}:idx={:?}:groups={:?}:collision={:?}",
        source.name,
        source.positions,
        source.normals,
        source.indices,
        source
            .groups
            .iter()
            .map(|g| (g.material_slot, g.start, g.count))
            .collect::<Vec<_>>(),
        source.collision,
    )
}

fn dependency(id: &str, hash: &AssetHash) -> Option<AssetReference> {
    let asset_id = AssetId::parse(id).ok()?;
    Some(AssetReference::new(
        asset_id,
        AssetVersionReq::Exact(1),
        Some(hash.clone()),
    ))
}

fn collision_policy(collision: &SourceCollision) -> MeshCollisionPolicy {
    match collision {
        SourceCollision::VisualOnly => MeshCollisionPolicy::VisualOnly,
        SourceCollision::AabbFallback => MeshCollisionPolicy::AabbFallback,
        SourceCollision::Proxy(name) => MeshCollisionPolicy::Proxy {
            proxy_asset: mesh_asset_id(name),
        },
    }
}

/// Import a parsed source mesh with the default context (no external-resource
/// resolution — texture references are assumed present).
pub fn import(source: &SourceMesh) -> ImportOutcome {
    import_with_context(source, &ImportContext::default())
}

/// Import a parsed source mesh into ASHA-native descriptors + catalog entries,
/// resolving external resources against `context`. A referenced texture that the
/// context cannot resolve is reported as a classified missing-resource warning
/// (the import proceeds; the gap is surfaced, never silently dropped).
pub fn import_with_context(source: &SourceMesh, context: &ImportContext) -> ImportOutcome {
    let mut diagnostics = Vec::new();
    let locus = mesh_asset_id(&source.name);

    let vertex_count = source.positions.len() / 3;
    // Topology + stream checks (classified, not silent).
    if !source.positions.len().is_multiple_of(3) {
        diagnostics.push(ImportDiagnostic::error(
            ImportCode::AttributeLengthMismatch,
            &locus,
            format!(
                "position stream length {} is not a multiple of 3",
                source.positions.len()
            ),
            "ensure positions are 3 floats per vertex",
        ));
    }
    if source.normals.len() != source.positions.len() {
        diagnostics.push(ImportDiagnostic::error(
            ImportCode::AttributeLengthMismatch,
            &locus,
            format!(
                "normal stream length {} does not match position stream length {}",
                source.normals.len(),
                source.positions.len()
            ),
            "provide one normal per vertex (3 floats each)",
        ));
    }
    if !source.indices.len().is_multiple_of(3) {
        diagnostics.push(ImportDiagnostic::error(
            ImportCode::UnsupportedTopology,
            &locus,
            format!(
                "index count {} is not a triangle list (multiple of 3)",
                source.indices.len()
            ),
            "only triangle-list topology is supported",
        ));
    }

    // Material slot bindings: every group's slot must be declared.
    let declared_slots: Vec<u16> = source.materials.iter().map(|m| m.slot).collect();
    for group in &source.groups {
        if !declared_slots.contains(&group.material_slot) {
            diagnostics.push(ImportDiagnostic::error(
                ImportCode::GroupSlotUnbound,
                &locus,
                format!(
                    "group references material slot {} with no declared material",
                    group.material_slot
                ),
                "declare a material for every slot a group uses",
            ));
        }
    }

    if diagnostics.iter().any(ImportDiagnostic::is_error) {
        return ImportOutcome {
            assets: None,
            diagnostics,
        };
    }

    // Build the static-mesh payload.
    let layout = MeshBufferLayout {
        vertex_count: vertex_count as u32,
        index_count: source.indices.len() as u32,
        index_width: MeshIndexWidth::U32,
        attributes: vec![
            MeshAttribute {
                name: MeshAttributeName::Position,
                components: 3,
                kind: MeshAttributeKind::F32,
            },
            MeshAttribute {
                name: MeshAttributeName::Normal,
                components: 3,
                kind: MeshAttributeKind::F32,
            },
        ],
    };
    let groups = source
        .groups
        .iter()
        .map(|g| MeshGroupDescriptor {
            material_slot: g.material_slot,
            start: g.start,
            count: g.count,
        })
        .collect();
    let payload = MeshPayloadDescriptor {
        layout,
        groups,
        bounds: bounds_of(&source.positions),
        source: MeshPayloadSource::Inline {
            positions: source.positions.clone(),
            normals: source.normals.clone(),
            indices: source.indices.clone(),
        },
        provenance: MeshProvenance::StaticAsset,
    };
    let material_slots = source
        .materials
        .iter()
        .map(|m| MeshMaterialSlot {
            slot: m.slot,
            material: material_asset_id(&m.name),
        })
        .collect();
    let static_mesh = StaticMeshAsset {
        asset: mesh_asset_id(&source.name),
        payload,
        material_slots,
        collision: collision_policy(&source.collision),
    };

    // Border validation of the generated descriptor.
    if let Err(e) = static_mesh.validate() {
        diagnostics.push(ImportDiagnostic::error(
            ImportCode::InvalidDescriptor,
            &locus,
            format!("generated static-mesh descriptor is invalid: {e:?}"),
            "fix the source geometry/material slots",
        ));
        return ImportOutcome {
            assets: None,
            diagnostics,
        };
    }

    // Catalog entries: textures, materials, then the mesh (deterministic id order).
    let mut entries: Vec<CatalogEntry> = Vec::new();
    let mut seen_ids: Vec<String> = Vec::new();

    let mut mesh_deps: Vec<AssetReference> = Vec::new();
    for material in &source.materials {
        let mut material_deps: Vec<AssetReference> = Vec::new();
        if let Some(texture) = &material.texture {
            if context.texture_is_missing(texture) {
                diagnostics.push(ImportDiagnostic::warning(
                    ImportCode::MissingTexture,
                    format!("{}#texture/{texture}", material_asset_id(&material.name)),
                    format!(
                        "material `{}` references texture `{texture}`, which is not available",
                        material.name
                    ),
                    "provide the external texture resource or remove the reference",
                ));
            }
            let tex_id = texture_asset_id(texture);
            let tex_hash = fingerprint_hash(tex_id.as_bytes());
            push_unique(
                &mut entries,
                &mut seen_ids,
                &mut diagnostics,
                &tex_id,
                || CatalogEntry {
                    id: AssetId::parse(&tex_id).expect("texture id is well-formed"),
                    version: 1,
                    hash: Some(tex_hash.clone()),
                    source_path: None,
                    label: Some(texture.clone()),
                    dependencies: Vec::new(),
                    material: None,
                },
            );
            if let Some(dep) = dependency(&tex_id, &tex_hash) {
                material_deps.push(dep);
            }
        }

        let mat_id = material_asset_id(&material.name);
        let mat_hash = fingerprint_hash(material_content(material).as_bytes());
        let material_def = MaterialDef {
            authority: MaterialAuthority::DECORATIVE,
            style: MaterialStyle {
                color: Rgba {
                    r: material.color[0],
                    g: material.color[1],
                    b: material.color[2],
                    a: material.color[3],
                },
                texture: material.texture.as_ref().and_then(|t| {
                    dependency(
                        &texture_asset_id(t),
                        &fingerprint_hash(texture_asset_id(t).as_bytes()),
                    )
                }),
                roughness: 1.0,
                texture_tint: Rgba::WHITE,
                emission_color: Rgba {
                    r: material.color[0],
                    g: material.color[1],
                    b: material.color[2],
                    a: material.color[3],
                },
                emissive: 0.0,
                uv_strategy: if material.texture.is_some() {
                    UvStrategy::Planar
                } else {
                    UvStrategy::Flat
                },
            },
        };
        let deps_for_entry = material_deps.clone();
        push_unique(
            &mut entries,
            &mut seen_ids,
            &mut diagnostics,
            &mat_id,
            || CatalogEntry {
                id: AssetId::parse(&mat_id).expect("material id is well-formed"),
                version: 1,
                hash: Some(mat_hash.clone()),
                source_path: None,
                label: Some(material.name.clone()),
                dependencies: deps_for_entry,
                material: Some(material_def.clone()),
            },
        );
        if let Some(dep) = dependency(&mat_id, &mat_hash) {
            mesh_deps.push(dep);
        }
    }

    let mesh_id = mesh_asset_id(&source.name);
    let mesh_hash = fingerprint_hash(mesh_content(source).as_bytes());
    push_unique(
        &mut entries,
        &mut seen_ids,
        &mut diagnostics,
        &mesh_id,
        || CatalogEntry {
            id: AssetId::parse(&mesh_id).expect("mesh id is well-formed"),
            version: 1,
            hash: Some(mesh_hash.clone()),
            source_path: None,
            label: Some(source.name.clone()),
            dependencies: mesh_deps.clone(),
            material: None,
        },
    );

    if diagnostics.iter().any(ImportDiagnostic::is_error) {
        return ImportOutcome {
            assets: None,
            diagnostics,
        };
    }

    let catalog = Catalog::from_entries(entries).canonical();
    ImportOutcome {
        assets: Some(ImportedAssets {
            static_mesh,
            catalog,
        }),
        diagnostics,
    }
}

/// Insert a catalog entry once per id; a second occurrence is a classified
/// duplicate diagnostic rather than a silent overwrite.
fn push_unique(
    entries: &mut Vec<CatalogEntry>,
    seen: &mut Vec<String>,
    diagnostics: &mut Vec<ImportDiagnostic>,
    id: &str,
    build: impl FnOnce() -> CatalogEntry,
) {
    if seen.iter().any(|s| s == id) {
        diagnostics.push(ImportDiagnostic::error(
            ImportCode::DuplicateAssetId,
            id,
            format!("two source declarations resolve to the same asset id `{id}`"),
            "give each material/texture a distinct name",
        ));
        return;
    }
    seen.push(id.to_string());
    entries.push(build());
}
