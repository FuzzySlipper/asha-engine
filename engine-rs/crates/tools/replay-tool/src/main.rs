//! `replay-tool` — a boring CLI for checking and inspecting replay artifacts.
//!
//! It is for agents and CI, not humans-at-a-terminal: command names and output
//! are stable, exit codes are meaningful, and a mismatch prints the routed
//! divergence report (never a bare "replay failed").
//!
//! Commands:
//!   replay-tool check <replay-path> [--name <name>]   Play a golden back.
//!   replay-tool show  <replay-path>                    Re-encode for inspection.
//!   replay-tool --help                                 Print usage.
//!
//! Exit codes: 0 = ok, 1 = divergence / malformed / read error, 2 = usage error.

use std::io::Write;
use std::process::ExitCode;

use sim_replay::{decode, encode, Divergence};
use sim_runner::playback;

const USAGE: &str = "\
replay-tool — check and inspect ASHA replay artifacts

USAGE:
    replay-tool check <replay-path> [--name <name>]
    replay-tool show  <replay-path>
    replay-tool --help

COMMANDS:
    check   Decode the replay at <replay-path> and play it back against current
            authority logic. Exits 0 if it reproduces exactly, or 1 and prints a
            routed divergence report on mismatch or a malformed artifact.
            --name labels the report (default: the file stem).

    show    Decode the replay and re-encode it to stdout. Useful to confirm an
            artifact parses and to view its canonical form. Exits 1 if malformed.

PATHS:
    <replay-path> is a `.replay` text artifact, e.g.
    harness/goldens/replays/tagged-entity-run.replay
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&args, &mut std::io::stdout(), &mut std::io::stderr());
    ExitCode::from(code)
}

/// Run the CLI with explicit output streams, returning the process exit code.
/// Separated from `main` so tests can drive it and capture output.
fn run<O: Write, E: Write>(args: &[String], out: &mut O, err: &mut E) -> u8 {
    match args.first().map(String::as_str) {
        None | Some("--help") | Some("-h") | Some("help") => {
            let _ = write!(out, "{USAGE}");
            if args.is_empty() {
                2
            } else {
                0
            }
        }
        Some("check") => cmd_check(&args[1..], out, err),
        Some("show") => cmd_show(&args[1..], out, err),
        Some(other) => {
            let _ = writeln!(err, "error: unknown command '{other}'\n");
            let _ = write!(err, "{USAGE}");
            2
        }
    }
}

fn cmd_check<O: Write, E: Write>(args: &[String], out: &mut O, err: &mut E) -> u8 {
    let Some(path) = args.first() else {
        let _ = writeln!(err, "error: `check` requires a <replay-path>");
        return 2;
    };

    let name = parse_name_flag(&args[1..]).unwrap_or_else(|| file_stem(path));

    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            let _ = writeln!(err, "error: cannot read {path}: {e}");
            return 1;
        }
    };

    let record = match decode(&text) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(err, "{}", Divergence::malformed(&e).report(&name));
            return 1;
        }
    };

    match playback(&record) {
        Ok(()) => {
            let _ = writeln!(
                out,
                "ok: {name} reproduces ({} steps, {} checkpoints)",
                record.steps.len(),
                record.snapshots.len()
            );
            0
        }
        Err(divergence) => {
            let _ = writeln!(err, "{}", divergence.report(&name));
            1
        }
    }
}

fn cmd_show<O: Write, E: Write>(args: &[String], out: &mut O, err: &mut E) -> u8 {
    let Some(path) = args.first() else {
        let _ = writeln!(err, "error: `show` requires a <replay-path>");
        return 2;
    };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            let _ = writeln!(err, "error: cannot read {path}: {e}");
            return 1;
        }
    };
    match decode(&text) {
        Ok(record) => {
            let _ = write!(out, "{}", encode(&record));
            0
        }
        Err(e) => {
            let _ = writeln!(
                err,
                "{}",
                Divergence::malformed(&e).report(&file_stem(path))
            );
            1
        }
    }
}

/// Find `--name <value>` in the given args.
fn parse_name_flag(args: &[String]) -> Option<String> {
    args.iter()
        .position(|a| a == "--name")
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn file_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
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

    fn golden() -> String {
        repo_root()
            .join("harness/goldens/replays/tagged-entity-run.replay")
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
    fn check_golden_succeeds() {
        let (code, out, _err) = run_str(&["check", &golden()]);
        assert_eq!(code, 0);
        assert!(out.contains("ok:"));
        assert!(out.contains("reproduces"));
    }

    #[test]
    fn check_tampered_reports_divergence_and_exits_one() {
        let original = std::fs::read_to_string(golden()).unwrap();
        let tampered = original.replacen("post 9245ad62d9fc0fab", "post 0000000000000000", 1);
        let path =
            std::env::temp_dir().join(format!("replay-tool-tamper-{}.replay", std::process::id()));
        std::fs::write(&path, tampered).unwrap();

        let (code, _out, err) = run_str(&["check", path.to_str().unwrap(), "--name", "tampered"]);
        std::fs::remove_file(&path).ok();

        assert_eq!(code, 1);
        assert!(err.contains("replay divergence: tampered"));
        assert!(err.contains("hash-checkpoint-mismatch"));
        assert!(err.contains("likely:"));
    }

    #[test]
    fn check_malformed_artifact_exits_one() {
        let path =
            std::env::temp_dir().join(format!("replay-tool-bad-{}.replay", std::process::id()));
        std::fs::write(&path, "this is not a replay\n").unwrap();
        let (code, _out, err) = run_str(&["check", path.to_str().unwrap()]);
        std::fs::remove_file(&path).ok();
        assert_eq!(code, 1);
        assert!(err.contains("malformed-artifact"));
    }

    #[test]
    fn check_missing_file_exits_one() {
        let (code, _out, err) = run_str(&["check", "/no/such/replay.replay"]);
        assert_eq!(code, 1);
        assert!(err.contains("cannot read"));
    }

    #[test]
    fn show_round_trips_the_golden() {
        let (code, out, _err) = run_str(&["show", &golden()]);
        assert_eq!(code, 0);
        assert_eq!(out, std::fs::read_to_string(golden()).unwrap());
    }

    #[test]
    fn help_and_usage_errors() {
        let (code, out, _err) = run_str(&["--help"]);
        assert_eq!(code, 0);
        assert!(out.contains("USAGE:"));

        // No args is a usage error (exit 2).
        let (code, _out, _err) = run_str(&[]);
        assert_eq!(code, 2);

        // Unknown command is a usage error.
        let (code, _out, err) = run_str(&["frobnicate"]);
        assert_eq!(code, 2);
        assert!(err.contains("unknown command"));

        // `check` with no path is a usage error.
        let (code, _out, _err) = run_str(&["check"]);
        assert_eq!(code, 2);
    }
}
