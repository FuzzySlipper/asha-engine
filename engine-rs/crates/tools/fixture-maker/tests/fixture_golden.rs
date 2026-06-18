//! Golden + integrity test for the committed canonical voxel fixture (#2434).
//!
//! Pins the committed payload to the generator output and proves the committed
//! bytes round-trip and match the hashes recorded in the committed manifest.
//! Regenerate with: `cargo run -p fixture-maker -- write`.

use std::path::PathBuf;

use fixture_maker::{
    render_fixture, render_interaction_fixture, FIXTURE_DIR, INTERACTION_FIXTURE_DIR, MANIFEST_NAME,
};
use rule_voxel_edit::persist::decode_chunk_snapshot;
use svc_serialization::BundleHash;

fn fixture_dir() -> PathBuf {
    repo_root().join(FIXTURE_DIR)
}

fn interaction_fixture_dir() -> PathBuf {
    repo_root().join(INTERACTION_FIXTURE_DIR)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repo root")
        .to_path_buf()
}

#[test]
fn committed_payload_matches_the_generator() {
    let dir = fixture_dir();
    for art in render_fixture() {
        let path = dir.join(&art.rel_path);
        let on_disk = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {} ({e}); run `fixture-maker write`", art.rel_path));
        assert_eq!(
            on_disk, art.contents,
            "{} drifted from the generator; regenerate with `fixture-maker write`",
            art.rel_path
        );
    }
}

#[test]
fn committed_chunks_round_trip_and_match_manifest_hashes() {
    let dir = fixture_dir();
    let manifest = std::fs::read_to_string(dir.join(MANIFEST_NAME)).expect("manifest present");

    let mut chunk_files = 0;
    for entry in std::fs::read_dir(&dir).expect("read fixture dir") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("snapshot") {
            continue;
        }
        chunk_files += 1;
        let text = std::fs::read_to_string(&path).unwrap();

        // The committed bytes decode and re-encode to themselves (fixed point).
        let chunk = decode_chunk_snapshot(&text).expect("committed snapshot decodes");
        assert_eq!(
            rule_voxel_edit::persist::encode_chunk_snapshot(&chunk),
            text,
            "{:?} is not a re-encode fixed point",
            path
        );

        // The manifest's recorded hashes match the committed bytes.
        let content_hex = BundleHash::of_str(&text).to_hex();
        let chunk_hex = format!("{:016x}", chunk.content_hash().0);
        assert!(
            manifest.contains(&content_hex),
            "manifest is missing contentHash {content_hex} for {path:?}"
        );
        assert!(
            manifest.contains(&chunk_hex),
            "manifest is missing chunkHash {chunk_hex} for {path:?}"
        );
    }
    assert_eq!(
        chunk_files, 4,
        "expected the 2x2x1 arrangement (4 chunk files)"
    );
}

#[test]
fn committed_interaction_payload_matches_the_generator() {
    let dir = interaction_fixture_dir();
    for art in render_interaction_fixture() {
        let path = dir.join(&art.rel_path);
        let on_disk = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {} ({e}); run `fixture-maker write`", art.rel_path));
        assert_eq!(
            on_disk, art.contents,
            "{} drifted from the generator; regenerate with `fixture-maker write`",
            art.rel_path
        );
    }
}

#[test]
fn committed_interaction_fixture_names_stable_selection_and_edit_anchor() {
    let dir = interaction_fixture_dir();
    let manifest = std::fs::read_to_string(dir.join("basic-selection.json"))
        .expect("interaction fixture manifest present");
    assert!(manifest.contains("\"scenario\": \"basic-voxel-landscape-interaction\""));
    assert!(manifest.contains("\"sourceFixture\": \"canonical-voxel-world\""));
    assert!(manifest.contains("\"voxel\": { \"x\": 1, \"y\": 1, \"z\": 0 }"));
    assert!(manifest.contains("\"face\": \"posZ\""));
    assert!(manifest.contains("\"editAnchor\": { \"x\": 1, \"y\": 1, \"z\": 1 }"));
}
