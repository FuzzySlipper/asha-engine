//! Cross-vocabulary lifecycle tests (#2387).
//!
//! Per the design gate, lifecycle is not "done" if it only passes spatial/voxel
//! fixtures. These exercise families 1–5 (spatial rendered, spatial non-rendered,
//! non-spatial logical, contained, diagnostic-tooling), plus save/replay stability
//! and the negative transitions that must fail closed.

use core_entity::command::EntityLifecycleCommand as Cmd;
use core_entity::core::{EntityLifecycle, EntitySource};
use core_entity::store::EntityStore;
use core_entity::{fixtures, EntityLifecycleError};
use core_ids::{EntityId, TagId};

fn e(id: u64) -> EntityId {
    EntityId::new(id)
}

// ── Create / destroy across the fixture matrix ───────────────────────────────

#[test]
fn create_destroy_is_replayable_and_hash_stable_across_families() {
    // Replaying the same build twice yields identical hashes for every family —
    // spatial and non-spatial alike.
    for (name, _) in fixtures::all_families() {
        let a = rebuild(name);
        let b = rebuild(name);
        assert_eq!(a.hash(), b.hash(), "family {name} hash not stable");
    }
}

fn rebuild(name: &str) -> EntityStore {
    fixtures::all_families()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, store)| store)
        .expect("known family")
}

#[test]
fn non_spatial_entity_has_no_phantom_capabilities() {
    // Family 3: logical entities must not gain a transform/render/collider.
    let store = fixtures::non_spatial_logical_family();
    for id in [1u64, 2, 3] {
        assert!(store.transform(e(id)).is_none(), "no phantom transform");
        assert!(
            store.render_projection(e(id)).is_none(),
            "no phantom render"
        );
        assert!(store.collision(e(id)).is_none(), "no phantom collider");
        assert!(store.bounds(e(id)).is_none(), "no phantom bounds");
    }
    assert_eq!(store.alive_count(), 3);
}

#[test]
fn spatial_non_rendered_entity_has_no_render_projection() {
    // Family 2: spatial authority is independent of rendering.
    let store = fixtures::spatial_non_rendered_family();
    assert!(store.bounds(e(1)).is_some());
    assert!(store.render_projection(e(1)).is_none());
    assert!(store.transform(e(2)).is_some());
    assert!(store.render_projection(e(2)).is_none());
}

#[test]
fn contained_entity_is_not_spatially_attached() {
    // Family 4: containment relation present, no world transform implied.
    let store = fixtures::contained_family();
    assert_eq!(store.containment(e(2)).unwrap().container, e(1));
    assert!(
        store.transform(e(2)).is_none(),
        "containment must not imply a transform"
    );
}

#[test]
fn diagnostic_tooling_entities_are_flagged_and_save_excluded() {
    // Family 5: UI/devtools entities are DiagnosticTooling and excluded from
    // durable saves by default policy.
    let store = fixtures::ui_devtools_family();
    for id in [1u64, 2, 3] {
        assert!(matches!(
            store.core(e(id)).unwrap().source,
            EntitySource::DiagnosticTooling
        ));
    }
    let durable = store.snapshot_durable();
    assert!(
        durable.records.is_empty(),
        "diagnostic entities are not durable truth"
    );
    // The full snapshot still includes them (for in-session save/inspection).
    assert_eq!(store.snapshot().records.len(), 3);
}

// ── Save / reload ────────────────────────────────────────────────────────────

#[test]
fn save_reload_round_trips_all_state_including_tombstones() {
    let store = fixtures::lifecycle_scenario();
    let snapshot = store.snapshot();
    let restored = EntityStore::from_snapshot(snapshot);
    assert_eq!(restored, store, "restore must reproduce the store exactly");
    assert_eq!(
        restored.hash(),
        store.hash(),
        "hash must survive a save→reload"
    );

    // The destroyed scene-sourced entity is represented as a tombstone, not gone.
    assert_eq!(restored.lifecycle(e(1)), Some(EntityLifecycle::Tombstoned));
    assert!(restored.core(e(1)).unwrap().source.scene_node().is_some());
    // The runtime-created survivor keeps its transform and labels.
    assert!(restored.transform(e(2)).is_some());
    assert_eq!(restored.core(e(2)).unwrap().labels.len(), 3);
}

#[test]
fn durable_save_drops_diagnostic_but_keeps_tombstones() {
    let store = fixtures::lifecycle_scenario();
    let durable = EntityStore::from_snapshot(store.snapshot_durable());
    // Entity 4 (DiagnosticTooling) is excluded.
    assert!(!durable.contains(e(4)));
    // The tombstoned scene entity is retained so reload reproduces the retired id.
    assert_eq!(durable.lifecycle(e(1)), Some(EntityLifecycle::Tombstoned));
    assert!(durable.contains(e(2)));
    assert!(durable.contains(e(3)));
}

// ── Negative transitions (fail closed, no partial mutation) ──────────────────

#[test]
fn duplicate_create_and_retired_id_fail_closed() {
    let mut store = EntityStore::new();
    store
        .apply(Cmd::Create {
            id: e(1),
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        })
        .unwrap();
    // Re-create an active id.
    assert_eq!(
        store.apply(Cmd::Create {
            id: e(1),
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        }),
        Err(EntityLifecycleError::AlreadyExists { id: e(1) })
    );
    // Tombstone it, then re-create the retired id.
    store.apply(Cmd::Destroy { id: e(1) }).unwrap();
    assert_eq!(
        store.apply(Cmd::Create {
            id: e(1),
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        }),
        Err(EntityLifecycleError::IdRetired { id: e(1) })
    );
}

#[test]
fn operations_on_unknown_and_tombstoned_fail_closed() {
    let mut store = EntityStore::new();
    assert_eq!(
        store.apply(Cmd::Disable { id: e(9) }),
        Err(EntityLifecycleError::UnknownEntity { id: e(9) })
    );

    store
        .apply(Cmd::Create {
            id: e(1),
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        })
        .unwrap();
    store.apply(Cmd::Destroy { id: e(1) }).unwrap();
    // A tombstoned entity rejects further mutation with a classified error.
    assert_eq!(
        store.apply(Cmd::Disable { id: e(1) }),
        Err(EntityLifecycleError::Tombstoned { id: e(1) })
    );
    assert_eq!(
        store.apply(Cmd::AddLabel {
            id: e(1),
            tag: TagId::new(1)
        }),
        Err(EntityLifecycleError::Tombstoned { id: e(1) })
    );
    // Double destroy is also a tombstoned error.
    assert_eq!(
        store.apply(Cmd::Destroy { id: e(1) }),
        Err(EntityLifecycleError::Tombstoned { id: e(1) })
    );
}

#[test]
fn invalid_enable_disable_transitions_fail_closed() {
    let mut store = EntityStore::new();
    store
        .apply(Cmd::Create {
            id: e(1),
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        })
        .unwrap();
    // Enable an already-active entity.
    assert!(matches!(
        store.apply(Cmd::Enable { id: e(1) }),
        Err(EntityLifecycleError::InvalidTransition { op: "enable", .. })
    ));
    store.apply(Cmd::Disable { id: e(1) }).unwrap();
    // Disable an already-disabled entity.
    assert!(matches!(
        store.apply(Cmd::Disable { id: e(1) }),
        Err(EntityLifecycleError::InvalidTransition { op: "disable", .. })
    ));
    store.apply(Cmd::Enable { id: e(1) }).unwrap();
    assert_eq!(store.lifecycle(e(1)), Some(EntityLifecycle::Active));
}

#[test]
fn label_set_rejects_duplicate_and_absent() {
    let mut store = EntityStore::new();
    store
        .apply(Cmd::Create {
            id: e(1),
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![TagId::new(5)],
        })
        .unwrap();
    assert_eq!(
        store.apply(Cmd::AddLabel {
            id: e(1),
            tag: TagId::new(5)
        }),
        Err(EntityLifecycleError::LabelAlreadyPresent {
            id: e(1),
            tag: TagId::new(5)
        })
    );
    assert_eq!(
        store.apply(Cmd::RemoveLabel {
            id: e(1),
            tag: TagId::new(99)
        }),
        Err(EntityLifecycleError::LabelAbsent {
            id: e(1),
            tag: TagId::new(99)
        })
    );
    // A rejected add did not mutate the label set.
    assert_eq!(store.core(e(1)).unwrap().labels, vec![TagId::new(5)]);
}

#[test]
fn failed_command_does_not_mutate_state_hash() {
    let mut store = fixtures::spatial_rendered_family();
    let before = store.hash();
    let _ = store.apply(Cmd::Disable { id: e(999) });
    let _ = store.apply(Cmd::Enable { id: e(1) }); // already active → rejected
    assert_eq!(
        store.hash(),
        before,
        "rejected commands must not change the hash"
    );
}

#[test]
fn attach_capability_to_unknown_or_tombstoned_is_a_noop() {
    let mut store = EntityStore::new();
    assert!(
        !store.attach_render_projection(e(7), true),
        "unknown entity"
    );
    store
        .apply(Cmd::Create {
            id: e(1),
            source: EntitySource::RuntimeCreated { by: None },
            labels: vec![],
        })
        .unwrap();
    store.apply(Cmd::Destroy { id: e(1) }).unwrap();
    assert!(
        !store.attach_render_projection(e(1), true),
        "tombstoned entity"
    );
    assert!(store.render_projection(e(1)).is_none());
}
