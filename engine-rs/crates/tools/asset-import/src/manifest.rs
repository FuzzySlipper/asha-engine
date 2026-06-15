//! Import manifest: source fingerprints, versions, artifact hashes, drift, and the
//! reimport plan (#2385).
//!
//! The manifest is the record that lets imported assets participate in asset locks
//! and drift detection. It pins the **source fingerprint** (so changed source is
//! detectable), the **importer and schema versions** (so a tool change is visible),
//! and the **hash of every generated artifact** (so output drift is detectable). A
//! reimport is classified as a no-op, a safe visual update, or a structural change
//! requiring a reload — a reimport never silently overwrites.

use crate::artifacts::GeneratedArtifact;
use crate::diagnostic::{ImportCode, ImportDiagnostic};
use crate::fingerprint::fingerprint_hex;
use crate::json::{Json, JsonWriter};

/// One generated artifact's path and content fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactFingerprint {
    pub rel_path: String,
    pub hash: String,
}

/// The import manifest written alongside the generated artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportManifest {
    pub source_path: String,
    pub source_fingerprint: String,
    pub importer_version: u32,
    pub schema_version: u64,
    pub mesh_asset_id: String,
    /// The stable sidecar GUID of the source asset, when one exists (#2486). Links
    /// the manifest's generated artifacts and catalog entry to a content/path-
    /// independent source identity, so a source-asset → artifact → catalog → lock
    /// trace survives moves and re-locks. `None` for a source with no sidecar yet.
    pub guid: Option<String>,
    /// Generated artifact fingerprints, in stable path order.
    pub artifacts: Vec<ArtifactFingerprint>,
}

impl ImportManifest {
    /// Attach the source asset's sidecar GUID (builder-style).
    pub fn with_guid(mut self, guid: &str) -> Self {
        self.guid = Some(guid.to_string());
        self
    }

    /// The fingerprint of one artifact by path, if present.
    pub fn artifact_hash(&self, rel_path: &str) -> Option<&str> {
        self.artifacts
            .iter()
            .find(|a| a.rel_path == rel_path)
            .map(|a| a.hash.as_str())
    }

    /// A deterministic source trace: source-asset GUID → each generated artifact
    /// (by hash) → the catalog/mesh asset id the lock pins. Stable text for agents
    /// and CI. The GUID anchors the trace so it survives a source move or re-lock.
    pub fn source_trace_report(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "sourceTrace guid={} source={}\n",
            self.guid.as_deref().unwrap_or("-"),
            self.source_path,
        ));
        for a in &self.artifacts {
            out.push_str(&format!("  artifact {} {}\n", a.rel_path, a.hash));
        }
        out.push_str(&format!("  catalog {}\n", self.mesh_asset_id));
        out
    }

    /// Deterministic JSON rendering for the on-disk manifest + golden fixtures.
    pub fn render(&self) -> String {
        let mut w = JsonWriter::new();
        w.begin_object();
        w.field_str("sourcePath", &self.source_path, false);
        w.field_str("sourceFingerprint", &self.source_fingerprint, false);
        w.field_num("importerVersion", self.importer_version as f64, false);
        w.field_num("schemaVersion", self.schema_version as f64, false);
        w.field_str("meshAssetId", &self.mesh_asset_id, false);
        w.field_opt_str("guid", self.guid.as_deref(), false);
        w.begin_array_field("artifacts");
        for (i, a) in self.artifacts.iter().enumerate() {
            let last = i + 1 == self.artifacts.len();
            w.array_element_indent();
            w.begin_object();
            w.field_str("path", &a.rel_path, false);
            w.field_str("hash", &a.hash, true);
            w.end_object(!last);
        }
        w.end_array(true);
        w.end_object(false);
        w.finish()
    }
}

/// Parse a previously written manifest (the importer's own output) so a reimport
/// can be classified against it. Returns `None` on any structural mismatch.
pub fn parse_manifest(text: &str) -> Option<ImportManifest> {
    let root = Json::parse(text).ok()?;
    let artifacts = root
        .get("artifacts")?
        .as_array()?
        .iter()
        .map(|a| {
            Some(ArtifactFingerprint {
                rel_path: a.get("path")?.as_str()?.to_string(),
                hash: a.get("hash")?.as_str()?.to_string(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ImportManifest {
        source_path: root.get("sourcePath")?.as_str()?.to_string(),
        source_fingerprint: root.get("sourceFingerprint")?.as_str()?.to_string(),
        importer_version: root.get("importerVersion")?.as_u64()? as u32,
        schema_version: root.get("schemaVersion")?.as_u64()?,
        mesh_asset_id: root.get("meshAssetId")?.as_str()?.to_string(),
        guid: root.get("guid").and_then(Json::as_str).map(str::to_string),
        artifacts,
    })
}

/// Build a manifest from the source bytes, versions, and the generated artifacts.
pub fn build_manifest(
    source_path: &str,
    source_text: &str,
    importer_version: u32,
    schema_version: u64,
    mesh_asset_id: &str,
    artifacts: &[GeneratedArtifact],
) -> ImportManifest {
    let mut fingerprints: Vec<ArtifactFingerprint> = artifacts
        .iter()
        .map(|a| ArtifactFingerprint {
            rel_path: a.rel_path.clone(),
            hash: fingerprint_hex(a.contents.as_bytes()),
        })
        .collect();
    fingerprints.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    ImportManifest {
        source_path: source_path.to_string(),
        source_fingerprint: fingerprint_hex(source_text.as_bytes()),
        importer_version,
        schema_version,
        mesh_asset_id: mesh_asset_id.to_string(),
        guid: None,
        artifacts: fingerprints,
    }
}

/// The classified plan for a reimport relative to a prior manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReimportPlan {
    /// Source, importer, and every artifact are unchanged — nothing to write.
    Noop,
    /// Only catalog/material (visual) artifacts changed — a safe in-place update.
    VisualUpdate { changed: Vec<String> },
    /// Geometry/structure or the importer version changed — a reload is required.
    StructuralReload {
        reason: String,
        changed: Vec<String>,
    },
}

impl ReimportPlan {
    pub fn label(&self) -> &'static str {
        match self {
            ReimportPlan::Noop => "noop",
            ReimportPlan::VisualUpdate { .. } => "visualUpdate",
            ReimportPlan::StructuralReload { .. } => "structuralReload",
        }
    }
}

/// Classify what a reimport would do, comparing a freshly built manifest against a
/// prior one. The static-mesh artifact is structural; the catalog artifact is a
/// visual/metadata update.
pub fn plan_reimport(prior: &ImportManifest, next: &ImportManifest) -> ReimportPlan {
    if prior.importer_version != next.importer_version {
        return ReimportPlan::StructuralReload {
            reason: format!(
                "importer version changed {} -> {}",
                prior.importer_version, next.importer_version
            ),
            changed: changed_artifacts(prior, next),
        };
    }

    let changed = changed_artifacts(prior, next);
    if changed.is_empty() && prior.source_fingerprint == next.source_fingerprint {
        return ReimportPlan::Noop;
    }

    let structural_changed = changed.iter().any(|p| p.ends_with(".staticmesh.json"));
    if structural_changed {
        ReimportPlan::StructuralReload {
            reason: "geometry/structure changed".to_string(),
            changed,
        }
    } else {
        ReimportPlan::VisualUpdate { changed }
    }
}

fn changed_artifacts(prior: &ImportManifest, next: &ImportManifest) -> Vec<String> {
    let mut changed = Vec::new();
    for a in &next.artifacts {
        match prior.artifact_hash(&a.rel_path) {
            Some(h) if h == a.hash => {}
            _ => changed.push(a.rel_path.clone()),
        }
    }
    // Artifacts that existed before but no longer do are also a change.
    for a in &prior.artifacts {
        if next.artifact_hash(&a.rel_path).is_none() {
            changed.push(a.rel_path.clone());
        }
    }
    changed.sort();
    changed.dedup();
    changed
}

/// Detect imported-source drift against an asset-lock's recorded source
/// fingerprint. Returns a classified diagnostic when they differ — the lock must
/// not be silently re-pinned.
pub fn detect_source_drift(
    locked_fingerprint: &str,
    current_fingerprint: &str,
    mesh_asset_id: &str,
) -> Option<ImportDiagnostic> {
    if locked_fingerprint == current_fingerprint {
        return None;
    }
    Some(ImportDiagnostic::warning(
        ImportCode::SourceFingerprintChanged,
        mesh_asset_id,
        format!(
            "source fingerprint changed {locked_fingerprint} -> {current_fingerprint} since the asset lock was written"
        ),
        "reimport and review the reimport plan before updating the lock",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifacts(catalog: &str, mesh: &str) -> Vec<GeneratedArtifact> {
        vec![
            GeneratedArtifact {
                rel_path: "a.catalog.json".into(),
                contents: catalog.into(),
            },
            GeneratedArtifact {
                rel_path: "a.staticmesh.json".into(),
                contents: mesh.into(),
            },
        ]
    }

    fn manifest(source: &str, catalog: &str, mesh: &str) -> ImportManifest {
        build_manifest(
            "a.mesh.json",
            source,
            1,
            1,
            "mesh/a",
            &artifacts(catalog, mesh),
        )
    }

    #[test]
    fn build_is_deterministic_and_sorted() {
        let a = manifest("src", "cat", "mesh");
        let b = manifest("src", "cat", "mesh");
        assert_eq!(a, b);
        assert_eq!(a.artifacts[0].rel_path, "a.catalog.json"); // sorted
    }

    #[test]
    fn unchanged_reimport_is_a_noop() {
        let prior = manifest("src", "cat", "mesh");
        let next = manifest("src", "cat", "mesh");
        assert_eq!(plan_reimport(&prior, &next), ReimportPlan::Noop);
    }

    #[test]
    fn catalog_only_change_is_a_visual_update() {
        let prior = manifest("src", "cat", "mesh");
        let next = manifest("src2", "cat-changed", "mesh");
        match plan_reimport(&prior, &next) {
            ReimportPlan::VisualUpdate { changed } => assert_eq!(changed, vec!["a.catalog.json"]),
            other => panic!("expected visual update, got {other:?}"),
        }
    }

    #[test]
    fn geometry_change_requires_structural_reload() {
        let prior = manifest("src", "cat", "mesh");
        let next = manifest("src2", "cat", "mesh-changed");
        assert!(matches!(
            plan_reimport(&prior, &next),
            ReimportPlan::StructuralReload { .. }
        ));
    }

    #[test]
    fn importer_version_change_is_visible_as_structural() {
        let prior = manifest("src", "cat", "mesh");
        let mut next = manifest("src", "cat", "mesh");
        next.importer_version = 2;
        assert!(matches!(
            plan_reimport(&prior, &next),
            ReimportPlan::StructuralReload { .. }
        ));
    }

    #[test]
    fn guid_round_trips_and_drives_a_source_trace() {
        let m = manifest("src", "cat", "mesh").with_guid("28426a627e8870ba9fdefd6a0d998bfc");
        // The GUID survives manifest encode/decode.
        let decoded = parse_manifest(&m.render()).expect("decode");
        assert_eq!(
            decoded.guid.as_deref(),
            Some("28426a627e8870ba9fdefd6a0d998bfc")
        );
        // The source trace links GUID → artifacts → catalog id deterministically.
        let trace = m.source_trace_report();
        assert!(trace.contains("guid=28426a627e8870ba9fdefd6a0d998bfc"));
        assert!(trace.contains("artifact a.catalog.json"));
        assert!(trace.contains("catalog mesh/a"));
    }

    #[test]
    fn a_guidless_manifest_still_round_trips() {
        let m = manifest("src", "cat", "mesh");
        assert_eq!(m.guid, None);
        assert_eq!(parse_manifest(&m.render()).unwrap().guid, None);
    }

    #[test]
    fn source_drift_is_detected_against_a_lock() {
        let none = detect_source_drift("abc", "abc", "mesh/a");
        assert!(none.is_none());
        let drift = detect_source_drift("abc", "def", "mesh/a").unwrap();
        assert_eq!(drift.code, ImportCode::SourceFingerprintChanged);
    }
}
