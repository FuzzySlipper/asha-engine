use core_assets::{AssetReference, AssetVersionReq};
use protocol_scene::{
    AssetReferenceDto, AssetVersionReqDto, FlatSceneDocumentDto, SceneBootstrapBindingsDto,
    SceneCatalogBindingDto, SceneEntityInstanceDto, SceneEntityReferenceDto,
    SceneGeneratorBindingDto, SceneLightDto, SceneLightShadowIntentDto, SceneMetadataDto,
    SceneNodeKindDto, SceneNodeRecordDto, SceneTransformDto,
};

/// Renderer-neutral projection of canonical scene authority used by the
/// project-content linker. Keeping this in a Rust service avoids making the
/// protocol DTO or a bridge callback into runtime admission authority.
pub fn project_scene_document_dto(
    document: &core_scene::FlatSceneDocument,
) -> FlatSceneDocumentDto {
    let document = document.canonical();
    FlatSceneDocumentDto {
        schema_version: document.schema_version,
        id: document.id,
        metadata: SceneMetadataDto {
            name: document.metadata.name,
            authoring_format_version: document.metadata.authoring_format_version,
        },
        dependencies: document.dependencies.iter().map(asset_dto).collect(),
        nodes: document
            .nodes
            .into_iter()
            .map(|record| SceneNodeRecordDto {
                id: record.id,
                parent: record.parent,
                child_order: record.child_order,
                label: record.metadata.label,
                tags: record.metadata.tags,
                transform: SceneTransformDto {
                    translation: record.transform.translation.to_array(),
                    rotation: [
                        record.transform.rotation.x,
                        record.transform.rotation.y,
                        record.transform.rotation.z,
                        record.transform.rotation.w,
                    ],
                    scale: record.transform.scale.to_array(),
                },
                kind: match record.kind {
                    core_scene::SceneNodeKind::EmptyGroup => SceneNodeKindDto::EmptyGroup,
                    core_scene::SceneNodeKind::StaticMesh(asset) => {
                        SceneNodeKindDto::StaticMesh(asset_dto(&asset))
                    }
                    core_scene::SceneNodeKind::Sprite(asset) => {
                        SceneNodeKindDto::Sprite(asset_dto(&asset))
                    }
                    core_scene::SceneNodeKind::VoxelVolume(asset) => {
                        SceneNodeKindDto::VoxelVolume(asset_dto(&asset))
                    }
                    core_scene::SceneNodeKind::Light(light) => {
                        SceneNodeKindDto::Light(light_dto(light))
                    }
                    core_scene::SceneNodeKind::Marker(marker) => SceneNodeKindDto::Marker {
                        marker_id: marker.marker_id,
                    },
                    core_scene::SceneNodeKind::EntityInstance(instance) => {
                        SceneNodeKindDto::EntityInstance {
                            instance: SceneEntityInstanceDto {
                                instance_id: instance.instance_id,
                                reference: match instance.reference {
                                    core_scene::SceneEntityReference::EntityDefinition {
                                        stable_id,
                                    } => SceneEntityReferenceDto::EntityDefinition { stable_id },
                                    core_scene::SceneEntityReference::Prefab {
                                        prefab_id,
                                        variant_id,
                                        instantiation_seed,
                                    } => SceneEntityReferenceDto::Prefab {
                                        prefab_id,
                                        variant_id,
                                        instantiation_seed,
                                    },
                                },
                                spawn_marker_id: instance.spawn_marker_id,
                            },
                        }
                    }
                    core_scene::SceneNodeKind::Bootstrap(bindings) => SceneNodeKindDto::Bootstrap {
                        bindings: SceneBootstrapBindingsDto {
                            generator: bindings.generator.map(|generator| {
                                SceneGeneratorBindingDto {
                                    provider_id: generator.provider_id,
                                    preset_id: generator.preset_id,
                                    seed: generator.seed,
                                }
                            }),
                            catalogs: bindings
                                .catalogs
                                .into_iter()
                                .map(|catalog| SceneCatalogBindingDto {
                                    binding_id: catalog.binding_id,
                                    catalog_id: catalog.catalog_id,
                                    source_path: catalog.source_path,
                                })
                                .collect(),
                        },
                    },
                },
            })
            .collect(),
    }
}

fn asset_dto(asset: &AssetReference) -> AssetReferenceDto {
    AssetReferenceDto {
        id: asset.id().as_str().to_owned(),
        version: match asset.version() {
            AssetVersionReq::Any => AssetVersionReqDto::Any,
            AssetVersionReq::Exact(value) => AssetVersionReqDto::Exact(value),
            AssetVersionReq::AtLeast(value) => AssetVersionReqDto::AtLeast(value),
        },
        hash: asset.hash().map(|hash| hash.as_str().to_owned()),
    }
}

fn light_dto(light: core_scene::SceneLight) -> SceneLightDto {
    let shadow = |intent| match intent {
        core_scene::SceneLightShadowIntent::Disabled => SceneLightShadowIntentDto::Disabled,
        core_scene::SceneLightShadowIntent::Requested => SceneLightShadowIntentDto::Requested,
    };
    match light {
        core_scene::SceneLight::Ambient {
            color,
            intensity,
            enabled,
            shadow_intent,
        } => SceneLightDto::Ambient {
            color,
            intensity,
            enabled,
            shadow_intent: shadow(shadow_intent),
        },
        core_scene::SceneLight::Directional {
            color,
            intensity,
            enabled,
            shadow_intent,
        } => SceneLightDto::Directional {
            color,
            intensity,
            enabled,
            shadow_intent: shadow(shadow_intent),
        },
        core_scene::SceneLight::Point {
            color,
            intensity,
            enabled,
            range,
            decay,
            shadow_intent,
        } => SceneLightDto::Point {
            color,
            intensity,
            enabled,
            range,
            decay,
            shadow_intent: shadow(shadow_intent),
        },
        core_scene::SceneLight::Spot {
            color,
            intensity,
            enabled,
            range,
            decay,
            outer_angle_radians,
            penumbra,
            shadow_intent,
        } => SceneLightDto::Spot {
            color,
            intensity,
            enabled,
            range,
            decay,
            outer_angle_radians,
            penumbra,
            shadow_intent: shadow(shadow_intent),
        },
    }
}
