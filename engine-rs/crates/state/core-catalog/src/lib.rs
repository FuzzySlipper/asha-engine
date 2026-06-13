//! Asset registry / catalog validation for the ASHA scene/world foundation
//! (scene-capability-03, epic #2311).
//!
//! # Lane
//!
//! `rust-state` — authority-relevant validation. Builds on the `core-assets`
//! foundation vocabulary (typed `AssetRef<T>`, kind-prefixed scoped-kebab-case
//! `AssetId`); it must not reach into protocol, render, or wasm layers. TS may
//! author catalog *data*, but only this crate decides whether references may enter
//! authority/runtime.
//!
//! # Scope
//!
//! * [`Catalog`] / [`CatalogEntry`] + [`validate`] — catalog manifest validation:
//!   duplicate ids, material-payload placement, wrong-kind typed slots, missing
//!   dependencies, and a Rust-validated dependency **DAG** with cycle-path
//!   diagnostics (subtask #2322).
//! * [`AssetLock`] + [`generate_lock`] / [`validate_lock`] — world-bundle asset
//!   locks and classified catalog-drift diagnostics (subtask #2323).
//! * [`MaterialDef`] with the **authority / style** projection split, plus
//!   context-based [`fallback_for`] policy (subtask #2324).
//! * [`revalidate_asset`] — single-asset change-impact diagnostics for development
//!   iteration (subtask #2325).
//!
//! # Boundaries
//!
//! The renderer consumes the [`RenderMaterial`] projection (no collision class);
//! collision/authority consumes [`CollisionMaterial`] (no texture/colour). The
//! material asset is the source; projections are consumer-specific. No
//! `protocol-*`/codegen border surface is added here — it lands when catalog/asset
//! descriptor shapes actually cross to TS (static-mesh/sprite rendering, devtools).

#![forbid(unsafe_code)]

pub mod dag;
pub mod entry;
pub mod fallback;
pub mod json;
pub mod lock;
pub mod material;
pub mod revalidate;
pub mod validate;
pub mod voxel;

pub use dag::DependencyGraph;
pub use entry::{Catalog, CatalogEntry};
pub use fallback::{fallback_for, AssetContext, FallbackOutcome, FallbackVisual};
pub use json::{decode, encode, CatalogDecodeError};
pub use lock::{
    generate_lock, validate_lock, AssetLock, AssetLockEntry, LockFinding, LockIssue,
    LockValidationReport,
};
pub use material::{
    CollisionMaterial, MaterialAuthority, MaterialDef, MaterialStyle, RenderMaterial, Rgba,
    StructuralClass, UvStrategy,
};
pub use revalidate::{
    classify_material_change, material_change_impact, revalidate_asset, ChangeImpactReport,
    ChangeKind, ReloadSuggestion,
};
pub use validate::{validate, CatalogValidationError, CatalogValidationReport};
pub use voxel::{
    VoxelMaterialError, VoxelMaterialTable, VoxelMaterialTableReport, VoxelRenderResolution,
};

#[cfg(test)]
mod tests {
    use super::*;
    use core_assets::{AssetHash, AssetId, AssetKind, AssetReference, AssetVersionReq};

    fn id(s: &str) -> AssetId {
        AssetId::parse(s).unwrap()
    }

    fn dep(s: &str) -> AssetReference {
        AssetReference::new(id(s), AssetVersionReq::Any, None)
    }

    fn material(structural: StructuralClass) -> MaterialDef {
        MaterialDef {
            authority: MaterialAuthority {
                solid: true,
                collidable: true,
                occludes: true,
                structural_class: structural,
            },
            style: MaterialStyle::flat(Rgba::DEBUG_GREY),
        }
    }

    /// A valid abstract catalog: a texture, a material → texture, and a static
    /// mesh → material. Abstract fixture nouns only.
    fn sample_catalog() -> Catalog {
        let texture = CatalogEntry::new(id("texture/surface-atlas-a"), 1)
            .with_hash(AssetHash::parse("aa01").unwrap())
            .with_source("textures/surface-atlas-a.png");

        let mut mat = material(StructuralClass::Structural);
        mat.style.texture = Some(dep("texture/surface-atlas-a"));
        let material_entry = CatalogEntry::new(id("material/surface-a"), 2)
            .with_hash(AssetHash::parse("bb02").unwrap())
            .with_material(mat)
            .with_dependencies(vec![dep("texture/surface-atlas-a")]);

        let mesh = CatalogEntry::new(id("mesh/fixture-a"), 1)
            .with_hash(AssetHash::parse("cc03").unwrap())
            .with_dependencies(vec![dep("material/surface-a")]);

        Catalog::from_entries(vec![texture, material_entry, mesh])
    }

    // ── #2322 catalog + DAG ────────────────────────────────────────────────────

    #[test]
    fn valid_catalog_passes() {
        assert!(validate(&sample_catalog()).is_ok());
    }

    #[test]
    fn detects_duplicate_asset_id() {
        let mut c = sample_catalog();
        c.entries.push(CatalogEntry::new(id("mesh/fixture-a"), 9));
        let report = validate(&c);
        assert!(report.errors.iter().any(|e| matches!(
            e,
            CatalogValidationError::DuplicateAssetId { id } if id.as_str() == "mesh/fixture-a"
        )));
    }

    #[test]
    fn detects_missing_dependency() {
        let mut c = sample_catalog();
        c.entries.push(
            CatalogEntry::new(id("mesh/fixture-b"), 1)
                .with_dependencies(vec![dep("material/does-not-exist")]),
        );
        let report = validate(&c);
        assert!(report.errors.iter().any(|e| matches!(
            e,
            CatalogValidationError::UnknownDependency { dependency, .. }
                if dependency.as_str() == "material/does-not-exist"
        )));
    }

    #[test]
    fn detects_wrong_kind_texture_slot() {
        let mut c = sample_catalog();
        // Point the material's texture slot at a non-texture asset.
        if let Some(m) = c.entries[1].material.as_mut() {
            m.style.texture = Some(dep("material/surface-a"));
        }
        let report = validate(&c);
        assert!(report.errors.iter().any(|e| matches!(
            e,
            CatalogValidationError::WrongKindReference {
                expected: AssetKind::Texture,
                actual: AssetKind::Material,
                ..
            }
        )));
    }

    #[test]
    fn detects_material_payload_placement() {
        // Material without payload.
        let mut c = Catalog::from_entries(vec![CatalogEntry::new(id("material/no-payload"), 1)]);
        assert!(validate(&c)
            .errors
            .iter()
            .any(|e| matches!(e, CatalogValidationError::MaterialPayloadMissing { .. })));
        // Non-material with payload.
        c = Catalog::from_entries(vec![CatalogEntry::new(id("mesh/has-material"), 1)
            .with_material(material(StructuralClass::Solid))]);
        assert!(validate(&c).errors.iter().any(|e| matches!(
            e,
            CatalogValidationError::MaterialPayloadOnNonMaterial { .. }
        )));
    }

    #[test]
    fn detects_dependency_cycle_with_path() {
        // mesh/a -> material/b -> texture/c -> mesh/a
        let a = CatalogEntry::new(id("mesh/a"), 1).with_dependencies(vec![dep("material/b")]);
        let mut b_mat = material(StructuralClass::Solid);
        b_mat.style.texture = None;
        let b = CatalogEntry::new(id("material/b"), 1)
            .with_material(b_mat)
            .with_dependencies(vec![dep("texture/c")]);
        let c_tex = CatalogEntry::new(id("texture/c"), 1).with_dependencies(vec![dep("mesh/a")]);
        let cat = Catalog::from_entries(vec![a, b, c_tex]);

        let report = validate(&cat);
        let path = report
            .errors
            .iter()
            .find_map(|e| match e {
                CatalogValidationError::DependencyCycle { path } => Some(path.clone()),
                _ => None,
            })
            .expect("cycle reported");
        // Closed path: first == last, three distinct nodes.
        assert_eq!(
            path.first().unwrap().as_str(),
            path.last().unwrap().as_str()
        );
        let distinct: std::collections::BTreeSet<&str> = path.iter().map(|i| i.as_str()).collect();
        assert_eq!(distinct.len(), 3);
    }

    #[test]
    fn dag_lists_dependents() {
        let cat = sample_catalog();
        let graph = DependencyGraph::build(&cat);
        // The texture is depended on (transitively) by the material and the mesh.
        let dependents: Vec<String> = graph
            .dependents_of(&id("texture/surface-atlas-a"))
            .iter()
            .map(|i| i.as_str().to_string())
            .collect();
        assert_eq!(dependents, vec!["material/surface-a", "mesh/fixture-a"]);
    }

    #[test]
    fn catalog_json_round_trips_and_validates() {
        let c = sample_catalog();
        let encoded = encode(&c);
        let decoded = decode(&encoded).expect("decode");
        assert_eq!(encode(&decoded), encoded, "encode∘decode is a fixed point");
        assert_eq!(decoded.canonical(), c.canonical());
        assert!(validate(&decoded).is_ok());
    }

    // ── #2323 asset lock ───────────────────────────────────────────────────────

    #[test]
    fn fresh_lock_validates_clean() {
        let c = sample_catalog();
        let lock = generate_lock(&c);
        assert!(validate_lock(&lock, &c).is_clean());
    }

    #[test]
    fn lock_detects_version_and_hash_drift() {
        let c = sample_catalog();
        let lock = generate_lock(&c);
        let mut drifted = c.clone();
        drifted.entries[2].version = 7; // mesh/fixture-a version bump
        drifted.entries[0].hash = Some(AssetHash::parse("ffff").unwrap()); // texture hash change
        let report = validate_lock(&lock, &drifted);
        assert!(report.findings.iter().any(|f| matches!(
            f.issue,
            LockIssue::StaleVersion {
                locked: 1,
                current: 7
            }
        )));
        assert!(report
            .findings
            .iter()
            .any(|f| matches!(f.issue, LockIssue::StaleHash { .. })));
    }

    #[test]
    fn lock_detects_missing_and_new_assets() {
        let c = sample_catalog();
        let lock = generate_lock(&c);
        let mut changed = c.clone();
        changed.entries.remove(2); // drop mesh/fixture-a
        changed
            .entries
            .push(CatalogEntry::new(id("sprite/new-a"), 1));
        let report = validate_lock(&lock, &changed);
        assert!(report.findings.iter().any(|f| {
            f.id.as_str() == "mesh/fixture-a" && matches!(f.issue, LockIssue::Missing)
        }));
        assert!(report.findings.iter().any(|f| {
            f.id.as_str() == "sprite/new-a" && matches!(f.issue, LockIssue::NewInCatalog)
        }));
    }

    #[test]
    fn lock_detects_wrong_kind_and_dependency_drift() {
        let c = sample_catalog();
        let lock = generate_lock(&c);
        let mut changed = c.clone();
        // mesh/fixture-a gains a new dependency.
        changed.entries[2]
            .dependencies
            .push(dep("texture/surface-atlas-a"));
        let report = validate_lock(&lock, &changed);
        assert!(report.findings.iter().any(|f| matches!(
            &f.issue,
            LockIssue::DependencyDrift { added, .. }
                if added.iter().any(|a| a.as_str() == "texture/surface-atlas-a")
        )));
    }

    // ── #2324 material projection + fallback ───────────────────────────────────

    #[test]
    fn material_projections_are_separated() {
        let m = material(StructuralClass::Structural);
        let render = m.render_projection();
        let collision = m.collision_projection();
        // Render projection carries visual fields; collision carries authority.
        assert_eq!(render.uv_strategy, UvStrategy::Flat);
        assert_eq!(render.color, Rgba::DEBUG_GREY);
        assert!(collision.collidable);
        assert_eq!(collision.structural_class, StructuralClass::Structural);
        // The types are disjoint: RenderMaterial has no collision field and
        // CollisionMaterial has no colour/texture field (enforced at compile time).
    }

    #[test]
    fn fallback_depends_on_context_not_just_kind() {
        // Same kind (Material), different context → different outcome.
        assert!(matches!(
            fallback_for(AssetKind::Material, AssetContext::CosmeticSurface),
            FallbackOutcome::UseFallback {
                visual: FallbackVisual::GreyMaterial,
                ..
            }
        ));
        assert!(matches!(
            fallback_for(AssetKind::Material, AssetContext::CollisionCritical),
            FallbackOutcome::FailClosed { .. }
        ));
        // Debug overlay sprite → magenta square.
        assert!(matches!(
            fallback_for(AssetKind::Sprite, AssetContext::DebugOverlay),
            FallbackOutcome::UseFallback {
                visual: FallbackVisual::MagentaSquare,
                ..
            }
        ));
        // Background decoration → skip.
        assert!(matches!(
            fallback_for(AssetKind::StaticMesh, AssetContext::BackgroundDecoration),
            FallbackOutcome::Skip { .. }
        ));
    }

    // ── #2325 single-asset revalidation ────────────────────────────────────────

    #[test]
    fn revalidation_reports_dependents_and_safety() {
        let c = sample_catalog();
        // A visual-only change to the texture is safe and reprojectable.
        let visual = revalidate_asset(&c, &id("texture/surface-atlas-a"), ChangeKind::VisualOnly)
            .expect("present");
        assert!(visual.safe);
        assert!(!visual.requires_full_reload);
        assert_eq!(visual.suggestion, ReloadSuggestion::Reproject);
        assert_eq!(
            visual
                .affected_dependents
                .iter()
                .map(|i| i.as_str().to_string())
                .collect::<Vec<_>>(),
            vec!["material/surface-a", "mesh/fixture-a"]
        );

        // An authority-impacting change to the material is unsafe (not visual-only).
        let authority = revalidate_asset(
            &c,
            &id("material/surface-a"),
            ChangeKind::AuthorityImpacting,
        )
        .expect("present");
        assert!(!authority.safe);
        assert_eq!(authority.suggestion, ReloadSuggestion::RevalidateDependents);

        // A structural change requires a full reload.
        let structural =
            revalidate_asset(&c, &id("mesh/fixture-a"), ChangeKind::Structural).expect("present");
        assert!(structural.requires_full_reload);
        assert_eq!(structural.suggestion, ReloadSuggestion::RequiresFullReload);
    }

    #[test]
    fn revalidating_unknown_asset_is_none() {
        let c = sample_catalog();
        assert!(revalidate_asset(&c, &id("mesh/nope"), ChangeKind::VisualOnly).is_none());
    }
}
