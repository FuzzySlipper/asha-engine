//! Deterministic Rust-to-TypeScript contract generator for the ASHA protocol
//! border.
//!
//! # Lane
//!
//! `contract-steward` — turns the Rust protocol source crates into the committed
//! TypeScript contracts under `ts/packages/contracts/src/generated`. May depend
//! on the `protocol-*` crates; it owns no product logic.
//!
//! # What this does
//!
//! The Rust protocol crates (`protocol-ids`, `protocol-script`,
//! `protocol-render`, `protocol-replay`) are the source of truth for the border
//! shapes. [`crate::model`] describes those shapes as a small TypeScript IR
//! ([`crate::schema`]), and [`generated_files`] renders that IR to deterministic
//! `.ts` source. The generator never reads the existing TypeScript; it produces
//! the canonical bytes from scratch every time, so output is reproducible.
//!
//! The binary (`src/main.rs`) writes those files in *generate* mode or compares
//! them against what is committed in *check* mode (`--check`), which is the
//! entrypoint `harness/ci/check-contracts.sh` builds on.

pub mod model;
pub mod schema;

use std::path::{Path, PathBuf};

/// Directory (relative to the repo root) that generated contracts are written to.
pub const OUTPUT_DIR: &str = "ts/packages/contracts/src/generated";

/// One generated file: its repo-relative path and full rendered contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    /// Path relative to the repository root, e.g.
    /// `ts/packages/contracts/src/generated/ids.ts`.
    pub rel_path: String,
    /// Complete, deterministic file contents (UTF-8, LF, trailing newline).
    pub contents: String,
}

/// Render every generated contract file. Pure and deterministic: calling this
/// twice yields byte-identical results and it touches no filesystem.
pub fn generated_files() -> Vec<GeneratedFile> {
    model::all_modules()
        .iter()
        .map(|module| GeneratedFile {
            rel_path: format!("{OUTPUT_DIR}/{}.ts", module.name),
            contents: schema::render_module(module),
        })
        .collect()
}

/// The repository root, derived from this crate's compile-time location:
/// `<repo>/engine-rs/crates/protocol/protocol-codegen` → up four components.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("protocol-codegen is nested four levels under the repo root")
        .to_path_buf()
}

/// A single mismatch found in [`check_against`] check mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drift {
    /// The committed file is missing entirely.
    Missing { rel_path: String },
    /// The committed file differs from freshly generated output.
    Changed {
        rel_path: String,
        /// First differing line (1-based), for a precise pointer.
        first_diff_line: usize,
    },
}

impl Drift {
    /// A human-readable, source-pointing description (never a generic panic).
    pub fn describe(&self) -> String {
        match self {
            Drift::Missing { rel_path } => format!(
                "missing generated file: {rel_path}\n  \
                 regenerate with `cargo run -p protocol-codegen`"
            ),
            Drift::Changed {
                rel_path,
                first_diff_line,
            } => format!(
                "generated file is stale: {rel_path} (first differs at line {first_diff_line})\n  \
                 a protocol source change was not regenerated; run `cargo run -p protocol-codegen`"
            ),
        }
    }
}

/// Compare freshly generated output against the files on disk under `root`,
/// returning every drift found. An empty result means everything is in sync.
pub fn check_against(root: &Path) -> Vec<Drift> {
    let mut drifts = Vec::new();
    for file in generated_files() {
        let path = root.join(&file.rel_path);
        match std::fs::read_to_string(&path) {
            Ok(existing) if existing == file.contents => {}
            Ok(existing) => drifts.push(Drift::Changed {
                rel_path: file.rel_path.clone(),
                first_diff_line: first_diff_line(&existing, &file.contents),
            }),
            Err(_) => drifts.push(Drift::Missing {
                rel_path: file.rel_path.clone(),
            }),
        }
    }
    drifts
}

/// 1-based index of the first line that differs between `a` and `b`.
fn first_diff_line(a: &str, b: &str) -> usize {
    let mut a_lines = a.lines();
    let mut b_lines = b.lines();
    let mut line = 1usize;
    loop {
        match (a_lines.next(), b_lines.next()) {
            (Some(x), Some(y)) if x == y => line += 1,
            (None, None) => return line,
            _ => return line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(
            generated_files(),
            generated_files(),
            "generator output must be byte-identical across runs"
        );
    }

    #[test]
    fn emits_the_expected_file_set_in_order() {
        let paths: Vec<String> = generated_files().into_iter().map(|f| f.rel_path).collect();
        assert_eq!(
            paths,
            vec![
                format!("{OUTPUT_DIR}/ids.ts"),
                format!("{OUTPUT_DIR}/script.ts"),
                format!("{OUTPUT_DIR}/render.ts"),
                format!("{OUTPUT_DIR}/replay.ts"),
                format!("{OUTPUT_DIR}/voxel.ts"),
                format!("{OUTPUT_DIR}/scene.ts"),
                format!("{OUTPUT_DIR}/worldBundle.ts"),
                format!("{OUTPUT_DIR}/assets.ts"),
                format!("{OUTPUT_DIR}/diagnostics.ts"),
                format!("{OUTPUT_DIR}/policyView.ts"),
                format!("{OUTPUT_DIR}/entityAuthoring.ts"),
                format!("{OUTPUT_DIR}/index.ts"),
            ],
        );
    }

    fn file(name: &str) -> String {
        generated_files()
            .into_iter()
            .find(|f| f.rel_path.ends_with(&format!("/{name}")))
            .unwrap_or_else(|| panic!("no generated file {name}"))
            .contents
    }

    #[test]
    fn every_file_carries_the_do_not_edit_banner() {
        for f in generated_files() {
            assert!(
                f.contents
                    .contains("@generated by protocol-codegen — DO NOT EDIT."),
                "{} is missing the banner",
                f.rel_path
            );
        }
    }

    /// Focused behavior test for the `ids` protocol family: every branded ID
    /// from `protocol-ids` is emitted as a branded type plus a constructor.
    #[test]
    fn ids_family_emits_branded_types_and_constructors() {
        let ids = file("ids.ts");
        for border in protocol_ids::BORDER_IDS {
            let brand = border.brand;
            assert!(
                ids.contains(&format!(
                    "export type {brand} = number & {{ readonly __brand: '{brand}' }};"
                )),
                "missing branded type for {brand}"
            );
        }
        assert!(ids.contains("export const entityId = (raw: number): EntityId => raw as EntityId;"));
    }

    /// Focused behavior test for the `script` family: the command union and a
    /// representative variant are present and well-formed.
    #[test]
    fn script_family_emits_command_union() {
        let script = file("script.ts");
        assert!(script.contains("import type { EntityId, SubjectId, ProcessId, ModeId, SignalId, TagId } from './ids.js';"));
        assert!(script.contains("export type Command =\n"));
        assert!(
            script.contains("  | { readonly domain: 'entity'; readonly command: EntityCommand }")
        );
        assert!(script.contains("export type CommandKind = 'input' | 'policy' | 'system';"));
        assert!(script.contains(
            "  | { readonly kind: 'addTag'; readonly id: EntityId; readonly tag: TagId }"
        ));
    }

    /// Focused behavior test for the `diagnostics` family: every stable code,
    /// severity, scope, and remedy from `protocol-diagnostics` is emitted, plus
    /// the report/trace/resource shapes. This is the "Rust and generated TS
    /// diagnostic contracts agree" guard for #2330.
    #[test]
    fn diagnostics_family_emits_codes_and_report_shapes() {
        let d = file("diagnostics.ts");
        for code in protocol_diagnostics::DIAGNOSTIC_CODES {
            assert!(d.contains(&format!("'{code}'")), "missing code {code}");
        }
        for sev in protocol_diagnostics::DIAGNOSTIC_SEVERITIES {
            assert!(d.contains(&format!("'{sev}'")), "missing severity {sev}");
        }
        for scope in protocol_diagnostics::DIAGNOSTIC_SCOPES {
            assert!(d.contains(&format!("'{scope}'")), "missing scope {scope}");
        }
        for action in protocol_diagnostics::REMEDY_ACTIONS {
            assert!(
                d.contains(&format!("'{action}'")),
                "missing remedy {action}"
            );
        }
        assert!(d.contains("export interface DiagnosticReport {"));
        assert!(d.contains("export interface DiagnosticReportSet {"));
        assert!(d.contains("export interface DiagnosticSourceRef {"));
        assert!(d.contains("export interface SourceTrace {"));
        assert!(d.contains("export interface RendererResourceReport {"));
        assert!(d.contains("readonly chunkCoord: readonly [number, number, number] | null;"));
    }

    /// Focused behavior test for the `scene` family: the node-kind tags and
    /// validation codes are sourced from `protocol-scene`, the branded scene ids
    /// are emitted, and the document/validation/trace/bootstrap shapes exist.
    /// This is the "Rust and generated TS scene contracts agree" guard for #2365.
    #[test]
    fn scene_family_emits_tags_codes_and_shapes() {
        let s = file("scene.ts");
        for tag in protocol_scene::SCENE_NODE_KIND_TAGS {
            assert!(
                s.contains(&format!("'{tag}'")),
                "missing node-kind tag {tag}"
            );
        }
        for code in protocol_scene::SCENE_VALIDATION_CODES {
            assert!(
                s.contains(&format!("'{code}'")),
                "missing validation code {code}"
            );
        }
        assert!(s.contains("export type SceneId ="));
        assert!(s.contains("export type WorldId ="));
        assert!(s.contains("export type SceneNodeId ="));
        assert!(s.contains("export interface FlatSceneDocument {"));
        assert!(s.contains("export interface SceneNodeRecord {"));
        assert!(s.contains("export interface SceneValidationReport {"));
        assert!(s.contains("export interface SceneSourceTrace {"));
        assert!(s.contains("export interface BootstrapRecord {"));
        // Scene reuses the runtime EntityId brand from ids.ts for source traces.
        assert!(s.contains("import type { EntityId } from './ids.js';"));
    }

    /// Focused behavior test for the `worldBundle` family: artifact classes,
    /// load stages, and suggested actions are sourced from `protocol-world-bundle`,
    /// and the manifest/load-plan/save/regen shapes exist with the right imports.
    /// This is the "Rust and generated TS world-bundle contracts agree" guard (#2366).
    #[test]
    fn world_bundle_family_emits_vocab_and_shapes() {
        let w = file("worldBundle.ts");
        for class in protocol_world_bundle::ARTIFACT_CLASSES {
            assert!(
                w.contains(&format!("'{class}'")),
                "missing artifact class {class}"
            );
        }
        for stage in protocol_world_bundle::LOAD_STAGES {
            assert!(
                w.contains(&format!("'{stage}'")),
                "missing load stage {stage}"
            );
        }
        for action in protocol_world_bundle::SUGGESTED_ACTIONS {
            assert!(
                w.contains(&format!("'{action}'")),
                "missing suggested action {action}"
            );
        }
        assert!(w.contains("export interface WorldBundleManifest {"));
        assert!(w.contains("export interface LoadPlan {"));
        assert!(w.contains("export type LoadStep ="));
        assert!(w.contains("export type LoadPlanError ="));
        assert!(w.contains("export interface SaveSummary {"));
        assert!(w.contains("export interface RegenConflictReport {"));
        assert!(w.contains("import type { SceneId, WorldId } from './scene.js';"));
        assert!(w.contains("import type { VoxelCoord, VoxelValue } from './voxel.js';"));
    }

    /// Focused behavior test for the `assets` family: kind/validation/lock/uv/
    /// structural vocabularies are emitted, the disjoint material projections keep
    /// their split (RenderMaterial has no collision class; CollisionMaterial has no
    /// texture/colour), and the catalog/lock/fallback shapes exist. Guard for #2367.
    #[test]
    fn assets_family_keeps_material_split_and_emits_vocab() {
        let a = file("assets.ts");
        for kind in protocol_assets::ASSET_KINDS {
            assert!(
                a.contains(&format!("'{kind}'")),
                "missing asset kind {kind}"
            );
        }
        for code in protocol_assets::CATALOG_VALIDATION_CODES {
            assert!(
                a.contains(&format!("'{code}'")),
                "missing catalog code {code}"
            );
        }
        for code in protocol_assets::LOCK_ISSUE_CODES {
            assert!(a.contains(&format!("'{code}'")), "missing lock code {code}");
        }
        // The authority/style split must survive to the border: the render
        // projection names no collision field, the collision projection no texture.
        let render = a
            .split("export interface RenderMaterial {")
            .nth(1)
            .and_then(|s| s.split('}').next())
            .unwrap_or("");
        assert!(!render.contains("collidable") && !render.contains("structuralClass"));
        let collision = a
            .split("export interface CollisionMaterial {")
            .nth(1)
            .and_then(|s| s.split('}').next())
            .unwrap_or("");
        assert!(!collision.contains("texture") && !collision.contains("color"));
        assert!(a.contains("export interface CatalogValidationReport {"));
        assert!(a.contains("export interface LockValidationReport {"));
        assert!(a.contains("export type FallbackDecision ="));
        assert!(a.contains("import type { AssetReference } from './scene.js';"));
    }

    #[test]
    fn replay_const_is_sourced_from_protocol_replay() {
        let replay = file("replay.ts");
        assert!(replay.contains(&format!(
            "export const REPLAY_FORMAT_VERSION = {};",
            protocol_replay::REPLAY_FORMAT_VERSION
        )));
        assert!(replay.contains("import type { CommandEnvelope } from './script.js';"));
    }

    #[test]
    fn first_diff_line_points_at_the_change() {
        assert_eq!(first_diff_line("a\nb\nc", "a\nb\nc"), 4);
        assert_eq!(first_diff_line("a\nb\nc", "a\nX\nc"), 2);
        assert_eq!(first_diff_line("a\nb", "a\nb\nc"), 3);
    }

    #[test]
    fn check_against_reports_missing_when_nothing_written() {
        // An empty temp dir has none of the generated files.
        let tmp = std::env::temp_dir().join(format!("asha-codegen-check-{}", std::process::id()));
        let drifts = check_against(&tmp);
        assert_eq!(drifts.len(), generated_files().len());
        assert!(drifts.iter().all(|d| matches!(d, Drift::Missing { .. })));
        assert!(drifts[0].describe().contains("regenerate"));
    }

    /// The CI guard's core contract: a hand-edit to a generated file is detected
    /// as drift. Mirrors what `harness/ci/check-contracts.sh --check` enforces,
    /// proven here against a temp tree so no real file is mutated.
    #[test]
    fn tampered_generated_file_is_detected_as_changed() {
        let root = std::env::temp_dir().join(format!(
            "asha-codegen-tamper-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));

        // Write a clean, in-sync tree first.
        for f in generated_files() {
            let path = root.join(&f.rel_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &f.contents).unwrap();
        }
        assert!(
            check_against(&root).is_empty(),
            "freshly written tree must be in sync"
        );

        // Hand-edit one generated file, as a careless human would.
        let ids_path = root.join(format!("{OUTPUT_DIR}/ids.ts"));
        let mut tampered = std::fs::read_to_string(&ids_path).unwrap();
        tampered.push_str("\nexport type Sneaky = string;\n");
        std::fs::write(&ids_path, &tampered).unwrap();

        let drifts = check_against(&root);
        assert_eq!(drifts.len(), 1, "only the edited file should drift");
        match &drifts[0] {
            Drift::Changed { rel_path, .. } => assert!(rel_path.ends_with("/ids.ts")),
            other => panic!("expected Changed drift, got {other:?}"),
        }
        assert!(drifts[0]
            .describe()
            .contains("cargo run -p protocol-codegen"));

        std::fs::remove_dir_all(&root).ok();
    }
}
