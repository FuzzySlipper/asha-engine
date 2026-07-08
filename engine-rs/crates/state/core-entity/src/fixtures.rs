//! Deterministic cross-vocabulary entity fixtures (design §6).
//!
//! Each family is built from the *real* lifecycle applier + capability attach, so
//! the fixtures exercise the same paths production code does. Names are abstract —
//! **no product nouns** (player/enemy/item). #2387 exercises families 1–5; the
//! attachment (6) and movement (7) families are added by #2389/#2390.

use core_assets::{AssetId, AssetReference, AssetVersionReq};
use core_ids::{EntityId, ProcessId, SceneNodeId, SubjectId, TagId};

use crate::capability::ControllerCapability;
use crate::command::EntityLifecycleCommand;
use crate::core::EntitySource;
use crate::store::EntityStore;
use crate::value::{Aabb, EntityTransform};
use core_math::Vec3;

fn mesh_asset(id: &str) -> AssetReference {
    AssetReference::new(
        AssetId::parse(id).expect("valid asset id"),
        AssetVersionReq::Any,
        None,
    )
}

/// Create an entity (panicking on the impossible — fixtures are deterministic).
fn create(store: &mut EntityStore, id: u64, source: EntitySource, labels: &[u64]) -> EntityId {
    let entity = EntityId::new(id);
    store
        .apply(EntityLifecycleCommand::Create {
            id: entity,
            source,
            labels: labels.iter().map(|t| TagId::new(*t)).collect(),
        })
        .expect("fixture create");
    entity
}

/// Family 1 — spatial rendered. Proves render projection works without using voxel
/// terminology as the entity model, and runtime transform is a capability.
pub fn spatial_rendered_family() -> EntityStore {
    let mut store = EntityStore::new();

    // spatial_marker_entity: runtime-created, transform + render projection.
    let marker = create(
        &mut store,
        1,
        EntitySource::RuntimeCreated { by: None },
        &[10],
    );
    store.attach_transform(marker, EntityTransform::at(Vec3::new(1.0, 2.0, 3.0)));
    store.attach_render_projection(marker, true);

    // rendered_static_asset_entity: imported asset, transform + render + binding.
    let asset = create(
        &mut store,
        2,
        EntitySource::Imported {
            asset: mesh_asset("mesh/static-fixture-a"),
        },
        &[],
    );
    store.attach_transform(asset, EntityTransform::IDENTITY);
    store.attach_render_projection(asset, true);
    store.attach_asset_binding(asset, mesh_asset("mesh/static-fixture-a"));

    // scene_sourced_transform_entity: scene bootstrap, transform + render.
    let scene = create(
        &mut store,
        3,
        EntitySource::SceneBootstrap {
            node: SceneNodeId::new(10),
        },
        &[],
    );
    store.attach_transform(scene, EntityTransform::at(Vec3::new(5.0, 0.0, 0.0)));
    store.attach_render_projection(scene, true);

    store
}

/// Family 2 — spatial non-rendered. Proves spatial authority is independent from
/// rendering: bounds/transform with no render projection capability.
pub fn spatial_non_rendered_family() -> EntityStore {
    let mut store = EntityStore::new();

    let trigger = create(
        &mut store,
        1,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_bounds(trigger, Aabb::new(Vec3::ZERO, Vec3::splat(2.0)));
    store.attach_collision(trigger, false);

    let anchor = create(
        &mut store,
        2,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(anchor, EntityTransform::at(Vec3::new(0.0, 1.0, 0.0)));

    let probe = create(
        &mut store,
        3,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_bounds(probe, Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0)));

    store
}

/// Family 3 — non-spatial logical. Proves existence/source/save/replay need no
/// position: no transform, no render, no collider.
pub fn non_spatial_logical_family() -> EntityStore {
    let mut store = EntityStore::new();

    let controller = create(
        &mut store,
        1,
        EntitySource::RuntimeCreated { by: None },
        &[7, 8],
    );
    store.attach_controller(
        controller,
        ControllerCapability::Process(ProcessId::new(42)),
    );

    create(
        &mut store,
        2,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );

    create(
        &mut store,
        3,
        EntitySource::PolicyProposed {
            by: SubjectId::new(5),
        },
        &[],
    );

    store
}

/// Family 4 — contained / inventory-like. Proves containment is not spatial
/// attachment and does not imply a world transform.
pub fn contained_family() -> EntityStore {
    let mut store = EntityStore::new();

    // A logical container (the relation target).
    let container = create(
        &mut store,
        1,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );

    let record = create(
        &mut store,
        2,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_containment(record, container);

    let slot = create(
        &mut store,
        3,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_containment(slot, container);

    let catalog_item = create(
        &mut store,
        4,
        EntitySource::Imported {
            asset: mesh_asset("mesh/logical-item"),
        },
        &[],
    );
    store.attach_containment(catalog_item, container);
    store.attach_asset_binding(catalog_item, mesh_asset("mesh/logical-item"));

    store
}

/// Family 5 — UI / devtools-projected. Proves identity can support UI/devtools
/// projection while being clearly marked `DiagnosticTooling` (not world authority).
pub fn ui_devtools_family() -> EntityStore {
    let mut store = EntityStore::new();

    create(&mut store, 1, EntitySource::DiagnosticTooling, &[]);

    let selection = create(&mut store, 2, EntitySource::DiagnosticTooling, &[]);
    store.attach_transform(selection, EntityTransform::at(Vec3::new(3.0, 3.0, 0.0)));

    let anchor = create(&mut store, 3, EntitySource::DiagnosticTooling, &[]);
    store.attach_render_projection(anchor, false);

    store
}

/// A mixed lifecycle scenario for save/replay: a scene-sourced entity destroyed at
/// runtime (tombstoned), a runtime-created entity that survives, a disabled
/// entity, and a diagnostic-tooling entity (save-excluded by default).
pub fn lifecycle_scenario() -> EntityStore {
    let mut store = EntityStore::new();

    // Scene-sourced, then destroyed → tombstone retained in saves.
    let scene = create(
        &mut store,
        1,
        EntitySource::SceneBootstrap {
            node: SceneNodeId::new(100),
        },
        &[],
    );
    store.attach_transform(scene, EntityTransform::at(Vec3::new(1.0, 0.0, 0.0)));
    store
        .apply(EntityLifecycleCommand::Destroy { id: scene })
        .expect("destroy scene entity");

    // Runtime-created survivor with labels.
    let survivor = create(
        &mut store,
        2,
        EntitySource::RuntimeCreated { by: None },
        &[1, 2, 3],
    );
    store.attach_transform(survivor, EntityTransform::at(Vec3::new(0.0, 5.0, 0.0)));

    // A disabled logical entity.
    let disabled = create(
        &mut store,
        3,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store
        .apply(EntityLifecycleCommand::Disable { id: disabled })
        .expect("disable entity");

    // A diagnostic-tooling entity (excluded from durable saves).
    create(&mut store, 4, EntitySource::DiagnosticTooling, &[]);

    store
}

/// Family 6 — attachment/parenting contrast (design §6). Proves the five relation
/// kinds behave differently: a transform parent/child pair, a containment pair, a
/// source-ancestry pair, and a controller-association pair, all distinct. (Render
/// grouping is projection-only and intentionally not stored as authority.)
pub fn attachment_contrast_family() -> EntityStore {
    use crate::relation::RelationCommand;

    let mut store = EntityStore::new();

    // transform_parent_pair: parent (1) and child (2), both spatial.
    let parent = create(
        &mut store,
        1,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(parent, EntityTransform::at(Vec3::new(10.0, 0.0, 0.0)));
    let child = create(
        &mut store,
        2,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(child, EntityTransform::at(Vec3::new(1.0, 0.0, 0.0)));
    store
        .apply_relation(RelationCommand::AttachTransformParent { child, parent })
        .expect("attach transform parent");

    // containment_pair: container (3) and member (4), member non-spatial.
    let container = create(
        &mut store,
        3,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    let member = create(
        &mut store,
        4,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store
        .apply_relation(RelationCommand::SetContainment { member, container })
        .expect("set containment");

    // source_ancestry_pair: derived (6) traces origin (5); not transform/containment.
    let origin = create(
        &mut store,
        5,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    let derived = create(
        &mut store,
        6,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store
        .apply_relation(RelationCommand::SetDerivedFrom { derived, origin })
        .expect("set derived_from");

    // controller_assoc_pair: entity (7) controlled by a process.
    let controlled = create(
        &mut store,
        7,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_controller(
        controlled,
        ControllerCapability::Process(ProcessId::new(99)),
    );

    store
}

/// Family 7 — kinematic eligible/ineligible (design §6). Proves movement operates
/// only on entities with the required spatial + collision capabilities, and that
/// render and collision are independent. A unit-box collider AABB is attached to
/// spatial obstacles; the obstacle at (2,0,0) blocks a +X mover.
pub fn movement_family() -> EntityStore {
    let mut store = EntityStore::new();
    let unit = Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.5));

    // movable_spatial_entity: transform + (non-static) collider.
    let movable = create(
        &mut store,
        1,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(movable, EntityTransform::at(Vec3::ZERO));
    store.attach_bounds(movable, unit);
    store.attach_collision(movable, false);

    // A static obstacle one step along +X (blocks the mover).
    let obstacle = create(
        &mut store,
        2,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(obstacle, EntityTransform::at(Vec3::new(1.0, 0.0, 0.0)));
    store.attach_bounds(obstacle, unit);
    store.attach_collision(obstacle, true);

    // nonspatial_movement_rejected: logical entity, no transform.
    create(
        &mut store,
        3,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );

    // immovable_spatial_entity: spatial + static collider.
    let immovable = create(
        &mut store,
        4,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(immovable, EntityTransform::at(Vec3::new(5.0, 0.0, 0.0)));
    store.attach_collision(immovable, true);

    // rendered_no_collider_entity: transform + render, but no collider.
    let rendered = create(
        &mut store,
        5,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(rendered, EntityTransform::at(Vec3::new(0.0, 5.0, 0.0)));
    store.attach_render_projection(rendered, true);

    // collider_no_render_entity: transform + collider, no render.
    let collider = create(
        &mut store,
        6,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(collider, EntityTransform::at(Vec3::new(0.0, 0.0, 5.0)));
    store.attach_bounds(collider, unit);
    store.attach_collision(collider, false);

    store
}

/// Family 8 — static room collision. A first-person actor starts inside a simple
/// enclosed room of authority AABB colliders. The fixture is intentionally abstract
/// and mirrors the upstream static-room render proof without using render data as
/// collision truth.
pub fn static_room_collision_family() -> EntityStore {
    let mut store = EntityStore::new();

    let actor = create(
        &mut store,
        1,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(actor, EntityTransform::at(Vec3::new(0.0, 0.0, 0.0)));
    store.attach_bounds(actor, Aabb::new(Vec3::splat(-0.25), Vec3::splat(0.25)));
    store.attach_collision(actor, false);
    store.attach_render_projection(actor, true);

    let wall_bounds = Aabb::new(Vec3::new(-3.0, -1.0, -0.5), Vec3::new(3.0, 2.0, 0.5));
    let north = create(
        &mut store,
        2,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(north, EntityTransform::at(Vec3::new(0.0, 0.0, -2.5)));
    store.attach_bounds(north, wall_bounds);
    store.attach_collision(north, true);

    let south = create(
        &mut store,
        3,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(south, EntityTransform::at(Vec3::new(0.0, 0.0, 2.5)));
    store.attach_bounds(south, wall_bounds);
    store.attach_collision(south, true);

    let side_bounds = Aabb::new(Vec3::new(-0.5, -1.0, -3.0), Vec3::new(0.5, 2.0, 3.0));
    let west = create(
        &mut store,
        4,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(west, EntityTransform::at(Vec3::new(-2.5, 0.0, 0.0)));
    store.attach_bounds(west, side_bounds);
    store.attach_collision(west, true);

    let east = create(
        &mut store,
        5,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store.attach_transform(east, EntityTransform::at(Vec3::new(2.5, 0.0, 0.0)));
    store.attach_bounds(east, side_bounds);
    store.attach_collision(east, true);

    store
}

/// The #2484 mixed-world **save** fixture: one store holding every fixture
/// vocabulary class a runtime session-state snapshot must persist in a single save —
/// a runtime-created spatial rendered entity, a spatial non-rendered collider, a
/// non-spatial logical entity, a containment relation, a transform attachment, an
/// asset-bound import, a source-ancestry trace, a scene-sourced entity whose
/// runtime transform has diverged from its authored origin, and a tombstone. It is
/// the canonical input for the session-state snapshot codec round-trip and the
/// committed equivalence golden.
pub fn mixed_world_save_fixture() -> EntityStore {
    use crate::relation::RelationCommand;

    let mut store = EntityStore::new();

    // 1. scene-sourced spatial rendered entity, runtime transform diverged.
    let scene = create(
        &mut store,
        1,
        EntitySource::SceneBootstrap {
            node: SceneNodeId::new(10),
        },
        &[3, 7],
    );
    store.attach_transform(scene, EntityTransform::at(Vec3::new(4.0, 0.5, -2.0)));
    store.attach_render_projection(scene, true);

    // 2. runtime-created spatial non-rendered collider.
    let collider = create(
        &mut store,
        2,
        EntitySource::RuntimeCreated {
            by: Some(ProcessId::new(9)),
        },
        &[],
    );
    store.attach_transform(collider, EntityTransform::IDENTITY);
    store.attach_bounds(collider, Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0)));
    store.attach_collision(collider, true);

    // 3. non-spatial logical entity (no transform), controller association.
    let logical = create(
        &mut store,
        3,
        EntitySource::PolicyProposed {
            by: SubjectId::new(5),
        },
        &[1],
    );
    store.attach_controller(logical, ControllerCapability::Subject(SubjectId::new(5)));

    // 4. contained member (containment relation into the collider).
    let member = create(
        &mut store,
        4,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store
        .apply_relation(RelationCommand::SetContainment {
            member,
            container: collider,
        })
        .expect("set containment");

    // 5. attached child: transform parent = scene, asset binding, source ancestry.
    let child = create(
        &mut store,
        5,
        EntitySource::Imported {
            asset: mesh_asset("mesh/static-fixture-a"),
        },
        &[],
    );
    store.attach_transform(child, EntityTransform::at(Vec3::new(0.0, 1.0, 0.0)));
    store.attach_asset_binding(child, mesh_asset("mesh/static-fixture-a"));
    store
        .apply_relation(RelationCommand::AttachTransformParent {
            child,
            parent: scene,
        })
        .expect("attach transform parent");
    store
        .apply_relation(RelationCommand::SetDerivedFrom {
            derived: child,
            origin: member,
        })
        .expect("set derived_from");

    // 6. a tombstone: a destroyed runtime entity is retained for replay/diagnostics.
    let removed = create(
        &mut store,
        6,
        EntitySource::RuntimeCreated { by: None },
        &[],
    );
    store
        .apply(EntityLifecycleCommand::Destroy { id: removed })
        .expect("destroy");

    store
}

/// All families plus the lifecycle scenario, labelled for the combined golden.
pub fn all_families() -> Vec<(&'static str, EntityStore)> {
    vec![
        ("spatial_rendered", spatial_rendered_family()),
        ("spatial_non_rendered", spatial_non_rendered_family()),
        ("non_spatial_logical", non_spatial_logical_family()),
        ("contained", contained_family()),
        ("ui_devtools", ui_devtools_family()),
        ("attachment_contrast", attachment_contrast_family()),
        ("movement", movement_family()),
        ("static_room_collision", static_room_collision_family()),
        ("lifecycle_scenario", lifecycle_scenario()),
    ]
}

/// Render every family's deterministic dump into one golden-pinnable string.
pub fn dump_all_families() -> String {
    let mut out = String::new();
    for (index, (name, store)) in all_families().into_iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&format!("# family {name}\n"));
        out.push_str(&store.dump());
    }
    out
}
