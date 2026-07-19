use super::*;

impl EngineBridge {
    /// Generated public activation boundary. The host-provided source identity
    /// is evidence about the shared adapter only; every authority identity in
    /// the accepted receipt is derived by Rust from the admitted source and
    /// statically linked composition.
    pub fn load_runtime_project_authority(
        &mut self,
        request: protocol_project_bundle::RuntimeProjectLoadRequest,
    ) -> protocol_project_bundle::RuntimeProjectLoadReceipt {
        let source = request.source;
        match self.activate_pending_runtime_project(request.expected_lifecycle) {
            Ok(active_project) => protocol_project_bundle::RuntimeProjectLoadReceipt {
                accepted: true,
                source,
                lifecycle: active_project.lifecycle,
                active_project: Some(active_project),
                diagnostics: Vec::new(),
            },
            Err(error) => {
                let lifecycle = self.runtime_project_lifecycle_version();
                protocol_project_bundle::RuntimeProjectLoadReceipt {
                    accepted: false,
                    source,
                    active_project: None,
                    lifecycle,
                    diagnostics: runtime_project_load_diagnostics(error),
                }
            }
        }
    }

    /// Generated explicit close boundary. It never infers replacement or
    /// silently tears down a different active project.
    pub fn close_runtime_project_authority(
        &mut self,
        request: protocol_project_bundle::RuntimeProjectCloseRequest,
    ) -> protocol_project_bundle::RuntimeProjectCloseReceipt {
        match self.unload_runtime_project(request.expected_lifecycle) {
            Ok(closed) => protocol_project_bundle::RuntimeProjectCloseReceipt {
                accepted: true,
                closed_project_id: Some(closed.project_id),
                closed_manifest_hash: Some(closed.manifest_hash),
                lifecycle: closed.lifecycle,
                diagnostics: Vec::new(),
            },
            Err(error) => protocol_project_bundle::RuntimeProjectCloseReceipt {
                accepted: false,
                closed_project_id: None,
                closed_manifest_hash: None,
                lifecycle: self.runtime_project_lifecycle_version(),
                diagnostics: runtime_project_load_diagnostics(error),
            },
        }
    }
}

fn runtime_project_load_diagnostics(
    error: RuntimeProjectLoadError,
) -> Vec<protocol_project_bundle::RuntimeProjectDiagnostic> {
    use protocol_project_bundle::{RuntimeProjectDiagnostic, RuntimeProjectDiagnosticPhase};

    match error {
        RuntimeProjectLoadError::Admission(report) => report
            .diagnostics
            .into_iter()
            .map(|diagnostic| RuntimeProjectDiagnostic {
                phase: RuntimeProjectDiagnosticPhase::RuntimeAdmission,
                code: diagnostic.code.as_str().to_owned(),
                document_id: diagnostic.document_id,
                path: Some(diagnostic.path),
                message: diagnostic.message,
            })
            .collect(),
        RuntimeProjectLoadError::Activation(error) => vec![RuntimeProjectDiagnostic {
            phase: RuntimeProjectDiagnosticPhase::RuntimeActivation,
            code: "activationRejected".to_owned(),
            document_id: None,
            path: None,
            message: error.to_string(),
        }],
        RuntimeProjectLoadError::Resource(message) => vec![RuntimeProjectDiagnostic {
            phase: RuntimeProjectDiagnosticPhase::RuntimeActivation,
            code: "resourceRejected".to_owned(),
            document_id: None,
            path: None,
            message,
        }],
        other => {
            let code = match &other {
                RuntimeProjectLoadError::NotInitialized => "notInitialized",
                RuntimeProjectLoadError::MissingStaticComposition => "missingStaticComposition",
                RuntimeProjectLoadError::MissingAdmittedSource => "missingAdmittedSource",
                RuntimeProjectLoadError::AlreadyActive { .. } => "alreadyActive",
                RuntimeProjectLoadError::NoActiveProject => "noActiveProject",
                RuntimeProjectLoadError::StaleLifecycle { .. } => "staleLifecycle",
                RuntimeProjectLoadError::Admission(_)
                | RuntimeProjectLoadError::Activation(_)
                | RuntimeProjectLoadError::Resource(_) => unreachable!(),
            };
            vec![RuntimeProjectDiagnostic {
                phase: RuntimeProjectDiagnosticPhase::Lifecycle,
                code: code.to_owned(),
                document_id: None,
                path: None,
                message: other.to_string(),
            }]
        }
    }
}
