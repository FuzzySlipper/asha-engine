//! Deterministic authority for parameter-driven animation controllers.
//!
//! # Lane and boundary
//!
//! `rust-rule` owns graph validation, parameter state, transition progress,
//! persistence, replay, and resolved clip/blend selection. Renderer-local pose
//! sampling, joints, bones, mixers, and wall-clock interpolation are deliberately
//! absent. The crate adapts transition application from `rule-state-machine`
//! instead of creating a second finite-state-machine authority.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use core_ids::{EntityId, ModeId, ProcessId};
use rule_state_machine::{
    apply_transition_to_instance, MachineInstance, StateMachineSpec, TransitionRequest,
};
use serde::{Deserialize, Serialize};

mod hashing;
mod timing;

use hashing::{
    canonical_catalog_hash, canonical_graph_hash, controller_state_hash, replay_hash, stable_hash,
    stable_id,
};
use timing::{transition_timing_fact, validate_input_origin};
pub use timing::{
    AnimationInputOrigin, AnimationTransitionFactMoment, AnimationTransitionTimingFact,
};

pub const ANIMATION_CATALOG_SCHEMA_VERSION: u32 = 1;
pub const ANIMATION_SNAPSHOT_SCHEMA_VERSION: u32 = 2;
pub const AUTHORITY_VERSION: &str = "rule-animation-controller.v1";
pub const BLEND_WEIGHT_SCALE: i32 = 1_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationCatalog {
    pub schema_version: u32,
    pub catalog_id: String,
    pub assets: Vec<AnimationClipAsset>,
    pub graphs: Vec<AnimationGraphDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationClipAsset {
    pub asset_id: String,
    pub clips: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationGraphDefinition {
    pub graph_id: String,
    pub version: u32,
    pub asset_id: String,
    pub initial_state_id: String,
    pub parameters: Vec<AnimationParameterDefinition>,
    pub states: Vec<AnimationStateDefinition>,
    pub transitions: Vec<AnimationTransitionDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationParameterDefinition {
    pub parameter_id: String,
    pub kind: AnimationParameterKind,
    pub default_value: AnimationParameterValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnimationParameterKind {
    Float,
    Bool,
    Trigger,
}

/// Deterministic parameter value. `Float` uses signed thousandths rather than
/// platform floating point; authored/public adapters may present it as a float.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum AnimationParameterValue {
    Float(i32),
    Bool(bool),
    Trigger(bool),
}

impl AnimationParameterValue {
    pub const fn kind(&self) -> AnimationParameterKind {
        match self {
            Self::Float(_) => AnimationParameterKind::Float,
            Self::Bool(_) => AnimationParameterKind::Bool,
            Self::Trigger(_) => AnimationParameterKind::Trigger,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationStateDefinition {
    pub state_id: String,
    pub motion: AnimationMotionDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AnimationMotionDefinition {
    Clip {
        clip_id: String,
        speed_milli: i32,
    },
    LinearBlend {
        parameter_id: String,
        low_clip_id: String,
        high_clip_id: String,
        minimum_milli: i32,
        maximum_milli: i32,
        speed_milli: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationTransitionDefinition {
    pub transition_id: String,
    pub from_state_id: String,
    pub to_state_id: String,
    /// Lower values win. Priorities must be unique per source state.
    pub priority: u16,
    pub duration_ticks: u32,
    pub conditions: Vec<AnimationCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AnimationCondition {
    FloatGreaterThan {
        parameter_id: String,
        threshold_milli: i32,
    },
    FloatLessThanOrEqual {
        parameter_id: String,
        threshold_milli: i32,
    },
    BoolEquals {
        parameter_id: String,
        value: bool,
    },
    TriggerSet {
        parameter_id: String,
    },
}

impl AnimationCondition {
    fn parameter_id(&self) -> &str {
        match self {
            Self::FloatGreaterThan { parameter_id, .. }
            | Self::FloatLessThanOrEqual { parameter_id, .. }
            | Self::BoolEquals { parameter_id, .. }
            | Self::TriggerSet { parameter_id } => parameter_id,
        }
    }

    const fn expected_kind(&self) -> AnimationParameterKind {
        match self {
            Self::FloatGreaterThan { .. } | Self::FloatLessThanOrEqual { .. } => {
                AnimationParameterKind::Float
            }
            Self::BoolEquals { .. } => AnimationParameterKind::Bool,
            Self::TriggerSet { .. } => AnimationParameterKind::Trigger,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationCatalogDiagnostic {
    pub code: AnimationCatalogDiagnosticCode,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationCatalogDiagnosticCode {
    UnsupportedSchema,
    InvalidId,
    DuplicateId,
    MissingAsset,
    MissingClip,
    MissingState,
    MissingParameter,
    ParameterTypeMismatch,
    InvalidPlaybackSpeed,
    InvalidBlendRange,
    AmbiguousTransition,
    UnreachableState,
    StableIdCollision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationCatalogValidationError {
    pub diagnostics: Vec<AnimationCatalogDiagnostic>,
}

impl core::fmt::Display for AnimationCatalogValidationError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "animation catalog rejected with {} diagnostic(s)",
            self.diagnostics.len()
        )
    }
}

impl std::error::Error for AnimationCatalogValidationError {}

#[derive(Debug, Clone)]
pub struct ValidatedAnimationCatalog {
    source: AnimationCatalog,
    catalog_hash: String,
    graphs: BTreeMap<String, ValidatedGraph>,
}

impl ValidatedAnimationCatalog {
    pub fn source(&self) -> &AnimationCatalog {
        &self.source
    }

    pub fn catalog_hash(&self) -> &str {
        &self.catalog_hash
    }
}

#[derive(Debug, Clone)]
struct ValidatedGraph {
    definition: AnimationGraphDefinition,
    graph_hash: String,
    machine_id: ProcessId,
    state_ids: BTreeMap<String, ModeId>,
    state_names: BTreeMap<ModeId, String>,
    states: BTreeMap<String, AnimationStateDefinition>,
    parameters: BTreeMap<String, AnimationParameterDefinition>,
    transitions: BTreeMap<String, Vec<AnimationTransitionDefinition>>,
    machine_spec: StateMachineSpec,
}

pub fn validate_animation_catalog(
    catalog: AnimationCatalog,
) -> Result<ValidatedAnimationCatalog, AnimationCatalogValidationError> {
    let mut diagnostics = Vec::new();
    if catalog.schema_version != ANIMATION_CATALOG_SCHEMA_VERSION {
        diagnostic(
            &mut diagnostics,
            AnimationCatalogDiagnosticCode::UnsupportedSchema,
            "schemaVersion",
            "only animation catalog schema version 1 is supported",
        );
    }
    validate_id(&catalog.catalog_id, "catalogId", &mut diagnostics);

    let mut assets = BTreeMap::<String, BTreeSet<String>>::new();
    for (asset_index, asset) in catalog.assets.iter().enumerate() {
        let path = format!("assets[{asset_index}]");
        validate_id(
            &asset.asset_id,
            &format!("{path}.assetId"),
            &mut diagnostics,
        );
        let mut clips = BTreeSet::new();
        for (clip_index, clip) in asset.clips.iter().enumerate() {
            validate_id(
                clip,
                &format!("{path}.clips[{clip_index}]"),
                &mut diagnostics,
            );
            if !clips.insert(clip.clone()) {
                diagnostic(
                    &mut diagnostics,
                    AnimationCatalogDiagnosticCode::DuplicateId,
                    format!("{path}.clips[{clip_index}]"),
                    "clip id is duplicated in the asset",
                );
            }
        }
        if assets.insert(asset.asset_id.clone(), clips).is_some() {
            diagnostic(
                &mut diagnostics,
                AnimationCatalogDiagnosticCode::DuplicateId,
                format!("{path}.assetId"),
                "asset id is duplicated",
            );
        }
    }

    let mut graph_ids = BTreeSet::new();
    let mut machine_ids = BTreeMap::new();
    let mut validated_graphs = BTreeMap::new();
    for (graph_index, graph) in catalog.graphs.iter().enumerate() {
        let path = format!("graphs[{graph_index}]");
        validate_id(
            &graph.graph_id,
            &format!("{path}.graphId"),
            &mut diagnostics,
        );
        if !graph_ids.insert(graph.graph_id.clone()) {
            diagnostic(
                &mut diagnostics,
                AnimationCatalogDiagnosticCode::DuplicateId,
                format!("{path}.graphId"),
                "graph id is duplicated",
            );
        }
        if graph.version == 0 {
            diagnostic(
                &mut diagnostics,
                AnimationCatalogDiagnosticCode::InvalidId,
                format!("{path}.version"),
                "graph version must be non-zero",
            );
        }
        let machine_id = ProcessId::new(stable_id("animation.graph", &graph.graph_id));
        if let Some(previous) = machine_ids.insert(machine_id, graph.graph_id.clone()) {
            if previous != graph.graph_id {
                diagnostic(
                    &mut diagnostics,
                    AnimationCatalogDiagnosticCode::StableIdCollision,
                    format!("{path}.graphId"),
                    "graph id collides with another graph's authority id",
                );
            }
        }
        let Some(asset_clips) = assets.get(&graph.asset_id) else {
            diagnostic(
                &mut diagnostics,
                AnimationCatalogDiagnosticCode::MissingAsset,
                format!("{path}.assetId"),
                "graph references an unknown clip asset",
            );
            continue;
        };
        if let Some(validated) =
            validate_graph(graph, machine_id, asset_clips, &path, &mut diagnostics)
        {
            validated_graphs.insert(graph.graph_id.clone(), validated);
        }
    }

    if diagnostics.is_empty() {
        Ok(ValidatedAnimationCatalog {
            catalog_hash: canonical_catalog_hash(&catalog),
            source: catalog,
            graphs: validated_graphs,
        })
    } else {
        Err(AnimationCatalogValidationError { diagnostics })
    }
}

fn validate_graph(
    graph: &AnimationGraphDefinition,
    machine_id: ProcessId,
    asset_clips: &BTreeSet<String>,
    path: &str,
    diagnostics: &mut Vec<AnimationCatalogDiagnostic>,
) -> Option<ValidatedGraph> {
    let mut parameters = BTreeMap::new();
    for (index, parameter) in graph.parameters.iter().enumerate() {
        let parameter_path = format!("{path}.parameters[{index}]");
        validate_id(
            &parameter.parameter_id,
            &format!("{parameter_path}.parameterId"),
            diagnostics,
        );
        if parameter.kind != parameter.default_value.kind() {
            diagnostic(
                diagnostics,
                AnimationCatalogDiagnosticCode::ParameterTypeMismatch,
                format!("{parameter_path}.defaultValue"),
                "default value does not match the declared parameter kind",
            );
        }
        if parameters
            .insert(parameter.parameter_id.clone(), parameter.clone())
            .is_some()
        {
            diagnostic(
                diagnostics,
                AnimationCatalogDiagnosticCode::DuplicateId,
                format!("{parameter_path}.parameterId"),
                "parameter id is duplicated",
            );
        }
    }

    let mut states = BTreeMap::new();
    let mut state_ids = BTreeMap::new();
    let mut state_names = BTreeMap::new();
    for (index, state) in graph.states.iter().enumerate() {
        let state_path = format!("{path}.states[{index}]");
        validate_id(
            &state.state_id,
            &format!("{state_path}.stateId"),
            diagnostics,
        );
        validate_motion(
            &state.motion,
            asset_clips,
            &parameters,
            &format!("{state_path}.motion"),
            diagnostics,
        );
        let mode_id = ModeId::new(stable_id("animation.state", &state.state_id));
        if let Some(previous) = state_names.insert(mode_id, state.state_id.clone()) {
            if previous != state.state_id {
                diagnostic(
                    diagnostics,
                    AnimationCatalogDiagnosticCode::StableIdCollision,
                    format!("{state_path}.stateId"),
                    "state id collides with another state's authority id",
                );
            }
        }
        state_ids.insert(state.state_id.clone(), mode_id);
        if states
            .insert(state.state_id.clone(), state.clone())
            .is_some()
        {
            diagnostic(
                diagnostics,
                AnimationCatalogDiagnosticCode::DuplicateId,
                format!("{state_path}.stateId"),
                "state id is duplicated",
            );
        }
    }
    if !states.contains_key(&graph.initial_state_id) {
        diagnostic(
            diagnostics,
            AnimationCatalogDiagnosticCode::MissingState,
            format!("{path}.initialStateId"),
            "initial state is not declared by the graph",
        );
    }

    let mut transition_ids = BTreeSet::new();
    let mut priorities = BTreeSet::new();
    let mut transitions = BTreeMap::<String, Vec<AnimationTransitionDefinition>>::new();
    let mut edges = Vec::new();
    for (index, transition) in graph.transitions.iter().enumerate() {
        let transition_path = format!("{path}.transitions[{index}]");
        validate_id(
            &transition.transition_id,
            &format!("{transition_path}.transitionId"),
            diagnostics,
        );
        if !transition_ids.insert(transition.transition_id.clone()) {
            diagnostic(
                diagnostics,
                AnimationCatalogDiagnosticCode::DuplicateId,
                format!("{transition_path}.transitionId"),
                "transition id is duplicated",
            );
        }
        let from = state_ids.get(&transition.from_state_id).copied();
        let to = state_ids.get(&transition.to_state_id).copied();
        if from.is_none() {
            diagnostic(
                diagnostics,
                AnimationCatalogDiagnosticCode::MissingState,
                format!("{transition_path}.fromStateId"),
                "transition source state is not declared",
            );
        }
        if to.is_none() {
            diagnostic(
                diagnostics,
                AnimationCatalogDiagnosticCode::MissingState,
                format!("{transition_path}.toStateId"),
                "transition target state is not declared",
            );
        }
        if !priorities.insert((transition.from_state_id.clone(), transition.priority)) {
            diagnostic(
                diagnostics,
                AnimationCatalogDiagnosticCode::AmbiguousTransition,
                format!("{transition_path}.priority"),
                "two transitions from the same state have equal priority",
            );
        }
        for (condition_index, condition) in transition.conditions.iter().enumerate() {
            let condition_path = format!("{transition_path}.conditions[{condition_index}]");
            match parameters.get(condition.parameter_id()) {
                None => diagnostic(
                    diagnostics,
                    AnimationCatalogDiagnosticCode::MissingParameter,
                    format!("{condition_path}.parameterId"),
                    "condition references an undeclared parameter",
                ),
                Some(parameter) if parameter.kind != condition.expected_kind() => diagnostic(
                    diagnostics,
                    AnimationCatalogDiagnosticCode::ParameterTypeMismatch,
                    format!("{condition_path}.parameterId"),
                    "condition kind does not match the referenced parameter",
                ),
                Some(_) => {}
            }
        }
        if let (Some(from), Some(to)) = (from, to) {
            edges.push((from, to));
        }
        transitions
            .entry(transition.from_state_id.clone())
            .or_default()
            .push(transition.clone());
    }
    for candidates in transitions.values_mut() {
        candidates.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.transition_id.cmp(&right.transition_id))
        });
    }

    validate_reachability(graph, &states, diagnostics, path);
    let mut machine_spec = StateMachineSpec::new(machine_id, state_ids.values().copied());
    for (from, to) in edges {
        machine_spec = machine_spec.allow(from, to);
    }

    Some(ValidatedGraph {
        definition: graph.clone(),
        graph_hash: canonical_graph_hash(graph),
        machine_id,
        state_ids,
        state_names,
        states,
        parameters,
        transitions,
        machine_spec,
    })
}

fn validate_motion(
    motion: &AnimationMotionDefinition,
    asset_clips: &BTreeSet<String>,
    parameters: &BTreeMap<String, AnimationParameterDefinition>,
    path: &str,
    diagnostics: &mut Vec<AnimationCatalogDiagnostic>,
) {
    match motion {
        AnimationMotionDefinition::Clip {
            clip_id,
            speed_milli,
        } => {
            if *speed_milli <= 0 {
                diagnostic(
                    diagnostics,
                    AnimationCatalogDiagnosticCode::InvalidPlaybackSpeed,
                    format!("{path}.speedMilli"),
                    "clip playback speed must be positive",
                );
            }
            if !asset_clips.contains(clip_id) {
                diagnostic(
                    diagnostics,
                    AnimationCatalogDiagnosticCode::MissingClip,
                    format!("{path}.clipId"),
                    "motion references a clip absent from the graph asset",
                );
            }
        }
        AnimationMotionDefinition::LinearBlend {
            parameter_id,
            low_clip_id,
            high_clip_id,
            minimum_milli,
            maximum_milli,
            speed_milli,
        } => {
            if *speed_milli <= 0 {
                diagnostic(
                    diagnostics,
                    AnimationCatalogDiagnosticCode::InvalidPlaybackSpeed,
                    format!("{path}.speedMilli"),
                    "linear blend playback speed must be positive",
                );
            }
            for (field, clip) in [("lowClipId", low_clip_id), ("highClipId", high_clip_id)] {
                if !asset_clips.contains(clip) {
                    diagnostic(
                        diagnostics,
                        AnimationCatalogDiagnosticCode::MissingClip,
                        format!("{path}.{field}"),
                        "linear blend references a clip absent from the graph asset",
                    );
                }
            }
            match parameters.get(parameter_id) {
                None => diagnostic(
                    diagnostics,
                    AnimationCatalogDiagnosticCode::MissingParameter,
                    format!("{path}.parameterId"),
                    "linear blend references an undeclared parameter",
                ),
                Some(parameter) if parameter.kind != AnimationParameterKind::Float => diagnostic(
                    diagnostics,
                    AnimationCatalogDiagnosticCode::ParameterTypeMismatch,
                    format!("{path}.parameterId"),
                    "linear blend parameter must be float",
                ),
                Some(_) => {}
            }
            if minimum_milli >= maximum_milli {
                diagnostic(
                    diagnostics,
                    AnimationCatalogDiagnosticCode::InvalidBlendRange,
                    format!("{path}.minimumMilli"),
                    "linear blend minimum must be less than maximum",
                );
            }
        }
    }
}

fn validate_reachability(
    graph: &AnimationGraphDefinition,
    states: &BTreeMap<String, AnimationStateDefinition>,
    diagnostics: &mut Vec<AnimationCatalogDiagnostic>,
    path: &str,
) {
    if !states.contains_key(&graph.initial_state_id) {
        return;
    }
    let mut reached = BTreeSet::from([graph.initial_state_id.as_str()]);
    let mut queue = VecDeque::from([graph.initial_state_id.as_str()]);
    while let Some(current) = queue.pop_front() {
        for transition in graph
            .transitions
            .iter()
            .filter(|transition| transition.from_state_id == current)
        {
            if states.contains_key(&transition.to_state_id)
                && reached.insert(transition.to_state_id.as_str())
            {
                queue.push_back(transition.to_state_id.as_str());
            }
        }
    }
    for (state_index, state) in graph.states.iter().enumerate() {
        if !reached.contains(state.state_id.as_str()) {
            diagnostic(
                diagnostics,
                AnimationCatalogDiagnosticCode::UnreachableState,
                format!("{path}.states[{state_index}].stateId"),
                "state is unreachable from the initial state",
            );
        }
    }
}

fn validate_id(value: &str, path: &str, diagnostics: &mut Vec<AnimationCatalogDiagnostic>) {
    let valid = !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'/' | b'_' | b'-')
        });
    if !valid {
        diagnostic(
            diagnostics,
            AnimationCatalogDiagnosticCode::InvalidId,
            path,
            "id must be 1-128 lowercase stable-id characters",
        );
    }
}

fn diagnostic(
    diagnostics: &mut Vec<AnimationCatalogDiagnostic>,
    code: AnimationCatalogDiagnosticCode,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    diagnostics.push(AnimationCatalogDiagnostic {
        code,
        path: path.into(),
        message: message.into(),
    });
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAnimationMotion {
    pub clip_a: String,
    pub clip_b: Option<String>,
    pub blend_weight_milli: i32,
    pub speed_milli: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationTransitionState {
    pub transition_id: String,
    pub from_state_id: String,
    pub to_state_id: String,
    pub elapsed_ticks: u32,
    pub duration_ticks: u32,
    pub target_motion: ResolvedAnimationMotion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationControllerState {
    pub entity: u64,
    pub graph_id: String,
    pub graph_version: u32,
    pub graph_hash: String,
    pub current_state_id: String,
    pub revision: u64,
    pub parameters: BTreeMap<String, AnimationParameterValue>,
    pub motion: ResolvedAnimationMotion,
    pub transition: Option<AnimationTransitionState>,
    pub timing_fact: Option<AnimationTransitionTimingFact>,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationControllerChange {
    pub previous_state_hash: Option<String>,
    pub state: AnimationControllerState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AnimationControllerInput {
    Attach {
        graph_id: String,
    },
    SetFloat {
        parameter_id: String,
        value_milli: i32,
    },
    SetBool {
        parameter_id: String,
        value: bool,
    },
    FireTrigger {
        parameter_id: String,
    },
    Tick {
        tick: u64,
        origin: AnimationInputOrigin,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationControllerInputRecord {
    pub sequence: u64,
    pub entity: u64,
    pub input: AnimationControllerInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationAuthorityReceipt {
    pub sequence: u64,
    pub state_hash: String,
    pub replay_hash: String,
    pub change: Option<AnimationControllerChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationAuthorityError {
    UnknownGraph(String),
    ControllerAlreadyAttached(u64),
    ControllerMissing(u64),
    UnknownParameter(String),
    ParameterTypeMismatch(String),
    TickNotContiguous { expected: u64, actual: u64 },
    CorruptGraph(String),
    SnapshotDecode(String),
    SnapshotCatalogMismatch { expected: String, actual: String },
    SnapshotStateMismatch { entity: u64 },
    SnapshotReplayMismatch,
    ReplaySequenceMismatch { expected: u64, actual: u64 },
    InvalidOrigin(String),
}

impl core::fmt::Display for AnimationAuthorityError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AnimationAuthorityError {}

#[derive(Debug, Clone)]
struct ControllerInstance {
    graph_id: String,
    machine: MachineInstance,
    parameters: BTreeMap<String, AnimationParameterValue>,
    transition: Option<ActiveTransition>,
    last_timing_fact: Option<AnimationTransitionTimingFact>,
    last_tick: u64,
    last_emitted_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveTransition {
    transition_id: String,
    from_state_id: String,
    to_state_id: String,
    elapsed_ticks: u32,
    duration_ticks: u32,
    origin: AnimationInputOrigin,
}

#[derive(Debug, Clone)]
pub struct AnimationControllerAuthority {
    catalog: ValidatedAnimationCatalog,
    controllers: BTreeMap<EntityId, ControllerInstance>,
    records: Vec<AnimationControllerInputRecord>,
}

impl AnimationControllerAuthority {
    pub fn new(catalog: ValidatedAnimationCatalog) -> Self {
        Self {
            catalog,
            controllers: BTreeMap::new(),
            records: Vec::new(),
        }
    }

    pub fn catalog_hash(&self) -> &str {
        self.catalog.catalog_hash()
    }

    pub fn records(&self) -> &[AnimationControllerInputRecord] {
        &self.records
    }

    pub fn state(
        &self,
        entity: EntityId,
    ) -> Result<AnimationControllerState, AnimationAuthorityError> {
        let controller = self
            .controllers
            .get(&entity)
            .ok_or(AnimationAuthorityError::ControllerMissing(entity.raw()))?;
        let graph = self
            .catalog
            .graphs
            .get(&controller.graph_id)
            .ok_or_else(|| AnimationAuthorityError::CorruptGraph(controller.graph_id.clone()))?;
        resolved_state(entity, graph, controller)
    }

    pub fn attach(
        &mut self,
        entity: EntityId,
        graph_id: impl Into<String>,
    ) -> Result<AnimationAuthorityReceipt, AnimationAuthorityError> {
        self.apply_input(
            entity,
            AnimationControllerInput::Attach {
                graph_id: graph_id.into(),
            },
            true,
        )
    }

    pub fn set_float(
        &mut self,
        entity: EntityId,
        parameter_id: impl Into<String>,
        value_milli: i32,
    ) -> Result<AnimationAuthorityReceipt, AnimationAuthorityError> {
        self.apply_input(
            entity,
            AnimationControllerInput::SetFloat {
                parameter_id: parameter_id.into(),
                value_milli,
            },
            true,
        )
    }

    pub fn set_bool(
        &mut self,
        entity: EntityId,
        parameter_id: impl Into<String>,
        value: bool,
    ) -> Result<AnimationAuthorityReceipt, AnimationAuthorityError> {
        self.apply_input(
            entity,
            AnimationControllerInput::SetBool {
                parameter_id: parameter_id.into(),
                value,
            },
            true,
        )
    }

    pub fn fire_trigger(
        &mut self,
        entity: EntityId,
        parameter_id: impl Into<String>,
    ) -> Result<AnimationAuthorityReceipt, AnimationAuthorityError> {
        self.apply_input(
            entity,
            AnimationControllerInput::FireTrigger {
                parameter_id: parameter_id.into(),
            },
            true,
        )
    }

    pub fn tick(
        &mut self,
        entity: EntityId,
        tick: u64,
    ) -> Result<AnimationAuthorityReceipt, AnimationAuthorityError> {
        self.tick_from_fact(
            entity,
            tick,
            AnimationInputOrigin {
                source_fact_id: format!("animation.input:{}:{tick}", entity.raw()),
                authority_tick: tick,
                causation_id: format!("animation.input:{}:{tick}", entity.raw()),
                correlation_id: format!("animation.entity:{}", entity.raw()),
            },
        )
    }

    pub fn tick_from_fact(
        &mut self,
        entity: EntityId,
        tick: u64,
        origin: AnimationInputOrigin,
    ) -> Result<AnimationAuthorityReceipt, AnimationAuthorityError> {
        validate_input_origin(&origin)?;
        self.apply_input(
            entity,
            AnimationControllerInput::Tick { tick, origin },
            true,
        )
    }

    fn apply_input(
        &mut self,
        entity: EntityId,
        input: AnimationControllerInput,
        record: bool,
    ) -> Result<AnimationAuthorityReceipt, AnimationAuthorityError> {
        let previous_emitted = self
            .controllers
            .get(&entity)
            .and_then(|controller| controller.last_emitted_hash.clone());
        match &input {
            AnimationControllerInput::Attach { graph_id } => {
                if self.controllers.contains_key(&entity) {
                    return Err(AnimationAuthorityError::ControllerAlreadyAttached(
                        entity.raw(),
                    ));
                }
                let graph = self
                    .catalog
                    .graphs
                    .get(graph_id)
                    .ok_or_else(|| AnimationAuthorityError::UnknownGraph(graph_id.clone()))?;
                let initial_mode = graph
                    .state_ids
                    .get(&graph.definition.initial_state_id)
                    .copied()
                    .ok_or_else(|| AnimationAuthorityError::CorruptGraph(graph_id.clone()))?;
                let parameters = graph
                    .parameters
                    .iter()
                    .map(|(id, definition)| (id.clone(), definition.default_value.clone()))
                    .collect();
                self.controllers.insert(
                    entity,
                    ControllerInstance {
                        graph_id: graph_id.clone(),
                        machine: MachineInstance {
                            entity,
                            machine: graph.machine_id,
                            current: initial_mode,
                            revision: 0,
                        },
                        parameters,
                        transition: None,
                        last_timing_fact: None,
                        last_tick: 0,
                        last_emitted_hash: None,
                    },
                );
            }
            AnimationControllerInput::SetFloat {
                parameter_id,
                value_milli,
            } => self.set_parameter(
                entity,
                parameter_id,
                AnimationParameterKind::Float,
                AnimationParameterValue::Float(*value_milli),
            )?,
            AnimationControllerInput::SetBool {
                parameter_id,
                value,
            } => self.set_parameter(
                entity,
                parameter_id,
                AnimationParameterKind::Bool,
                AnimationParameterValue::Bool(*value),
            )?,
            AnimationControllerInput::FireTrigger { parameter_id } => self.set_parameter(
                entity,
                parameter_id,
                AnimationParameterKind::Trigger,
                AnimationParameterValue::Trigger(true),
            )?,
            AnimationControllerInput::Tick { tick, origin } => {
                let controller = self
                    .controllers
                    .get_mut(&entity)
                    .ok_or(AnimationAuthorityError::ControllerMissing(entity.raw()))?;
                let expected = controller.last_tick.saturating_add(1);
                if *tick != expected {
                    return Err(AnimationAuthorityError::TickNotContiguous {
                        expected,
                        actual: *tick,
                    });
                }
                let graph = self
                    .catalog
                    .graphs
                    .get(&controller.graph_id)
                    .ok_or_else(|| {
                        AnimationAuthorityError::CorruptGraph(controller.graph_id.clone())
                    })?;
                let input_sequence = self.records.len() as u64;
                if let Some(fact) = evaluate_tick(graph, controller, input_sequence, *tick, origin)?
                {
                    controller.last_timing_fact = Some(fact);
                }
                controller.last_tick = *tick;
            }
        }

        if record {
            self.records.push(AnimationControllerInputRecord {
                sequence: self.records.len() as u64,
                entity: entity.raw(),
                input,
            });
        }
        let mut state = self.state(entity)?;
        let should_emit = matches!(
            self.records.last().map(|record| &record.input),
            Some(AnimationControllerInput::Attach { .. })
                | Some(AnimationControllerInput::Tick { .. })
        ) && previous_emitted.as_deref() != Some(state.state_hash.as_str());
        let change = if should_emit {
            let previous_state_hash = previous_emitted;
            self.controllers
                .get_mut(&entity)
                .expect("controller exists after successful input")
                .last_emitted_hash = Some(state.state_hash.clone());
            Some(AnimationControllerChange {
                previous_state_hash,
                state: state.clone(),
            })
        } else {
            None
        };
        state = self.state(entity)?;
        let sequence = self.records.len().saturating_sub(1) as u64;
        Ok(AnimationAuthorityReceipt {
            sequence,
            state_hash: state.state_hash,
            replay_hash: replay_hash(&self.records),
            change,
        })
    }

    fn set_parameter(
        &mut self,
        entity: EntityId,
        parameter_id: &str,
        expected_kind: AnimationParameterKind,
        value: AnimationParameterValue,
    ) -> Result<(), AnimationAuthorityError> {
        let controller = self
            .controllers
            .get_mut(&entity)
            .ok_or(AnimationAuthorityError::ControllerMissing(entity.raw()))?;
        let graph = self
            .catalog
            .graphs
            .get(&controller.graph_id)
            .ok_or_else(|| AnimationAuthorityError::CorruptGraph(controller.graph_id.clone()))?;
        let definition = graph
            .parameters
            .get(parameter_id)
            .ok_or_else(|| AnimationAuthorityError::UnknownParameter(parameter_id.to_string()))?;
        if definition.kind != expected_kind {
            return Err(AnimationAuthorityError::ParameterTypeMismatch(
                parameter_id.to_string(),
            ));
        }
        controller
            .parameters
            .insert(parameter_id.to_string(), value);
        Ok(())
    }

    pub fn encode_snapshot(&self) -> Result<String, AnimationAuthorityError> {
        serde_json::to_string(&self.snapshot())
            .map_err(|error| AnimationAuthorityError::SnapshotDecode(error.to_string()))
    }

    pub fn snapshot_hash(&self) -> Result<String, AnimationAuthorityError> {
        self.encode_snapshot()
            .map(|encoded| stable_hash(encoded.as_bytes()))
    }

    pub fn decode_snapshot(
        catalog: ValidatedAnimationCatalog,
        encoded: &str,
    ) -> Result<Self, AnimationAuthorityError> {
        let snapshot: AnimationAuthoritySnapshot = serde_json::from_str(encoded)
            .map_err(|error| AnimationAuthorityError::SnapshotDecode(error.to_string()))?;
        if snapshot.schema_version != ANIMATION_SNAPSHOT_SCHEMA_VERSION {
            return Err(AnimationAuthorityError::SnapshotDecode(
                "unsupported animation snapshot schema".to_string(),
            ));
        }
        if snapshot.catalog_hash != catalog.catalog_hash() {
            return Err(AnimationAuthorityError::SnapshotCatalogMismatch {
                expected: catalog.catalog_hash().to_string(),
                actual: snapshot.catalog_hash,
            });
        }
        let mut authority = Self::new(catalog);
        authority.records = snapshot.records;
        for stored in snapshot.controllers {
            let entity = EntityId::new(stored.entity);
            let graph = authority
                .catalog
                .graphs
                .get(&stored.graph_id)
                .ok_or_else(|| AnimationAuthorityError::UnknownGraph(stored.graph_id.clone()))?;
            let current = graph
                .state_ids
                .get(&stored.current_state_id)
                .copied()
                .ok_or(AnimationAuthorityError::SnapshotStateMismatch {
                    entity: stored.entity,
                })?;
            validate_restored_parameters(graph, &stored.parameters, stored.entity)?;
            let controller = ControllerInstance {
                graph_id: stored.graph_id,
                machine: MachineInstance {
                    entity,
                    machine: graph.machine_id,
                    current,
                    revision: stored.revision,
                },
                parameters: stored.parameters,
                transition: stored.transition,
                last_timing_fact: stored.last_timing_fact,
                last_tick: stored.last_tick,
                last_emitted_hash: stored.last_emitted_hash,
            };
            validate_restored_transition(graph, &controller, stored.entity)?;
            let state = resolved_state(entity, graph, &controller)?;
            if state.state_hash != stored.state_hash {
                return Err(AnimationAuthorityError::SnapshotStateMismatch {
                    entity: stored.entity,
                });
            }
            authority.controllers.insert(entity, controller);
        }
        let replayed = Self::replay(authority.catalog.clone(), &authority.records)?;
        if authority.snapshot() != replayed.snapshot() {
            return Err(AnimationAuthorityError::SnapshotReplayMismatch);
        }
        Ok(authority)
    }

    pub fn replay(
        catalog: ValidatedAnimationCatalog,
        records: &[AnimationControllerInputRecord],
    ) -> Result<Self, AnimationAuthorityError> {
        let mut authority = Self::new(catalog);
        for record in records {
            let expected = authority.records.len() as u64;
            if record.sequence != expected {
                return Err(AnimationAuthorityError::ReplaySequenceMismatch {
                    expected,
                    actual: record.sequence,
                });
            }
            authority.apply_input(EntityId::new(record.entity), record.input.clone(), true)?;
        }
        Ok(authority)
    }

    fn snapshot(&self) -> AnimationAuthoritySnapshot {
        let controllers = self
            .controllers
            .iter()
            .map(|(entity, controller)| {
                let state = self
                    .state(*entity)
                    .expect("validated catalog resolves stored controller state");
                StoredController {
                    entity: entity.raw(),
                    graph_id: controller.graph_id.clone(),
                    current_state_id: state.current_state_id,
                    revision: controller.machine.revision,
                    parameters: controller.parameters.clone(),
                    transition: controller.transition.clone(),
                    last_timing_fact: controller.last_timing_fact.clone(),
                    last_tick: controller.last_tick,
                    last_emitted_hash: controller.last_emitted_hash.clone(),
                    state_hash: state.state_hash,
                }
            })
            .collect();
        AnimationAuthoritySnapshot {
            schema_version: ANIMATION_SNAPSHOT_SCHEMA_VERSION,
            catalog_hash: self.catalog.catalog_hash().to_string(),
            controllers,
            records: self.records.clone(),
        }
    }
}

fn evaluate_tick(
    graph: &ValidatedGraph,
    controller: &mut ControllerInstance,
    input_sequence: u64,
    tick: u64,
    origin: &AnimationInputOrigin,
) -> Result<Option<AnimationTransitionTimingFact>, AnimationAuthorityError> {
    if let Some(active) = controller.transition.as_mut() {
        active.elapsed_ticks = active.elapsed_ticks.saturating_add(1);
        if active.elapsed_ticks >= active.duration_ticks {
            let completed = active.clone();
            complete_transition(graph, controller)?;
            return Ok(Some(transition_timing_fact(
                graph,
                controller,
                &completed,
                input_sequence,
                tick,
                AnimationTransitionFactMoment::Completed,
            )));
        }
        return Ok(None);
    }

    let current_state = graph
        .state_names
        .get(&controller.machine.current)
        .ok_or_else(|| AnimationAuthorityError::CorruptGraph(controller.graph_id.clone()))?;
    let selected = graph
        .transitions
        .get(current_state)
        .and_then(|candidates| {
            candidates
                .iter()
                .find(|candidate| conditions_match(&candidate.conditions, &controller.parameters))
        })
        .cloned();
    let Some(selected) = selected else {
        return Ok(None);
    };
    consume_transition_triggers(&selected.conditions, &mut controller.parameters);
    controller.transition = Some(ActiveTransition {
        transition_id: selected.transition_id,
        from_state_id: selected.from_state_id,
        to_state_id: selected.to_state_id,
        elapsed_ticks: 0,
        duration_ticks: selected.duration_ticks,
        origin: origin.clone(),
    });
    if selected.duration_ticks == 0 {
        let completed = controller
            .transition
            .clone()
            .expect("zero-duration transition was just installed");
        complete_transition(graph, controller)?;
        return Ok(Some(transition_timing_fact(
            graph,
            controller,
            &completed,
            input_sequence,
            tick,
            AnimationTransitionFactMoment::Completed,
        )));
    }
    let started = controller
        .transition
        .as_ref()
        .expect("transition was just installed");
    Ok(Some(transition_timing_fact(
        graph,
        controller,
        started,
        input_sequence,
        tick,
        AnimationTransitionFactMoment::Started,
    )))
}

fn complete_transition(
    graph: &ValidatedGraph,
    controller: &mut ControllerInstance,
) -> Result<(), AnimationAuthorityError> {
    let active = controller
        .transition
        .take()
        .ok_or_else(|| AnimationAuthorityError::CorruptGraph(controller.graph_id.clone()))?;
    let from = graph
        .state_ids
        .get(&active.from_state_id)
        .copied()
        .ok_or_else(|| AnimationAuthorityError::CorruptGraph(controller.graph_id.clone()))?;
    let to = graph
        .state_ids
        .get(&active.to_state_id)
        .copied()
        .ok_or_else(|| AnimationAuthorityError::CorruptGraph(controller.graph_id.clone()))?;
    let applied = apply_transition_to_instance(
        &graph.machine_spec,
        controller.machine,
        TransitionRequest::new(controller.machine.entity, graph.machine_id, from, to)
            .expecting_revision(controller.machine.revision),
    )
    .map_err(|_| AnimationAuthorityError::CorruptGraph(controller.graph_id.clone()))?;
    controller.machine = applied.instance;
    Ok(())
}

fn conditions_match(
    conditions: &[AnimationCondition],
    parameters: &BTreeMap<String, AnimationParameterValue>,
) -> bool {
    conditions.iter().all(|condition| match condition {
        AnimationCondition::FloatGreaterThan {
            parameter_id,
            threshold_milli,
        } => matches!(parameters.get(parameter_id), Some(AnimationParameterValue::Float(value)) if value > threshold_milli),
        AnimationCondition::FloatLessThanOrEqual {
            parameter_id,
            threshold_milli,
        } => matches!(parameters.get(parameter_id), Some(AnimationParameterValue::Float(value)) if value <= threshold_milli),
        AnimationCondition::BoolEquals { parameter_id, value } =>
            matches!(parameters.get(parameter_id), Some(AnimationParameterValue::Bool(actual)) if actual == value),
        AnimationCondition::TriggerSet { parameter_id } =>
            matches!(parameters.get(parameter_id), Some(AnimationParameterValue::Trigger(true))),
    })
}

fn consume_transition_triggers(
    conditions: &[AnimationCondition],
    parameters: &mut BTreeMap<String, AnimationParameterValue>,
) {
    for condition in conditions {
        if let AnimationCondition::TriggerSet { parameter_id } = condition {
            parameters.insert(
                parameter_id.clone(),
                AnimationParameterValue::Trigger(false),
            );
        }
    }
}

fn resolved_state(
    entity: EntityId,
    graph: &ValidatedGraph,
    controller: &ControllerInstance,
) -> Result<AnimationControllerState, AnimationAuthorityError> {
    let current_state_id = graph
        .state_names
        .get(&controller.machine.current)
        .cloned()
        .ok_or_else(|| AnimationAuthorityError::CorruptGraph(controller.graph_id.clone()))?;
    let motion = resolve_motion(graph, &current_state_id, &controller.parameters)?;
    let transition = controller
        .transition
        .as_ref()
        .map(|active| {
            Ok(AnimationTransitionState {
                transition_id: active.transition_id.clone(),
                from_state_id: active.from_state_id.clone(),
                to_state_id: active.to_state_id.clone(),
                elapsed_ticks: active.elapsed_ticks,
                duration_ticks: active.duration_ticks,
                target_motion: resolve_motion(graph, &active.to_state_id, &controller.parameters)?,
            })
        })
        .transpose()?;
    let mut state = AnimationControllerState {
        entity: entity.raw(),
        graph_id: controller.graph_id.clone(),
        graph_version: graph.definition.version,
        graph_hash: graph.graph_hash.clone(),
        current_state_id,
        revision: controller.machine.revision,
        parameters: controller.parameters.clone(),
        motion,
        transition,
        timing_fact: controller.last_timing_fact.clone(),
        state_hash: String::new(),
    };
    state.state_hash = controller_state_hash(&state);
    Ok(state)
}

fn resolve_motion(
    graph: &ValidatedGraph,
    state_id: &str,
    parameters: &BTreeMap<String, AnimationParameterValue>,
) -> Result<ResolvedAnimationMotion, AnimationAuthorityError> {
    let state = graph
        .states
        .get(state_id)
        .ok_or_else(|| AnimationAuthorityError::CorruptGraph(graph.definition.graph_id.clone()))?;
    match &state.motion {
        AnimationMotionDefinition::Clip {
            clip_id,
            speed_milli,
        } => Ok(ResolvedAnimationMotion {
            clip_a: clip_id.clone(),
            clip_b: None,
            blend_weight_milli: 0,
            speed_milli: *speed_milli,
        }),
        AnimationMotionDefinition::LinearBlend {
            parameter_id,
            low_clip_id,
            high_clip_id,
            minimum_milli,
            maximum_milli,
            speed_milli,
        } => {
            let value = match parameters.get(parameter_id) {
                Some(AnimationParameterValue::Float(value)) => *value,
                _ => {
                    return Err(AnimationAuthorityError::CorruptGraph(
                        graph.definition.graph_id.clone(),
                    ))
                }
            };
            let clamped = value.clamp(*minimum_milli, *maximum_milli);
            let numerator = i64::from(clamped - minimum_milli) * i64::from(BLEND_WEIGHT_SCALE);
            let denominator = i64::from(maximum_milli - minimum_milli);
            let blend_weight_milli = i32::try_from(numerator / denominator)
                .expect("validated fixed-point blend fits i32");
            Ok(ResolvedAnimationMotion {
                clip_a: low_clip_id.clone(),
                clip_b: Some(high_clip_id.clone()),
                blend_weight_milli,
                speed_milli: *speed_milli,
            })
        }
    }
}

fn validate_restored_parameters(
    graph: &ValidatedGraph,
    values: &BTreeMap<String, AnimationParameterValue>,
    entity: u64,
) -> Result<(), AnimationAuthorityError> {
    if values.len() != graph.parameters.len() {
        return Err(AnimationAuthorityError::SnapshotStateMismatch { entity });
    }
    for (id, definition) in &graph.parameters {
        if values.get(id).map(AnimationParameterValue::kind) != Some(definition.kind) {
            return Err(AnimationAuthorityError::SnapshotStateMismatch { entity });
        }
    }
    Ok(())
}

fn validate_restored_transition(
    graph: &ValidatedGraph,
    controller: &ControllerInstance,
    entity: u64,
) -> Result<(), AnimationAuthorityError> {
    let Some(active) = &controller.transition else {
        return Ok(());
    };
    let current_state = graph
        .state_names
        .get(&controller.machine.current)
        .ok_or(AnimationAuthorityError::SnapshotStateMismatch { entity })?;
    let Some(definition) = graph
        .transitions
        .get(current_state)
        .and_then(|transitions| {
            transitions
                .iter()
                .find(|transition| transition.transition_id == active.transition_id)
        })
    else {
        return Err(AnimationAuthorityError::SnapshotStateMismatch { entity });
    };
    if active.from_state_id != *current_state
        || active.from_state_id != definition.from_state_id
        || active.to_state_id != definition.to_state_id
        || active.duration_ticks != definition.duration_ticks
        || active.duration_ticks == 0
        || active.elapsed_ticks >= active.duration_ticks
    {
        return Err(AnimationAuthorityError::SnapshotStateMismatch { entity });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnimationAuthoritySnapshot {
    schema_version: u32,
    catalog_hash: String,
    controllers: Vec<StoredController>,
    records: Vec<AnimationControllerInputRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredController {
    entity: u64,
    graph_id: String,
    current_state_id: String,
    revision: u64,
    parameters: BTreeMap<String, AnimationParameterValue>,
    transition: Option<ActiveTransition>,
    last_timing_fact: Option<AnimationTransitionTimingFact>,
    last_tick: u64,
    last_emitted_hash: Option<String>,
    state_hash: String,
}
