//! Canonical scene-object read model and typed hierarchy command validation.
//!
//! This module is the authority-side counterpart to Studio's hierarchy panel:
//! scene objects are authored scene nodes, not render handles or runtime
//! entities. Every mutation is an explicit command over the canonical flat scene
//! document and is validated before and after application.

use std::collections::BTreeSet;

use core_assets::AssetReference;
use core_ids::SceneNodeId;

use crate::document::{FlatSceneDocument, SceneNodeKind, SceneNodeRecord};
use crate::json::encode;
use crate::validate::{validate, SceneValidationError};
use crate::{SceneLight, SceneTransform};

/// Stable fingerprint for a canonical scene document snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SceneObjectSnapshotHash(pub u64);

/// One scene object projected from the canonical flat document.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObjectRecord {
    pub id: SceneNodeId,
    pub parent: Option<SceneNodeId>,
    pub child_order: u32,
    pub label: Option<String>,
    pub kind: &'static str,
    pub has_renderable_asset: bool,
}

/// Deterministic read model for scene hierarchy consumers.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObjectSnapshot {
    pub document_hash: SceneObjectSnapshotHash,
    pub objects: Vec<SceneObjectRecord>,
}

/// Explicit scene hierarchy commands. Selection is modeled here so GUI and
/// agent surfaces can use the same command identity even though it does not
/// mutate the authored document.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneObjectCommand {
    Create {
        record: SceneNodeRecord,
    },
    Delete {
        id: SceneNodeId,
    },
    Rename {
        id: SceneNodeId,
        label: Option<String>,
    },
    Reparent {
        id: SceneNodeId,
        parent: Option<SceneNodeId>,
        child_order: u32,
    },
    UpdateLight {
        id: SceneNodeId,
        light: SceneLight,
    },
    SetTransform {
        id: SceneNodeId,
        transform: SceneTransform,
    },
    RetargetVoxelAsset {
        id: SceneNodeId,
        asset: AssetReference,
        tags: Vec<String>,
    },
    Select {
        id: Option<SceneNodeId>,
    },
}

/// A successfully applied scene-object command.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObjectCommandOutcome {
    pub document: FlatSceneDocument,
    pub snapshot: SceneObjectSnapshot,
    pub selected: Option<SceneNodeId>,
}

/// Classified command rejection.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneObjectCommandRejection {
    StaleSnapshot {
        expected: SceneObjectSnapshotHash,
        actual: SceneObjectSnapshotHash,
    },
    InvalidBefore {
        errors: Vec<SceneValidationError>,
    },
    InvalidAfter {
        errors: Vec<SceneValidationError>,
    },
    MissingObject {
        id: SceneNodeId,
    },
    DuplicateObject {
        id: SceneNodeId,
    },
    MissingParent {
        id: SceneNodeId,
        parent: SceneNodeId,
    },
    SelfParent {
        id: SceneNodeId,
    },
    BlankLabel {
        id: SceneNodeId,
    },
    WrongObjectKind {
        id: SceneNodeId,
    },
}

impl SceneObjectCommandRejection {
    /// Stable diagnostic code for protocol/UI surfaces.
    pub fn label(&self) -> &'static str {
        match self {
            SceneObjectCommandRejection::StaleSnapshot { .. } => "stale-scene-object-snapshot",
            SceneObjectCommandRejection::InvalidBefore { .. } => "invalid-scene-before-command",
            SceneObjectCommandRejection::InvalidAfter { .. } => "invalid-scene-after-command",
            SceneObjectCommandRejection::MissingObject { .. } => "missing-scene-object",
            SceneObjectCommandRejection::DuplicateObject { .. } => "duplicate-scene-object",
            SceneObjectCommandRejection::MissingParent { .. } => "missing-scene-object-parent",
            SceneObjectCommandRejection::SelfParent { .. } => "scene-object-self-parent",
            SceneObjectCommandRejection::BlankLabel { .. } => "blank-scene-object-label",
            SceneObjectCommandRejection::WrongObjectKind { .. } => "invalid-scene-object-kind",
        }
    }
}

/// Build the deterministic scene-object snapshot for a document.
pub fn scene_object_snapshot(doc: &FlatSceneDocument) -> SceneObjectSnapshot {
    let canonical = doc.canonical();
    SceneObjectSnapshot {
        document_hash: document_hash(&canonical),
        objects: canonical
            .nodes
            .iter()
            .map(|node| SceneObjectRecord {
                id: node.id,
                parent: node.parent,
                child_order: node.child_order,
                label: node.metadata.label.clone(),
                kind: node.kind.tag(),
                has_renderable_asset: node.kind.asset().is_some(),
            })
            .collect(),
    }
}

/// Apply a typed scene-object command to a flat document after verifying the
/// caller's expected snapshot hash.
pub fn apply_scene_object_command(
    doc: &FlatSceneDocument,
    expected_hash: SceneObjectSnapshotHash,
    command: SceneObjectCommand,
) -> Result<SceneObjectCommandOutcome, SceneObjectCommandRejection> {
    let actual_hash = scene_object_snapshot(doc).document_hash;
    if actual_hash != expected_hash {
        return Err(SceneObjectCommandRejection::StaleSnapshot {
            expected: expected_hash,
            actual: actual_hash,
        });
    }

    let before = validate(doc);
    if !before.is_ok() {
        return Err(SceneObjectCommandRejection::InvalidBefore {
            errors: before.errors,
        });
    }

    let mut next = doc.canonical();
    let mut reconcile_asset_dependencies = false;
    let selected = match command {
        SceneObjectCommand::Create { record } => {
            if contains_node(&next, record.id) {
                return Err(SceneObjectCommandRejection::DuplicateObject { id: record.id });
            }
            if let Some(parent) = record.parent {
                if !contains_node(&next, parent) {
                    return Err(SceneObjectCommandRejection::MissingParent {
                        id: record.id,
                        parent,
                    });
                }
                if parent == record.id {
                    return Err(SceneObjectCommandRejection::SelfParent { id: record.id });
                }
            }
            require_non_blank_label(record.id, record.metadata.label.as_deref())?;
            if matches!(record.kind, SceneNodeKind::Light(_)) {
                next.schema_version = next.schema_version.max(2);
                next.metadata.authoring_format_version =
                    next.metadata.authoring_format_version.max(2);
            }
            next.nodes.push(record);
            reconcile_asset_dependencies = true;
            None
        }
        SceneObjectCommand::Delete { id } => {
            if !contains_node(&next, id) {
                return Err(SceneObjectCommandRejection::MissingObject { id });
            }
            delete_subtree(&mut next, id);
            reconcile_asset_dependencies = true;
            None
        }
        SceneObjectCommand::Rename { id, label } => {
            require_non_blank_label(id, label.as_deref())?;
            let node = find_node_mut(&mut next, id)
                .ok_or(SceneObjectCommandRejection::MissingObject { id })?;
            node.metadata.label = label;
            Some(id)
        }
        SceneObjectCommand::Reparent {
            id,
            parent,
            child_order,
        } => {
            if let Some(parent) = parent {
                if parent == id {
                    return Err(SceneObjectCommandRejection::SelfParent { id });
                }
                if !contains_node(&next, parent) {
                    return Err(SceneObjectCommandRejection::MissingParent { id, parent });
                }
            }
            let node = find_node_mut(&mut next, id)
                .ok_or(SceneObjectCommandRejection::MissingObject { id })?;
            node.parent = parent;
            node.child_order = child_order;
            Some(id)
        }
        SceneObjectCommand::UpdateLight { id, light } => {
            {
                let node = find_node_mut(&mut next, id)
                    .ok_or(SceneObjectCommandRejection::MissingObject { id })?;
                if !matches!(node.kind, SceneNodeKind::Light(_)) {
                    return Err(SceneObjectCommandRejection::WrongObjectKind { id });
                }
                node.kind = SceneNodeKind::Light(light);
            }
            next.schema_version = next.schema_version.max(2);
            next.metadata.authoring_format_version = next.metadata.authoring_format_version.max(2);
            Some(id)
        }
        SceneObjectCommand::SetTransform { id, transform } => {
            let node = find_node_mut(&mut next, id)
                .ok_or(SceneObjectCommandRejection::MissingObject { id })?;
            node.transform = transform;
            Some(id)
        }
        SceneObjectCommand::RetargetVoxelAsset { id, asset, tags } => {
            let node = find_node_mut(&mut next, id)
                .ok_or(SceneObjectCommandRejection::MissingObject { id })?;
            if !matches!(node.kind, SceneNodeKind::VoxelVolume(_)) {
                return Err(SceneObjectCommandRejection::WrongObjectKind { id });
            }
            node.kind = SceneNodeKind::VoxelVolume(asset);
            node.metadata.tags = tags;
            reconcile_asset_dependencies = true;
            Some(id)
        }
        SceneObjectCommand::Select { id } => {
            if let Some(id) = id {
                if !contains_node(&next, id) {
                    return Err(SceneObjectCommandRejection::MissingObject { id });
                }
            }
            id
        }
    };

    if reconcile_asset_dependencies {
        reconcile_dependencies(&mut next);
    }
    next.canonicalize();
    let after = validate(&next);
    if !after.is_ok() {
        return Err(SceneObjectCommandRejection::InvalidAfter {
            errors: after.errors,
        });
    }

    Ok(SceneObjectCommandOutcome {
        snapshot: scene_object_snapshot(&next),
        document: next,
        selected,
    })
}

fn reconcile_dependencies(doc: &mut FlatSceneDocument) {
    let mut dependencies = Vec::new();
    for node in &doc.nodes {
        let Some(asset) = node.kind.asset() else {
            continue;
        };
        if dependencies
            .iter()
            .any(|existing: &AssetReference| existing.id() == asset.id())
        {
            continue;
        }
        dependencies.push(asset.clone());
    }
    doc.dependencies = dependencies;
}

fn contains_node(doc: &FlatSceneDocument, id: SceneNodeId) -> bool {
    doc.nodes.iter().any(|node| node.id == id)
}

fn find_node_mut(doc: &mut FlatSceneDocument, id: SceneNodeId) -> Option<&mut SceneNodeRecord> {
    doc.nodes.iter_mut().find(|node| node.id == id)
}

fn require_non_blank_label(
    id: SceneNodeId,
    label: Option<&str>,
) -> Result<(), SceneObjectCommandRejection> {
    if matches!(label, Some(label) if label.trim().is_empty()) {
        return Err(SceneObjectCommandRejection::BlankLabel { id });
    }
    Ok(())
}

fn delete_subtree(doc: &mut FlatSceneDocument, root: SceneNodeId) {
    let mut doomed = BTreeSet::from([root.raw()]);
    loop {
        let before = doomed.len();
        for node in &doc.nodes {
            if let Some(parent) = node.parent {
                if doomed.contains(&parent.raw()) {
                    doomed.insert(node.id.raw());
                }
            }
        }
        if doomed.len() == before {
            break;
        }
    }
    doc.nodes.retain(|node| !doomed.contains(&node.id.raw()));
}

fn document_hash(doc: &FlatSceneDocument) -> SceneObjectSnapshotHash {
    let mut h = Fnv1a::new();
    for byte in encode(doc).as_bytes() {
        h.write_u8(*byte);
    }
    SceneObjectSnapshotHash(h.finish())
}

const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
const FNV_PRIME: u64 = 1_099_511_628_211;

struct Fnv1a(u64);

impl Fnv1a {
    fn new() -> Self {
        Fnv1a(FNV_OFFSET)
    }

    fn write_u8(&mut self, b: u8) {
        self.0 ^= b as u64;
        self.0 = self.0.wrapping_mul(FNV_PRIME);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
