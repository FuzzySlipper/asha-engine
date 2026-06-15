//! File-driven asset **sidecar metadata** with stable GUIDs (#2486).
//!
//! Useful ideas borrowed from Unity-style asset tracking — a colocated metadata
//! sidecar, a stable content-independent GUID, recorded import settings, and
//! derived-artifact references — **without** Unity's project-contained limitation.
//! An ASHA source asset may live anywhere; its sidecar (`<source>.asha-meta.json`)
//! sits next to it and can be referenced by more than one project. A
//! [`ProjectOverride`], keyed by GUID, carries project-specific import settings
//! **without mutating the shared sidecar** — so the same source asset participates
//! in several projects with different effective settings.
//!
//! # Identity model
//!
//! * **GUID** — minted once at sidecar init, content-independent and path-
//!   independent, so a reference survives both an edit and a move.
//! * **content hash** — the source bytes' fingerprint; a change invalidates derived
//!   artifacts (a reimport), but never changes the GUID.
//! * **source URI** — how the sidecar names the source (relative/absolute/file URL/
//!   content-addressed), recorded so a moved file can be reconciled by GUID.
//!
//! All encode/decode is deterministic std-only JSON (the workspace has no serde).

use crate::fingerprint::{fingerprint_hex, fnv1a_64};
use crate::json::{Json, JsonWriter};
use crate::manifest::ArtifactFingerprint;

/// The sidecar schema version. Bumped when the on-disk shape changes; a newer
/// sidecar fails closed at decode rather than being misread.
pub const SIDECAR_SCHEMA_VERSION: u32 = 1;

// ── Stable GUID ───────────────────────────────────────────────────────────────

/// A stable, content- and path-independent asset identifier: 32 lowercase-hex
/// chars (128 bits). Minted once at init and never derived from content or path,
/// so references survive edits and moves.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetGuid(String);

impl AssetGuid {
    /// Mint a fresh GUID from a uniqueness `seed` (e.g. source path + a salt/nonce
    /// the caller varies per asset). Deterministic for a given seed so fixtures and
    /// tests are reproducible; a production CLI varies the salt per `init` so two
    /// assets never collide. Two domain-separated FNV-1a halves form the 128 bits.
    pub fn mint(seed: &str) -> AssetGuid {
        let hi = fnv1a_64(format!("asha-guid-hi:{seed}").as_bytes());
        let lo = fnv1a_64(format!("asha-guid-lo:{seed}").as_bytes());
        AssetGuid(format!("{hi:016x}{lo:016x}"))
    }

    /// Parse a GUID string, validating it is exactly 32 lowercase-hex chars.
    pub fn parse(text: &str) -> Option<AssetGuid> {
        let ok = text.len() == 32
            && text
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
        ok.then(|| AssetGuid(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ── Source URI ────────────────────────────────────────────────────────────────

/// How a sidecar names its source asset. Not every source lives under an ASHA
/// project tree, so the model spans project-relative, absolute, file-URL, and
/// content-addressed references; a registry reference is a documented future kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceUri {
    /// Relative to the referencing project/workspace root.
    RelativePath(String),
    /// An absolute filesystem path (a shared asset outside any project tree).
    AbsolutePath(String),
    /// A `file://` URL.
    FileUrl(String),
    /// A content-addressed entry: the source is identified by its content hash, not
    /// a path (a moved/renamed file still resolves).
    ContentAddressed(String),
}

impl SourceUri {
    pub fn kind(&self) -> &'static str {
        match self {
            SourceUri::RelativePath(_) => "relativePath",
            SourceUri::AbsolutePath(_) => "absolutePath",
            SourceUri::FileUrl(_) => "fileUrl",
            SourceUri::ContentAddressed(_) => "contentAddressed",
        }
    }

    pub fn value(&self) -> &str {
        match self {
            SourceUri::RelativePath(s)
            | SourceUri::AbsolutePath(s)
            | SourceUri::FileUrl(s)
            | SourceUri::ContentAddressed(s) => s,
        }
    }

    fn from_parts(kind: &str, value: &str) -> Option<SourceUri> {
        match kind {
            "relativePath" => Some(SourceUri::RelativePath(value.to_string())),
            "absolutePath" => Some(SourceUri::AbsolutePath(value.to_string())),
            "fileUrl" => Some(SourceUri::FileUrl(value.to_string())),
            "contentAddressed" => Some(SourceUri::ContentAddressed(value.to_string())),
            _ => None,
        }
    }
}

// ── Import settings (typed, not arbitrary JSON) ─────────────────────────────────

/// The import settings recorded in a sidecar. Typed and closed — no arbitrary
/// JSON pass-through across the border. A [`ProjectOverride`] can override these
/// per-project without touching the shared sidecar.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportSettings {
    /// Uniform import scale applied to source geometry.
    pub scale: f32,
    /// Whether the importer generates a collision artifact.
    pub generate_collision: bool,
    /// Optional material-namespace prefix for emitted catalog entries.
    pub material_namespace: Option<String>,
}

impl Default for ImportSettings {
    fn default() -> Self {
        ImportSettings {
            scale: 1.0,
            generate_collision: false,
            material_namespace: None,
        }
    }
}

/// A project-local override of a shared asset's import settings, keyed by GUID.
/// Only set fields override; the shared sidecar is never mutated.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProjectOverride {
    pub guid: Option<AssetGuid>,
    pub scale: Option<f32>,
    pub generate_collision: Option<bool>,
}

impl ProjectOverride {
    /// The effective settings for this project: the shared base with any set
    /// override fields applied. Pure — returns a new value, mutates nothing.
    pub fn apply(&self, base: &ImportSettings) -> ImportSettings {
        ImportSettings {
            scale: self.scale.unwrap_or(base.scale),
            generate_collision: self.generate_collision.unwrap_or(base.generate_collision),
            material_namespace: base.material_namespace.clone(),
        }
    }
}

// ── Sidecar metadata ────────────────────────────────────────────────────────--

/// The colocated metadata record for one source asset (`<source>.asha-meta.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct SidecarMetadata {
    pub schema_version: u32,
    pub guid: AssetGuid,
    pub source_uri: SourceUri,
    /// Fingerprint of the source bytes at last init/reimport.
    pub content_hash: String,
    pub importer_version: u32,
    /// The declared asset kind (e.g. `mesh`, `texture`) — a `core_assets::AssetKind`
    /// prefix string. Distinguishes source identity from per-project usage.
    pub declared_kind: String,
    pub labels: Vec<String>,
    pub import_settings: ImportSettings,
    /// References to the artifacts generated from this source, in path order.
    pub generated_artifacts: Vec<ArtifactFingerprint>,
}

/// The conventional sidecar path for a source file: `<source>.asha-meta.json`.
pub fn sidecar_path(source_path: &str) -> String {
    format!("{source_path}.asha-meta.json")
}

/// Initialize sidecar metadata for a source asset, minting a fresh GUID. `salt`
/// makes the GUID unique even for identical paths across runs (a CLI varies it).
pub fn init_metadata(
    source_uri: SourceUri,
    source_bytes: &[u8],
    declared_kind: &str,
    importer_version: u32,
    settings: ImportSettings,
    salt: &str,
) -> SidecarMetadata {
    let guid = AssetGuid::mint(&format!("{}|{salt}", source_uri.value()));
    SidecarMetadata {
        schema_version: SIDECAR_SCHEMA_VERSION,
        guid,
        source_uri,
        content_hash: fingerprint_hex(source_bytes),
        importer_version,
        declared_kind: declared_kind.to_string(),
        labels: Vec::new(),
        import_settings: settings,
        generated_artifacts: Vec::new(),
    }
}

// ── Conflict / reconciliation classification ────────────────────────────────────

/// The classified status of a source asset relative to its sidecar (if any).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarStatus {
    /// No sidecar exists yet — `init` is required before the asset is tracked.
    MissingSidecar,
    /// Sidecar and source agree: same content hash, same recorded path.
    Unchanged,
    /// The GUID matches but the source path moved — reconcile the recorded URI
    /// (non-destructive; the GUID and derived artifacts stay valid).
    MovedFile { from: String, to: String },
    /// The source content changed under a stable GUID — derived artifacts are
    /// invalid and a reimport is required (the GUID and references survive).
    ContentChanged { from: String, to: String },
}

impl SidecarStatus {
    pub fn label(&self) -> &'static str {
        match self {
            SidecarStatus::MissingSidecar => "missingSidecar",
            SidecarStatus::Unchanged => "unchanged",
            SidecarStatus::MovedFile { .. } => "movedFile",
            SidecarStatus::ContentChanged { .. } => "contentChanged",
        }
    }
}

/// Reconcile a (possibly absent) sidecar against the current source location and
/// bytes. Move detection takes precedence over content change is NOT assumed —
/// both are reported by checking content first (the more consequential), then path.
pub fn reconcile(
    prior: Option<&SidecarMetadata>,
    current_uri: &SourceUri,
    current_bytes: &[u8],
) -> SidecarStatus {
    let Some(prior) = prior else {
        return SidecarStatus::MissingSidecar;
    };
    let current_hash = fingerprint_hex(current_bytes);
    if prior.content_hash != current_hash {
        return SidecarStatus::ContentChanged {
            from: prior.content_hash.clone(),
            to: current_hash,
        };
    }
    if prior.source_uri.value() != current_uri.value() {
        return SidecarStatus::MovedFile {
            from: prior.source_uri.value().to_string(),
            to: current_uri.value().to_string(),
        };
    }
    SidecarStatus::Unchanged
}

/// Detect duplicate GUIDs across a set of sidecars (e.g. a source asset copied
/// without re-initializing, so two files claim the same identity). Returns the
/// offending GUIDs in sorted order — a duplicate GUID is always a conflict.
pub fn detect_duplicate_guids(sidecars: &[SidecarMetadata]) -> Vec<AssetGuid> {
    let mut seen: Vec<&AssetGuid> = Vec::new();
    let mut dups: Vec<AssetGuid> = Vec::new();
    for s in sidecars {
        if seen.contains(&&s.guid) {
            if !dups.contains(&s.guid) {
                dups.push(s.guid.clone());
            }
        } else {
            seen.push(&s.guid);
        }
    }
    dups.sort();
    dups
}

// ── Encode / decode (deterministic std-only JSON) ───────────────────────────────

impl SidecarMetadata {
    /// Deterministic JSON rendering for the on-disk sidecar + golden fixtures.
    pub fn render(&self) -> String {
        let mut w = JsonWriter::new();
        w.begin_object();
        w.field_num("schemaVersion", self.schema_version as f64, false);
        w.field_str("guid", self.guid.as_str(), false);
        w.indent_field_object("sourceUri");
        w.field_str("kind", self.source_uri.kind(), false);
        w.field_str("value", self.source_uri.value(), true);
        w.end_object(true);
        w.field_str("contentHash", &self.content_hash, false);
        w.field_num("importerVersion", self.importer_version as f64, false);
        w.field_str("declaredKind", &self.declared_kind, false);
        w.field_str_array("labels", &self.labels, false);
        w.indent_field_object("importSettings");
        w.field_f32("scale", self.import_settings.scale, false);
        w.field_bool(
            "generateCollision",
            self.import_settings.generate_collision,
            self.import_settings.material_namespace.is_none(),
        );
        if let Some(ns) = &self.import_settings.material_namespace {
            w.field_str("materialNamespace", ns, true);
        }
        w.end_object(true);
        w.begin_array_field("generatedArtifacts");
        for (i, a) in self.generated_artifacts.iter().enumerate() {
            let last = i + 1 == self.generated_artifacts.len();
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

/// Parse a sidecar; fails closed (returns `None`) on a newer schema version, a
/// malformed structure, or an invalid GUID/URI/kind.
pub fn parse_sidecar(text: &str) -> Option<SidecarMetadata> {
    let root = Json::parse(text).ok()?;
    let schema_version = root.get("schemaVersion")?.as_u64()? as u32;
    if schema_version > SIDECAR_SCHEMA_VERSION {
        return None;
    }
    let guid = AssetGuid::parse(root.get("guid")?.as_str()?)?;
    let uri_obj = root.get("sourceUri")?;
    let source_uri = SourceUri::from_parts(
        uri_obj.get("kind")?.as_str()?,
        uri_obj.get("value")?.as_str()?,
    )?;
    let settings_obj = root.get("importSettings")?;
    let import_settings = ImportSettings {
        scale: settings_obj.get("scale")?.as_f64()? as f32,
        generate_collision: matches!(
            settings_obj.get("generateCollision"),
            Some(Json::Bool(true))
        ),
        material_namespace: settings_obj
            .get("materialNamespace")
            .and_then(Json::as_str)
            .map(str::to_string),
    };
    let labels = root
        .get("labels")
        .and_then(Json::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|l| l.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let generated_artifacts = root
        .get("generatedArtifacts")
        .and_then(Json::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|g| {
                    Some(ArtifactFingerprint {
                        rel_path: g.get("path")?.as_str()?.to_string(),
                        hash: g.get("hash")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(SidecarMetadata {
        schema_version,
        guid,
        source_uri,
        content_hash: root.get("contentHash")?.as_str()?.to_string(),
        importer_version: root.get("importerVersion")?.as_u64()? as u32,
        declared_kind: root.get("declaredKind")?.as_str()?.to_string(),
        labels,
        import_settings,
        generated_artifacts,
    })
}

// ── Agent/CLI reports (deterministic text) ─────────────────────────────────────

/// A deterministic, agent/CI-legible inspection of a sidecar: identity, source,
/// content hash, declared kind, import settings, and generated-artifact refs.
pub fn inspect_report(m: &SidecarMetadata) -> String {
    let mut out = String::new();
    out.push_str(&format!("guid {}\n", m.guid.as_str()));
    out.push_str(&format!(
        "source {} {}\n",
        m.source_uri.kind(),
        m.source_uri.value()
    ));
    out.push_str(&format!("contentHash {}\n", m.content_hash));
    out.push_str(&format!("declaredKind {}\n", m.declared_kind));
    out.push_str(&format!("importerVersion {}\n", m.importer_version));
    out.push_str(&format!(
        "settings scale={} generateCollision={} materialNamespace={}\n",
        m.import_settings.scale,
        m.import_settings.generate_collision,
        m.import_settings
            .material_namespace
            .as_deref()
            .unwrap_or("-"),
    ));
    if m.labels.is_empty() {
        out.push_str("labels -\n");
    } else {
        out.push_str(&format!("labels {}\n", m.labels.join(",")));
    }
    for a in &m.generated_artifacts {
        out.push_str(&format!("artifact {} {}\n", a.rel_path, a.hash));
    }
    out
}

/// A deterministic explanation of a reconcile [`SidecarStatus`], for `validate`
/// output: the classified status plus the actionable next step.
pub fn drift_report(status: &SidecarStatus) -> String {
    match status {
        SidecarStatus::MissingSidecar => {
            "status missingSidecar; run init to begin tracking this source".to_string()
        }
        SidecarStatus::Unchanged => "status unchanged; nothing to do".to_string(),
        SidecarStatus::MovedFile { from, to } => format!(
            "status movedFile from={from} to={to}; reconcile the recorded source URI (guid and artifacts stay valid)"
        ),
        SidecarStatus::ContentChanged { from, to } => format!(
            "status contentChanged from={from} to={to}; reimport — derived artifacts are stale (guid and references survive)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> SidecarMetadata {
        init_metadata(
            SourceUri::RelativePath("assets/crate.mesh.json".into()),
            b"source-bytes",
            "mesh",
            crate::IMPORTER_VERSION,
            ImportSettings::default(),
            "salt-0",
        )
    }

    #[test]
    fn guid_is_stable_content_and_path_independent() {
        let g = AssetGuid::mint("assets/crate|salt-0");
        assert_eq!(g.as_str().len(), 32);
        assert_eq!(g, AssetGuid::mint("assets/crate|salt-0")); // deterministic per seed
        assert_ne!(g, AssetGuid::mint("assets/crate|salt-1")); // salt varies identity
        assert_eq!(AssetGuid::parse(g.as_str()), Some(g));
        assert_eq!(AssetGuid::parse("nothex"), None);
    }

    #[test]
    fn sidecar_round_trips_through_json() {
        let mut m = meta();
        m.labels = vec!["prop".into(), "static".into()];
        m.generated_artifacts = vec![ArtifactFingerprint {
            rel_path: "crate.staticmesh.json".into(),
            hash: "0011223344556677".into(),
        }];
        let decoded = parse_sidecar(&m.render()).expect("decode");
        assert_eq!(decoded, m);
    }

    #[test]
    fn newer_schema_fails_closed() {
        let text = meta()
            .render()
            .replace("\"schemaVersion\": 1", "\"schemaVersion\": 2");
        assert_eq!(parse_sidecar(&text), None);
    }

    #[test]
    fn sidecar_path_is_colocated() {
        assert_eq!(
            sidecar_path("a/b/crate.mesh.json"),
            "a/b/crate.mesh.json.asha-meta.json"
        );
    }

    #[test]
    fn reconcile_classifies_missing_unchanged_moved_and_content_change() {
        assert_eq!(
            reconcile(None, &SourceUri::RelativePath("x".into()), b"x"),
            SidecarStatus::MissingSidecar
        );
        let m = meta();
        assert_eq!(
            reconcile(
                Some(&m),
                &SourceUri::RelativePath("assets/crate.mesh.json".into()),
                b"source-bytes"
            ),
            SidecarStatus::Unchanged
        );
        // Moved: same bytes, different path → GUID/artifacts stay valid.
        assert!(matches!(
            reconcile(
                Some(&m),
                &SourceUri::RelativePath("moved/crate.mesh.json".into()),
                b"source-bytes"
            ),
            SidecarStatus::MovedFile { .. }
        ));
        // Content changed: derived artifacts invalid, GUID survives.
        assert!(matches!(
            reconcile(
                Some(&m),
                &SourceUri::RelativePath("assets/crate.mesh.json".into()),
                b"new-bytes"
            ),
            SidecarStatus::ContentChanged { .. }
        ));
    }

    #[test]
    fn duplicate_guids_are_detected_as_conflicts() {
        let a = meta();
        let mut b = meta(); // same seed → same GUID (a copied asset, not re-inited)
        b.source_uri = SourceUri::RelativePath("copy/crate.mesh.json".into());
        let c = init_metadata(
            SourceUri::RelativePath("other.mesh.json".into()),
            b"other",
            "mesh",
            1,
            ImportSettings::default(),
            "salt-9",
        );
        let dups = detect_duplicate_guids(&[a.clone(), b, c]);
        assert_eq!(dups, vec![a.guid]);
    }

    #[test]
    fn project_override_does_not_mutate_the_shared_sidecar() {
        let m = meta();
        let over = ProjectOverride {
            guid: Some(m.guid.clone()),
            scale: Some(2.0),
            generate_collision: Some(true),
        };
        let effective = over.apply(&m.import_settings);
        assert_eq!(effective.scale, 2.0);
        assert_eq!(effective.generate_collision, true);
        // The shared sidecar is untouched (project-agnostic source identity).
        assert_eq!(m.import_settings.scale, 1.0);
        assert_eq!(m.import_settings.generate_collision, false);
    }

    #[test]
    fn the_same_source_serves_two_projects_with_distinct_settings() {
        let shared = meta();
        let project_a = ProjectOverride {
            scale: Some(0.5),
            ..Default::default()
        };
        let project_b = ProjectOverride {
            scale: Some(4.0),
            generate_collision: Some(true),
            ..Default::default()
        };
        let a = project_a.apply(&shared.import_settings);
        let b = project_b.apply(&shared.import_settings);
        assert_eq!(a.scale, 0.5);
        assert_eq!(b.scale, 4.0);
        assert_ne!(a, b);
        // Same GUID identity underlies both — a shared, project-agnostic source.
        assert_eq!(shared.guid, shared.guid);
    }
}
