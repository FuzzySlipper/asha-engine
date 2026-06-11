//! wasm-api — the narrow WASM host boundary (design §8.8, ADR 0006).
//!
//! # Lane
//!
//! `rust-wasm-bridge`. This crate is the WASM export surface for the **replay /
//! golden verification** path — not the runtime transport (that is native
//! `napi-rs` behind `@asha/runtime-bridge`). It exposes the authoritative replay
//! divergence logic from `sim-replay`, compiled to `wasm32`, so CI/goldens can run
//! deterministic checks under WASM semantics and classify native-vs-WASM divergence.
//!
//! The surface stays narrow: decode + diff + classification. No product-domain
//! logic, no renderer logic, no policy logic.

#![forbid(unsafe_code)]

use sim_replay::{decode, diff, DivergenceClass};
use wasm_bindgen::prelude::wasm_bindgen;

/// Classify the divergence between two replay artifacts in `sim-replay`'s text
/// format (the same format as `harness/goldens/replays/*.replay`).
///
/// WASM is authoritative: `expected` is the golden, `actual` is the run under
/// test. Returns a tab-separated `"<class>\t<step>"` pair (`step` is the diverging
/// step index, or `-` for whole-record / no divergence). `class` is `match` when
/// the records reproduce, otherwise a stable `DivergenceClass` label.
///
/// The terse return shape avoids JSON-escaping concerns across the boundary; the
/// TypeScript `@asha/wasm-replay-bridge` parses it into a typed report.
#[wasm_bindgen]
pub fn classify_divergence(expected: &str, actual: &str) -> String {
    let expected_record = match decode(expected) {
        Ok(r) => r,
        Err(_) => return format!("{}\t-", DivergenceClass::MalformedArtifact.label()),
    };
    let actual_record = match decode(actual) {
        Ok(r) => r,
        Err(_) => return format!("{}\t-", DivergenceClass::MalformedArtifact.label()),
    };

    match diff(&expected_record, &actual_record) {
        None => "match\t-".to_string(),
        Some(d) => {
            let step = d
                .step
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string());
            format!("{}\t{}", d.class.label(), step)
        }
    }
}

/// The set of divergence class labels this module can return, newline-joined.
/// Lets the TS side assert its label↔kind mapping stays in sync with the Rust enum.
#[wasm_bindgen]
pub fn divergence_class_labels() -> String {
    [
        "match",
        DivergenceClass::CommandMismatch.label(),
        DivergenceClass::AcceptedEventMismatch.label(),
        DivergenceClass::RejectionMismatch.label(),
        DivergenceClass::HashCheckpointMismatch.label(),
        DivergenceClass::StructuralMismatch.label(),
        DivergenceClass::MalformedArtifact.label(),
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN: &str = "replay 1\ninit 0000000000000abc\nstep 0\ncmd input entity.create 5\nevent entity.created 5\npost 0000000000000011\n";

    #[test]
    fn identical_records_match() {
        assert_eq!(classify_divergence(GOLDEN, GOLDEN), "match\t-");
    }

    #[test]
    fn tampered_post_hash_is_classified() {
        let tampered = GOLDEN.replace("0000000000000011", "00000000000000ff");
        let out = classify_divergence(GOLDEN, &tampered);
        assert_eq!(out, "hash-checkpoint-mismatch\t0");
    }

    #[test]
    fn malformed_actual_is_classified() {
        let out = classify_divergence(GOLDEN, "not a replay artifact");
        assert_eq!(out, "malformed-artifact\t-");
    }
}
