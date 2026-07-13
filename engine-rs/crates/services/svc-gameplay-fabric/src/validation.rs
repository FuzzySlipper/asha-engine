use protocol_diagnostics::DiagnosticSeverity;
use protocol_game_extension::{
    GameplayContractRef, GameplayModuleManifest, GameplayRegistryDiagnostic,
    GameplayRegistryDiagnosticCode,
};

pub(crate) fn validate_contract(
    contract: &GameplayContractRef,
    path: &str,
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
) {
    if !is_namespace(&contract.namespace) || !is_kebab_segment(&contract.name) {
        push_diagnostic(
            diagnostics,
            GameplayRegistryDiagnosticCode::InvalidNamespace,
            path,
            format!("invalid contract identity `{}`", contract.key()),
        );
    }
    if contract.version == 0 || !is_hash(&contract.schema_hash) {
        push_diagnostic(
            diagnostics,
            GameplayRegistryDiagnosticCode::InvalidIdentifier,
            path,
            format!(
                "contract `{}` needs a positive version and schema hash",
                contract.key()
            ),
        );
    }
}

pub(crate) fn budget_values(manifest: &GameplayModuleManifest) -> [u32; 5] {
    [
        manifest.budget.max_waves,
        manifest.budget.max_events_per_root,
        manifest.budget.max_proposals_per_root,
        manifest.budget.max_invocations_per_root,
        manifest.budget.max_payload_bytes_per_root,
    ]
}

pub(crate) fn is_hash(value: &str) -> bool {
    let (algorithm, digest) = match value.split_once(':') {
        Some(parts) => parts,
        None => return false,
    };
    let expected_len = match algorithm {
        "fnv1a64" => 16,
        "sha256" => 64,
        _ => return false,
    };
    digest.len() == expected_len
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn is_version(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_whitespace)
}

pub(crate) fn is_stable_id(value: &str) -> bool {
    is_namespace(value)
}

pub(crate) fn is_namespace(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(is_kebab_segment)
}

fn is_kebab_segment(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.starts_with('-')
        && !segment.ends_with('-')
        && !segment.contains("--")
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

pub(crate) fn namespace_owns(owner: &str, candidate: &str) -> bool {
    candidate == owner
        || candidate
            .strip_prefix(owner)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

pub(crate) fn push_diagnostic(
    diagnostics: &mut Vec<GameplayRegistryDiagnostic>,
    code: GameplayRegistryDiagnosticCode,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    diagnostics.push(GameplayRegistryDiagnostic {
        code,
        severity: DiagnosticSeverity::Error,
        path: path.into(),
        message: message.into(),
    });
}

pub(crate) fn canonicalize_diagnostics(diagnostics: &mut [GameplayRegistryDiagnostic]) {
    diagnostics.sort_by(|a, b| (&a.path, a.code, &a.message).cmp(&(&b.path, b.code, &b.message)));
}
