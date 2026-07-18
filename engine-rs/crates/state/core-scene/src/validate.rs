//! Flat-canonical scene validation with a classified report.
//!
//! Validation runs on the [`FlatSceneDocument`] (the canonical form). Every
//! failure is a typed [`SceneValidationError`] so a future protocol diagnostic
//! can route on the variant rather than parse prose (scene-capability-01,
//! "Rust validation and error classification").

use std::collections::{BTreeMap, HashMap, HashSet};

use core_assets::AssetKind;
use core_ids::SceneNodeId;

use crate::document::{FlatSceneDocument, SceneNodeKind};
use crate::transform::TransformInvalid;
use crate::SceneLightInvalid;

/// One classified validation failure.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneValidationError {
    /// Two node records share a stable id.
    DuplicateNodeId { id: SceneNodeId },
    /// A record names a parent that is not present in the document.
    UnknownParent {
        node: SceneNodeId,
        parent: SceneNodeId,
    },
    /// The parent pointers form a cycle; `path` lists the ids in cycle order.
    Cycle { path: Vec<SceneNodeId> },
    /// A node's initial transform is invalid.
    InvalidTransform {
        node: SceneNodeId,
        reason: TransformInvalid,
    },
    /// A voxel-volume instance uses a composed transform the shared
    /// render/collision path cannot preserve yet.
    InvalidVoxelVolumeTransform { node: SceneNodeId, reason: String },
    /// A node references an asset of the wrong kind for its variant.
    AssetKindMismatch {
        node: SceneNodeId,
        expected: AssetKind,
        actual: AssetKind,
    },
    /// A stored light has malformed fields or a scaled pose.
    InvalidLight {
        node: SceneNodeId,
        reason: SceneLightInvalid,
    },
    /// Two marker nodes share one durable marker identity.
    DuplicateMarkerId {
        node: SceneNodeId,
        marker_id: String,
    },
    /// A marker node carries malformed typed identity or an old schema.
    InvalidMarker { node: SceneNodeId, reason: String },
    /// Two authored runtime placements share one durable instance identity.
    DuplicateEntityInstanceId {
        node: SceneNodeId,
        instance_id: String,
    },
    /// An entity/prefab placement carries malformed stored binding data.
    InvalidEntityInstance { node: SceneNodeId, reason: String },
    /// More than one scene-wide bootstrap binding node was authored.
    DuplicateBootstrapNode { node: SceneNodeId },
    /// A scene-wide bootstrap binding is malformed or placed spatially.
    InvalidBootstrap { node: SceneNodeId, reason: String },
    /// Two catalog inputs claim the same scene-local binding identity.
    DuplicateCatalogBinding {
        node: SceneNodeId,
        binding_id: String,
    },
}

impl SceneValidationError {
    /// Short, stable label for diagnostics/serialization.
    pub fn label(&self) -> &'static str {
        match self {
            SceneValidationError::DuplicateNodeId { .. } => "duplicate-node-id",
            SceneValidationError::UnknownParent { .. } => "unknown-parent",
            SceneValidationError::Cycle { .. } => "cycle",
            SceneValidationError::InvalidTransform { .. } => "invalid-transform",
            SceneValidationError::InvalidVoxelVolumeTransform { .. } => {
                "invalid-voxel-volume-transform"
            }
            SceneValidationError::AssetKindMismatch { .. } => "asset-kind-mismatch",
            SceneValidationError::InvalidLight { .. } => "invalid-light",
            SceneValidationError::DuplicateMarkerId { .. } => "duplicate-marker-id",
            SceneValidationError::InvalidMarker { .. } => "invalid-marker",
            SceneValidationError::DuplicateEntityInstanceId { .. } => {
                "duplicate-entity-instance-id"
            }
            SceneValidationError::InvalidEntityInstance { .. } => "invalid-entity-instance",
            SceneValidationError::DuplicateBootstrapNode { .. } => "duplicate-bootstrap-node",
            SceneValidationError::InvalidBootstrap { .. } => "invalid-bootstrap",
            SceneValidationError::DuplicateCatalogBinding { .. } => "duplicate-catalog-binding",
        }
    }
}

/// The outcome of validating a document: every error found, not just the first.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneValidationReport {
    pub errors: Vec<SceneValidationError>,
}

impl SceneValidationReport {
    /// `true` if no errors were found.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate a flat scene document, returning every classified error.
pub fn validate(doc: &FlatSceneDocument) -> SceneValidationReport {
    let mut errors = Vec::new();
    let mut instance_ids = HashSet::new();
    let mut marker_ids = HashSet::new();
    let mut bootstrap_seen = false;

    // 1. Duplicate stable ids. `known` is the set of all ids present (used by the
    //    parent/cycle checks below); `seen`/`reported` track duplicates so each
    //    colliding id is reported exactly once.
    let mut known: HashSet<u64> = HashSet::new();
    let mut seen: HashSet<u64> = HashSet::new();
    let mut reported: HashSet<u64> = HashSet::new();
    for rec in &doc.nodes {
        let raw = rec.id.raw();
        known.insert(raw);
        if !seen.insert(raw) && reported.insert(raw) {
            errors.push(SceneValidationError::DuplicateNodeId { id: rec.id });
        }
    }

    // 2. Per-node checks: unknown parent, transform, asset kind.
    for rec in &doc.nodes {
        if let Some(parent) = rec.parent {
            if !known.contains(&parent.raw()) {
                errors.push(SceneValidationError::UnknownParent {
                    node: rec.id,
                    parent,
                });
            }
        }
        if let Err(reason) = rec.transform.validate() {
            errors.push(SceneValidationError::InvalidTransform {
                node: rec.id,
                reason,
            });
        }
        if let (Some(expected), Some(asset)) = (rec.kind.expected_asset_kind(), rec.kind.asset()) {
            if asset.kind() != expected {
                errors.push(SceneValidationError::AssetKindMismatch {
                    node: rec.id,
                    expected,
                    actual: asset.kind(),
                });
            }
        }
        if let SceneNodeKind::Light(light) = &rec.kind {
            let result = if doc.schema_version < 2 || doc.metadata.authoring_format_version < 2 {
                Err(SceneLightInvalid::RequiresSchema2)
            } else if rec.transform.scale != core_math::Vec3::ONE {
                Err(SceneLightInvalid::NonUnitScale)
            } else {
                light.validate()
            };
            if let Err(reason) = result {
                errors.push(SceneValidationError::InvalidLight {
                    node: rec.id,
                    reason,
                });
            }
        }
        match &rec.kind {
            SceneNodeKind::Marker(marker) => {
                if doc.schema_version < 4 || doc.metadata.authoring_format_version < 4 {
                    errors.push(SceneValidationError::InvalidMarker {
                        node: rec.id,
                        reason: "requires-schema-4".into(),
                    });
                }
                if !valid_stable_id(&marker.marker_id) {
                    errors.push(SceneValidationError::InvalidMarker {
                        node: rec.id,
                        reason: "invalid-marker-id".into(),
                    });
                } else if !marker_ids.insert(marker.marker_id.clone()) {
                    errors.push(SceneValidationError::DuplicateMarkerId {
                        node: rec.id,
                        marker_id: marker.marker_id.clone(),
                    });
                }
            }
            SceneNodeKind::EntityInstance(instance) => {
                if doc.schema_version < 3 || doc.metadata.authoring_format_version < 3 {
                    errors.push(SceneValidationError::InvalidEntityInstance {
                        node: rec.id,
                        reason: "requires-schema-3".into(),
                    });
                }
                if !valid_stable_id(&instance.instance_id) {
                    errors.push(SceneValidationError::InvalidEntityInstance {
                        node: rec.id,
                        reason: "invalid-instance-id".into(),
                    });
                } else if !instance_ids.insert(instance.instance_id.clone()) {
                    errors.push(SceneValidationError::DuplicateEntityInstanceId {
                        node: rec.id,
                        instance_id: instance.instance_id.clone(),
                    });
                }
                match &instance.reference {
                    crate::document::SceneEntityReference::EntityDefinition { stable_id } => {
                        if !valid_stable_id(stable_id) {
                            errors.push(SceneValidationError::InvalidEntityInstance {
                                node: rec.id,
                                reason: "invalid-entity-definition-id".into(),
                            });
                        }
                    }
                    crate::document::SceneEntityReference::Prefab {
                        prefab_id,
                        variant_id,
                        ..
                    } => {
                        if doc.schema_version < 4 || doc.metadata.authoring_format_version < 4 {
                            errors.push(SceneValidationError::InvalidEntityInstance {
                                node: rec.id,
                                reason: "prefab-instantiation-seed-requires-schema-4".into(),
                            });
                        }
                        if *prefab_id == 0 {
                            errors.push(SceneValidationError::InvalidEntityInstance {
                                node: rec.id,
                                reason: "invalid-prefab-id".into(),
                            });
                        }
                        if variant_id
                            .as_deref()
                            .is_some_and(|value| !valid_stable_id(value))
                        {
                            errors.push(SceneValidationError::InvalidEntityInstance {
                                node: rec.id,
                                reason: "invalid-prefab-variant-id".into(),
                            });
                        }
                    }
                }
                if instance
                    .spawn_marker_id
                    .as_deref()
                    .is_some_and(|value| !valid_stable_id(value))
                {
                    errors.push(SceneValidationError::InvalidEntityInstance {
                        node: rec.id,
                        reason: "invalid-spawn-marker-id".into(),
                    });
                }
            }
            SceneNodeKind::Bootstrap(bindings) => {
                if bootstrap_seen {
                    errors.push(SceneValidationError::DuplicateBootstrapNode { node: rec.id });
                }
                bootstrap_seen = true;
                if doc.schema_version < 3 || doc.metadata.authoring_format_version < 3 {
                    errors.push(SceneValidationError::InvalidBootstrap {
                        node: rec.id,
                        reason: "requires-schema-3".into(),
                    });
                }
                if rec.parent.is_some() {
                    errors.push(SceneValidationError::InvalidBootstrap {
                        node: rec.id,
                        reason: "bootstrap-must-be-root".into(),
                    });
                }
                if rec.transform != crate::SceneTransform::IDENTITY {
                    errors.push(SceneValidationError::InvalidBootstrap {
                        node: rec.id,
                        reason: "bootstrap-transform-must-be-identity".into(),
                    });
                }
                if let Some(generator) = &bindings.generator {
                    if !valid_stable_id(&generator.provider_id)
                        || !valid_stable_id(&generator.preset_id)
                    {
                        errors.push(SceneValidationError::InvalidBootstrap {
                            node: rec.id,
                            reason: "invalid-generator-binding".into(),
                        });
                    }
                }
                let mut catalog_bindings = HashSet::new();
                for catalog in &bindings.catalogs {
                    if !valid_stable_id(&catalog.binding_id)
                        || !valid_stable_id(&catalog.catalog_id)
                        || !valid_project_relative_path(&catalog.source_path)
                    {
                        errors.push(SceneValidationError::InvalidBootstrap {
                            node: rec.id,
                            reason: "invalid-catalog-binding".into(),
                        });
                    } else if !catalog_bindings.insert(catalog.binding_id.clone()) {
                        errors.push(SceneValidationError::DuplicateCatalogBinding {
                            node: rec.id,
                            binding_id: catalog.binding_id.clone(),
                        });
                    }
                }
            }
            SceneNodeKind::EmptyGroup
            | SceneNodeKind::StaticMesh(_)
            | SceneNodeKind::Sprite(_)
            | SceneNodeKind::VoxelVolume(_)
            | SceneNodeKind::Light(_) => {}
        }
    }

    // 3. Cycles via the parent map. Only meaningful with present parents; an
    //    unknown parent is already reported above and terminates a walk.
    detect_cycles(doc, &known, &mut errors);

    // Stored voxel assets remain in local coordinates. The first shared
    // render/collision admission slice supports composed translation only, so
    // reject transforms that would otherwise drift between those consumers.
    if !errors.iter().any(|error| {
        matches!(
            error,
            SceneValidationError::DuplicateNodeId { .. }
                | SceneValidationError::UnknownParent { .. }
                | SceneValidationError::Cycle { .. }
                | SceneValidationError::InvalidTransform { .. }
        )
    }) {
        let world_transforms = composed_world_transforms(doc);
        for record in &doc.nodes {
            if !matches!(record.kind, SceneNodeKind::VoxelVolume(_)) {
                continue;
            }
            let transform = world_transforms[&record.id.raw()];
            if transform.rotation != crate::Quat::IDENTITY {
                errors.push(SceneValidationError::InvalidVoxelVolumeTransform {
                    node: record.id,
                    reason: "nonIdentityRotation".to_owned(),
                });
            }
            if transform.scale != core_math::Vec3::ONE {
                errors.push(SceneValidationError::InvalidVoxelVolumeTransform {
                    node: record.id,
                    reason: "nonUnitScale".to_owned(),
                });
            }
        }
    }

    SceneValidationReport { errors }
}

/// Resolve canonical authored world transforms for a structurally valid flat
/// document. Callers must validate parent references and cycles first.
pub fn composed_world_transforms(doc: &FlatSceneDocument) -> BTreeMap<u64, crate::SceneTransform> {
    let records = doc
        .nodes
        .iter()
        .map(|record| (record.id.raw(), record))
        .collect::<BTreeMap<_, _>>();
    let mut resolved = BTreeMap::new();
    for record in &doc.nodes {
        resolve_world_transform(record.id.raw(), &records, &mut resolved);
    }
    resolved
}

fn resolve_world_transform(
    node: u64,
    records: &BTreeMap<u64, &crate::SceneNodeRecord>,
    resolved: &mut BTreeMap<u64, crate::SceneTransform>,
) -> crate::SceneTransform {
    if let Some(transform) = resolved.get(&node) {
        return *transform;
    }
    let record = records[&node];
    let world = record.parent.map_or(record.transform, |parent| {
        resolve_world_transform(parent.raw(), records, resolved).compose(record.transform)
    });
    resolved.insert(node, world);
    world
}

fn valid_stable_id(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':' | b'@')
        })
}

fn valid_project_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !value.starts_with('/')
        && !value.contains('\\')
        && value
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "..")
}

fn detect_cycles(
    doc: &FlatSceneDocument,
    known: &HashSet<u64>,
    errors: &mut Vec<SceneValidationError>,
) {
    // Last-wins parent map (duplicate ids are reported separately).
    let mut parent_of: HashMap<u64, Option<u64>> = HashMap::new();
    let mut id_of: HashMap<u64, SceneNodeId> = HashMap::new();
    for rec in &doc.nodes {
        parent_of.insert(rec.id.raw(), rec.parent.map(|p| p.raw()));
        id_of.insert(rec.id.raw(), rec.id);
    }

    let mut acyclic: HashSet<u64> = HashSet::new();
    let mut cyclic: HashSet<u64> = HashSet::new();

    // Walk starts in ascending id order so any reported cycle path is
    // deterministic regardless of hash-map iteration order.
    let mut starts: Vec<u64> = parent_of.keys().copied().collect();
    starts.sort_unstable();

    for start in starts {
        if acyclic.contains(&start) || cyclic.contains(&start) {
            continue;
        }
        let mut order: Vec<u64> = Vec::new();
        let mut local: HashSet<u64> = HashSet::new();
        let mut cur = start;
        loop {
            if cyclic.contains(&cur) {
                break;
            }
            if acyclic.contains(&cur) {
                acyclic.extend(order.iter().copied());
                break;
            }
            if local.contains(&cur) {
                // Cycle: from the first occurrence of `cur` to the end.
                let pos = order.iter().position(|&x| x == cur).unwrap();
                let path: Vec<SceneNodeId> = order[pos..].iter().map(|raw| id_of[raw]).collect();
                for raw in &order[pos..] {
                    cyclic.insert(*raw);
                }
                errors.push(SceneValidationError::Cycle { path });
                break;
            }
            local.insert(cur);
            order.push(cur);
            match parent_of.get(&cur).copied().flatten() {
                None => {
                    acyclic.extend(order.iter().copied());
                    break;
                }
                Some(p) => {
                    if !known.contains(&p) {
                        // Unknown parent: not a cycle, already reported.
                        acyclic.extend(order.iter().copied());
                        break;
                    }
                    cur = p;
                }
            }
        }
    }
}
