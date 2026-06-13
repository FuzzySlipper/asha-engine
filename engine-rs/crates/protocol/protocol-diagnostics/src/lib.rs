//! Protocol-level diagnostic report types for scene, asset, world-bundle, and
//! renderer-resource failures.
//!
//! # Lane
//!
//! `contract-steward` — these are border shapes mirrored to generated TypeScript
//! by `protocol-codegen`. The crate owns *types and stable codes only*: it has
//! no product logic, performs no validation, and never mutates authority.
//! Validators and renderer layers (the diagnostics *emitters*) live elsewhere
//! and construct these reports.
//!
//! # Posture (scene-capability-06)
//!
//! - Diagnostics are **observational**: a `DiagnosticReport` describes a problem;
//!   it never repairs it.
//! - Codes are **stable**: [`DiagnosticCode`] string values are a contract. New
//!   variants are added; existing strings are not renamed.
//! - Severity ties to **recovery policy**: [`DiagnosticSeverity::Fatal`] stops a
//!   load; [`DiagnosticSeverity::Error`] degrades one node/entity/asset;
//!   `Warning`/`Info` never block.
//! - **No Den coupling.** These are generic ASHA artifacts. There are no
//!   Den-specific fields, ids, or imports here, and there must never be: an
//!   external workflow system may consume the codes/refs without ASHA depending
//!   on it.
//!
//! Border representation is deliberately dependency-light: ids and coordinates
//! are plain integers/strings at the border (matching `protocol-render`'s
//! posture for `source_scene_node`), so this crate needs no other crate.

#![forbid(unsafe_code)]

// ── Severity ──────────────────────────────────────────────────────────────────

/// How serious a diagnostic is, and therefore which recovery path applies.
///
/// Ordering (via [`DiagnosticSeverity::rank`]) is `Info < Warning < Error <
/// Fatal`; only `Fatal` blocks a load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticSeverity {
    /// Counts, stats, timings, trace summaries. Never blocks.
    Info,
    /// Load/save completes; stale cache, older asset, non-critical fallback.
    Warning,
    /// A specific node/entity/asset fails or is skipped; load may continue
    /// degraded if policy allows.
    Error,
    /// Stop the load entirely: incompatible version, corrupt durable artifact,
    /// required asset missing with no safe fallback.
    Fatal,
}

impl DiagnosticSeverity {
    /// The stable border string for this severity.
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticSeverity::Info => "info",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Fatal => "fatal",
        }
    }

    /// Monotonic rank used for "most severe" comparisons (`Info` = 0).
    pub fn rank(self) -> u8 {
        match self {
            DiagnosticSeverity::Info => 0,
            DiagnosticSeverity::Warning => 1,
            DiagnosticSeverity::Error => 2,
            DiagnosticSeverity::Fatal => 3,
        }
    }

    /// True when this severity stops a load (only `Fatal`).
    pub fn blocks_load(self) -> bool {
        matches!(self, DiagnosticSeverity::Fatal)
    }
}

/// Every severity string, in declaration order. The codegen source of truth for
/// the generated `DiagnosticSeverity` union.
pub const DIAGNOSTIC_SEVERITIES: &[&str] = &["info", "warning", "error", "fatal"];

// ── Scope ─────────────────────────────────────────────────────────────────────

/// Which subsystem a diagnostic belongs to — the lane an agent should route it
/// back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticScope {
    /// Authored scene / world document validation.
    Scene,
    /// Asset catalog / registry validation.
    AssetCatalog,
    /// World bundle manifest / serialization / load.
    WorldBundle,
    /// Render handle → scene node → entity → asset projection traces.
    RenderProjection,
    /// Renderer resource lifecycle / leak / count reports.
    RendererResources,
    /// World load/save execution and save→reload round-trip equivalence
    /// (runtime composition). Distinct from `WorldBundle`, which is the bundle
    /// *format*; this scope is the *executed* composition of a world.
    WorldComposition,
}

impl DiagnosticScope {
    /// The stable border string for this scope.
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticScope::Scene => "scene",
            DiagnosticScope::AssetCatalog => "assetCatalog",
            DiagnosticScope::WorldBundle => "worldBundle",
            DiagnosticScope::RenderProjection => "renderProjection",
            DiagnosticScope::RendererResources => "rendererResources",
            DiagnosticScope::WorldComposition => "worldComposition",
        }
    }
}

/// Every scope string, in declaration order. Codegen source of truth.
pub const DIAGNOSTIC_SCOPES: &[&str] = &[
    "scene",
    "assetCatalog",
    "worldBundle",
    "renderProjection",
    "rendererResources",
    "worldComposition",
];

// ── Codes ─────────────────────────────────────────────────────────────────────

/// A stable, machine-routable diagnostic code.
///
/// The string form ([`DiagnosticCode::as_str`]) is the contract: it is what
/// crosses to TypeScript and what external workflow systems key on. Add
/// variants; never rename an existing string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    // ── Scene ──
    /// Two scene nodes share an id.
    DuplicateSceneId,
    /// A node names a parent that does not exist / is out of range.
    InvalidSceneParent,
    /// The scene parent pointers form a cycle (report carries the path).
    SceneParentCycle,
    /// A node transform is non-finite or otherwise invalid.
    InvalidSceneTransform,
    /// A scene node references an asset that the catalog does not contain.
    SceneAssetMissing,
    /// A scene node references an asset of the wrong kind for its slot.
    SceneAssetWrongKind,
    // ── Asset catalog ──
    /// Two catalog entries share a stable asset id.
    DuplicateAssetId,
    /// A catalog structural rule was violated (material payload placement,
    /// empty source path) that is not itself a missing/wrong-kind/cycle case.
    CatalogStructuralError,
    /// A referenced asset id is absent from the catalog.
    MissingAsset,
    /// A referenced asset resolves to an older version than required.
    StaleAsset,
    /// An asset reference points at an asset of the wrong kind.
    WrongKindAssetRef,
    /// The asset dependency graph contains a cycle (report carries the path).
    AssetCycle,
    // ── World bundle ──
    /// The bundle manifest declares an unsupported bundle/protocol version.
    ManifestProtocolMismatch,
    /// A durable bundle artifact failed its content-hash / decode check.
    CorruptBundleArtifact,
    /// An optional cache artifact is stale or absent (non-blocking).
    MissingCacheWarning,
    /// A chunk's terrain generator metadata does not match the saved data.
    GeneratorMismatch,
    // ── Render projection ──
    /// A fallback material/texture was substituted for a missing/invalid asset.
    FallbackUsed,
    /// A render object cannot be traced back to a scene node / asset ref.
    MissingSourceTrace,
    // ── Renderer resources ──
    /// An informational renderer resource/count summary.
    RendererResourceSummary,
    /// Created-vs-disposed accounting suggests a geometry/material leak.
    SuspectedResourceLeak,
    // ── World composition (load/save execution + round-trip) ──
    /// A stage of an executed load plan failed (the `reference` names the stage).
    LoadStageFailed,
    /// Final load consistency (hashes / required assets / source traces) did not
    /// hold after composition.
    FinalConsistencyMismatch,
    /// A save→reload round-trip lost authority-equivalent state (entity, runtime
    /// transform, voxel content hash, asset ref, source trace, or world hash).
    RoundTripMismatch,
}

impl DiagnosticCode {
    /// The stable border string for this code (camelCase).
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticCode::DuplicateSceneId => "duplicateSceneId",
            DiagnosticCode::InvalidSceneParent => "invalidSceneParent",
            DiagnosticCode::SceneParentCycle => "sceneParentCycle",
            DiagnosticCode::InvalidSceneTransform => "invalidSceneTransform",
            DiagnosticCode::SceneAssetMissing => "sceneAssetMissing",
            DiagnosticCode::SceneAssetWrongKind => "sceneAssetWrongKind",
            DiagnosticCode::DuplicateAssetId => "duplicateAssetId",
            DiagnosticCode::CatalogStructuralError => "catalogStructuralError",
            DiagnosticCode::MissingAsset => "missingAsset",
            DiagnosticCode::StaleAsset => "staleAsset",
            DiagnosticCode::WrongKindAssetRef => "wrongKindAssetRef",
            DiagnosticCode::AssetCycle => "assetCycle",
            DiagnosticCode::ManifestProtocolMismatch => "manifestProtocolMismatch",
            DiagnosticCode::CorruptBundleArtifact => "corruptBundleArtifact",
            DiagnosticCode::MissingCacheWarning => "missingCacheWarning",
            DiagnosticCode::GeneratorMismatch => "generatorMismatch",
            DiagnosticCode::FallbackUsed => "fallbackUsed",
            DiagnosticCode::MissingSourceTrace => "missingSourceTrace",
            DiagnosticCode::RendererResourceSummary => "rendererResourceSummary",
            DiagnosticCode::SuspectedResourceLeak => "suspectedResourceLeak",
            DiagnosticCode::LoadStageFailed => "loadStageFailed",
            DiagnosticCode::FinalConsistencyMismatch => "finalConsistencyMismatch",
            DiagnosticCode::RoundTripMismatch => "roundTripMismatch",
        }
    }

    /// The scope this code belongs to.
    pub fn scope(self) -> DiagnosticScope {
        match self {
            DiagnosticCode::DuplicateSceneId
            | DiagnosticCode::InvalidSceneParent
            | DiagnosticCode::SceneParentCycle
            | DiagnosticCode::InvalidSceneTransform
            | DiagnosticCode::SceneAssetMissing
            | DiagnosticCode::SceneAssetWrongKind => DiagnosticScope::Scene,
            DiagnosticCode::DuplicateAssetId
            | DiagnosticCode::CatalogStructuralError
            | DiagnosticCode::MissingAsset
            | DiagnosticCode::StaleAsset
            | DiagnosticCode::WrongKindAssetRef
            | DiagnosticCode::AssetCycle => DiagnosticScope::AssetCatalog,
            DiagnosticCode::ManifestProtocolMismatch
            | DiagnosticCode::CorruptBundleArtifact
            | DiagnosticCode::MissingCacheWarning
            | DiagnosticCode::GeneratorMismatch => DiagnosticScope::WorldBundle,
            DiagnosticCode::FallbackUsed | DiagnosticCode::MissingSourceTrace => {
                DiagnosticScope::RenderProjection
            }
            DiagnosticCode::RendererResourceSummary | DiagnosticCode::SuspectedResourceLeak => {
                DiagnosticScope::RendererResources
            }
            DiagnosticCode::LoadStageFailed
            | DiagnosticCode::FinalConsistencyMismatch
            | DiagnosticCode::RoundTripMismatch => DiagnosticScope::WorldComposition,
        }
    }

    /// The severity an emitter should use unless context escalates it (e.g. a
    /// missing asset with no fallback becomes `Fatal`).
    pub fn default_severity(self) -> DiagnosticSeverity {
        match self {
            DiagnosticCode::DuplicateSceneId
            | DiagnosticCode::InvalidSceneParent
            | DiagnosticCode::SceneParentCycle
            | DiagnosticCode::InvalidSceneTransform
            | DiagnosticCode::SceneAssetMissing
            | DiagnosticCode::SceneAssetWrongKind
            | DiagnosticCode::DuplicateAssetId
            | DiagnosticCode::CatalogStructuralError
            | DiagnosticCode::MissingAsset
            | DiagnosticCode::WrongKindAssetRef
            | DiagnosticCode::AssetCycle
            | DiagnosticCode::RoundTripMismatch => DiagnosticSeverity::Error,
            DiagnosticCode::StaleAsset
            | DiagnosticCode::MissingCacheWarning
            | DiagnosticCode::FallbackUsed
            | DiagnosticCode::MissingSourceTrace
            | DiagnosticCode::SuspectedResourceLeak => DiagnosticSeverity::Warning,
            DiagnosticCode::ManifestProtocolMismatch
            | DiagnosticCode::CorruptBundleArtifact
            | DiagnosticCode::GeneratorMismatch
            | DiagnosticCode::LoadStageFailed
            | DiagnosticCode::FinalConsistencyMismatch => DiagnosticSeverity::Fatal,
            DiagnosticCode::RendererResourceSummary => DiagnosticSeverity::Info,
        }
    }
}

/// Every diagnostic code string, in declaration order. The codegen source of
/// truth for the generated `DiagnosticCode` union; kept honest by
/// `codes_table_matches_variants`.
pub const DIAGNOSTIC_CODES: &[&str] = &[
    "duplicateSceneId",
    "invalidSceneParent",
    "sceneParentCycle",
    "invalidSceneTransform",
    "sceneAssetMissing",
    "sceneAssetWrongKind",
    "duplicateAssetId",
    "catalogStructuralError",
    "missingAsset",
    "staleAsset",
    "wrongKindAssetRef",
    "assetCycle",
    "manifestProtocolMismatch",
    "corruptBundleArtifact",
    "missingCacheWarning",
    "generatorMismatch",
    "fallbackUsed",
    "missingSourceTrace",
    "rendererResourceSummary",
    "suspectedResourceLeak",
    "loadStageFailed",
    "finalConsistencyMismatch",
    "roundTripMismatch",
];

/// Every [`DiagnosticCode`] variant, in declaration order. Lets emitters and
/// tests enumerate the full taxonomy.
pub const ALL_DIAGNOSTIC_CODES: &[DiagnosticCode] = &[
    DiagnosticCode::DuplicateSceneId,
    DiagnosticCode::InvalidSceneParent,
    DiagnosticCode::SceneParentCycle,
    DiagnosticCode::InvalidSceneTransform,
    DiagnosticCode::SceneAssetMissing,
    DiagnosticCode::SceneAssetWrongKind,
    DiagnosticCode::DuplicateAssetId,
    DiagnosticCode::CatalogStructuralError,
    DiagnosticCode::MissingAsset,
    DiagnosticCode::StaleAsset,
    DiagnosticCode::WrongKindAssetRef,
    DiagnosticCode::AssetCycle,
    DiagnosticCode::ManifestProtocolMismatch,
    DiagnosticCode::CorruptBundleArtifact,
    DiagnosticCode::MissingCacheWarning,
    DiagnosticCode::GeneratorMismatch,
    DiagnosticCode::FallbackUsed,
    DiagnosticCode::MissingSourceTrace,
    DiagnosticCode::RendererResourceSummary,
    DiagnosticCode::SuspectedResourceLeak,
    DiagnosticCode::LoadStageFailed,
    DiagnosticCode::FinalConsistencyMismatch,
    DiagnosticCode::RoundTripMismatch,
];

// ── Remedy ────────────────────────────────────────────────────────────────────

/// A suggested next action. Advisory only — diagnostics never authorize an
/// automatic mutation; a separate repair tool would act on these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RemedyAction {
    /// Inspect the named source (node/asset/artifact) by hand.
    Inspect,
    /// Provide the missing asset / artifact.
    ProvideAsset,
    /// Correct the offending reference (kind or target).
    FixReference,
    /// Break the dependency cycle reported in the path.
    BreakCycle,
    /// Regenerate the affected data (e.g. terrain from its generator).
    Regenerate,
    /// Restore the corrupt durable artifact from a known-good copy.
    RestoreArtifact,
    /// Refresh or discard the stale optional cache.
    RefreshCache,
    /// Accept the fallback that was substituted (no action required).
    AcceptFallback,
}

impl RemedyAction {
    /// The stable border string for this action.
    pub fn as_str(self) -> &'static str {
        match self {
            RemedyAction::Inspect => "inspect",
            RemedyAction::ProvideAsset => "provideAsset",
            RemedyAction::FixReference => "fixReference",
            RemedyAction::BreakCycle => "breakCycle",
            RemedyAction::Regenerate => "regenerate",
            RemedyAction::RestoreArtifact => "restoreArtifact",
            RemedyAction::RefreshCache => "refreshCache",
            RemedyAction::AcceptFallback => "acceptFallback",
        }
    }
}

/// Every remedy-action string, in declaration order. Codegen source of truth.
pub const REMEDY_ACTIONS: &[&str] = &[
    "inspect",
    "provideAsset",
    "fixReference",
    "breakCycle",
    "regenerate",
    "restoreArtifact",
    "refreshCache",
    "acceptFallback",
];

/// A suggested remedy: a categorized action plus a human-readable detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestedRemedy {
    pub action: RemedyAction,
    pub detail: String,
}

impl SuggestedRemedy {
    /// Construct a remedy.
    pub fn new(action: RemedyAction, detail: impl Into<String>) -> Self {
        Self {
            action,
            detail: detail.into(),
        }
    }
}

// ── Source ref ────────────────────────────────────────────────────────────────

/// Where a diagnostic points in authority terms. Every field is optional;
/// fields are populated where the data exists and left `None` where the hop is
/// not applicable, so a consumer can tell "no scene node" from "unknown".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticSourceRef {
    /// Authored scene node id (`SceneNodeId` at the border: a plain integer).
    pub scene_node_id: Option<u64>,
    /// Runtime entity id (`EntityId` at the border: a plain integer).
    pub runtime_entity_id: Option<u64>,
    /// Scoped, kind-prefixed asset id (e.g. `mesh/belt-straight`).
    pub asset_id: Option<String>,
    /// Voxel chunk coordinate, when the diagnostic is chunk-local.
    pub chunk_coord: Option<[i64; 3]>,
    /// Retained-render handle, when the diagnostic is a projection trace.
    pub render_handle: Option<u64>,
    /// Bundle-relative artifact path, when the diagnostic is bundle-local.
    pub bundle_path: Option<String>,
}

impl DiagnosticSourceRef {
    /// An empty source ref (every hop unknown / not applicable).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Builder: set the scene node.
    pub fn with_scene_node(mut self, id: u64) -> Self {
        self.scene_node_id = Some(id);
        self
    }

    /// Builder: set the runtime entity.
    pub fn with_entity(mut self, id: u64) -> Self {
        self.runtime_entity_id = Some(id);
        self
    }

    /// Builder: set the asset id.
    pub fn with_asset(mut self, asset_id: impl Into<String>) -> Self {
        self.asset_id = Some(asset_id.into());
        self
    }

    /// Builder: set the chunk coordinate.
    pub fn with_chunk(mut self, coord: [i64; 3]) -> Self {
        self.chunk_coord = Some(coord);
        self
    }

    /// Builder: set the render handle.
    pub fn with_render_handle(mut self, handle: u64) -> Self {
        self.render_handle = Some(handle);
        self
    }

    /// Builder: set the bundle path.
    pub fn with_bundle_path(mut self, path: impl Into<String>) -> Self {
        self.bundle_path = Some(path.into());
        self
    }
}

// ── Report ────────────────────────────────────────────────────────────────────

/// One structured diagnostic. The agent-legible unit: scope + severity + stable
/// code + where it points + what to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticReport {
    /// Owning subsystem / lane.
    pub scope: DiagnosticScope,
    /// Seriousness / recovery path.
    pub severity: DiagnosticSeverity,
    /// Stable machine code.
    pub code: DiagnosticCode,
    /// Human-readable locus (a path, id, or short reference string).
    pub reference: String,
    /// Machine-routable source pointers.
    pub source: DiagnosticSourceRef,
    /// Human-readable message.
    pub message: String,
    /// Optional suggested remedy.
    pub remedy: Option<SuggestedRemedy>,
}

impl DiagnosticReport {
    /// Construct a report using the code's [`DiagnosticCode::default_severity`].
    pub fn new(
        code: DiagnosticCode,
        reference: impl Into<String>,
        source: DiagnosticSourceRef,
        message: impl Into<String>,
    ) -> Self {
        Self {
            scope: code.scope(),
            severity: code.default_severity(),
            code,
            reference: reference.into(),
            source,
            message: message.into(),
            remedy: None,
        }
    }

    /// Override the default severity (e.g. escalate a missing asset to `Fatal`).
    pub fn with_severity(mut self, severity: DiagnosticSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Attach a suggested remedy.
    pub fn with_remedy(mut self, remedy: SuggestedRemedy) -> Self {
        self.remedy = Some(remedy);
        self
    }
}

/// A collection of reports plus the aggregate severity policy over them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticReportSet {
    pub reports: Vec<DiagnosticReport>,
}

impl DiagnosticReportSet {
    /// An empty set (a clean validation).
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a report.
    pub fn push(&mut self, report: DiagnosticReport) {
        self.reports.push(report);
    }

    /// True when there are no reports.
    pub fn is_empty(&self) -> bool {
        self.reports.is_empty()
    }

    /// The most severe report's severity, or `None` for an empty set.
    pub fn max_severity(&self) -> Option<DiagnosticSeverity> {
        self.reports.iter().map(|r| r.severity).max()
    }

    /// How many reports carry a given severity.
    pub fn count_at(&self, severity: DiagnosticSeverity) -> usize {
        self.reports
            .iter()
            .filter(|r| r.severity == severity)
            .count()
    }

    /// True when any report is `Fatal` — the load must not proceed.
    pub fn blocks_load(&self) -> bool {
        self.reports.iter().any(|r| r.severity.blocks_load())
    }
}

// ── Source trace (render projection) ──────────────────────────────────────────

/// A render-handle → scene-node → entity → asset trace. The highest-value
/// projection diagnostic. Hops that do not apply are `None`; `asset_resolved`
/// records whether the asset ref resolved against the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceTrace {
    /// The retained-render handle this trace starts from.
    pub render_handle: u64,
    /// Authored scene node, where the handle projects one.
    pub scene_node_id: Option<u64>,
    /// Runtime entity, where the node maps to one.
    pub runtime_entity_id: Option<u64>,
    /// Asset ref the node/entity draws, where applicable.
    pub asset_id: Option<String>,
    /// Whether `asset_id` resolved to a valid catalog entry.
    pub asset_resolved: bool,
}

impl SourceTrace {
    /// A bare trace from a render handle, with every hop unknown.
    pub fn from_handle(render_handle: u64) -> Self {
        Self {
            render_handle,
            scene_node_id: None,
            runtime_entity_id: None,
            asset_id: None,
            asset_resolved: false,
        }
    }

    /// True when the chain is broken (no scene node OR an unresolved asset),
    /// i.e. it warrants a [`DiagnosticCode::MissingSourceTrace`].
    pub fn is_broken(&self) -> bool {
        self.scene_node_id.is_none() || (self.asset_id.is_some() && !self.asset_resolved)
    }
}

// ── Renderer resource report ──────────────────────────────────────────────────

/// An observational snapshot of renderer resource usage. Counts only — the
/// renderer never gains authority by reporting; consumers route, they do not act.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RendererResourceReport {
    /// Live retained render handles.
    pub live_handles: u32,
    /// Distinct uploaded geometries in memory.
    pub geometries: u32,
    /// Distinct materials in memory.
    pub materials: u32,
    /// Live sprite instances.
    pub sprite_instances: u32,
    /// Sprites updated on the most recent tick.
    pub sprites_updated_last_tick: u32,
    /// Resources created over the report window.
    pub resources_created: u32,
    /// Resources disposed over the report window.
    pub resources_disposed: u32,
    /// Fallback materials/textures currently substituted.
    pub fallback_materials: u32,
}

impl RendererResourceReport {
    /// Created-minus-disposed: a positive value is a leak suspicion.
    pub fn outstanding_resources(&self) -> i64 {
        i64::from(self.resources_created) - i64::from(self.resources_disposed)
    }

    /// True when more resources were created than disposed (leak hint).
    pub fn suspects_leak(&self) -> bool {
        self.outstanding_resources() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_orders_info_to_fatal() {
        assert!(DiagnosticSeverity::Info < DiagnosticSeverity::Warning);
        assert!(DiagnosticSeverity::Warning < DiagnosticSeverity::Error);
        assert!(DiagnosticSeverity::Error < DiagnosticSeverity::Fatal);
        assert!(!DiagnosticSeverity::Error.blocks_load());
        assert!(DiagnosticSeverity::Fatal.blocks_load());
    }

    #[test]
    fn severity_table_matches_variants() {
        let variants = [
            DiagnosticSeverity::Info,
            DiagnosticSeverity::Warning,
            DiagnosticSeverity::Error,
            DiagnosticSeverity::Fatal,
        ];
        let from_variants: Vec<&str> = variants.iter().map(|s| s.as_str()).collect();
        assert_eq!(from_variants, DIAGNOSTIC_SEVERITIES);
        // rank() agrees with declaration order.
        for (i, s) in variants.iter().enumerate() {
            assert_eq!(s.rank() as usize, i);
        }
    }

    #[test]
    fn scope_table_matches_variants() {
        let variants = [
            DiagnosticScope::Scene,
            DiagnosticScope::AssetCatalog,
            DiagnosticScope::WorldBundle,
            DiagnosticScope::RenderProjection,
            DiagnosticScope::RendererResources,
            DiagnosticScope::WorldComposition,
        ];
        let from_variants: Vec<&str> = variants.iter().map(|s| s.as_str()).collect();
        assert_eq!(from_variants, DIAGNOSTIC_SCOPES);
    }

    /// Consolidation guarantee (#2368): every diagnostic scope — scene, asset
    /// catalog, world bundle, render projection, renderer resources, and world
    /// composition — is reachable by at least one stable code. A diagnostic
    /// source that has no code to map into would be a hole in the taxonomy.
    #[test]
    fn every_scope_has_at_least_one_code() {
        for scope_str in DIAGNOSTIC_SCOPES {
            assert!(
                ALL_DIAGNOSTIC_CODES
                    .iter()
                    .any(|c| c.scope().as_str() == *scope_str),
                "scope `{scope_str}` has no diagnostic code mapping into it"
            );
        }
    }

    /// The world-composition codes added in #2368 carry the expected recovery
    /// policy: load/final-consistency failures block a load (Fatal); a round-trip
    /// equivalence loss is a correctness Error but not itself a load blocker.
    #[test]
    fn world_composition_codes_have_expected_policy() {
        assert_eq!(
            DiagnosticCode::LoadStageFailed.scope(),
            DiagnosticScope::WorldComposition
        );
        assert_eq!(
            DiagnosticCode::LoadStageFailed.default_severity(),
            DiagnosticSeverity::Fatal
        );
        assert_eq!(
            DiagnosticCode::FinalConsistencyMismatch.default_severity(),
            DiagnosticSeverity::Fatal
        );
        assert_eq!(
            DiagnosticCode::RoundTripMismatch.default_severity(),
            DiagnosticSeverity::Error
        );
        assert!(!DiagnosticCode::RoundTripMismatch
            .default_severity()
            .blocks_load());
    }

    /// The codegen source-of-truth tables must mirror the enum exactly: every
    /// variant's `as_str` appears in the const list, in order, with no extras.
    /// This is what keeps the generated TS union honest.
    #[test]
    fn codes_table_matches_variants() {
        let from_variants: Vec<&str> = ALL_DIAGNOSTIC_CODES.iter().map(|c| c.as_str()).collect();
        assert_eq!(from_variants, DIAGNOSTIC_CODES);
        // Every code's scope/severity is well defined and string is unique.
        let mut seen = std::collections::BTreeSet::new();
        for code in ALL_DIAGNOSTIC_CODES {
            assert!(
                seen.insert(code.as_str()),
                "duplicate code {}",
                code.as_str()
            );
            // scope/default_severity are total — these calls must not panic.
            let _ = code.scope();
            let _ = code.default_severity();
        }
    }

    #[test]
    fn remedy_table_matches_variants() {
        let variants = [
            RemedyAction::Inspect,
            RemedyAction::ProvideAsset,
            RemedyAction::FixReference,
            RemedyAction::BreakCycle,
            RemedyAction::Regenerate,
            RemedyAction::RestoreArtifact,
            RemedyAction::RefreshCache,
            RemedyAction::AcceptFallback,
        ];
        let from_variants: Vec<&str> = variants.iter().map(|a| a.as_str()).collect();
        assert_eq!(from_variants, REMEDY_ACTIONS);
    }

    #[test]
    fn required_codes_exist_with_expected_policy() {
        // Acceptance criteria for #2330: these seven stable codes must exist.
        assert_eq!(
            DiagnosticCode::DuplicateSceneId.scope(),
            DiagnosticScope::Scene
        );
        assert_eq!(
            DiagnosticCode::MissingAsset.scope(),
            DiagnosticScope::AssetCatalog
        );
        assert_eq!(
            DiagnosticCode::WrongKindAssetRef.scope(),
            DiagnosticScope::AssetCatalog
        );
        assert_eq!(
            DiagnosticCode::AssetCycle.scope(),
            DiagnosticScope::AssetCatalog
        );
        assert_eq!(
            DiagnosticCode::CorruptBundleArtifact.default_severity(),
            DiagnosticSeverity::Fatal
        );
        assert_eq!(
            DiagnosticCode::GeneratorMismatch.default_severity(),
            DiagnosticSeverity::Fatal
        );
        assert_eq!(
            DiagnosticCode::FallbackUsed.default_severity(),
            DiagnosticSeverity::Warning
        );
    }

    #[test]
    fn report_set_aggregates_severity_and_load_policy() {
        let mut set = DiagnosticReportSet::new();
        assert!(set.is_empty());
        assert_eq!(set.max_severity(), None);
        assert!(!set.blocks_load());

        set.push(DiagnosticReport::new(
            DiagnosticCode::StaleAsset,
            "mesh/belt-straight",
            DiagnosticSourceRef::empty().with_asset("mesh/belt-straight"),
            "asset is one version behind the lock",
        ));
        assert_eq!(set.max_severity(), Some(DiagnosticSeverity::Warning));
        assert!(!set.blocks_load());

        set.push(
            DiagnosticReport::new(
                DiagnosticCode::CorruptBundleArtifact,
                "chunks/0_0_0.snap",
                DiagnosticSourceRef::empty().with_bundle_path("chunks/0_0_0.snap"),
                "durable artifact failed its content hash",
            )
            .with_remedy(SuggestedRemedy::new(
                RemedyAction::RestoreArtifact,
                "restore from a known-good bundle copy",
            )),
        );
        assert_eq!(set.max_severity(), Some(DiagnosticSeverity::Fatal));
        assert!(set.blocks_load());
        assert_eq!(set.count_at(DiagnosticSeverity::Fatal), 1);
        assert_eq!(set.count_at(DiagnosticSeverity::Warning), 1);
    }

    #[test]
    fn source_trace_detects_broken_chains() {
        let complete = SourceTrace {
            render_handle: 42,
            scene_node_id: Some(7),
            runtime_entity_id: Some(123),
            asset_id: Some("mesh/belt-straight".to_string()),
            asset_resolved: true,
        };
        assert!(!complete.is_broken());

        let no_node = SourceTrace::from_handle(43);
        assert!(no_node.is_broken());

        let unresolved = SourceTrace {
            render_handle: 44,
            scene_node_id: Some(8),
            runtime_entity_id: Some(456),
            asset_id: Some("sprite/hard-hat".to_string()),
            asset_resolved: false,
        };
        assert!(unresolved.is_broken());
    }

    #[test]
    fn resource_report_flags_leak_when_created_exceeds_disposed() {
        let balanced = RendererResourceReport {
            resources_created: 10,
            resources_disposed: 10,
            ..Default::default()
        };
        assert_eq!(balanced.outstanding_resources(), 0);
        assert!(!balanced.suspects_leak());

        let leaking = RendererResourceReport {
            resources_created: 12,
            resources_disposed: 9,
            ..Default::default()
        };
        assert_eq!(leaking.outstanding_resources(), 3);
        assert!(leaking.suspects_leak());
    }
}
