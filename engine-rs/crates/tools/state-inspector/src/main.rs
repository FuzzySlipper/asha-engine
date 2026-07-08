//! `state-inspector` — deterministic readout for ASHA authority state artifacts.
//!
//! The currently committed structured runtime state artifact is the
//! `core-entity` session-state snapshot JSON under `harness/fixtures/session-state`.
//! This tool decodes that authority snapshot, rebuilds the `EntityStore`, and
//! prints stable summaries without mutating state or touching renderer/UI paths.
//!
//! Commands:
//!   state-inspector summary <session-state.snapshot.json>
//!   state-inspector entity <session-state.snapshot.json> <entity-id>
//!   state-inspector category <session-state.snapshot.json> <category>
//!   state-inspector --help
//!
//! Exit codes: 0 = ok, 1 = missing query result, 2 = malformed/read error,
//! 3 = usage error.

use std::io::Write;
use std::process::ExitCode;

use core_entity::{
    decode_snapshot, EntityHash, EntityLifecycle, EntityRecord, EntitySnapshot, EntitySource,
    EntityStore, SNAPSHOT_SCHEMA_VERSION,
};

const USAGE: &str = "\
state-inspector — inspect ASHA authority session-state snapshots

USAGE:
    state-inspector summary <session-state.snapshot.json>
    state-inspector entity <session-state.snapshot.json> <entity-id>
    state-inspector category <session-state.snapshot.json> <category>
    state-inspector --help

COMMANDS:
    summary   Decode a canonical core-entity session-state snapshot and print
              deterministic counts, hash, source counts, lifecycle counts, and
              capability counts.

    entity    Print a focused readout for one entity id.

    category  Print entity ids matching one category. Supported categories:
              all, active, disabled, tombstoned, spatial, non-spatial,
              rendered, colliding, contained, asset-bound.

EXIT CODES:
    0 ok
    1 missing entity or empty category
    2 malformed artifact or read error
    3 usage error
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&args, &mut std::io::stdout(), &mut std::io::stderr());
    ExitCode::from(code)
}

fn run<O: Write, E: Write>(args: &[String], out: &mut O, err: &mut E) -> u8 {
    match args.first().map(String::as_str) {
        None | Some("--help") | Some("-h") | Some("help") => {
            let _ = write!(out, "{USAGE}");
            if args.is_empty() {
                3
            } else {
                0
            }
        }
        Some("summary") => cmd_summary(&args[1..], out, err),
        Some("entity") => cmd_entity(&args[1..], out, err),
        Some("category") => cmd_category(&args[1..], out, err),
        Some(other) => {
            let _ = writeln!(err, "error: unknown command '{other}'\n");
            let _ = write!(err, "{USAGE}");
            3
        }
    }
}

fn cmd_summary<O: Write, E: Write>(args: &[String], out: &mut O, err: &mut E) -> u8 {
    let Some(path) = only_path(args, "summary", err) else {
        return 3;
    };
    let Some(snapshot) = read_snapshot(path, err) else {
        return 2;
    };

    let report = SnapshotReport::from_snapshot(snapshot);
    write_summary(&report, out);
    0
}

fn cmd_entity<O: Write, E: Write>(args: &[String], out: &mut O, err: &mut E) -> u8 {
    let [path, id] = args else {
        let _ = writeln!(
            err,
            "error: `entity` requires <session-state.snapshot.json> <entity-id>"
        );
        return 3;
    };
    let entity_id = match id.parse::<u64>() {
        Ok(value) => value,
        Err(e) => {
            let _ = writeln!(err, "error: invalid entity id {id:?}: {e}");
            return 3;
        }
    };
    let Some(snapshot) = read_snapshot(path, err) else {
        return 2;
    };

    let report = SnapshotReport::from_snapshot(snapshot);
    let Some(record) = report
        .snapshot
        .records
        .iter()
        .find(|record| record.core.id.raw() == entity_id)
    else {
        let _ = writeln!(err, "missing entity: {entity_id}");
        return 1;
    };

    write_entity(record, &mut *out);
    0
}

fn cmd_category<O: Write, E: Write>(args: &[String], out: &mut O, err: &mut E) -> u8 {
    let [path, category] = args else {
        let _ = writeln!(
            err,
            "error: `category` requires <session-state.snapshot.json> <category>"
        );
        return 3;
    };
    let Some(snapshot) = read_snapshot(path, err) else {
        return 2;
    };

    let report = SnapshotReport::from_snapshot(snapshot);
    if !is_supported_category(category) {
        let _ = writeln!(err, "error: unsupported category {category:?}");
        return 3;
    }

    let ids: Vec<String> = report
        .snapshot
        .records
        .iter()
        .filter(|record| matches_category(record, category))
        .map(|record| record.core.id.raw().to_string())
        .collect();

    if ids.is_empty() {
        let _ = writeln!(err, "empty category: {category}");
        return 1;
    }

    let _ = writeln!(out, "category: {category}");
    let _ = writeln!(out, "entities: [{}]", ids.join(","));
    0
}

fn only_path<'a, E: Write>(args: &'a [String], command: &str, err: &mut E) -> Option<&'a str> {
    match args {
        [path] => Some(path.as_str()),
        _ => {
            let _ = writeln!(
                err,
                "error: `{command}` requires <session-state.snapshot.json>"
            );
            None
        }
    }
}

fn read_snapshot<E: Write>(path: &str, err: &mut E) -> Option<EntitySnapshot> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            let _ = writeln!(err, "error: cannot read {path}: {e}");
            return None;
        }
    };
    match decode_snapshot(&text) {
        Ok(snapshot) => Some(snapshot),
        Err(e) => {
            let _ = writeln!(err, "error: malformed snapshot {path}: {e}");
            None
        }
    }
}

#[derive(Debug, Clone)]
struct SnapshotReport {
    snapshot: EntitySnapshot,
    hash: EntityHash,
    total: usize,
    active: usize,
    disabled: usize,
    tombstoned: usize,
    source_counts: SourceCounts,
    capability_counts: CapabilityCounts,
}

impl SnapshotReport {
    fn from_snapshot(snapshot: EntitySnapshot) -> Self {
        let store = EntityStore::from_snapshot(snapshot.clone());
        let hash = store.hash();
        let mut active = 0usize;
        let mut disabled = 0usize;
        let mut tombstoned = 0usize;
        let mut source_counts = SourceCounts::default();
        let mut capability_counts = CapabilityCounts::default();

        for record in &snapshot.records {
            match record.core.lifecycle {
                EntityLifecycle::Active => active += 1,
                EntityLifecycle::Disabled => disabled += 1,
                EntityLifecycle::Tombstoned => tombstoned += 1,
            }
            source_counts.add(&record.core.source);
            capability_counts.add(record);
        }

        Self {
            total: snapshot.records.len(),
            snapshot,
            hash,
            active,
            disabled,
            tombstoned,
            source_counts,
            capability_counts,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct SourceCounts {
    scene_bootstrap: usize,
    runtime_created: usize,
    imported: usize,
    diagnostic_tooling: usize,
    policy_proposed: usize,
}

impl SourceCounts {
    fn add(&mut self, source: &EntitySource) {
        match source {
            EntitySource::SceneBootstrap { .. } => self.scene_bootstrap += 1,
            EntitySource::RuntimeCreated { .. } => self.runtime_created += 1,
            EntitySource::Imported { .. } => self.imported += 1,
            EntitySource::DiagnosticTooling => self.diagnostic_tooling += 1,
            EntitySource::PolicyProposed { .. } => self.policy_proposed += 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct CapabilityCounts {
    transform: usize,
    bounds: usize,
    render: usize,
    collision: usize,
    containment: usize,
    controller: usize,
    asset_binding: usize,
    transform_parent: usize,
    derived_from: usize,
}

impl CapabilityCounts {
    fn add(&mut self, record: &EntityRecord) {
        self.transform += usize::from(record.transform.is_some());
        self.bounds += usize::from(record.bounds.is_some());
        self.render += usize::from(record.render.is_some());
        self.collision += usize::from(record.collision.is_some());
        self.containment += usize::from(record.containment.is_some());
        self.controller += usize::from(record.controller.is_some());
        self.asset_binding += usize::from(record.asset_binding.is_some());
        self.transform_parent += usize::from(record.transform_parent.is_some());
        self.derived_from += usize::from(record.derived_from.is_some());
    }
}

fn write_summary<O: Write>(report: &SnapshotReport, out: &mut O) {
    let _ = writeln!(out, "artifact: session-state-snapshot");
    let _ = writeln!(out, "schema_version: {SNAPSHOT_SCHEMA_VERSION}");
    let _ = writeln!(out, "entity_hash: {}", format_hash(report.hash));
    let _ = writeln!(
        out,
        "entities: total={} active={} disabled={} tombstoned={}",
        report.total, report.active, report.disabled, report.tombstoned
    );
    let _ = writeln!(
        out,
        "sources: sceneBootstrap={} runtimeCreated={} imported={} diagnosticTooling={} policyProposed={}",
        report.source_counts.scene_bootstrap,
        report.source_counts.runtime_created,
        report.source_counts.imported,
        report.source_counts.diagnostic_tooling,
        report.source_counts.policy_proposed
    );
    let _ = writeln!(
        out,
        "capabilities: transform={} bounds={} render={} collision={} containment={} controller={} assetBinding={} transformParent={} derivedFrom={}",
        report.capability_counts.transform,
        report.capability_counts.bounds,
        report.capability_counts.render,
        report.capability_counts.collision,
        report.capability_counts.containment,
        report.capability_counts.controller,
        report.capability_counts.asset_binding,
        report.capability_counts.transform_parent,
        report.capability_counts.derived_from
    );
    let ids: Vec<String> = report
        .snapshot
        .records
        .iter()
        .map(|record| record.core.id.raw().to_string())
        .collect();
    let _ = writeln!(out, "entity_ids: [{}]", ids.join(","));
}

fn write_entity<O: Write>(record: &EntityRecord, out: &mut O) {
    let _ = writeln!(out, "entity: {}", record.core.id.raw());
    let _ = writeln!(out, "lifecycle: {}", record.core.lifecycle.label());
    let _ = writeln!(out, "source: {}", record.core.source.label());
    let labels: Vec<String> = record
        .core
        .labels
        .iter()
        .map(|label| label.raw().to_string())
        .collect();
    let _ = writeln!(out, "labels: [{}]", labels.join(","));
    let _ = writeln!(
        out,
        "capabilities: [{}]",
        capability_names(record).join(",")
    );
    let _ = writeln!(out, "relations: [{}]", relation_names(record).join(","));
}

fn capability_names(record: &EntityRecord) -> Vec<&'static str> {
    let mut names = Vec::new();
    if record.transform.is_some() {
        names.push("transform");
    }
    if record.bounds.is_some() {
        names.push("bounds");
    }
    if record.render.is_some() {
        names.push("render");
    }
    if record.collision.is_some() {
        names.push("collision");
    }
    if record.containment.is_some() {
        names.push("containment");
    }
    if record.controller.is_some() {
        names.push("controller");
    }
    if record.asset_binding.is_some() {
        names.push("assetBinding");
    }
    names
}

fn relation_names(record: &EntityRecord) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(parent) = record.transform_parent {
        names.push(format!("transformParent={}", parent.raw()));
    }
    if let Some(origin) = record.derived_from {
        names.push(format!("derivedFrom={}", origin.raw()));
    }
    names
}

fn is_supported_category(category: &str) -> bool {
    matches!(
        category,
        "all"
            | "active"
            | "disabled"
            | "tombstoned"
            | "spatial"
            | "non-spatial"
            | "rendered"
            | "colliding"
            | "contained"
            | "asset-bound"
    )
}

fn matches_category(record: &EntityRecord, category: &str) -> bool {
    match category {
        "all" => true,
        "active" => record.core.lifecycle == EntityLifecycle::Active,
        "disabled" => record.core.lifecycle == EntityLifecycle::Disabled,
        "tombstoned" => record.core.lifecycle == EntityLifecycle::Tombstoned,
        "spatial" => record.transform.is_some(),
        "non-spatial" => record.transform.is_none(),
        "rendered" => record.render.is_some(),
        "colliding" => record.collision.is_some(),
        "contained" => record.containment.is_some(),
        "asset-bound" => record.asset_binding.is_some(),
        _ => false,
    }
}

fn format_hash(hash: EntityHash) -> String {
    format!("{:016x}", hash.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .expect("repo root")
            .to_path_buf()
    }

    fn fixture() -> String {
        repo_root()
            .join("harness/fixtures/session-state/mixed-world.snapshot.json")
            .to_string_lossy()
            .into_owned()
    }

    fn run_str(args: &[&str]) -> (u8, String, String) {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run(&owned, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn help_is_stable() {
        let (code, out, err) = run_str(&["--help"]);
        assert_eq!(code, 0);
        assert!(err.is_empty());
        assert!(out.contains("USAGE:"));
        assert!(out.contains("category"));
    }

    #[test]
    fn valid_fixture_summary_reports_authority_counts() {
        let path = fixture();
        let (code, out, err) = run_str(&["summary", &path]);
        assert_eq!(code, 0);
        assert!(err.is_empty());
        assert!(out.contains("artifact: session-state-snapshot"));
        assert!(out.contains("entity_hash: 52a209a7aa37a092"));
        assert!(out.contains("entities: total=6 active=5 disabled=0 tombstoned=1"));
        assert!(out.contains("entity_ids: [1,2,3,4,5,6]"));
    }

    #[test]
    fn focused_entity_query_reports_capabilities_and_relations() {
        let path = fixture();
        let (code, out, err) = run_str(&["entity", &path, "5"]);
        assert_eq!(code, 0);
        assert!(err.is_empty());
        assert!(out.contains("entity: 5"));
        assert!(out.contains("source: imported"));
        assert!(out.contains("capabilities: [transform,assetBinding]"));
        assert!(out.contains("relations: [transformParent=1,derivedFrom=4]"));
    }

    #[test]
    fn missing_entity_exits_one() {
        let path = fixture();
        let (code, out, err) = run_str(&["entity", &path, "777"]);
        assert_eq!(code, 1);
        assert!(out.is_empty());
        assert!(err.contains("missing entity: 777"));
    }

    #[test]
    fn category_query_reports_ids() {
        let path = fixture();
        let (code, out, err) = run_str(&["category", &path, "colliding"]);
        assert_eq!(code, 0);
        assert!(err.is_empty());
        assert!(out.contains("category: colliding"));
        assert!(out.contains("entities: [2]"));
    }

    #[test]
    fn malformed_snapshot_exits_two() {
        let path =
            std::env::temp_dir().join(format!("state-inspector-bad-{}.json", std::process::id()));
        std::fs::write(&path, "{ not json").unwrap();

        let (code, out, err) = run_str(&["summary", path.to_str().unwrap()]);
        std::fs::remove_file(&path).ok();

        assert_eq!(code, 2);
        assert!(out.is_empty());
        assert!(err.contains("malformed snapshot"));
    }
}
