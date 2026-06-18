//! fixture-maker CLI — (re)generate or verify the canonical voxel fixture (#2434).
//!
//!   cargo run -p fixture-maker -- write    # (re)write harness/fixtures/voxel-world/*
//!   cargo run -p fixture-maker -- check    # verify committed payload matches (CI/dev)
//!
//! `check` exits non-zero on any drift, naming the first mismatching artifact.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fixture_maker::{
    render_fixture, render_interaction_fixture, FIXTURE_DIR, INTERACTION_FIXTURE_DIR,
};

fn repo_root() -> PathBuf {
    // <repo>/engine-rs/crates/tools/fixture-maker -> up five to <repo>.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repo root")
        .to_path_buf()
}

fn fixture_path(root: &Path, dir_name: &str) -> PathBuf {
    root.join(dir_name)
}

fn write_artifacts(root: &Path, dir_name: &str, artifacts: Vec<fixture_maker::GeneratedArtifact>) {
    let dir = root.join(dir_name);
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    for art in artifacts {
        let path = dir.join(&art.rel_path);
        std::fs::write(&path, &art.contents).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
        println!("wrote {}", path.display());
    }
}

fn write() -> ExitCode {
    let root = repo_root();
    write_artifacts(&root, FIXTURE_DIR, render_fixture());
    write_artifacts(&root, INTERACTION_FIXTURE_DIR, render_interaction_fixture());
    ExitCode::SUCCESS
}

fn check_artifacts(
    root: &Path,
    dir_name: &str,
    artifacts: Vec<fixture_maker::GeneratedArtifact>,
) -> bool {
    let dir = fixture_path(root, dir_name);
    let mut drift = false;
    for art in artifacts {
        let path = dir.join(&art.rel_path);
        match std::fs::read_to_string(&path) {
            Ok(on_disk) if on_disk == art.contents => {}
            Ok(_) => {
                eprintln!(
                    "DRIFT: {}/{} differs from generator output",
                    dir_name, art.rel_path
                );
                drift = true;
            }
            Err(e) => {
                eprintln!("MISSING: {}/{} ({e})", dir_name, art.rel_path);
                drift = true;
            }
        }
    }
    drift
}

fn check() -> ExitCode {
    let root = repo_root();
    let drift = check_artifacts(&root, FIXTURE_DIR, render_fixture())
        | check_artifacts(&root, INTERACTION_FIXTURE_DIR, render_interaction_fixture());
    if drift {
        eprintln!("voxel fixtures are stale — regenerate with `fixture-maker write`");
        ExitCode::FAILURE
    } else {
        println!("voxel fixtures: OK");
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("write") => write(),
        Some("check") | None => check(),
        Some(other) => {
            eprintln!("unknown command {other:?}; expected `write` or `check`");
            ExitCode::FAILURE
        }
    }
}
