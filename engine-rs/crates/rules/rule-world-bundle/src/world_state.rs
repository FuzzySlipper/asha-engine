//! Runtime world-state snapshot **save composition** (post-launchable-02, #2484).
//!
//! The bootstrap scene document plus the voxel edit log/snapshots already persist
//! the scene-authored baseline and voxel authority. This module composes the third
//! durable artifact a world bundle may carry: the **runtime-diverged entity
//! authority** — runtime-created entities, diverged transforms, capability tables,
//! relations, and source traces — encoded with `core_entity`'s snapshot codec.
//!
//! The artifact is only emitted when runtime state has actually diverged from the
//! bootstrapped scene baseline ([`runtime_diverged`]); an unchanged world saves no
//! redundant snapshot and reloads purely from its scene document. The voxel
//! edit-log/snapshot model is untouched — this is an additional durable artifact,
//! not a replacement.

use core_entity::{encode_snapshot, EntityHash, EntitySnapshot};
use svc_serialization::{ArtifactEntry, ArtifactRole};

/// Canonical bundle-relative path for the runtime world-state snapshot artifact.
pub const WORLD_STATE_SNAPSHOT_PATH: &str = "world/state.snapshot.json";

/// A composed runtime world-state snapshot artifact: its manifest entry (durable,
/// content-hashed, classified `worldStateSnapshot`) plus the bytes to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldStateArtifact {
    /// The classified manifest row (`durable` / `worldStateSnapshot`, hashed).
    pub entry: ArtifactEntry,
    /// The canonical encoded snapshot bytes the entry hashes.
    pub text: String,
}

/// `true` when the runtime entity authority has diverged from the bootstrapped
/// scene baseline and therefore must be persisted as a world-state snapshot. Both
/// hashes are the capability/relation-aware [`EntityHash`], so this compares the
/// full authority the snapshot would carry — not just the spatial transform view.
pub fn runtime_diverged(bootstrap_baseline: EntityHash, runtime: EntityHash) -> bool {
    bootstrap_baseline != runtime
}

/// Compose the durable world-state snapshot artifact from a runtime entity
/// snapshot. The bytes are the canonical `core_entity` encoding and the manifest
/// entry carries their content hash for drift/round-trip diagnostics.
pub fn compose_world_state_snapshot(runtime: &EntitySnapshot) -> WorldStateArtifact {
    let text = encode_snapshot(runtime);
    let entry = ArtifactEntry::durable(
        WORLD_STATE_SNAPSHOT_PATH,
        ArtifactRole::WorldStateSnapshot,
        text.as_bytes(),
    );
    WorldStateArtifact { entry, text }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_entity::{EntityLifecycleCommand, EntitySource, EntityStore, EntityTransform};
    use core_ids::EntityId;

    fn add_runtime_entity(store: &mut EntityStore, id: u64) {
        let id = EntityId::new(id);
        store
            .apply(EntityLifecycleCommand::Create {
                id,
                source: EntitySource::RuntimeCreated { by: None },
                labels: vec![],
            })
            .unwrap();
        store.attach_transform(id, EntityTransform::IDENTITY);
    }

    #[test]
    fn divergence_is_detected_by_capability_aware_hash() {
        let baseline = EntityStore::new();
        let mut runtime = EntityStore::new();
        assert!(!runtime_diverged(baseline.hash(), runtime.hash()));
        add_runtime_entity(&mut runtime, 1);
        assert!(runtime_diverged(baseline.hash(), runtime.hash()));
    }

    #[test]
    fn composed_artifact_is_durable_hashed_and_classified() {
        let mut runtime = EntityStore::new();
        add_runtime_entity(&mut runtime, 1);
        let artifact = compose_world_state_snapshot(&runtime.snapshot_durable());
        assert_eq!(artifact.entry.role, ArtifactRole::WorldStateSnapshot);
        assert!(artifact.entry.content_hash.is_some());
        assert_eq!(artifact.entry.path, WORLD_STATE_SNAPSHOT_PATH);
        // The entry's hash is over exactly the bytes we would write.
        assert_eq!(
            artifact.entry.content_hash,
            Some(svc_serialization::BundleHash::of(artifact.text.as_bytes()))
        );
    }
}
