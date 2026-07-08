//! Committed mixed-world session-state snapshot fixtures + equivalence golden
//! (post-launchable-02, Den task #2484).
//!
//! Pins (1) the canonical encoded session-state snapshot artifact for the #2484
//! mixed-world save fixture and (2) the deterministic round-trip equivalence
//! report it produces. The fixtures double as agent-inspectable examples of a
//! durable runtime-authority snapshot under `harness/fixtures/session-state/`.
//!
//! Regenerate after an intended change:
//!
//! ```text
//! BLESS=1 cargo test -p scene-diagnostics --test session_state_goldens
//! ```

use std::path::PathBuf;

use core_entity::{encode_snapshot, fixtures, EntityStore};
use scene_diagnostics::session_state_round_trip;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../harness/fixtures/session-state")
        .join(name)
}

fn check_golden(name: &str, actual: &str) {
    let path = fixture_path(name);
    if std::env::var_os("BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "missing golden {}: {e}\nregenerate with BLESS=1 cargo test -p scene-diagnostics \
             --test session_state_goldens",
            path.display()
        )
    });
    assert_eq!(actual, expected, "golden mismatch for {}", path.display());
}

#[test]
fn mixed_world_snapshot_matches_committed_artifact() {
    let store = fixtures::mixed_world_save_fixture();
    let encoded = encode_snapshot(&store.snapshot());
    check_golden("mixed-world.snapshot.json", &encoded);
}

#[test]
fn committed_snapshot_reloads_to_an_equivalent_world() {
    // The committed artifact decodes and reconstructs the exact authority the
    // fixture produced — proving the on-disk golden is itself round-trip clean.
    let store = fixtures::mixed_world_save_fixture();
    let report = session_state_round_trip(&store);
    assert!(report.is_equivalent(), "{}", report.to_report_text());
    check_golden("mixed-world-equivalence.txt", &report.to_report_text());
}

#[test]
fn committed_artifact_decodes_to_the_fixture_hash() {
    // Decode the committed bytes (not a freshly-encoded copy) and confirm the
    // restored store reproduces the fixture's deterministic fingerprint.
    let store = fixtures::mixed_world_save_fixture();
    let bytes = std::fs::read_to_string(fixture_path("mixed-world.snapshot.json"))
        .expect("committed snapshot fixture present");
    let decoded = core_entity::decode_snapshot(&bytes).expect("committed fixture decodes");
    let restored = EntityStore::from_snapshot(decoded);
    assert_eq!(restored.hash(), store.hash());
}
