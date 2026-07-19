use super::*;

pub(super) fn decode_voxel_assets(
    source: &AdmittedRuntimeProjectSourceBatch,
    report: &mut RuntimeProjectAdmissionReport,
) -> BTreeMap<String, VoxelVolumeAsset> {
    let mut assets = BTreeMap::new();
    for artifact in source
        .manifest()
        .artifacts
        .iter()
        .filter(|artifact| artifact.role == ArtifactRole::VoxelVolumeAsset)
    {
        let Some(body) = source.body(&artifact.path) else {
            report.push(
                RuntimeProjectAdmissionDiagnosticCode::ResourceNotStaged,
                None,
                &artifact.path,
                "manifest voxel asset body was not retained in the admitted closure",
            );
            continue;
        };
        let text = match std::str::from_utf8(body) {
            Ok(text) => text,
            Err(error) => {
                report.push(
                    RuntimeProjectAdmissionDiagnosticCode::ResourceDecode,
                    None,
                    &artifact.path,
                    format!("voxel asset is not canonical UTF-8 JSON: {error}"),
                );
                continue;
            }
        };
        match svc_voxel_asset::decode_asset(text) {
            Ok(asset) if !assets.contains_key(&asset.asset_id) => {
                assets.insert(asset.asset_id.clone(), asset);
            }
            Ok(asset) => report.push(
                RuntimeProjectAdmissionDiagnosticCode::AmbiguousReference,
                None,
                &artifact.path,
                format!(
                    "voxel asset id `{}` is supplied by more than one manifest artifact",
                    asset.asset_id
                ),
            ),
            Err(error) => report.push(
                RuntimeProjectAdmissionDiagnosticCode::ResourceDecode,
                None,
                &artifact.path,
                format!("voxel asset failed canonical decode/validation: {error:?}"),
            ),
        }
    }
    assets
}

pub(super) fn bundle_artifacts(
    source: &AdmittedRuntimeProjectSourceBatch,
    report: &mut RuntimeProjectAdmissionReport,
) -> BundleArtifacts {
    let mut artifacts = BundleArtifacts::new();
    for entry in &source.manifest().artifacts {
        if matches!(entry.role, ArtifactRole::Resource(_)) {
            continue;
        }
        let Some(body) = source.body(&entry.path) else {
            continue;
        };
        match std::str::from_utf8(body) {
            Ok(text) => {
                artifacts = artifacts.with_artifact(entry.path.clone(), text.to_owned());
            }
            Err(error) if entry.class.is_load_required() => report.push(
                RuntimeProjectAdmissionDiagnosticCode::ResourceDecode,
                None,
                &entry.path,
                format!("load-required artifact is not UTF-8: {error}"),
            ),
            Err(_) => {}
        }
    }
    artifacts
}

pub(super) fn check_voxel_asset_links(
    source: &AdmittedRuntimeProjectSourceBatch,
    scenes: &BTreeMap<u64, FlatSceneDocument>,
    documents: &[ProjectContentDocumentDto],
    voxel_assets: &BTreeMap<String, VoxelVolumeAsset>,
    report: &mut RuntimeProjectAdmissionReport,
) {
    let catalog_paths = documents
        .iter()
        .filter_map(|document| match document {
            ProjectContentDocumentDto::AssetCatalog { catalog, .. } => Some(catalog),
            _ => None,
        })
        .flat_map(|catalog| &catalog.entries)
        .filter_map(|entry| {
            entry
                .source_path
                .as_ref()
                .map(|path| (entry.id.as_str(), path.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut artifact_paths = BTreeMap::new();
    for artifact in source
        .manifest()
        .artifacts
        .iter()
        .filter(|artifact| artifact.role == ArtifactRole::VoxelVolumeAsset)
    {
        let Some(body) = source.body(&artifact.path) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(body) else {
            continue;
        };
        let Ok(asset) = svc_voxel_asset::decode_asset(text) else {
            continue;
        };
        artifact_paths.insert(asset.asset_id, artifact.path.as_str());
    }

    for (asset_id, artifact_path) in &artifact_paths {
        match catalog_paths.get(asset_id.as_str()) {
            Some(catalog_path) if catalog_path == artifact_path => {}
            Some(catalog_path) => report.push(
                RuntimeProjectAdmissionDiagnosticCode::DanglingReference,
                None,
                format!("assets.{asset_id}.sourcePath"),
                format!(
                    "voxel asset catalog path `{catalog_path}` does not match manifest artifact `{artifact_path}`"
                ),
            ),
            None => report.push(
                RuntimeProjectAdmissionDiagnosticCode::DanglingReference,
                None,
                format!("assets.{asset_id}"),
                "manifest voxel asset is not identified by a stored asset-catalog entry",
            ),
        }
    }

    for (scene_id, scene) in scenes {
        for node in &scene.nodes {
            let SceneNodeKind::VoxelVolume(reference) = &node.kind else {
                continue;
            };
            let asset_id = reference.id().as_str();
            if !voxel_assets.contains_key(asset_id) {
                report.push(
                    RuntimeProjectAdmissionDiagnosticCode::DanglingReference,
                    None,
                    format!("scenes.{scene_id}.nodes.{}.voxelVolume", node.id.raw()),
                    format!(
                        "scene references voxel asset `{asset_id}` without a decoded voxelVolumeAsset artifact"
                    ),
                );
            }
        }
    }
}
