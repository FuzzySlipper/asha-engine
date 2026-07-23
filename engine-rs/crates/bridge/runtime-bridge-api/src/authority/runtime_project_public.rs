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

    pub fn save_runtime_project_gameplay_checkpoint_authority(
        &mut self,
        request: protocol_project_bundle::RuntimeProjectGameplayCheckpointSaveRequest,
    ) -> protocol_project_bundle::RuntimeProjectGameplayCheckpointSaveReceipt {
        use protocol_project_bundle::{
            RuntimeProjectCheckpointTimeMode, RuntimeProjectGameplayCheckpoint,
            RuntimeProjectGameplayCheckpointSaveReceipt,
            RUNTIME_PROJECT_GAMEPLAY_CHECKPOINT_SCHEMA_VERSION,
        };

        let actual = self.runtime_project_lifecycle_version();
        let result = (|| {
            if request.expected_lifecycle != actual {
                return Err(RuntimeProjectLoadError::StaleLifecycle {
                    expected: request.expected_lifecycle,
                    actual,
                });
            }
            let active = self
                .runtime_project
                .active_runtime_project
                .clone()
                .ok_or(RuntimeProjectLoadError::NoActiveProject)?;
            let snapshot = self
                .with_static_gameplay_runtime("save_runtime_project_gameplay_checkpoint", |host| {
                    host.compose_snapshot()
                })
                .map_err(|error| RuntimeProjectLoadError::Resource(error.to_string()))?
                .ok_or(RuntimeProjectLoadError::MissingStaticComposition)?;
            let time = self.time.time_controller.state();
            let mut checkpoint = RuntimeProjectGameplayCheckpoint {
                schema_version: RUNTIME_PROJECT_GAMEPLAY_CHECKPOINT_SCHEMA_VERSION,
                project_id: active.project_id,
                manifest_hash: active.manifest_hash,
                admission_hash: active.admission_hash,
                content_set_hash: active.content_set_hash,
                composition_hash: active.composition_hash,
                authority_tick: self.time.authority_tick,
                time_mode: match time.mode {
                    sim_runner::TimeControlMode::Paused => RuntimeProjectCheckpointTimeMode::Paused,
                    sim_runner::TimeControlMode::Running => {
                        RuntimeProjectCheckpointTimeMode::Running
                    }
                },
                speed_multiplier: time.speed_multiplier,
                time_revision: time.revision,
                gameplay_snapshot: snapshot.text,
                checkpoint_hash: String::new(),
            };
            checkpoint.checkpoint_hash = runtime_project_gameplay_checkpoint_hash(&checkpoint);
            Ok(checkpoint)
        })();
        match result {
            Ok(checkpoint) => RuntimeProjectGameplayCheckpointSaveReceipt {
                accepted: true,
                checkpoint: Some(checkpoint),
                lifecycle: self.runtime_project_lifecycle_version(),
                diagnostics: Vec::new(),
            },
            Err(error) => RuntimeProjectGameplayCheckpointSaveReceipt {
                accepted: false,
                checkpoint: None,
                lifecycle: self.runtime_project_lifecycle_version(),
                diagnostics: runtime_project_load_diagnostics(error),
            },
        }
    }

    pub fn restore_runtime_project_gameplay_checkpoint_authority(
        &mut self,
        request: protocol_project_bundle::RuntimeProjectGameplayCheckpointRestoreRequest,
    ) -> protocol_project_bundle::RuntimeProjectGameplayCheckpointRestoreReceipt {
        use protocol_project_bundle::{
            RuntimeProjectCheckpointTimeMode, RuntimeProjectGameplayCheckpointRestoreReceipt,
            RUNTIME_PROJECT_GAMEPLAY_CHECKPOINT_SCHEMA_VERSION,
        };

        let actual = self.runtime_project_lifecycle_version();
        let result = (|| {
            if request.expected_lifecycle != actual {
                return Err(RuntimeProjectLoadError::StaleLifecycle {
                    expected: request.expected_lifecycle,
                    actual,
                });
            }
            let active = self
                .runtime_project
                .active_runtime_project
                .clone()
                .ok_or(RuntimeProjectLoadError::NoActiveProject)?;
            let checkpoint = request.checkpoint;
            if checkpoint.schema_version != RUNTIME_PROJECT_GAMEPLAY_CHECKPOINT_SCHEMA_VERSION
                || checkpoint.checkpoint_hash
                    != runtime_project_gameplay_checkpoint_hash(&checkpoint)
            {
                return Err(RuntimeProjectLoadError::Resource(
                    "runtime project gameplay checkpoint version or hash mismatch".to_owned(),
                ));
            }
            if checkpoint.project_id != active.project_id
                || checkpoint.manifest_hash != active.manifest_hash
                || checkpoint.admission_hash != active.admission_hash
                || checkpoint.content_set_hash != active.content_set_hash
                || checkpoint.composition_hash != active.composition_hash
            {
                return Err(RuntimeProjectLoadError::Resource(
                    "runtime project gameplay checkpoint targets different admitted content"
                        .to_owned(),
                ));
            }
            let restored_time = sim_runner::TimeController::restore(sim_runner::TimeControlState {
                mode: match checkpoint.time_mode {
                    RuntimeProjectCheckpointTimeMode::Paused => sim_runner::TimeControlMode::Paused,
                    RuntimeProjectCheckpointTimeMode::Running => {
                        sim_runner::TimeControlMode::Running
                    }
                },
                speed_multiplier: checkpoint.speed_multiplier,
                revision: checkpoint.time_revision,
            })
            .map_err(|_| {
                RuntimeProjectLoadError::Resource(
                    "runtime project gameplay checkpoint has invalid time state".to_owned(),
                )
            })?;
            let engine = self
                .runtime_project
                .engine
                .ok_or(RuntimeProjectLoadError::NotInitialized)?;
            let composition = self
                .gameplay
                .static_gameplay_composition
                .clone()
                .ok_or(RuntimeProjectLoadError::MissingStaticComposition)?;
            let domain_adapter = self.gameplay.static_project_domain_adapter;
            let input_session = self.input.input_session.clone();

            let mut restoring = EngineBridge::new();
            initialization::initialize(&mut restoring, EngineConfig { seed: engine.raw() })
                .map_err(|error| RuntimeProjectLoadError::Resource(error.to_string()))?;
            restoring.gameplay.static_project_content_admission =
                Some(rule_project_bundle::GameplayProjectContentAdmission::new(
                    composition.project_configuration_authority(),
                ));
            restoring.gameplay.static_gameplay_composition = Some(composition);
            restoring.gameplay.static_project_domain_adapter = domain_adapter;
            restoring.runtime_project.runtime_project_generation = actual.generation;
            restoring.runtime_project.runtime_project_revision = actual.revision;
            restoring.runtime_project.pending_project_source = Some(active.source);
            restoring.runtime_project.pending_gameplay_snapshot =
                Some(checkpoint.gameplay_snapshot);
            let receipt = restoring.activate_pending_runtime_project(actual)?;
            restoring.time.authority_tick = checkpoint.authority_tick;
            restoring.time.time_controller = restored_time;
            restoring.input.input_session = input_session;
            *self = restoring;
            Ok(receipt)
        })();
        match result {
            Ok(active_project) => RuntimeProjectGameplayCheckpointRestoreReceipt {
                accepted: true,
                active_project: Some(active_project),
                lifecycle: self.runtime_project_lifecycle_version(),
                diagnostics: Vec::new(),
            },
            Err(error) => RuntimeProjectGameplayCheckpointRestoreReceipt {
                accepted: false,
                active_project: None,
                lifecycle: self.runtime_project_lifecycle_version(),
                diagnostics: runtime_project_load_diagnostics(error),
            },
        }
    }
}

fn runtime_project_gameplay_checkpoint_hash(
    checkpoint: &protocol_project_bundle::RuntimeProjectGameplayCheckpoint,
) -> String {
    let canonical = serde_json::to_string(&(
        checkpoint.schema_version,
        checkpoint.project_id,
        &checkpoint.manifest_hash,
        &checkpoint.admission_hash,
        &checkpoint.content_set_hash,
        &checkpoint.composition_hash,
        checkpoint.authority_tick,
        checkpoint.time_mode,
        checkpoint.speed_multiplier,
        checkpoint.time_revision,
        &checkpoint.gameplay_snapshot,
    ))
    .expect("runtime project checkpoint fields serialize");
    format!("fnv1a64:{}", EngineBridge::fnv1a64(&canonical))
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
        RuntimeProjectLoadError::Domain {
            code,
            document_id,
            path,
            message,
        } => vec![RuntimeProjectDiagnostic {
            phase: RuntimeProjectDiagnosticPhase::RuntimeActivation,
            code,
            document_id,
            path,
            message,
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
                | RuntimeProjectLoadError::Domain { .. }
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
