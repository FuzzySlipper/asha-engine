//! fixture-maker CLI — (re)generate or verify the canonical voxel fixture (#2434).
//!
//!   cargo run -p fixture-maker -- write    # (re)write harness/fixtures/voxel-world/*
//!   cargo run -p fixture-maker -- check    # verify committed payload matches (CI/dev)
//!
//! `check` exits non-zero on any drift, naming the first mismatching artifact.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fixture_maker::{render_fixture, FIXTURE_DIR};

fn repo_root() -> PathBuf {
    // <repo>/engine-rs/crates/tools/fixture-maker -> up five to <repo>.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repo root")
        .to_path_buf()
}

fn fixture_path(root: &Path) -> PathBuf {
    root.join(FIXTURE_DIR)
}

fn write() -> ExitCode {
    let dir = fixture_path(&repo_root());
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    for art in render_fixture() {
        let path = dir.join(&art.rel_path);
        std::fs::write(&path, &art.contents).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
        println!("wrote {}", path.display());
    }
    ExitCode::SUCCESS
}

fn check() -> ExitCode {
    let dir = fixture_path(&repo_root());
    let mut drift = false;
    for art in render_fixture() {
        let path = dir.join(&art.rel_path);
        match std::fs::read_to_string(&path) {
            Ok(on_disk) if on_disk == art.contents => {}
            Ok(_) => {
                eprintln!("DRIFT: {} differs from generator output", art.rel_path);
                drift = true;
            }
            Err(e) => {
                eprintln!("MISSING: {} ({e})", art.rel_path);
                drift = true;
            }
        }
    }
    if drift {
        eprintln!("canonical voxel fixture is stale — regenerate with `fixture-maker write`");
        ExitCode::FAILURE
    } else {
        println!("canonical voxel fixture: OK");
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
