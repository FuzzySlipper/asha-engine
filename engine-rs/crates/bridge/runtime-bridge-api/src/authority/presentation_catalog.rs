use super::*;

pub(super) const PRIMARY_FIRE_PRESENTATION_SIGNAL: &str = "fps.primary-fire.accepted";
pub(super) const PRIMARY_FIRE_ANIMATION_CUE: &str = "fps.primary-fire.animation";

#[derive(Debug, Clone)]
pub(super) struct InstalledPresentationCue {
    pub asset_id: String,
    pub content_hash: String,
    pub value: f32,
}

#[derive(Debug, Clone)]
pub(super) struct InstalledAnimationCue {
    pub asset_id: String,
    pub clip_ids: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct InstalledPresentationCatalog {
    catalog: Catalog,
    animated_meshes: BTreeMap<String, protocol_render::AnimatedMeshAsset>,
    audio_signals: BTreeMap<String, InstalledPresentationCue>,
    particle_signals: BTreeMap<String, InstalledPresentationCue>,
    animation_cues: BTreeMap<String, InstalledAnimationCue>,
}

impl InstalledPresentationCatalog {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_documents(documents: &[ProjectContentDocumentDto]) -> Result<Self, String> {
        let catalog_entries = documents
            .iter()
            .filter_map(|document| match document {
                ProjectContentDocumentDto::AssetCatalog { catalog, .. } => Some(catalog),
                _ => None,
            })
            .flat_map(|catalog| &catalog.entries)
            .map(|entry| (entry.id.as_str(), entry))
            .collect::<BTreeMap<_, _>>();
        let resources = documents
            .iter()
            .filter_map(|document| match document {
                ProjectContentDocumentDto::PresentationCatalog { catalog, .. } => Some(catalog),
                _ => None,
            })
            .flat_map(|catalog| &catalog.resources)
            .map(|resource| (resource.resource_id.as_str(), resource))
            .collect::<BTreeMap<_, _>>();

        let mut projected_entries = Vec::with_capacity(resources.len());
        for resource in resources.values() {
            let stored = catalog_entries
                .get(resource.asset_id.as_str())
                .ok_or_else(|| {
                    format!(
                        "presentation resource `{}` has no admitted asset catalog entry",
                        resource.resource_id
                    )
                })?;
            let asset_id = AssetId::parse(&resource.asset_id).map_err(|error| error.to_string())?;
            let hash =
                AssetHash::parse(&resource.content_hash).map_err(|error| error.to_string())?;
            projected_entries.push(CatalogEntry::new(asset_id, stored.version).with_hash(hash));
        }

        let mut installed = Self {
            catalog: Catalog::from_entries(projected_entries),
            animated_meshes: BTreeMap::new(),
            audio_signals: BTreeMap::new(),
            particle_signals: BTreeMap::new(),
            animation_cues: BTreeMap::new(),
        };
        for catalog in documents.iter().filter_map(|document| match document {
            ProjectContentDocumentDto::PresentationCatalog { catalog, .. } => Some(catalog),
            _ => None,
        }) {
            for resource in &catalog.resources {
                if let Some(animated_mesh) = &resource.animated_mesh {
                    installed.animated_meshes.insert(
                        resource.resource_id.clone(),
                        live_animated_mesh_descriptor(animated_mesh),
                    );
                }
            }
            for cue in &catalog.cues {
                if let ProjectPresentationCueDto::Animation {
                    cue_id,
                    resource_id,
                    ..
                } = cue
                {
                    let resource = resources.get(resource_id.as_str()).ok_or_else(|| {
                        format!("presentation cue references unknown resource `{resource_id}`")
                    })?;
                    let installed_cue = InstalledAnimationCue {
                        asset_id: resource.asset_id.clone(),
                        clip_ids: resource
                            .animated_mesh
                            .as_ref()
                            .expect("validated animation resource has a descriptor")
                            .clips
                            .iter()
                            .map(|clip| clip.id.clone())
                            .collect(),
                    };
                    if installed
                        .animation_cues
                        .insert(cue_id.clone(), installed_cue)
                        .is_some()
                    {
                        return Err(format!(
                            "presentation animation cue `{cue_id}` is bound more than once"
                        ));
                    }
                    continue;
                }
                let (signals, signal_id, resource_id, value) = match cue {
                    ProjectPresentationCueDto::Audio {
                        signal_id,
                        resource_id,
                        gain,
                        ..
                    } => (&mut installed.audio_signals, signal_id, resource_id, *gain),
                    ProjectPresentationCueDto::Particle {
                        signal_id,
                        resource_id,
                        scale,
                        ..
                    } => (
                        &mut installed.particle_signals,
                        signal_id,
                        resource_id,
                        *scale,
                    ),
                    _ => continue,
                };
                let resource = resources.get(resource_id.as_str()).ok_or_else(|| {
                    format!("presentation cue references unknown resource `{resource_id}`")
                })?;
                let installed_cue = InstalledPresentationCue {
                    asset_id: resource.asset_id.clone(),
                    content_hash: resource.content_hash.clone(),
                    value,
                };
                if signals.insert(signal_id.clone(), installed_cue).is_some() {
                    return Err(format!(
                        "presentation signal `{signal_id}` is bound more than once in its domain"
                    ));
                }
            }
        }
        Ok(installed)
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn audio(&self, signal_id: &str) -> Option<&InstalledPresentationCue> {
        self.audio_signals.get(signal_id)
    }

    pub fn particle(&self, signal_id: &str) -> Option<&InstalledPresentationCue> {
        self.particle_signals.get(signal_id)
    }

    pub fn animation(&self, cue_id: &str) -> Option<&InstalledAnimationCue> {
        self.animation_cues.get(cue_id)
    }

    pub fn animated_mesh(&self, resource_id: &str) -> Option<&protocol_render::AnimatedMeshAsset> {
        self.animated_meshes.get(resource_id)
    }
}

fn live_animated_mesh_descriptor(
    descriptor: &ProjectAnimatedMeshDescriptorDto,
) -> protocol_render::AnimatedMeshAsset {
    protocol_render::AnimatedMeshAsset {
        asset: descriptor.asset.clone(),
        runtime_format: match descriptor.runtime_format {
            ProjectAnimatedMeshRuntimeFormat::Glb => {
                protocol_render::AnimatedMeshRuntimeFormat::Glb
            }
        },
        content_hash: descriptor.content_hash.clone(),
        clips: descriptor
            .clips
            .iter()
            .map(|clip| protocol_render::AnimationClipDescriptor {
                id: clip.id.clone(),
                name: clip.name.clone(),
                duration_seconds: clip.duration_seconds,
            })
            .collect(),
        default_clip: descriptor.default_clip.clone(),
        material_slots: descriptor
            .material_slots
            .iter()
            .map(|slot| protocol_render::MeshMaterialSlot {
                slot: slot.slot,
                material: slot.material.clone(),
            })
            .collect(),
        bounds: protocol_render::MeshBoundsDescriptor {
            min: descriptor.bounds.min,
            max: descriptor.bounds.max,
        },
    }
}
