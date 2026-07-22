//! Session-owned named-input catalog validation and deterministic resolution.
//!
//! Platform hosts normalize input into `RawInputSample`. This rule validates an
//! authored catalog before Session activation, owns the active context stack,
//! and resolves one sample without depending on browser APIs or Entity state.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use protocol_input::{
    ActiveInputContext, InputActionDefinition, InputActionReplayReceipt, InputBindingCatalog,
    InputBindingRecord, InputContextChangeReceipt, InputContextCommand, InputContextDefinition,
    InputContextStackState, InputDiagnostic, InputDiagnosticCode, InputResolutionReceipt,
    InputSessionSnapshot, InputValue, InputValueKind, PlatformInputKind, ProjectInputCatalog,
    RawInputSample, RecordedInputAction, ResolvedInputAction, INPUT_ACTION_RECORD_SCHEMA_VERSION,
    INPUT_BINDING_CATALOG_SCHEMA_VERSION, INPUT_CONTEXT_STATE_SCHEMA_VERSION,
    PROJECT_INPUT_CATALOG_SCHEMA_VERSION,
};

const MAX_CONTEXT_PRIORITY: i32 = 10_000;
const MAX_CATALOG_ACTIONS: usize = 128;
const MAX_CATALOG_CONTEXTS: usize = 32;
const MAX_CATALOG_BINDINGS: usize = 256;
const MAX_PROJECT_ACTIONS: usize = 64;
const MAX_PROJECT_CONTEXTS: usize = 16;
const MAX_PROJECT_BINDINGS: usize = 128;
const RESERVED_PROJECT_NAMESPACES: &[&str] = &[
    "asha", "gameplay", "runtime", "camera", "menu", "dialog", "editor", "host",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputCatalogValidationError {
    diagnostics: Vec<InputDiagnostic>,
}

impl InputCatalogValidationError {
    pub fn diagnostics(&self) -> &[InputDiagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for InputCatalogValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "input catalog validation failed with {} diagnostic(s)",
            self.diagnostics.len()
        )
    }
}

impl Error for InputCatalogValidationError {}

#[derive(Debug, Clone)]
pub struct ValidatedInputBindingCatalog {
    catalog: InputBindingCatalog,
    actions: BTreeMap<String, InputActionDefinition>,
    contexts: BTreeMap<String, InputContextDefinition>,
    bindings: BTreeMap<(String, PlatformInputKind, String), InputBindingRecord>,
    bindings_by_id: BTreeMap<String, InputBindingRecord>,
    catalog_hash: String,
}

impl ValidatedInputBindingCatalog {
    pub fn validate(mut catalog: InputBindingCatalog) -> Result<Self, InputCatalogValidationError> {
        let diagnostics = validate_catalog(&catalog);
        if !diagnostics.is_empty() {
            return Err(InputCatalogValidationError { diagnostics });
        }

        catalog
            .actions
            .sort_by(|left, right| left.action_id.cmp(&right.action_id));
        for action in &mut catalog.actions {
            action.accepted_phases.sort();
        }
        catalog
            .contexts
            .sort_by(|left, right| left.context_id.cmp(&right.context_id));
        catalog
            .bindings
            .sort_by(|left, right| left.binding_id.cmp(&right.binding_id));

        let catalog_hash = hash_catalog(&catalog);
        let actions = catalog
            .actions
            .iter()
            .cloned()
            .map(|action| (action.action_id.clone(), action))
            .collect();
        let contexts = catalog
            .contexts
            .iter()
            .cloned()
            .map(|context| (context.context_id.clone(), context))
            .collect();
        let bindings = catalog
            .bindings
            .iter()
            .cloned()
            .map(|binding| {
                (
                    (
                        binding.context_id.clone(),
                        binding.platform_kind,
                        binding.control.clone(),
                    ),
                    binding,
                )
            })
            .collect();
        let bindings_by_id = catalog
            .bindings
            .iter()
            .cloned()
            .map(|binding| (binding.binding_id.clone(), binding))
            .collect();

        Ok(Self {
            catalog,
            actions,
            contexts,
            bindings,
            bindings_by_id,
            catalog_hash,
        })
    }

    pub fn catalog(&self) -> &InputBindingCatalog {
        &self.catalog
    }

    pub fn catalog_hash(&self) -> &str {
        &self.catalog_hash
    }
}

/// Compose at most one immutable Game Project extension over a caller's base
/// catalog. The base remains protected: project declarations can add new
/// namespaced meaning and bind unused controls, but cannot replace it.
pub fn compose_project_input_catalog(
    base: InputBindingCatalog,
    project_catalogs: &[ProjectInputCatalog],
) -> Result<InputBindingCatalog, InputCatalogValidationError> {
    let validated_base = ValidatedInputBindingCatalog::validate(base)?;
    if project_catalogs.len() > 1 {
        return Err(InputCatalogValidationError {
            diagnostics: vec![diagnostic(
                InputDiagnosticCode::DuplicateProjectCatalog,
                "projectInputCatalogs",
                "a RuntimeSession accepts at most one Game Project input catalog",
            )],
        });
    }
    let Some(project) = project_catalogs.first() else {
        return Ok(validated_base.catalog().clone());
    };
    let mut diagnostics = validate_project_catalog(validated_base.catalog(), project);
    if !diagnostics.is_empty() {
        return Err(InputCatalogValidationError { diagnostics });
    }

    let mut catalog = validated_base.catalog().clone();
    catalog.actions.extend(project.actions.clone());
    catalog.contexts.extend(project.contexts.clone());
    catalog.bindings.extend(project.bindings.clone());
    match ValidatedInputBindingCatalog::validate(catalog) {
        Ok(validated) => Ok(validated.catalog().clone()),
        Err(mut error) => {
            diagnostics.append(&mut error.diagnostics);
            Err(InputCatalogValidationError { diagnostics })
        }
    }
}

/// Canonical Engine browser-input base used to validate project declarations
/// before RuntimeSession activation. Browser hosts still normalize DOM input;
/// the semantic catalog and project merge are Rust-owned.
pub fn default_browser_input_catalog() -> InputBindingCatalog {
    let button_phases = vec![
        protocol_input::InputActionPhase::Pressed,
        protocol_input::InputActionPhase::Held,
        protocol_input::InputActionPhase::Released,
    ];
    let pressed = vec![protocol_input::InputActionPhase::Pressed];
    let changed = vec![protocol_input::InputActionPhase::Changed];
    let button = |action_id: &str| InputActionDefinition {
        action_id: action_id.to_owned(),
        value_kind: InputValueKind::Button,
        accepted_phases: button_phases.clone(),
    };
    let pressed_button = |action_id: &str| InputActionDefinition {
        action_id: action_id.to_owned(),
        value_kind: InputValueKind::Button,
        accepted_phases: pressed.clone(),
    };
    let axis = |action_id: &str, value_kind| InputActionDefinition {
        action_id: action_id.to_owned(),
        value_kind,
        accepted_phases: changed.clone(),
    };
    let binding =
        |binding_id: &str, action_id: &str, context_id: &str, platform_kind, control: &str| {
            InputBindingRecord {
                binding_id: binding_id.to_owned(),
                action_id: action_id.to_owned(),
                context_id: context_id.to_owned(),
                platform_kind,
                control: control.to_owned(),
                scale: 1.0,
                extension: None,
            }
        };
    InputBindingCatalog {
        schema_version: INPUT_BINDING_CATALOG_SCHEMA_VERSION,
        actions: vec![
            button("gameplay.move.forward"),
            button("gameplay.move.backward"),
            button("gameplay.move.left"),
            button("gameplay.move.right"),
            axis("gameplay.look", InputValueKind::Axis2d),
            button("gameplay.primaryFire"),
            pressed_button("runtime.time.pause"),
            pressed_button("runtime.time.resume"),
            pressed_button("runtime.session.restart"),
            pressed_button("camera.mode.firstPerson"),
            pressed_button("camera.mode.orbit"),
            pressed_button("camera.mode.topDown"),
            axis("camera.navigation.rotate", InputValueKind::Axis2d),
            axis("camera.navigation.zoom", InputValueKind::Axis1d),
            button("camera.navigation.panForward"),
            button("camera.navigation.panBackward"),
            button("camera.navigation.panLeft"),
            button("camera.navigation.panRight"),
            button("menu.open"),
            button("menu.close"),
            button("menu.navigateUp"),
            button("menu.navigateDown"),
            button("dialog.confirm"),
            button("dialog.cancel"),
            button("editor.camera.forward"),
            button("editor.camera.backward"),
            button("editor.camera.left"),
            button("editor.camera.right"),
            axis("editor.camera.look", InputValueKind::Axis2d),
            button("editor.tool.primary"),
            button("editor.tool.cancel"),
        ],
        contexts: vec![
            input_context("gameplay", 100, false),
            input_context("editor", 200, false),
            input_context("cameraNavigation", 300, true),
            input_context("menu", 1_000, true),
            input_context("dialog", 2_000, true),
        ],
        bindings: vec![
            binding(
                "gameplay-forward",
                "gameplay.move.forward",
                "gameplay",
                PlatformInputKind::KeyboardKey,
                "KeyW",
            ),
            binding(
                "gameplay-backward",
                "gameplay.move.backward",
                "gameplay",
                PlatformInputKind::KeyboardKey,
                "KeyS",
            ),
            binding(
                "gameplay-left",
                "gameplay.move.left",
                "gameplay",
                PlatformInputKind::KeyboardKey,
                "KeyA",
            ),
            binding(
                "gameplay-right",
                "gameplay.move.right",
                "gameplay",
                PlatformInputKind::KeyboardKey,
                "KeyD",
            ),
            binding(
                "gameplay-look",
                "gameplay.look",
                "gameplay",
                PlatformInputKind::MouseDelta,
                "pointer",
            ),
            binding(
                "gameplay-fire",
                "gameplay.primaryFire",
                "gameplay",
                PlatformInputKind::MouseButton,
                "button0",
            ),
            binding(
                "gameplay-menu",
                "runtime.time.pause",
                "gameplay",
                PlatformInputKind::KeyboardKey,
                "Escape",
            ),
            binding(
                "gameplay-restart",
                "runtime.session.restart",
                "gameplay",
                PlatformInputKind::KeyboardKey,
                "KeyR",
            ),
            binding(
                "gameplay-camera-orbit",
                "camera.mode.orbit",
                "gameplay",
                PlatformInputKind::KeyboardKey,
                "KeyO",
            ),
            binding(
                "gameplay-camera-top-down",
                "camera.mode.topDown",
                "gameplay",
                PlatformInputKind::KeyboardKey,
                "KeyT",
            ),
            binding(
                "camera-first-person",
                "camera.mode.firstPerson",
                "cameraNavigation",
                PlatformInputKind::KeyboardKey,
                "KeyF",
            ),
            binding(
                "camera-orbit",
                "camera.mode.orbit",
                "cameraNavigation",
                PlatformInputKind::KeyboardKey,
                "KeyO",
            ),
            binding(
                "camera-top-down",
                "camera.mode.topDown",
                "cameraNavigation",
                PlatformInputKind::KeyboardKey,
                "KeyT",
            ),
            binding(
                "camera-rotate",
                "camera.navigation.rotate",
                "cameraNavigation",
                PlatformInputKind::MouseDelta,
                "pointer",
            ),
            binding(
                "camera-zoom",
                "camera.navigation.zoom",
                "cameraNavigation",
                PlatformInputKind::MouseWheel,
                "wheel",
            ),
            binding(
                "camera-pan-forward",
                "camera.navigation.panForward",
                "cameraNavigation",
                PlatformInputKind::KeyboardKey,
                "KeyW",
            ),
            binding(
                "camera-pan-backward",
                "camera.navigation.panBackward",
                "cameraNavigation",
                PlatformInputKind::KeyboardKey,
                "KeyS",
            ),
            binding(
                "camera-pan-left",
                "camera.navigation.panLeft",
                "cameraNavigation",
                PlatformInputKind::KeyboardKey,
                "KeyA",
            ),
            binding(
                "camera-pan-right",
                "camera.navigation.panRight",
                "cameraNavigation",
                PlatformInputKind::KeyboardKey,
                "KeyD",
            ),
            binding(
                "menu-close",
                "runtime.time.resume",
                "menu",
                PlatformInputKind::KeyboardKey,
                "Escape",
            ),
            binding(
                "menu-restart",
                "runtime.session.restart",
                "menu",
                PlatformInputKind::KeyboardKey,
                "KeyR",
            ),
            binding(
                "menu-up",
                "menu.navigateUp",
                "menu",
                PlatformInputKind::KeyboardKey,
                "ArrowUp",
            ),
            binding(
                "menu-down",
                "menu.navigateDown",
                "menu",
                PlatformInputKind::KeyboardKey,
                "ArrowDown",
            ),
            binding(
                "dialog-confirm",
                "dialog.confirm",
                "dialog",
                PlatformInputKind::KeyboardKey,
                "Enter",
            ),
            binding(
                "dialog-cancel",
                "dialog.cancel",
                "dialog",
                PlatformInputKind::KeyboardKey,
                "Escape",
            ),
            binding(
                "editor-forward",
                "editor.camera.forward",
                "editor",
                PlatformInputKind::KeyboardKey,
                "KeyW",
            ),
            binding(
                "editor-backward",
                "editor.camera.backward",
                "editor",
                PlatformInputKind::KeyboardKey,
                "KeyS",
            ),
            binding(
                "editor-left",
                "editor.camera.left",
                "editor",
                PlatformInputKind::KeyboardKey,
                "KeyA",
            ),
            binding(
                "editor-right",
                "editor.camera.right",
                "editor",
                PlatformInputKind::KeyboardKey,
                "KeyD",
            ),
            binding(
                "editor-look",
                "editor.camera.look",
                "editor",
                PlatformInputKind::MouseDelta,
                "pointer",
            ),
            binding(
                "editor-primary",
                "editor.tool.primary",
                "editor",
                PlatformInputKind::MouseButton,
                "button0",
            ),
            binding(
                "editor-cancel",
                "editor.tool.cancel",
                "editor",
                PlatformInputKind::KeyboardKey,
                "Escape",
            ),
        ],
    }
}

fn input_context(
    context_id: &str,
    priority: i32,
    consumes_lower_priority: bool,
) -> InputContextDefinition {
    InputContextDefinition {
        context_id: context_id.to_owned(),
        priority,
        consumes_lower_priority,
    }
}

#[derive(Debug, Clone)]
pub struct InputSessionResolver {
    catalog: ValidatedInputBindingCatalog,
    context_state: InputContextStackState,
    replayed_record_hashes: BTreeSet<String>,
}

impl InputSessionResolver {
    pub fn activate(
        catalog: InputBindingCatalog,
        initial_contexts: Vec<String>,
    ) -> Result<Self, InputCatalogValidationError> {
        let catalog = ValidatedInputBindingCatalog::validate(catalog)?;
        let context_state = build_context_state(&catalog, 0, initial_contexts)?;
        Ok(Self {
            catalog,
            context_state,
            replayed_record_hashes: BTreeSet::new(),
        })
    }

    pub fn restore(
        catalog: InputBindingCatalog,
        snapshot: InputSessionSnapshot,
    ) -> Result<Self, InputCatalogValidationError> {
        let catalog = ValidatedInputBindingCatalog::validate(catalog)?;
        let mut diagnostics = validate_context_state(&catalog, &snapshot.context_state);
        if snapshot.catalog_hash != catalog.catalog_hash {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::CatalogHashMismatch,
                "snapshot.catalogHash",
                "snapshot catalog hash does not match the validated catalog",
            ));
        }
        if !diagnostics.is_empty() {
            return Err(InputCatalogValidationError { diagnostics });
        }
        Ok(Self {
            catalog,
            context_state: snapshot.context_state,
            replayed_record_hashes: BTreeSet::new(),
        })
    }

    pub fn catalog_hash(&self) -> &str {
        self.catalog.catalog_hash()
    }

    pub fn context_state(&self) -> &InputContextStackState {
        &self.context_state
    }

    pub fn snapshot(&self) -> InputSessionSnapshot {
        InputSessionSnapshot {
            catalog_hash: self.catalog.catalog_hash.clone(),
            context_state: self.context_state.clone(),
        }
    }

    pub fn apply_context_command(
        &mut self,
        command: InputContextCommand,
    ) -> InputContextChangeReceipt {
        let candidate = match command {
            InputContextCommand::Push { context_id } => {
                if !self.catalog.contexts.contains_key(&context_id) {
                    return self.rejected_context_change(
                        InputDiagnosticCode::UnknownContext,
                        "command.contextId",
                        format!("unknown input context '{context_id}'"),
                    );
                }
                if self
                    .context_state
                    .active_contexts
                    .iter()
                    .any(|active| active.context_id == context_id)
                {
                    return self.rejected_context_change(
                        InputDiagnosticCode::DuplicateActiveContext,
                        "command.contextId",
                        format!("input context '{context_id}' is already active"),
                    );
                }
                let mut ids = active_context_ids(&self.context_state);
                ids.push(context_id);
                ids
            }
            InputContextCommand::Pop {
                expected_context_id,
            } => {
                let mut ids = active_context_ids(&self.context_state);
                if ids.last() != Some(&expected_context_id) {
                    return self.rejected_context_change(
                        InputDiagnosticCode::ContextStackMismatch,
                        "command.expectedContextId",
                        format!(
                            "expected top input context '{expected_context_id}' does not match active stack"
                        ),
                    );
                }
                ids.pop();
                ids
            }
            InputContextCommand::Replace { context_ids } => context_ids,
        };

        match build_context_state(
            &self.catalog,
            self.context_state.revision.saturating_add(1),
            candidate,
        ) {
            Ok(state) => {
                self.context_state = state.clone();
                InputContextChangeReceipt {
                    accepted: true,
                    state,
                    diagnostics: Vec::new(),
                }
            }
            Err(error) => InputContextChangeReceipt {
                accepted: false,
                state: self.context_state.clone(),
                diagnostics: error.diagnostics,
            },
        }
    }

    pub fn resolve(&self, sample: RawInputSample) -> InputResolutionReceipt {
        resolve_input(&self.catalog, &self.context_state, sample)
    }

    pub fn replay(&mut self, record: RecordedInputAction) -> InputActionReplayReceipt {
        let diagnostics = validate_recorded_action(&self.catalog, &self.context_state, &record);
        if !diagnostics.is_empty() {
            return replay_receipt(
                &self.catalog,
                &self.context_state,
                &record,
                false,
                None,
                diagnostics,
            );
        }
        if !self
            .replayed_record_hashes
            .insert(record.record_hash.clone())
        {
            return replay_receipt(
                &self.catalog,
                &self.context_state,
                &record,
                false,
                None,
                vec![diagnostic(
                    InputDiagnosticCode::ReplayAlreadyDelivered,
                    "record.recordHash",
                    "recorded action has already been delivered in this input Session",
                )],
            );
        }
        let action = record.action.clone();
        replay_receipt(
            &self.catalog,
            &self.context_state,
            &record,
            true,
            Some(action),
            Vec::new(),
        )
    }

    fn rejected_context_change(
        &self,
        code: InputDiagnosticCode,
        path: &str,
        message: String,
    ) -> InputContextChangeReceipt {
        InputContextChangeReceipt {
            accepted: false,
            state: self.context_state.clone(),
            diagnostics: vec![diagnostic(code, path, message)],
        }
    }
}

pub fn resolve_input(
    catalog: &ValidatedInputBindingCatalog,
    context_state: &InputContextStackState,
    sample: RawInputSample,
) -> InputResolutionReceipt {
    let input_hash = hash_input(&sample);
    let mut diagnostics = validate_context_state(catalog, context_state);
    diagnostics.extend(validate_raw_input(&sample));
    if !diagnostics.is_empty() {
        return resolution_receipt(
            catalog,
            context_state,
            sample.sequence,
            ResolutionDecision {
                accepted: false,
                consumed: false,
                action: None,
                diagnostics,
            },
            input_hash,
        );
    }

    let mut active = context_state.active_contexts.clone();
    active.sort_by(|left, right| {
        let left_priority = catalog.contexts[&left.context_id].priority;
        let right_priority = catalog.contexts[&right.context_id].priority;
        right_priority
            .cmp(&left_priority)
            .then_with(|| right.stack_order.cmp(&left.stack_order))
            .then_with(|| left.context_id.cmp(&right.context_id))
    });

    for active_context in active {
        let context = &catalog.contexts[&active_context.context_id];
        let key = (
            active_context.context_id.clone(),
            sample.platform_kind,
            sample.control.clone(),
        );
        if let Some(binding) = catalog.bindings.get(&key) {
            let action = &catalog.actions[&binding.action_id];
            if !action.accepted_phases.contains(&sample.phase) {
                diagnostics.push(diagnostic(
                    InputDiagnosticCode::UnsupportedPhase,
                    "sample.phase",
                    format!(
                        "action '{}' does not accept phase {:?}",
                        action.action_id, sample.phase
                    ),
                ));
                return resolution_receipt(
                    catalog,
                    context_state,
                    sample.sequence,
                    ResolutionDecision {
                        accepted: false,
                        consumed: true,
                        action: None,
                        diagnostics,
                    },
                    input_hash,
                );
            }
            let resolved = ResolvedInputAction {
                sequence: sample.sequence,
                action_id: binding.action_id.clone(),
                context_id: binding.context_id.clone(),
                binding_id: binding.binding_id.clone(),
                phase: sample.phase,
                value: scaled_value(sample.value, binding.scale),
            };
            return resolution_receipt(
                catalog,
                context_state,
                sample.sequence,
                ResolutionDecision {
                    accepted: true,
                    consumed: true,
                    action: Some(resolved),
                    diagnostics,
                },
                input_hash,
            );
        }
        if context.consumes_lower_priority {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::ConsumedByContext,
                "contextState.activeContexts",
                format!(
                    "input was consumed by context '{}' before lower-priority resolution",
                    context.context_id
                ),
            ));
            return resolution_receipt(
                catalog,
                context_state,
                sample.sequence,
                ResolutionDecision {
                    accepted: false,
                    consumed: true,
                    action: None,
                    diagnostics,
                },
                input_hash,
            );
        }
    }

    diagnostics.push(diagnostic(
        InputDiagnosticCode::UnboundInput,
        "sample.control",
        format!("no active context binds control '{}'", sample.control),
    ));
    resolution_receipt(
        catalog,
        context_state,
        sample.sequence,
        ResolutionDecision {
            accepted: false,
            consumed: false,
            action: None,
            diagnostics,
        },
        input_hash,
    )
}

fn validate_project_catalog(
    base: &InputBindingCatalog,
    project: &ProjectInputCatalog,
) -> Vec<InputDiagnostic> {
    let mut diagnostics = Vec::new();
    if project.schema_version != PROJECT_INPUT_CATALOG_SCHEMA_VERSION {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::UnsupportedCatalogSchema,
            "projectCatalog.schemaVersion",
            format!(
                "project input catalog schema {} is unsupported; expected {}",
                project.schema_version, PROJECT_INPUT_CATALOG_SCHEMA_VERSION
            ),
        ));
    }
    if !is_project_namespace(&project.namespace)
        || project
            .namespace
            .split('.')
            .next()
            .is_some_and(|root| RESERVED_PROJECT_NAMESPACES.contains(&root))
    {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::ReservedNamespace,
            "projectCatalog.namespace",
            "project input namespace must be a portable consumer-owned namespace",
        ));
    }
    for (count, maximum, path, label) in [
        (
            project.actions.len(),
            MAX_PROJECT_ACTIONS,
            "projectCatalog.actions",
            "actions",
        ),
        (
            project.contexts.len(),
            MAX_PROJECT_CONTEXTS,
            "projectCatalog.contexts",
            "contexts",
        ),
        (
            project.bindings.len(),
            MAX_PROJECT_BINDINGS,
            "projectCatalog.bindings",
            "bindings",
        ),
    ] {
        if count > maximum {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::CatalogLimitExceeded,
                path,
                format!("project input catalog {label} exceed the bounded maximum of {maximum}"),
            ));
        }
    }

    let prefix = format!("{}.", project.namespace);
    let base_actions = base
        .actions
        .iter()
        .map(|action| action.action_id.as_str())
        .collect::<BTreeSet<_>>();
    let base_contexts = base
        .contexts
        .iter()
        .map(|context| context.context_id.as_str())
        .collect::<BTreeSet<_>>();
    let project_actions = project
        .actions
        .iter()
        .map(|action| action.action_id.as_str())
        .collect::<BTreeSet<_>>();
    let project_contexts = project
        .contexts
        .iter()
        .map(|context| context.context_id.as_str())
        .collect::<BTreeSet<_>>();

    for (index, action) in project.actions.iter().enumerate() {
        if !action.action_id.starts_with(&prefix)
            || base_actions.contains(action.action_id.as_str())
        {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::ReservedNamespace,
                format!("projectCatalog.actions[{index}].actionId"),
                format!("project action ids must begin with '{prefix}' and must not replace Engine actions"),
            ));
        }
    }
    for (index, context) in project.contexts.iter().enumerate() {
        if !context.context_id.starts_with(&prefix)
            || base_contexts.contains(context.context_id.as_str())
        {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::ReservedNamespace,
                format!("projectCatalog.contexts[{index}].contextId"),
                format!("project context ids must begin with '{prefix}' and must not replace Engine contexts"),
            ));
        }
    }
    for (index, binding) in project.bindings.iter().enumerate() {
        let path = format!("projectCatalog.bindings[{index}]");
        if !binding.binding_id.starts_with(&prefix) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::ReservedNamespace,
                format!("{path}.bindingId"),
                format!("project binding ids must begin with '{prefix}'"),
            ));
        }
        if !project_actions.contains(binding.action_id.as_str()) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::UnknownAction,
                format!("{path}.actionId"),
                "project bindings may target only actions declared by the same project catalog",
            ));
        }
        if !project_contexts.contains(binding.context_id.as_str())
            && !base_contexts.contains(binding.context_id.as_str())
        {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::UnknownContext,
                format!("{path}.contextId"),
                "project binding references an unknown project or compatible Engine context",
            ));
        }
        if !valid_platform_control(binding.platform_kind, &binding.control) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::InvalidControl,
                format!("{path}.control"),
                format!(
                    "'{}' is not a bounded normalized {:?} control",
                    binding.control, binding.platform_kind
                ),
            ));
        }
        if base.bindings.iter().any(|candidate| {
            candidate.context_id == binding.context_id
                && candidate.platform_kind == binding.platform_kind
                && candidate.control == binding.control
        }) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::ProtectedControl,
                format!("{path}.control"),
                "project binding would replace a protected Engine control in the same context",
            ));
        }
    }
    diagnostics
}

fn validate_catalog(catalog: &InputBindingCatalog) -> Vec<InputDiagnostic> {
    let mut diagnostics = Vec::new();
    if catalog.schema_version != INPUT_BINDING_CATALOG_SCHEMA_VERSION {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::UnsupportedCatalogSchema,
            "schemaVersion",
            format!(
                "input catalog schema {} is unsupported; expected {}",
                catalog.schema_version, INPUT_BINDING_CATALOG_SCHEMA_VERSION
            ),
        ));
    }

    for (count, maximum, path, label) in [
        (
            catalog.actions.len(),
            MAX_CATALOG_ACTIONS,
            "actions",
            "actions",
        ),
        (
            catalog.contexts.len(),
            MAX_CATALOG_CONTEXTS,
            "contexts",
            "contexts",
        ),
        (
            catalog.bindings.len(),
            MAX_CATALOG_BINDINGS,
            "bindings",
            "bindings",
        ),
    ] {
        if count > maximum {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::CatalogLimitExceeded,
                path,
                format!("input catalog {label} exceed the bounded maximum of {maximum}"),
            ));
        }
    }

    let mut action_ids = BTreeSet::new();
    for (index, action) in catalog.actions.iter().enumerate() {
        let path = format!("actions[{index}]");
        validate_identifier(
            &action.action_id,
            &format!("{path}.actionId"),
            &mut diagnostics,
        );
        if !action_ids.insert(action.action_id.clone()) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::DuplicateAction,
                format!("{path}.actionId"),
                format!("duplicate input action '{}'", action.action_id),
            ));
        }
        if action.accepted_phases.is_empty() {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::UnsupportedPhase,
                format!("{path}.acceptedPhases"),
                "an input action must accept at least one phase",
            ));
        }
        let unique_phases: BTreeSet<_> = action.accepted_phases.iter().copied().collect();
        if unique_phases.len() != action.accepted_phases.len() {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::UnsupportedPhase,
                format!("{path}.acceptedPhases"),
                "accepted phases must not contain duplicates",
            ));
        }
        let phases_match_value =
            action
                .accepted_phases
                .iter()
                .all(|phase| match action.value_kind {
                    InputValueKind::Button => matches!(
                        phase,
                        protocol_input::InputActionPhase::Pressed
                            | protocol_input::InputActionPhase::Held
                            | protocol_input::InputActionPhase::Released
                    ),
                    InputValueKind::Axis1d | InputValueKind::Axis2d => {
                        *phase == protocol_input::InputActionPhase::Changed
                    }
                });
        if !phases_match_value {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::UnsupportedPhase,
                format!("{path}.acceptedPhases"),
                "button actions accept pressed/held/released; axis actions accept changed",
            ));
        }
    }

    let mut context_ids = BTreeSet::new();
    for (index, context) in catalog.contexts.iter().enumerate() {
        let path = format!("contexts[{index}]");
        validate_identifier(
            &context.context_id,
            &format!("{path}.contextId"),
            &mut diagnostics,
        );
        if !context_ids.insert(context.context_id.clone()) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::DuplicateContext,
                format!("{path}.contextId"),
                format!("duplicate input context '{}'", context.context_id),
            ));
        }
        if !(-MAX_CONTEXT_PRIORITY..=MAX_CONTEXT_PRIORITY).contains(&context.priority) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::InvalidPriority,
                format!("{path}.priority"),
                format!("context priority must be within +/-{MAX_CONTEXT_PRIORITY}"),
            ));
        }
    }

    let mut binding_ids = BTreeSet::new();
    let mut binding_controls = BTreeSet::new();
    for (index, binding) in catalog.bindings.iter().enumerate() {
        let path = format!("bindings[{index}]");
        validate_identifier(
            &binding.binding_id,
            &format!("{path}.bindingId"),
            &mut diagnostics,
        );
        if !binding_ids.insert(binding.binding_id.clone()) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::DuplicateBinding,
                format!("{path}.bindingId"),
                format!("duplicate input binding '{}'", binding.binding_id),
            ));
        }
        if !action_ids.contains(&binding.action_id) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::UnknownAction,
                format!("{path}.actionId"),
                format!("unknown input action '{}'", binding.action_id),
            ));
        }
        if !context_ids.contains(&binding.context_id) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::UnknownContext,
                format!("{path}.contextId"),
                format!("unknown input context '{}'", binding.context_id),
            ));
        }
        if binding.control.trim().is_empty() {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::InvalidIdentifier,
                format!("{path}.control"),
                "platform control must not be blank",
            ));
        }
        if !valid_platform_control(binding.platform_kind, &binding.control) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::InvalidControl,
                format!("{path}.control"),
                format!(
                    "'{}' is not a bounded normalized {:?} control",
                    binding.control, binding.platform_kind
                ),
            ));
        }
        let control_key = (
            binding.context_id.clone(),
            binding.platform_kind,
            binding.control.clone(),
        );
        if !binding_controls.insert(control_key) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::ConflictingBinding,
                format!("{path}.control"),
                "one context cannot bind the same normalized control twice",
            ));
        }
        if !binding.scale.is_finite()
            || binding.scale == 0.0
            || (expected_value_kind(binding.platform_kind) == InputValueKind::Button
                && binding.scale != 1.0)
        {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::ValueKindMismatch,
                format!("{path}.scale"),
                "binding scale must be finite and non-zero; button scale must equal one",
            ));
        }
        if let Some(action) = catalog
            .actions
            .iter()
            .find(|action| action.action_id == binding.action_id)
        {
            let expected = expected_value_kind(binding.platform_kind);
            if action.value_kind != expected {
                diagnostics.push(diagnostic(
                    InputDiagnosticCode::ValueKindMismatch,
                    format!("{path}.actionId"),
                    format!(
                        "platform input {:?} produces {:?}, but action '{}' expects {:?}",
                        binding.platform_kind, expected, action.action_id, action.value_kind
                    ),
                ));
            }
        }
        if binding.extension.is_some() {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::UnsupportedBindingExtension,
                format!("{path}.extension"),
                "binding extensions are reserved but not executable in schema v1",
            ));
        }
    }
    diagnostics
}

fn is_project_namespace(value: &str) -> bool {
    let mut parts = value.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    if !portable_namespace_part(first) {
        return false;
    }
    parts.all(portable_namespace_part)
}

fn portable_namespace_part(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 48
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| byte.is_ascii_lowercase() || byte.is_ascii_digit() && index > 0)
}

fn valid_platform_control(kind: PlatformInputKind, control: &str) -> bool {
    if control.is_empty() || control.len() > 64 || !control.is_ascii() {
        return false;
    }
    match kind {
        PlatformInputKind::KeyboardKey => {
            matches!(
                control,
                "Escape"
                    | "Enter"
                    | "Space"
                    | "Tab"
                    | "Backspace"
                    | "Delete"
                    | "Home"
                    | "End"
                    | "PageUp"
                    | "PageDown"
                    | "ArrowUp"
                    | "ArrowDown"
                    | "ArrowLeft"
                    | "ArrowRight"
                    | "ShiftLeft"
                    | "ShiftRight"
                    | "ControlLeft"
                    | "ControlRight"
                    | "AltLeft"
                    | "AltRight"
            ) || control.strip_prefix("Key").is_some_and(|suffix| {
                suffix.len() == 1 && suffix.as_bytes()[0].is_ascii_uppercase()
            }) || control
                .strip_prefix("Digit")
                .is_some_and(|suffix| suffix.len() == 1 && suffix.as_bytes()[0].is_ascii_digit())
        }
        PlatformInputKind::MouseButton => control
            .strip_prefix("button")
            .is_some_and(|suffix| suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'0'..=b'7')),
        PlatformInputKind::MouseDelta => control == "pointer",
        PlatformInputKind::MouseWheel => control == "wheel",
    }
}

fn validate_context_state(
    catalog: &ValidatedInputBindingCatalog,
    state: &InputContextStackState,
) -> Vec<InputDiagnostic> {
    let mut diagnostics = Vec::new();
    if state.schema_version != INPUT_CONTEXT_STATE_SCHEMA_VERSION {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::UnsupportedContextSchema,
            "contextState.schemaVersion",
            format!(
                "context state schema {} is unsupported; expected {}",
                state.schema_version, INPUT_CONTEXT_STATE_SCHEMA_VERSION
            ),
        ));
    }
    let mut ids = BTreeSet::new();
    for (index, active) in state.active_contexts.iter().enumerate() {
        if !catalog.contexts.contains_key(&active.context_id) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::UnknownContext,
                format!("contextState.activeContexts[{index}].contextId"),
                format!("unknown input context '{}'", active.context_id),
            ));
        }
        if !ids.insert(active.context_id.clone()) {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::DuplicateActiveContext,
                format!("contextState.activeContexts[{index}].contextId"),
                format!(
                    "input context '{}' is active more than once",
                    active.context_id
                ),
            ));
        }
        if active.stack_order != index as u32 {
            diagnostics.push(diagnostic(
                InputDiagnosticCode::NonCanonicalStackOrder,
                format!("contextState.activeContexts[{index}].stackOrder"),
                "active input contexts must use contiguous zero-based stack order",
            ));
        }
    }
    if state.state_hash != hash_context_state(state.revision, &state.active_contexts) {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::ContextHashMismatch,
            "contextState.stateHash",
            "context state hash does not match its canonical fields",
        ));
    }
    diagnostics
}

fn validate_raw_input(sample: &RawInputSample) -> Vec<InputDiagnostic> {
    let mut diagnostics = Vec::new();
    if sample.control.trim().is_empty() {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::InvalidIdentifier,
            "sample.control",
            "normalized platform control must not be blank",
        ));
    }
    let expected = expected_value_kind(sample.platform_kind);
    if sample.value.value_kind() != expected {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::ValueKindMismatch,
            "sample.value",
            format!(
                "platform input {:?} requires {:?} value",
                sample.platform_kind, expected
            ),
        ));
    }
    let finite = match sample.value {
        InputValue::Button { .. } => true,
        InputValue::Axis1d { value } => value.is_finite(),
        InputValue::Axis2d { x, y } => x.is_finite() && y.is_finite(),
    };
    if !finite {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::NonFiniteInput,
            "sample.value",
            "normalized input values must be finite",
        ));
    }
    diagnostics
}

fn validate_recorded_action(
    catalog: &ValidatedInputBindingCatalog,
    context_state: &InputContextStackState,
    record: &RecordedInputAction,
) -> Vec<InputDiagnostic> {
    let mut diagnostics = validate_context_state(catalog, context_state);
    if record.schema_version != INPUT_ACTION_RECORD_SCHEMA_VERSION {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::UnsupportedReplaySchema,
            "record.schemaVersion",
            format!(
                "recorded input schema {} is unsupported; expected {}",
                record.schema_version, INPUT_ACTION_RECORD_SCHEMA_VERSION
            ),
        ));
    }
    if record.catalog_hash != catalog.catalog_hash {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::CatalogHashMismatch,
            "record.catalogHash",
            "recorded action catalog hash does not match the active catalog",
        ));
    }
    if record.context_hash != context_state.state_hash {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::ContextHashMismatch,
            "record.contextHash",
            "recorded action context hash does not match the active context state",
        ));
    }
    if record.record_hash != hash_recorded_action(record) {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::ReplayRecordHashMismatch,
            "record.recordHash",
            "recorded action hash does not match its canonical semantic fields",
        ));
    }

    let Some(action_definition) = catalog.actions.get(&record.action.action_id) else {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::UnknownAction,
            "record.action.actionId",
            format!("unknown input action '{}'", record.action.action_id),
        ));
        return diagnostics;
    };
    let Some(binding) = catalog.bindings_by_id.get(&record.action.binding_id) else {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::DuplicateBinding,
            "record.action.bindingId",
            format!("unknown input binding '{}'", record.action.binding_id),
        ));
        return diagnostics;
    };
    if binding.action_id != record.action.action_id
        || binding.context_id != record.action.context_id
    {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::ConflictingBinding,
            "record.action.bindingId",
            "recorded binding does not resolve to the recorded action and context",
        ));
    }
    if !context_state
        .active_contexts
        .iter()
        .any(|context| context.context_id == record.action.context_id)
    {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::UnknownContext,
            "record.action.contextId",
            "recorded action context is not active",
        ));
    } else {
        diagnostics.extend(validate_recorded_context_winner(
            catalog,
            context_state,
            binding,
        ));
    }
    if !action_definition
        .accepted_phases
        .contains(&record.action.phase)
    {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::UnsupportedPhase,
            "record.action.phase",
            "recorded action phase is not accepted by the active action definition",
        ));
    }
    if record.action.value.value_kind() != action_definition.value_kind {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::ValueKindMismatch,
            "record.action.value",
            "recorded action value kind does not match the active action definition",
        ));
    }
    let finite = match &record.action.value {
        InputValue::Button { .. } => true,
        InputValue::Axis1d { value } => value.is_finite(),
        InputValue::Axis2d { x, y } => x.is_finite() && y.is_finite(),
    };
    if !finite {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::NonFiniteInput,
            "record.action.value",
            "recorded input action values must be finite",
        ));
    }
    diagnostics
}

fn validate_recorded_context_winner(
    catalog: &ValidatedInputBindingCatalog,
    context_state: &InputContextStackState,
    recorded_binding: &InputBindingRecord,
) -> Vec<InputDiagnostic> {
    let mut active = context_state.active_contexts.clone();
    active.sort_by(|left, right| {
        let left_priority = catalog.contexts[&left.context_id].priority;
        let right_priority = catalog.contexts[&right.context_id].priority;
        right_priority
            .cmp(&left_priority)
            .then_with(|| right.stack_order.cmp(&left.stack_order))
            .then_with(|| left.context_id.cmp(&right.context_id))
    });
    for active_context in active {
        let context = &catalog.contexts[&active_context.context_id];
        let key = (
            active_context.context_id.clone(),
            recorded_binding.platform_kind,
            recorded_binding.control.clone(),
        );
        if let Some(candidate) = catalog.bindings.get(&key) {
            if candidate.binding_id == recorded_binding.binding_id {
                return Vec::new();
            }
            return vec![diagnostic(
                InputDiagnosticCode::ConflictingBinding,
                "record.action.bindingId",
                format!(
                    "recorded action is shadowed by higher-priority binding '{}'",
                    candidate.binding_id
                ),
            )];
        }
        if context.consumes_lower_priority {
            return vec![diagnostic(
                InputDiagnosticCode::ConsumedByContext,
                "record.action.contextId",
                format!(
                    "recorded action is consumed by higher-priority context '{}'",
                    context.context_id
                ),
            )];
        }
    }
    vec![diagnostic(
        InputDiagnosticCode::UnknownContext,
        "record.action.contextId",
        "recorded action context is not reachable in the active context order",
    )]
}

fn build_context_state(
    catalog: &ValidatedInputBindingCatalog,
    revision: u64,
    context_ids: Vec<String>,
) -> Result<InputContextStackState, InputCatalogValidationError> {
    let active_contexts: Vec<_> = context_ids
        .into_iter()
        .enumerate()
        .map(|(stack_order, context_id)| ActiveInputContext {
            context_id,
            stack_order: stack_order as u32,
        })
        .collect();
    let state = InputContextStackState {
        schema_version: INPUT_CONTEXT_STATE_SCHEMA_VERSION,
        revision,
        state_hash: hash_context_state(revision, &active_contexts),
        active_contexts,
    };
    let diagnostics = validate_context_state(catalog, &state);
    if diagnostics.is_empty() {
        Ok(state)
    } else {
        Err(InputCatalogValidationError { diagnostics })
    }
}

struct ResolutionDecision {
    accepted: bool,
    consumed: bool,
    action: Option<ResolvedInputAction>,
    diagnostics: Vec<InputDiagnostic>,
}

fn resolution_receipt(
    catalog: &ValidatedInputBindingCatalog,
    context_state: &InputContextStackState,
    sequence: u64,
    decision: ResolutionDecision,
    input_hash: String,
) -> InputResolutionReceipt {
    let resolution_hash = hash_resolution(
        sequence,
        decision.accepted,
        decision.consumed,
        decision.action.as_ref(),
        &decision.diagnostics,
        catalog.catalog_hash(),
        &context_state.state_hash,
        &input_hash,
    );
    let record = decision.action.as_ref().map(|action| {
        let mut record = RecordedInputAction {
            schema_version: INPUT_ACTION_RECORD_SCHEMA_VERSION,
            action: action.clone(),
            catalog_hash: catalog.catalog_hash.clone(),
            context_hash: context_state.state_hash.clone(),
            record_hash: String::new(),
        };
        record.record_hash = hash_recorded_action(&record);
        record
    });
    InputResolutionReceipt {
        sequence,
        accepted: decision.accepted,
        consumed: decision.consumed,
        action: decision.action,
        diagnostics: decision.diagnostics,
        catalog_hash: catalog.catalog_hash.clone(),
        context_hash: context_state.state_hash.clone(),
        input_hash,
        resolution_hash,
        record,
    }
}

fn replay_receipt(
    catalog: &ValidatedInputBindingCatalog,
    context_state: &InputContextStackState,
    record: &RecordedInputAction,
    accepted: bool,
    action: Option<ResolvedInputAction>,
    diagnostics: Vec<InputDiagnostic>,
) -> InputActionReplayReceipt {
    let replay_hash = hash_replay(
        accepted,
        action.as_ref(),
        &diagnostics,
        catalog.catalog_hash(),
        &context_state.state_hash,
        &record.record_hash,
    );
    InputActionReplayReceipt {
        accepted,
        action,
        diagnostics,
        catalog_hash: catalog.catalog_hash.clone(),
        context_hash: context_state.state_hash.clone(),
        record_hash: record.record_hash.clone(),
        replay_hash,
    }
}

fn scaled_value(value: InputValue, scale: f64) -> InputValue {
    match value {
        InputValue::Button { pressed } => InputValue::Button { pressed },
        InputValue::Axis1d { value } => InputValue::Axis1d {
            value: value * scale,
        },
        InputValue::Axis2d { x, y } => InputValue::Axis2d {
            x: x * scale,
            y: y * scale,
        },
    }
}

fn expected_value_kind(kind: PlatformInputKind) -> InputValueKind {
    match kind {
        PlatformInputKind::KeyboardKey | PlatformInputKind::MouseButton => InputValueKind::Button,
        PlatformInputKind::MouseDelta => InputValueKind::Axis2d,
        PlatformInputKind::MouseWheel => InputValueKind::Axis1d,
    }
}

fn validate_identifier(value: &str, path: &str, diagnostics: &mut Vec<InputDiagnostic>) {
    let valid = !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if !valid {
        diagnostics.push(diagnostic(
            InputDiagnosticCode::InvalidIdentifier,
            path,
            "identifier must be 1..=96 ASCII letters, digits, '.', '_' or '-'",
        ));
    }
}

fn active_context_ids(state: &InputContextStackState) -> Vec<String> {
    state
        .active_contexts
        .iter()
        .map(|active| active.context_id.clone())
        .collect()
}

fn diagnostic(
    code: InputDiagnosticCode,
    path: impl Into<String>,
    message: impl Into<String>,
) -> InputDiagnostic {
    InputDiagnostic {
        code,
        path: path.into(),
        message: message.into(),
    }
}

#[derive(Debug, Clone, Copy)]
struct StableHasher(u64);

impl StableHasher {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    fn write_str(&mut self, value: &str) {
        self.write_u64(value.len() as u64);
        self.write_bytes(value.as_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn finish(self) -> String {
        format!("fnv1a64:{:016x}", self.0)
    }
}

fn hash_catalog(catalog: &InputBindingCatalog) -> String {
    let mut hash = StableHasher::new();
    hash.write_u64(u64::from(catalog.schema_version));
    for action in &catalog.actions {
        hash.write_str(&action.action_id);
        hash.write_str(&format!("{:?}", action.value_kind));
        for phase in &action.accepted_phases {
            hash.write_str(&format!("{phase:?}"));
        }
    }
    for context in &catalog.contexts {
        hash.write_str(&context.context_id);
        hash.write_bytes(&context.priority.to_le_bytes());
        hash.write_bytes(&[u8::from(context.consumes_lower_priority)]);
    }
    for binding in &catalog.bindings {
        hash.write_str(&binding.binding_id);
        hash.write_str(&binding.action_id);
        hash.write_str(&binding.context_id);
        hash.write_str(&format!("{:?}", binding.platform_kind));
        hash.write_str(&binding.control);
        hash.write_u64(binding.scale.to_bits());
    }
    hash.finish()
}

fn hash_context_state(revision: u64, contexts: &[ActiveInputContext]) -> String {
    let mut hash = StableHasher::new();
    hash.write_u64(u64::from(INPUT_CONTEXT_STATE_SCHEMA_VERSION));
    hash.write_u64(revision);
    for context in contexts {
        hash.write_str(&context.context_id);
        hash.write_u64(u64::from(context.stack_order));
    }
    hash.finish()
}

fn hash_input(sample: &RawInputSample) -> String {
    let mut hash = StableHasher::new();
    hash.write_u64(sample.sequence);
    hash.write_str(&format!("{:?}", sample.platform_kind));
    hash.write_str(&sample.control);
    hash.write_str(&format!("{:?}", sample.phase));
    write_value(&mut hash, &sample.value);
    hash.finish()
}

#[allow(clippy::too_many_arguments)]
fn hash_resolution(
    sequence: u64,
    accepted: bool,
    consumed: bool,
    action: Option<&ResolvedInputAction>,
    diagnostics: &[InputDiagnostic],
    catalog_hash: &str,
    context_hash: &str,
    input_hash: &str,
) -> String {
    let mut hash = StableHasher::new();
    hash.write_u64(sequence);
    hash.write_bytes(&[u8::from(accepted), u8::from(consumed)]);
    hash.write_str(catalog_hash);
    hash.write_str(context_hash);
    hash.write_str(input_hash);
    if let Some(action) = action {
        hash.write_str(&action.action_id);
        hash.write_str(&action.context_id);
        hash.write_str(&action.binding_id);
        hash.write_str(&format!("{:?}", action.phase));
        write_value(&mut hash, &action.value);
    }
    for item in diagnostics {
        hash.write_str(&format!("{:?}", item.code));
        hash.write_str(&item.path);
        hash.write_str(&item.message);
    }
    hash.finish()
}

fn hash_recorded_action(record: &RecordedInputAction) -> String {
    let mut hash = StableHasher::new();
    hash.write_u64(u64::from(record.schema_version));
    hash.write_u64(record.action.sequence);
    hash.write_str(&record.action.action_id);
    hash.write_str(&record.action.context_id);
    hash.write_str(&record.action.binding_id);
    hash.write_str(&format!("{:?}", record.action.phase));
    write_value(&mut hash, &record.action.value);
    hash.write_str(&record.catalog_hash);
    hash.write_str(&record.context_hash);
    hash.finish()
}

fn hash_replay(
    accepted: bool,
    action: Option<&ResolvedInputAction>,
    diagnostics: &[InputDiagnostic],
    catalog_hash: &str,
    context_hash: &str,
    record_hash: &str,
) -> String {
    let mut hash = StableHasher::new();
    hash.write_bytes(&[u8::from(accepted)]);
    hash.write_str(catalog_hash);
    hash.write_str(context_hash);
    hash.write_str(record_hash);
    if let Some(action) = action {
        hash.write_u64(action.sequence);
        hash.write_str(&action.action_id);
        hash.write_str(&action.context_id);
        hash.write_str(&action.binding_id);
        hash.write_str(&format!("{:?}", action.phase));
        write_value(&mut hash, &action.value);
    }
    for item in diagnostics {
        hash.write_str(&format!("{:?}", item.code));
        hash.write_str(&item.path);
        hash.write_str(&item.message);
    }
    hash.finish()
}

fn write_value(hash: &mut StableHasher, value: &InputValue) {
    match value {
        InputValue::Button { pressed } => {
            hash.write_str("button");
            hash.write_bytes(&[u8::from(*pressed)]);
        }
        InputValue::Axis1d { value } => {
            hash.write_str("axis1d");
            hash.write_u64(value.to_bits());
        }
        InputValue::Axis2d { x, y } => {
            hash.write_str("axis2d");
            hash.write_u64(x.to_bits());
            hash.write_u64(y.to_bits());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_input::{InputActionPhase, InputBindingExtension};

    fn catalog() -> InputBindingCatalog {
        InputBindingCatalog {
            schema_version: INPUT_BINDING_CATALOG_SCHEMA_VERSION,
            actions: vec![
                InputActionDefinition {
                    action_id: "game.move.forward".into(),
                    value_kind: InputValueKind::Button,
                    accepted_phases: vec![
                        InputActionPhase::Pressed,
                        InputActionPhase::Held,
                        InputActionPhase::Released,
                    ],
                },
                InputActionDefinition {
                    action_id: "menu.close".into(),
                    value_kind: InputValueKind::Button,
                    accepted_phases: vec![InputActionPhase::Pressed],
                },
                InputActionDefinition {
                    action_id: "camera.look".into(),
                    value_kind: InputValueKind::Axis2d,
                    accepted_phases: vec![InputActionPhase::Changed],
                },
            ],
            contexts: vec![
                InputContextDefinition {
                    context_id: "gameplay".into(),
                    priority: 10,
                    consumes_lower_priority: false,
                },
                InputContextDefinition {
                    context_id: "menu".into(),
                    priority: 100,
                    consumes_lower_priority: true,
                },
            ],
            bindings: vec![
                InputBindingRecord {
                    binding_id: "game.forward.w".into(),
                    action_id: "game.move.forward".into(),
                    context_id: "gameplay".into(),
                    platform_kind: PlatformInputKind::KeyboardKey,
                    control: "KeyW".into(),
                    scale: 1.0,
                    extension: None,
                },
                InputBindingRecord {
                    binding_id: "game.look.mouse".into(),
                    action_id: "camera.look".into(),
                    context_id: "gameplay".into(),
                    platform_kind: PlatformInputKind::MouseDelta,
                    control: "pointer".into(),
                    scale: 0.5,
                    extension: None,
                },
                InputBindingRecord {
                    binding_id: "menu.close.escape".into(),
                    action_id: "menu.close".into(),
                    context_id: "menu".into(),
                    platform_kind: PlatformInputKind::KeyboardKey,
                    control: "Escape".into(),
                    scale: 1.0,
                    extension: None,
                },
            ],
        }
    }

    fn key(sequence: u64, control: &str, phase: InputActionPhase) -> RawInputSample {
        RawInputSample {
            sequence,
            platform_kind: PlatformInputKind::KeyboardKey,
            control: control.into(),
            phase,
            value: InputValue::Button {
                pressed: phase != InputActionPhase::Released,
            },
        }
    }

    fn project_catalog(control: &str) -> ProjectInputCatalog {
        ProjectInputCatalog {
            schema_version: PROJECT_INPUT_CATALOG_SCHEMA_VERSION,
            namespace: "demo".into(),
            actions: vec![InputActionDefinition {
                action_id: "demo.interact".into(),
                value_kind: InputValueKind::Button,
                accepted_phases: vec![InputActionPhase::Pressed],
            }],
            contexts: Vec::new(),
            bindings: vec![InputBindingRecord {
                binding_id: "demo.interact.primary".into(),
                action_id: "demo.interact".into(),
                context_id: "gameplay".into(),
                platform_kind: PlatformInputKind::KeyboardKey,
                control: control.into(),
                scale: 1.0,
                extension: None,
            }],
        }
    }

    #[test]
    fn project_catalog_adds_a_semantic_action_without_replacing_engine_defaults() {
        let merged = compose_project_input_catalog(
            default_browser_input_catalog(),
            &[project_catalog("KeyE")],
        )
        .unwrap();
        let resolver = InputSessionResolver::activate(merged, vec!["gameplay".into()]).unwrap();
        let receipt = resolver.resolve(key(0, "KeyE", InputActionPhase::Pressed));
        assert_eq!(receipt.action.as_ref().unwrap().action_id, "demo.interact");
        assert_eq!(receipt.action.as_ref().unwrap().context_id, "gameplay");
        let record = receipt.record.unwrap();
        assert!(!serde_json::to_string(&record).unwrap().contains("KeyE"));

        let movement = resolver.resolve(key(1, "KeyW", InputActionPhase::Held));
        assert_eq!(
            movement.action.as_ref().unwrap().action_id,
            "gameplay.move.forward"
        );
    }

    #[test]
    fn project_catalog_rejects_reserved_replacement_invalid_controls_and_bounds() {
        let mut reserved = project_catalog("KeyE");
        reserved.namespace = "gameplay".into();
        reserved.actions[0].action_id = "gameplay.interact".into();
        reserved.bindings[0].binding_id = "gameplay.interact.primary".into();
        reserved.bindings[0].action_id = "gameplay.interact".into();
        let error = compose_project_input_catalog(default_browser_input_catalog(), &[reserved])
            .unwrap_err();
        assert!(error
            .diagnostics()
            .iter()
            .any(|item| item.code == InputDiagnosticCode::ReservedNamespace));

        let mut nested_reserved = project_catalog("KeyE");
        nested_reserved.namespace = "gameplay.demo".into();
        nested_reserved.actions[0].action_id = "gameplay.demo.interact".into();
        nested_reserved.bindings[0].binding_id = "gameplay.demo.interact.primary".into();
        nested_reserved.bindings[0].action_id = "gameplay.demo.interact".into();
        let error =
            compose_project_input_catalog(default_browser_input_catalog(), &[nested_reserved])
                .unwrap_err();
        assert!(error
            .diagnostics()
            .iter()
            .any(|item| item.code == InputDiagnosticCode::ReservedNamespace));

        let protected = project_catalog("KeyW");
        let error = compose_project_input_catalog(default_browser_input_catalog(), &[protected])
            .unwrap_err();
        assert!(error
            .diagnostics()
            .iter()
            .any(|item| item.code == InputDiagnosticCode::ProtectedControl));

        let malformed = project_catalog("the e key");
        let error = compose_project_input_catalog(default_browser_input_catalog(), &[malformed])
            .unwrap_err();
        assert!(error
            .diagnostics()
            .iter()
            .any(|item| item.code == InputDiagnosticCode::InvalidControl));

        let mut noncanonical_mouse_button = project_catalog("button01");
        noncanonical_mouse_button.bindings[0].platform_kind = PlatformInputKind::MouseButton;
        let error = compose_project_input_catalog(
            default_browser_input_catalog(),
            &[noncanonical_mouse_button],
        )
        .unwrap_err();
        assert!(error
            .diagnostics()
            .iter()
            .any(|item| item.code == InputDiagnosticCode::InvalidControl));

        let mut oversized = project_catalog("KeyE");
        oversized.actions = (0..=MAX_PROJECT_ACTIONS)
            .map(|index| InputActionDefinition {
                action_id: format!("demo.action{index}"),
                value_kind: InputValueKind::Button,
                accepted_phases: vec![InputActionPhase::Pressed],
            })
            .collect();
        let error = compose_project_input_catalog(default_browser_input_catalog(), &[oversized])
            .unwrap_err();
        assert!(error
            .diagnostics()
            .iter()
            .any(|item| item.code == InputDiagnosticCode::CatalogLimitExceeded));
    }

    #[test]
    fn project_context_priority_and_saved_binding_changes_are_deterministic() {
        let mut project = project_catalog("KeyE");
        project.actions.push(InputActionDefinition {
            action_id: "demo.modalConfirm".into(),
            value_kind: InputValueKind::Button,
            accepted_phases: vec![InputActionPhase::Pressed],
        });
        project.contexts.push(InputContextDefinition {
            context_id: "demo.modal".into(),
            priority: 1_500,
            consumes_lower_priority: true,
        });
        project.bindings.push(InputBindingRecord {
            binding_id: "demo.modal.confirm".into(),
            action_id: "demo.modalConfirm".into(),
            context_id: "demo.modal".into(),
            platform_kind: PlatformInputKind::KeyboardKey,
            control: "KeyE".into(),
            scale: 1.0,
            extension: None,
        });
        let merged =
            compose_project_input_catalog(default_browser_input_catalog(), &[project]).unwrap();
        let resolver =
            InputSessionResolver::activate(merged, vec!["gameplay".into(), "demo.modal".into()])
                .unwrap();
        assert_eq!(
            resolver
                .resolve(key(0, "KeyE", InputActionPhase::Pressed))
                .action
                .unwrap()
                .action_id,
            "demo.modalConfirm"
        );

        let with_e = compose_project_input_catalog(
            default_browser_input_catalog(),
            &[project_catalog("KeyE")],
        )
        .unwrap();
        let with_f = compose_project_input_catalog(
            default_browser_input_catalog(),
            &[project_catalog("KeyF")],
        )
        .unwrap();
        let e = InputSessionResolver::activate(with_e, vec!["gameplay".into()]).unwrap();
        let f = InputSessionResolver::activate(with_f, vec!["gameplay".into()]).unwrap();
        assert_ne!(e.catalog_hash(), f.catalog_hash());
        assert!(
            e.resolve(key(1, "KeyE", InputActionPhase::Pressed))
                .accepted
        );
        assert!(
            !f.resolve(key(1, "KeyE", InputActionPhase::Pressed))
                .accepted
        );
        assert!(
            f.resolve(key(2, "KeyF", InputActionPhase::Pressed))
                .accepted
        );
    }

    #[test]
    fn identical_inputs_catalogs_and_contexts_produce_identical_receipts() {
        let left = InputSessionResolver::activate(catalog(), vec!["gameplay".into()]).unwrap();
        let right = InputSessionResolver::activate(catalog(), vec!["gameplay".into()]).unwrap();
        let left_receipt = left.resolve(key(7, "KeyW", InputActionPhase::Held));
        let right_receipt = right.resolve(key(7, "KeyW", InputActionPhase::Held));
        assert_eq!(left_receipt, right_receipt);
        assert!(left_receipt.accepted);
        assert_eq!(left_receipt.action.unwrap().action_id, "game.move.forward");
    }

    #[test]
    fn accepted_actions_issue_platform_free_records_for_direct_replay() {
        let source = InputSessionResolver::activate(catalog(), vec!["gameplay".into()]).unwrap();
        let resolved = source.resolve(key(7, "KeyW", InputActionPhase::Held));
        let record = resolved
            .record
            .expect("accepted input must issue a replay record");
        let serialized = serde_json::to_string(&record).unwrap();
        assert!(!serialized.contains("KeyW"));
        assert!(!serialized.contains("keyboardKey"));

        let mut replay =
            InputSessionResolver::activate(catalog(), vec!["gameplay".into()]).unwrap();
        let first = replay.replay(record.clone());
        assert!(first.accepted);
        assert_eq!(first.action, Some(record.action.clone()));

        let duplicate = replay.replay(record);
        assert!(!duplicate.accepted);
        assert_eq!(
            duplicate.diagnostics[0].code,
            InputDiagnosticCode::ReplayAlreadyDelivered
        );
    }

    #[test]
    fn replay_fails_closed_on_tampering_or_a_different_context_snapshot() {
        let source = InputSessionResolver::activate(catalog(), vec!["gameplay".into()]).unwrap();
        let mut record = source
            .resolve(key(8, "KeyW", InputActionPhase::Pressed))
            .record
            .unwrap();
        record.action.phase = InputActionPhase::Released;
        let mut replay =
            InputSessionResolver::activate(catalog(), vec!["gameplay".into()]).unwrap();
        let tampered = replay.replay(record);
        assert!(!tampered.accepted);
        assert!(tampered
            .diagnostics
            .iter()
            .any(|item| item.code == InputDiagnosticCode::ReplayRecordHashMismatch));

        let record = source
            .resolve(key(9, "KeyW", InputActionPhase::Released))
            .record
            .unwrap();
        let mut changed =
            InputSessionResolver::activate(catalog(), vec!["gameplay".into(), "menu".into()])
                .unwrap();
        let mismatch = changed.replay(record);
        assert!(!mismatch.accepted);
        assert!(mismatch
            .diagnostics
            .iter()
            .any(|item| item.code == InputDiagnosticCode::ContextHashMismatch));
    }

    #[test]
    fn replay_cannot_deliver_a_lower_action_past_a_consuming_context() {
        let mut replay =
            InputSessionResolver::activate(catalog(), vec!["gameplay".into(), "menu".into()])
                .unwrap();
        let mut record = RecordedInputAction {
            schema_version: INPUT_ACTION_RECORD_SCHEMA_VERSION,
            action: ResolvedInputAction {
                sequence: 10,
                action_id: "game.move.forward".into(),
                context_id: "gameplay".into(),
                binding_id: "game.forward.w".into(),
                phase: InputActionPhase::Held,
                value: InputValue::Button { pressed: true },
            },
            catalog_hash: replay.catalog_hash().to_string(),
            context_hash: replay.context_state().state_hash.clone(),
            record_hash: String::new(),
        };
        record.record_hash = hash_recorded_action(&record);
        let rejected = replay.replay(record);
        assert!(!rejected.accepted);
        assert_eq!(
            rejected.diagnostics[0].code,
            InputDiagnosticCode::ConsumedByContext
        );
    }

    #[test]
    fn higher_context_resolves_or_consumes_before_gameplay() {
        let resolver =
            InputSessionResolver::activate(catalog(), vec!["gameplay".into(), "menu".into()])
                .unwrap();
        let close = resolver.resolve(key(1, "Escape", InputActionPhase::Pressed));
        assert_eq!(close.action.unwrap().action_id, "menu.close");

        let blocked = resolver.resolve(key(2, "KeyW", InputActionPhase::Held));
        assert!(!blocked.accepted);
        assert!(blocked.consumed);
        assert_eq!(
            blocked.diagnostics[0].code,
            InputDiagnosticCode::ConsumedByContext
        );
    }

    #[test]
    fn axis_values_are_scaled_and_non_finite_values_fail_closed() {
        let resolver = InputSessionResolver::activate(catalog(), vec!["gameplay".into()]).unwrap();
        let moved = resolver.resolve(RawInputSample {
            sequence: 3,
            platform_kind: PlatformInputKind::MouseDelta,
            control: "pointer".into(),
            phase: InputActionPhase::Changed,
            value: InputValue::Axis2d { x: 8.0, y: -4.0 },
        });
        assert_eq!(
            moved.action.unwrap().value,
            InputValue::Axis2d { x: 4.0, y: -2.0 }
        );

        let invalid = resolver.resolve(RawInputSample {
            sequence: 4,
            platform_kind: PlatformInputKind::MouseDelta,
            control: "pointer".into(),
            phase: InputActionPhase::Changed,
            value: InputValue::Axis2d {
                x: f64::NAN,
                y: 0.0,
            },
        });
        assert_eq!(
            invalid.diagnostics[0].code,
            InputDiagnosticCode::NonFiniteInput
        );
    }

    #[test]
    fn context_changes_are_typed_hashed_and_restore_verifies_evidence() {
        let mut resolver =
            InputSessionResolver::activate(catalog(), vec!["gameplay".into()]).unwrap();
        let before = resolver.context_state().clone();
        let pushed = resolver.apply_context_command(InputContextCommand::Push {
            context_id: "menu".into(),
        });
        assert!(pushed.accepted);
        assert_eq!(pushed.state.revision, before.revision + 1);
        assert_ne!(pushed.state.state_hash, before.state_hash);

        let snapshot = resolver.snapshot();
        let restored = InputSessionResolver::restore(catalog(), snapshot.clone()).unwrap();
        assert_eq!(restored.snapshot(), snapshot);

        let rejected = resolver.apply_context_command(InputContextCommand::Pop {
            expected_context_id: "gameplay".into(),
        });
        assert!(!rejected.accepted);
        assert_eq!(
            rejected.diagnostics[0].code,
            InputDiagnosticCode::ContextStackMismatch
        );
        assert_eq!(resolver.snapshot(), snapshot);
    }

    #[test]
    fn invalid_catalogs_fail_before_session_activation() {
        let mut invalid = catalog();
        invalid.schema_version = 99;
        invalid.bindings.push(InputBindingRecord {
            binding_id: "game.forward.duplicate".into(),
            action_id: "missing.action".into(),
            context_id: "gameplay".into(),
            platform_kind: PlatformInputKind::KeyboardKey,
            control: "KeyW".into(),
            scale: 1.0,
            extension: Some(InputBindingExtension {
                schema_version: 2,
                required_controls: vec!["ShiftLeft".into()],
            }),
        });
        let error = InputSessionResolver::activate(invalid, vec!["gameplay".into()])
            .expect_err("invalid catalog must fail before activation");
        let codes: BTreeSet<_> = error.diagnostics().iter().map(|item| item.code).collect();
        assert!(codes.contains(&InputDiagnosticCode::UnsupportedCatalogSchema));
        assert!(codes.contains(&InputDiagnosticCode::UnknownAction));
        assert!(codes.contains(&InputDiagnosticCode::ConflictingBinding));
        assert!(codes.contains(&InputDiagnosticCode::UnsupportedBindingExtension));
    }

    #[test]
    fn catalog_order_is_canonical_before_hashing() {
        let mut reversed = catalog();
        reversed.actions.reverse();
        reversed.contexts.reverse();
        reversed.bindings.reverse();
        let left = ValidatedInputBindingCatalog::validate(catalog()).unwrap();
        let right = ValidatedInputBindingCatalog::validate(reversed).unwrap();
        assert_eq!(left.catalog_hash(), right.catalog_hash());
    }

    #[test]
    fn tampered_snapshot_hash_fails_restore() {
        let resolver = InputSessionResolver::activate(catalog(), vec!["gameplay".into()]).unwrap();
        let mut snapshot = resolver.snapshot();
        snapshot.context_state.state_hash = "fnv1a64:0000000000000000".into();
        let error = InputSessionResolver::restore(catalog(), snapshot).unwrap_err();
        assert_eq!(
            error.diagnostics()[0].code,
            InputDiagnosticCode::ContextHashMismatch
        );
    }
}
