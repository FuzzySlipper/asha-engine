use std::collections::BTreeMap;

use core_scene::FlatSceneDocument;
use protocol_voxel_asset::VoxelVolumeAsset;
use svc_serialization::BundleHash;

use crate::{
    GameplayRuntimeHost, GameplayRuntimeHostError, GameplayRuntimeProjectInput,
    ValidatedRuntimeProjectAdmission,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayRuntimeActivatedProjectIdentity {
    project_id: u64,
    manifest_hash: BundleHash,
    admission_hash: String,
}

impl GameplayRuntimeActivatedProjectIdentity {
    pub fn project_id(&self) -> u64 {
        self.project_id
    }

    pub fn manifest_hash(&self) -> BundleHash {
        self.manifest_hash
    }

    pub fn admission_hash(&self) -> &str {
        &self.admission_hash
    }
}

pub(super) struct ValidatedRuntimeProjectState {
    identity: GameplayRuntimeActivatedProjectIdentity,
    entry_scene: FlatSceneDocument,
    voxel_assets: BTreeMap<String, VoxelVolumeAsset>,
}

impl GameplayRuntimeHost {
    /// Activate only from the opaque compiler/linker artifact. Stored source,
    /// provider topology, prefab placement, and gameplay inputs remain private
    /// to the linker and cannot be substituted between validation and commit.
    pub fn activate_validated_project(
        admission: ValidatedRuntimeProjectAdmission,
    ) -> Result<Self, GameplayRuntimeHostError> {
        let parts = admission.into_activation_parts();
        let state = ValidatedRuntimeProjectState {
            identity: GameplayRuntimeActivatedProjectIdentity {
                project_id: parts.project_id,
                manifest_hash: parts.manifest_hash,
                admission_hash: parts.admission_hash,
            },
            entry_scene: parts.entry_scene,
            voxel_assets: parts.voxel_assets,
        };
        let mut host = Self::activate_project_with_prefabs(
            GameplayRuntimeProjectInput {
                load_plan: parts.load_plan,
                artifacts: parts.artifacts,
                bootstrap_resolution: parts.bootstrap_resolution,
                composition: parts.composition,
                composition_requirement: None,
                bindings: parts.bindings,
                entity_targets: parts.entity_targets,
                spatial_entities: parts.spatial_entities,
                declared_reads: parts.declared_reads,
                triggers: parts.triggers,
                scheduler: parts.scheduler,
            },
            parts.prefabs,
        )?;
        host.activated_project = Some(state);
        Ok(host)
    }

    pub fn activated_project_identity(&self) -> Option<&GameplayRuntimeActivatedProjectIdentity> {
        self.activated_project
            .as_ref()
            .map(|project| &project.identity)
    }

    #[doc(hidden)]
    pub fn activated_entry_scene(&self) -> Option<&FlatSceneDocument> {
        self.activated_project
            .as_ref()
            .map(|project| &project.entry_scene)
    }

    #[doc(hidden)]
    pub fn take_activated_voxel_assets(&mut self) -> BTreeMap<String, VoxelVolumeAsset> {
        self.activated_project
            .as_mut()
            .map(|project| core::mem::take(&mut project.voxel_assets))
            .unwrap_or_default()
    }
}
