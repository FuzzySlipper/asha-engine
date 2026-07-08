//! Shared deterministic renderer for the bootstrap golden fixture, used by both
//! the `dump_bootstrap_summary` example and the `bootstrap_golden` test so the
//! committed bytes have a single source of truth.
//!
//! Not a test module itself (no `#[test]`); it is `#[path]`-included by both.
#![allow(dead_code)]

use core_scene::BootstrapRecord;

/// Render a `BootstrapRecord` as deterministic, hand-checkable JSON.
pub fn render(record: &BootstrapRecord) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"sceneId\": {},\n", record.scene_id.raw()));
    out.push_str(&format!(
        "  \"runtimeSessionId\": {},\n",
        record.runtime_session_id.raw()
    ));
    out.push_str(&format!(
        "  \"schemaVersion\": {},\n",
        record.schema_version
    ));
    out.push_str(&format!("  \"nodeCount\": {},\n", record.node_count));
    out.push_str(&format!("  \"entityCount\": {},\n", record.entity_count));
    out.push_str(&format!(
        "  \"spatialSessionHash\": {},\n",
        record.spatial_session_hash.0
    ));
    out.push_str("  \"sourceTrace\": [\n");
    for (i, t) in record.source_trace.iter().enumerate() {
        out.push_str(&format!(
            "    {{ \"node\": {}, \"entity\": {} }}",
            t.node.raw(),
            t.entity.raw()
        ));
        if i + 1 < record.source_trace.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}
