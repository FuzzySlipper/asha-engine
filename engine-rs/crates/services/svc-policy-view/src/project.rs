//! Project authority state into the read-only [`PolicyWorldView`] (#2391).
//!
//! The projection is deterministic and redacting: tombstoned entities and
//! `DiagnosticTooling`-sourced entities never appear, renderer handles and collider
//! geometry are never included, and entities/assets are emitted in a stable id
//! order. A policy reasons about identity, lifecycle, transform, source, labels,
//! and asset *status* — nothing that would let it treat cached/render state as
//! truth.

use std::collections::BTreeMap;

use core_assets::AssetReference;
use core_entity::{EntityCore, EntityLifecycle, EntitySource, EntityStore, EntityTransform};
use protocol_policy_view::{
    PolicyAssetStatus, PolicyAssetView, PolicyEntityLifecycle, PolicyEntitySource,
    PolicyEntityView, PolicyTransform, PolicyWorldSummary, PolicyWorldView,
};

/// Catalog-supplied asset resolution status keyed by asset id. An asset referenced
/// by an entity but absent from this map projects as [`PolicyAssetStatus::Missing`]
/// — a missing status is never silently dropped.
pub type AssetStatusMap = BTreeMap<String, PolicyAssetStatus>;

fn to_policy_transform(t: &EntityTransform) -> PolicyTransform {
    PolicyTransform {
        translation: [t.translation.x, t.translation.y, t.translation.z],
        rotation: [t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w],
        scale: [t.scale.x, t.scale.y, t.scale.z],
    }
}

/// Map an authority source to its policy-visible form, or `None` for sources that
/// must be redacted from policy truth (`DiagnosticTooling`).
fn to_policy_source(source: &EntitySource) -> Option<PolicyEntitySource> {
    match source {
        EntitySource::SceneBootstrap { node } => {
            Some(PolicyEntitySource::SceneNode { node: node.raw() })
        }
        EntitySource::RuntimeCreated { .. } => Some(PolicyEntitySource::Runtime),
        EntitySource::Imported { asset } => Some(PolicyEntitySource::Imported {
            asset: asset.id().as_str().to_string(),
        }),
        EntitySource::PolicyProposed { .. } => Some(PolicyEntitySource::Policy),
        EntitySource::DiagnosticTooling => None,
    }
}

/// The asset reference an entity carries, if any: an `Imported` source or an
/// explicit asset-binding capability. The binding capability wins when both exist.
fn entity_asset<'a>(store: &'a EntityStore, core: &'a EntityCore) -> Option<&'a AssetReference> {
    if let Some(binding) = store.asset_binding(core.id) {
        return Some(&binding.asset);
    }
    match &core.source {
        EntitySource::Imported { asset } => Some(asset),
        _ => None,
    }
}

/// Project the authority entity store into a deterministic [`PolicyWorldView`] at
/// `tick`. `asset_statuses` carries the catalog's resolution classification.
pub fn project_world_view(
    tick: u64,
    store: &EntityStore,
    asset_statuses: &AssetStatusMap,
) -> PolicyWorldView {
    // Stable id order; collect cores first so iteration order is independent of the
    // store's internal map ordering.
    let mut cores: Vec<&EntityCore> = store.entities().collect();
    cores.sort_by_key(|c| c.id.raw());

    let mut entities: Vec<PolicyEntityView> = Vec::new();
    // Dedup asset ids in first-seen order, then sort for a stable view.
    let mut asset_refs: BTreeMap<String, AssetReference> = BTreeMap::new();

    for core in cores {
        // Redactions: tombstoned and diagnostic-tooling entities are never policy truth.
        if core.lifecycle == EntityLifecycle::Tombstoned {
            continue;
        }
        let Some(source) = to_policy_source(&core.source) else {
            continue;
        };

        if let Some(asset) = entity_asset(store, core) {
            asset_refs
                .entry(asset.id().as_str().to_string())
                .or_insert_with(|| asset.clone());
        }

        let lifecycle = match core.lifecycle {
            EntityLifecycle::Active => PolicyEntityLifecycle::Active,
            EntityLifecycle::Disabled => PolicyEntityLifecycle::Disabled,
            EntityLifecycle::Tombstoned => unreachable!("tombstoned filtered above"),
        };
        let transform = store
            .transform(core.id)
            .map(|cap| to_policy_transform(&cap.transform));

        entities.push(PolicyEntityView {
            id: core.id,
            lifecycle,
            transform,
            source,
            labels: core.labels.clone(),
            spatial: transform.is_some(),
        });
    }

    let assets: Vec<PolicyAssetView> = asset_refs
        .into_iter()
        .map(|(id, asset)| {
            let status = asset_statuses
                .get(&id)
                .copied()
                .unwrap_or(PolicyAssetStatus::Missing);
            PolicyAssetView {
                id,
                kind: asset.kind().prefix().to_string(),
                status,
            }
        })
        .collect();

    let summary = PolicyWorldSummary {
        tick,
        active_entities: entities
            .iter()
            .filter(|e| e.lifecycle == PolicyEntityLifecycle::Active)
            .count() as u32,
        spatial_entities: entities.iter().filter(|e| e.spatial).count() as u32,
        asset_count: assets.len() as u32,
        missing_assets: assets
            .iter()
            .filter(|a| a.status == PolicyAssetStatus::Missing)
            .count() as u32,
    };

    PolicyWorldView {
        tick,
        entities,
        assets,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_assets::{markers, AssetRef, AssetVersionReq};
    use core_entity::{EntityLifecycleCommand, EntitySource};
    use core_ids::{EntityId, SceneNodeId, TagId};

    fn mesh_ref(id: &str) -> AssetReference {
        AssetRef::<markers::StaticMesh>::parse(id, AssetVersionReq::Any, None)
            .unwrap()
            .erase()
    }

    /// A small abstract world: a scene-sourced spatial entity, a runtime logical
    /// entity, an imported asset-bound entity, a diagnostic-tooling entity (redacted),
    /// and a tombstoned entity (redacted).
    fn sample_store() -> EntityStore {
        let mut store = EntityStore::new();
        store
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(1),
                source: EntitySource::SceneBootstrap {
                    node: SceneNodeId::new(10),
                },
                labels: Vec::new(),
            })
            .unwrap();
        store.attach_transform(
            EntityId::new(1),
            EntityTransform::at(core_math::Vec3::new(1.0, 2.0, 3.0)),
        );
        store
            .apply(EntityLifecycleCommand::AddLabel {
                id: EntityId::new(1),
                tag: TagId::new(7),
            })
            .unwrap();

        store
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(2),
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .unwrap();

        store
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(3),
                source: EntitySource::Imported {
                    asset: mesh_ref("mesh/crate"),
                },
                labels: Vec::new(),
            })
            .unwrap();

        store
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(4),
                source: EntitySource::DiagnosticTooling,
                labels: Vec::new(),
            })
            .unwrap();

        store
            .apply(EntityLifecycleCommand::Create {
                id: EntityId::new(5),
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .unwrap();
        store
            .apply(EntityLifecycleCommand::Destroy {
                id: EntityId::new(5),
            })
            .unwrap();

        store
    }

    #[test]
    fn projection_is_deterministic() {
        let store = sample_store();
        let statuses = AssetStatusMap::new();
        assert_eq!(
            project_world_view(42, &store, &statuses),
            project_world_view(42, &store, &statuses),
        );
    }

    #[test]
    fn redacts_tombstoned_and_diagnostic_entities() {
        let view = project_world_view(1, &sample_store(), &AssetStatusMap::new());
        let ids: Vec<u64> = view.entities.iter().map(|e| e.id.raw()).collect();
        // 1 (scene), 2 (runtime), 3 (imported) survive; 4 (diagnostic) and 5
        // (tombstoned) are redacted.
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn carries_transform_source_and_labels() {
        let view = project_world_view(1, &sample_store(), &AssetStatusMap::new());
        let e1 = &view.entities[0];
        assert_eq!(e1.id.raw(), 1);
        assert_eq!(e1.source, PolicyEntitySource::SceneNode { node: 10 });
        assert!(e1.spatial);
        assert_eq!(e1.transform.unwrap().translation, [1.0, 2.0, 3.0]);
        assert_eq!(e1.labels, vec![TagId::new(7)]);

        let e2 = &view.entities[1];
        assert_eq!(e2.source, PolicyEntitySource::Runtime);
        assert!(!e2.spatial);
        assert!(e2.transform.is_none());
    }

    #[test]
    fn classifies_asset_status_and_counts() {
        let mut statuses = AssetStatusMap::new();
        statuses.insert("mesh/crate".to_string(), PolicyAssetStatus::Stale);
        let view = project_world_view(9, &sample_store(), &statuses);
        assert_eq!(view.assets.len(), 1);
        assert_eq!(view.assets[0].id, "mesh/crate");
        assert_eq!(view.assets[0].kind, "mesh");
        assert_eq!(view.assets[0].status, PolicyAssetStatus::Stale);

        assert_eq!(view.summary.tick, 9);
        assert_eq!(view.summary.active_entities, 3);
        assert_eq!(view.summary.spatial_entities, 1);
        assert_eq!(view.summary.asset_count, 1);
        assert_eq!(view.summary.missing_assets, 0);
    }

    #[test]
    fn unmapped_referenced_asset_is_missing() {
        let view = project_world_view(1, &sample_store(), &AssetStatusMap::new());
        assert_eq!(view.assets[0].status, PolicyAssetStatus::Missing);
        assert_eq!(view.summary.missing_assets, 1);
    }
}
