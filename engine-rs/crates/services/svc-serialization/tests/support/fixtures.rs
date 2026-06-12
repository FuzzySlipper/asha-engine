//! Shared sample manifest for the golden drift test and the regenerator example.
//! Abstract fixture nouns only (no product-domain content).

use core_ids::{SceneId, WorldId};
use svc_serialization::{
    ArtifactEntry, ArtifactRole, AssetLockSection, GeneratorMetadata, SceneSection,
    WorldBundleManifest, WorldSection, BUNDLE_SCHEMA_VERSION, SUPPORTED_PROTOCOL_VERSION,
};

/// A minimal but representative world-bundle manifest: durable scene + asset lock,
/// a durable voxel edit log, a generated chunk snapshot, and a disposable cache.
pub fn sample_manifest() -> WorldBundleManifest {
    WorldBundleManifest {
        bundle_schema_version: BUNDLE_SCHEMA_VERSION,
        protocol_version: SUPPORTED_PROTOCOL_VERSION,
        world: WorldSection {
            id: WorldId::new(7),
            name: Some("sample-world".into()),
        },
        scene: SceneSection {
            id: SceneId::new(100),
            schema_version: 1,
            artifact: "scene/scene.json".into(),
        },
        asset_lock: AssetLockSection {
            artifact: "assets/lock.json".into(),
            asset_count: 1,
        },
        generator: GeneratorMetadata {
            seed: 42,
            version: 1,
            params: "default".into(),
        },
        artifacts: vec![
            ArtifactEntry::durable(
                "scene/scene.json",
                ArtifactRole::SceneDocument,
                b"scene-bytes",
            ),
            ArtifactEntry::durable("assets/lock.json", ArtifactRole::AssetLock, b"lock-bytes"),
            ArtifactEntry::durable("voxel/edits.log", ArtifactRole::VoxelEditLog, b"edit-bytes"),
            ArtifactEntry::generated(
                "voxel/chunk_0_0_0.snapshot",
                ArtifactRole::VoxelChunkSnapshot,
                b"chunk-bytes",
            ),
            ArtifactEntry::cache("cache/mesh_0_0_0.bin", ArtifactRole::Cache),
        ],
    }
}
