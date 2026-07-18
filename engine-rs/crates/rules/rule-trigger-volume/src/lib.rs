//! Deterministic kinematic trigger-volume lifecycle.
//!
//! `core-entity` remains the authority for entity lifecycle, transform, bounds,
//! and collision participation. This Rule owns only the semantic trigger role
//! and the durable active-overlap set needed to derive exactly-once enter/exit
//! facts. It does not implement rigid-body dynamics, collision response, or a
//! callback-shaped per-tick `stay` event.

#![forbid(unsafe_code)]

use core_entity::{EntityLifecycle, EntityStore};
use core_ids::EntityId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TRIGGER_VOLUME_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const TRIGGER_VOLUME_OWNER_ID: &str = "rule-trigger-volume";

/// Stored semantic role for one entity whose geometry is supplied by the
/// existing collision/bounds/transform capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KinematicTriggerDefinition {
    pub trigger: u64,
    pub scope: String,
    pub tags: Vec<String>,
}

impl KinematicTriggerDefinition {
    pub fn new(
        trigger: EntityId,
        scope: impl Into<String>,
        tags: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let mut tags = tags.into_iter().map(Into::into).collect::<Vec<_>>();
        tags.sort();
        tags.dedup();
        Self {
            trigger: trigger.raw(),
            scope: scope.into(),
            tags,
        }
    }

    pub fn trigger_id(&self) -> EntityId {
        EntityId::new(self.trigger)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerOverlapPair {
    pub trigger: u64,
    pub subject: u64,
}

impl TriggerOverlapPair {
    pub fn new(trigger: EntityId, subject: EntityId) -> Self {
        Self {
            trigger: trigger.raw(),
            subject: subject.raw(),
        }
    }

    pub fn trigger_id(self) -> EntityId {
        EntityId::new(self.trigger)
    }

    pub fn subject_id(self) -> EntityId {
        EntityId::new(self.subject)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerOverlapFactKind {
    Exit,
    Enter,
}

impl TriggerOverlapFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exit => "exit",
            Self::Enter => "enter",
        }
    }
}

/// Why authority sampled the collision state. The cause is evidence only; pair
/// membership is always computed from current authoritative capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerReconcileCause {
    Tick,
    Spawn,
    Movement,
    Teleport,
    ActivationChanged,
    LifecycleChanged,
    Restore,
}

impl TriggerReconcileCause {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tick => "tick",
            Self::Spawn => "spawn",
            Self::Movement => "movement",
            Self::Teleport => "teleport",
            Self::ActivationChanged => "activationChanged",
            Self::LifecycleChanged => "lifecycleChanged",
            Self::Restore => "restore",
        }
    }
}

/// Accepted owner fact. Exits sort before enters, then by trigger/subject id, so
/// a replacement at one authority moment cannot depend on map iteration order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerOverlapFact {
    pub kind: TriggerOverlapFactKind,
    pub trigger: u64,
    pub subject: u64,
    pub scope: String,
    pub tags: Vec<String>,
    pub tick: u64,
    pub cause: TriggerReconcileCause,
    pub pair_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerVolumeDiagnosticCode {
    DuplicateDefinition,
    InvalidIdentifier,
    InvalidTag,
    StaleEntity,
    MissingCollision,
    InactiveCollision,
    MissingBounds,
    MissingTransform,
    InvalidBounds,
    InvalidTransform,
    SnapshotDecode,
    SnapshotVersion,
    SnapshotInvariant,
    QuotaExceeded,
}

impl TriggerVolumeDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DuplicateDefinition => "duplicateDefinition",
            Self::InvalidIdentifier => "invalidIdentifier",
            Self::InvalidTag => "invalidTag",
            Self::StaleEntity => "staleEntity",
            Self::MissingCollision => "missingCollision",
            Self::InactiveCollision => "inactiveCollision",
            Self::MissingBounds => "missingBounds",
            Self::MissingTransform => "missingTransform",
            Self::InvalidBounds => "invalidBounds",
            Self::InvalidTransform => "invalidTransform",
            Self::SnapshotDecode => "snapshotDecode",
            Self::SnapshotVersion => "snapshotVersion",
            Self::SnapshotInvariant => "snapshotInvariant",
            Self::QuotaExceeded => "quotaExceeded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerVolumeDiagnostic {
    pub code: TriggerVolumeDiagnosticCode,
    pub entity: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerVolumeError {
    pub diagnostics: Vec<TriggerVolumeDiagnostic>,
}

impl core::fmt::Display for TriggerVolumeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "trigger-volume operation rejected with {} diagnostic(s)",
            self.diagnostics.len()
        )
    }
}

impl std::error::Error for TriggerVolumeError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerOverlapReadout {
    pub trigger: u64,
    pub subjects: Vec<u64>,
    pub revision: u64,
    pub overlap_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerReconcileReceipt {
    pub tick: u64,
    pub cause: TriggerReconcileCause,
    pub revision: u64,
    pub facts: Vec<TriggerOverlapFact>,
    pub active_overlaps: Vec<TriggerOverlapPair>,
    pub diagnostics: Vec<TriggerVolumeDiagnostic>,
    pub overlap_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerVolumeSnapshot {
    pub schema_version: u32,
    pub revision: u64,
    pub definitions: Vec<KinematicTriggerDefinition>,
    pub active_overlaps: Vec<TriggerOverlapPair>,
    pub snapshot_hash: String,
}

/// Collision-owned trigger lifecycle state. Definitions and active pairs use
/// ordered maps/sets, making scans, facts, reads, snapshots, and hashes stable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TriggerVolumeRule {
    definitions: BTreeMap<EntityId, KinematicTriggerDefinition>,
    active_overlaps: BTreeSet<TriggerOverlapPair>,
    revision: u64,
}

impl TriggerVolumeRule {
    pub fn new(
        definitions: impl IntoIterator<Item = KinematicTriggerDefinition>,
    ) -> Result<Self, TriggerVolumeError> {
        let mut rule = Self::default();
        for definition in definitions {
            rule.register(definition)?;
        }
        Ok(rule)
    }

    /// Register one semantic trigger role. This validates the durable definition;
    /// geometry/provider validation occurs against the live EntityStore during
    /// reconcile so save/restore and activation changes remain observable.
    pub fn register(
        &mut self,
        mut definition: KinematicTriggerDefinition,
    ) -> Result<(), TriggerVolumeError> {
        let mut diagnostics = validate_kinematic_trigger_definition(&definition);
        let trigger = definition.trigger_id();
        if self.definitions.contains_key(&trigger) {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::DuplicateDefinition,
                Some(trigger),
                "trigger entity already has a definition",
            ));
        }
        if !diagnostics.is_empty() {
            return Err(TriggerVolumeError { diagnostics });
        }
        definition.tags.sort();
        definition.tags.dedup();
        self.definitions.insert(trigger, definition);
        Ok(())
    }

    pub fn definitions(&self) -> impl Iterator<Item = &KinematicTriggerDefinition> {
        self.definitions.values()
    }

    pub fn active_overlaps(&self) -> impl Iterator<Item = TriggerOverlapPair> + '_ {
        self.active_overlaps.iter().copied()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Sample current authoritative entity geometry and derive pair transitions.
    /// Invalid or inactive trigger providers produce typed diagnostics and no new
    /// pairs; any previously active pairs still emit their deterministic exits.
    pub fn reconcile(
        &mut self,
        entities: &EntityStore,
        tick: u64,
        cause: TriggerReconcileCause,
    ) -> TriggerReconcileReceipt {
        let (next, diagnostics) = self.compute_overlaps(entities);
        let exits = self
            .active_overlaps
            .difference(&next)
            .copied()
            .collect::<Vec<_>>();
        let enters = next
            .difference(&self.active_overlaps)
            .copied()
            .collect::<Vec<_>>();

        let changed = !exits.is_empty() || !enters.is_empty();
        if changed {
            self.revision = self.revision.saturating_add(1);
        }
        self.active_overlaps = next;

        let mut facts = Vec::with_capacity(exits.len() + enters.len());
        for (kind, pairs) in [
            (TriggerOverlapFactKind::Exit, exits),
            (TriggerOverlapFactKind::Enter, enters),
        ] {
            for pair in pairs {
                let definition = self
                    .definitions
                    .get(&pair.trigger_id())
                    .cloned()
                    .unwrap_or_else(|| KinematicTriggerDefinition {
                        trigger: pair.trigger,
                        scope: "removed-trigger".to_owned(),
                        tags: Vec::new(),
                    });
                facts.push(make_fact(kind, pair, definition, tick, cause));
            }
        }
        let active_overlaps = self.active_overlaps().collect::<Vec<_>>();
        let overlap_hash = overlap_hash(self.revision, &active_overlaps);
        TriggerReconcileReceipt {
            tick,
            cause,
            revision: self.revision,
            facts,
            active_overlaps,
            diagnostics,
            overlap_hash,
        }
    }

    pub fn current_overlaps(
        &self,
        trigger: EntityId,
        max_items: u32,
    ) -> Result<TriggerOverlapReadout, TriggerVolumeError> {
        let subjects = self
            .active_overlaps
            .range(
                TriggerOverlapPair::new(trigger, EntityId::new(0))
                    ..=TriggerOverlapPair::new(trigger, EntityId::new(u64::MAX)),
            )
            .map(|pair| pair.subject)
            .collect::<Vec<_>>();
        if subjects.len() > max_items as usize {
            return Err(TriggerVolumeError {
                diagnostics: vec![diagnostic(
                    TriggerVolumeDiagnosticCode::QuotaExceeded,
                    Some(trigger),
                    format!(
                        "trigger has {} overlaps but declared read quota is {max_items}",
                        subjects.len()
                    ),
                )],
            });
        }
        Ok(TriggerOverlapReadout {
            trigger: trigger.raw(),
            subjects,
            revision: self.revision,
            overlap_hash: overlap_hash(
                self.revision,
                &self
                    .active_overlaps
                    .iter()
                    .filter(|pair| pair.trigger == trigger.raw())
                    .copied()
                    .collect::<Vec<_>>(),
            ),
        })
    }

    pub fn snapshot(&self) -> TriggerVolumeSnapshot {
        let definitions = self.definitions.values().cloned().collect::<Vec<_>>();
        let active_overlaps = self.active_overlaps().collect::<Vec<_>>();
        let mut snapshot = TriggerVolumeSnapshot {
            schema_version: TRIGGER_VOLUME_SNAPSHOT_SCHEMA_VERSION,
            revision: self.revision,
            definitions,
            active_overlaps,
            snapshot_hash: String::new(),
        };
        snapshot.snapshot_hash = snapshot_content_hash(&snapshot);
        snapshot
    }

    pub fn from_snapshot(snapshot: TriggerVolumeSnapshot) -> Result<Self, TriggerVolumeError> {
        let mut diagnostics = Vec::new();
        if snapshot.schema_version != TRIGGER_VOLUME_SNAPSHOT_SCHEMA_VERSION {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::SnapshotVersion,
                None,
                format!(
                    "unsupported trigger snapshot schema version {}",
                    snapshot.schema_version
                ),
            ));
        }
        if snapshot.snapshot_hash != snapshot_content_hash(&snapshot) {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::SnapshotInvariant,
                None,
                "trigger snapshot content hash does not match",
            ));
        }
        let mut definitions = BTreeMap::new();
        let definition_ids = snapshot
            .definitions
            .iter()
            .map(|definition| definition.trigger)
            .collect::<Vec<_>>();
        if definition_ids.windows(2).any(|pair| pair[0] >= pair[1])
            || snapshot
                .definitions
                .iter()
                .any(|definition| definition.tags.windows(2).any(|pair| pair[0] >= pair[1]))
        {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::SnapshotInvariant,
                None,
                "snapshot definitions and tags must be sorted and unique",
            ));
        }
        for definition in &snapshot.definitions {
            diagnostics.extend(validate_kinematic_trigger_definition(definition));
            if definitions
                .insert(definition.trigger_id(), definition.clone())
                .is_some()
            {
                diagnostics.push(diagnostic(
                    TriggerVolumeDiagnosticCode::DuplicateDefinition,
                    Some(definition.trigger_id()),
                    "snapshot repeats a trigger definition",
                ));
            }
        }
        let active_overlaps = snapshot
            .active_overlaps
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if active_overlaps.len() != snapshot.active_overlaps.len()
            || active_overlaps.iter().copied().collect::<Vec<_>>() != snapshot.active_overlaps
            || active_overlaps.iter().any(|pair| {
                pair.trigger == pair.subject
                    || !definitions.contains_key(&EntityId::new(pair.trigger))
            })
        {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::SnapshotInvariant,
                None,
                "snapshot active pairs must be unique, non-self, and reference a definition",
            ));
        }
        if !diagnostics.is_empty() {
            return Err(TriggerVolumeError { diagnostics });
        }
        Ok(Self {
            definitions,
            active_overlaps,
            revision: snapshot.revision,
        })
    }

    pub fn encode_snapshot(&self) -> String {
        serde_json::to_string_pretty(&self.snapshot())
            .expect("trigger snapshot contains only serializable values")
            + "\n"
    }

    pub fn decode_snapshot(text: &str) -> Result<Self, TriggerVolumeError> {
        let snapshot = serde_json::from_str::<TriggerVolumeSnapshot>(text).map_err(|error| {
            TriggerVolumeError {
                diagnostics: vec![diagnostic(
                    TriggerVolumeDiagnosticCode::SnapshotDecode,
                    None,
                    error.to_string(),
                )],
            }
        })?;
        Self::from_snapshot(snapshot)
    }

    fn compute_overlaps(
        &self,
        entities: &EntityStore,
    ) -> (BTreeSet<TriggerOverlapPair>, Vec<TriggerVolumeDiagnostic>) {
        let trigger_ids = self.definitions.keys().copied().collect::<BTreeSet<_>>();
        let mut next = BTreeSet::new();
        let mut diagnostics = Vec::new();
        for trigger in self.definitions.keys() {
            let Some(trigger_aabb) =
                live_collision_aabb(entities, *trigger, true, &mut diagnostics)
            else {
                continue;
            };
            for core in entities.entities() {
                let subject = core.id;
                if subject == *trigger || trigger_ids.contains(&subject) {
                    continue;
                }
                let Some(subject_aabb) =
                    live_collision_aabb(entities, subject, false, &mut diagnostics)
                else {
                    continue;
                };
                if aabb_overlap(trigger_aabb, subject_aabb) {
                    next.insert(TriggerOverlapPair::new(*trigger, subject));
                }
            }
        }
        diagnostics.sort_by(|a, b| {
            a.entity
                .cmp(&b.entity)
                .then(a.code.cmp(&b.code))
                .then(a.message.cmp(&b.message))
        });
        diagnostics.dedup();
        (next, diagnostics)
    }
}

#[derive(Debug, Clone, Copy)]
struct WorldAabb {
    min: [f32; 3],
    max: [f32; 3],
}

fn live_collision_aabb(
    entities: &EntityStore,
    entity: EntityId,
    report_ineligible: bool,
    diagnostics: &mut Vec<TriggerVolumeDiagnostic>,
) -> Option<WorldAabb> {
    let Some(core) = entities.core(entity) else {
        if report_ineligible {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::StaleEntity,
                Some(entity),
                "trigger entity is missing",
            ));
        }
        return None;
    };
    if core.lifecycle != EntityLifecycle::Active {
        if report_ineligible && core.lifecycle == EntityLifecycle::Tombstoned {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::StaleEntity,
                Some(entity),
                "trigger entity is tombstoned",
            ));
        }
        return None;
    }
    if entities.collision(entity).is_none() {
        if report_ineligible {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::MissingCollision,
                Some(entity),
                "trigger requires the collision capability owned by CollisionRule",
            ));
        }
        return None;
    }
    if entities.active_collision(entity).is_none() {
        if report_ineligible {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::InactiveCollision,
                Some(entity),
                "trigger collision capability is inactive",
            ));
        }
        return None;
    }
    let Some(bounds) = entities.bounds(entity).map(|value| value.bounds) else {
        if report_ineligible {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::MissingBounds,
                Some(entity),
                "trigger collision provider requires bounds",
            ));
        }
        return None;
    };
    let Some(transform) = entities.transform(entity).map(|value| value.transform) else {
        if report_ineligible {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::MissingTransform,
                Some(entity),
                "trigger collision provider requires a transform",
            ));
        }
        return None;
    };
    let bounds_values = [
        bounds.min.x,
        bounds.min.y,
        bounds.min.z,
        bounds.max.x,
        bounds.max.y,
        bounds.max.z,
    ];
    if bounds_values.iter().any(|value| !value.is_finite())
        || bounds.min.x > bounds.max.x
        || bounds.min.y > bounds.max.y
        || bounds.min.z > bounds.max.z
    {
        if report_ineligible {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::InvalidBounds,
                Some(entity),
                "trigger bounds must be finite and ordered",
            ));
        }
        return None;
    }
    let translation = transform.translation;
    if [translation.x, translation.y, translation.z]
        .iter()
        .any(|value| !value.is_finite())
    {
        if report_ineligible {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::InvalidTransform,
                Some(entity),
                "trigger translation must be finite",
            ));
        }
        return None;
    }
    Some(WorldAabb {
        min: [
            bounds.min.x + translation.x,
            bounds.min.y + translation.y,
            bounds.min.z + translation.z,
        ],
        max: [
            bounds.max.x + translation.x,
            bounds.max.y + translation.y,
            bounds.max.z + translation.z,
        ],
    })
}

fn aabb_overlap(a: WorldAabb, b: WorldAabb) -> bool {
    a.min[0] < b.max[0]
        && a.max[0] > b.min[0]
        && a.min[1] < b.max[1]
        && a.max[1] > b.min[1]
        && a.min[2] < b.max[2]
        && a.max[2] > b.min[2]
}

/// Validate the same trigger metadata used by runtime installation. Stored
/// authoring calls this before promotion so invalid scope/tag syntax cannot be
/// deferred until RuntimeSession activation.
pub fn validate_kinematic_trigger_definition(
    definition: &KinematicTriggerDefinition,
) -> Vec<TriggerVolumeDiagnostic> {
    let mut diagnostics = Vec::new();
    if !valid_identifier(&definition.scope) {
        diagnostics.push(diagnostic(
            TriggerVolumeDiagnosticCode::InvalidIdentifier,
            Some(definition.trigger_id()),
            "trigger scope must be a non-empty dot/dash/underscore identifier",
        ));
    }
    for tag in &definition.tags {
        if !valid_identifier(tag) {
            diagnostics.push(diagnostic(
                TriggerVolumeDiagnosticCode::InvalidTag,
                Some(definition.trigger_id()),
                format!("invalid trigger tag `{tag}`"),
            ));
        }
    }
    diagnostics
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn make_fact(
    kind: TriggerOverlapFactKind,
    pair: TriggerOverlapPair,
    definition: KinematicTriggerDefinition,
    tick: u64,
    cause: TriggerReconcileCause,
) -> TriggerOverlapFact {
    TriggerOverlapFact {
        kind,
        trigger: pair.trigger,
        subject: pair.subject,
        scope: definition.scope,
        tags: definition.tags,
        tick,
        cause,
        pair_hash: pair_hash(pair),
    }
}

fn diagnostic(
    code: TriggerVolumeDiagnosticCode,
    entity: Option<EntityId>,
    message: impl Into<String>,
) -> TriggerVolumeDiagnostic {
    TriggerVolumeDiagnostic {
        code,
        entity: entity.map(EntityId::raw),
        message: message.into(),
    }
}

fn pair_hash(pair: TriggerOverlapPair) -> String {
    hash_bytes(format!("{}|{}", pair.trigger, pair.subject).as_bytes())
}

fn overlap_hash(revision: u64, pairs: &[TriggerOverlapPair]) -> String {
    let mut text = format!("{revision}");
    for pair in pairs {
        text.push_str(&format!("|{}:{}", pair.trigger, pair.subject));
    }
    hash_bytes(text.as_bytes())
}

fn snapshot_content_hash(snapshot: &TriggerVolumeSnapshot) -> String {
    let mut unhashed = snapshot.clone();
    unhashed.snapshot_hash.clear();
    let bytes = serde_json::to_vec(&unhashed).expect("trigger snapshot serializes");
    hash_bytes(&bytes)
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_entity::{
        CapabilityActivationAction, CapabilityActivationCommand, EntityLifecycleCommand,
        EntitySource, EntityTransform, TransformCommand,
    };
    use core_math::Vec3;

    fn create_spatial(
        store: &mut EntityStore,
        id: u64,
        at: f32,
        static_collider: bool,
    ) -> EntityId {
        let entity = EntityId::new(id);
        store
            .apply(EntityLifecycleCommand::Create {
                id: entity,
                source: EntitySource::RuntimeCreated { by: None },
                labels: Vec::new(),
            })
            .unwrap();
        store.attach_transform(entity, EntityTransform::at(Vec3::new(at, 0.0, 0.0)));
        store.attach_bounds(
            entity,
            core_entity::Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.5)),
        );
        store.attach_collision(entity, static_collider);
        entity
    }

    fn fixture() -> (EntityStore, TriggerVolumeRule, EntityId, EntityId) {
        let mut entities = EntityStore::new();
        let trigger = create_spatial(&mut entities, 10, 0.0, true);
        let subject = create_spatial(&mut entities, 20, 2.0, false);
        let rule = TriggerVolumeRule::new([KinematicTriggerDefinition::new(
            trigger,
            "zone.exit",
            ["door", "exit"],
        )])
        .unwrap();
        (entities, rule, trigger, subject)
    }

    #[test]
    fn enter_continue_exit_are_exactly_once_and_stably_hashed() {
        let (mut entities, mut rule, trigger, subject) = fixture();
        let empty = rule.reconcile(&entities, 1, TriggerReconcileCause::Tick);
        assert!(empty.facts.is_empty());

        entities
            .apply_transform(TransformCommand::Set {
                id: subject,
                transform: EntityTransform::at(Vec3::ZERO),
            })
            .unwrap();
        let entered = rule.reconcile(&entities, 2, TriggerReconcileCause::Teleport);
        assert_eq!(entered.facts.len(), 1);
        assert_eq!(entered.facts[0].kind, TriggerOverlapFactKind::Enter);
        assert_eq!(entered.facts[0].trigger, trigger.raw());

        let continued = rule.reconcile(&entities, 3, TriggerReconcileCause::Tick);
        assert!(continued.facts.is_empty());
        assert_eq!(continued.overlap_hash, entered.overlap_hash);

        entities
            .apply_transform(TransformCommand::Set {
                id: subject,
                transform: EntityTransform::at(Vec3::new(2.0, 0.0, 0.0)),
            })
            .unwrap();
        let exited = rule.reconcile(&entities, 4, TriggerReconcileCause::Teleport);
        assert_eq!(exited.facts.len(), 1);
        assert_eq!(exited.facts[0].kind, TriggerOverlapFactKind::Exit);
    }

    #[test]
    fn spawn_inside_and_teleport_through_have_explicit_endpoint_semantics() {
        let (mut entities, mut rule, _trigger, subject) = fixture();
        entities
            .apply_transform(TransformCommand::Set {
                id: subject,
                transform: EntityTransform::at(Vec3::ZERO),
            })
            .unwrap();
        let spawned = rule.reconcile(&entities, 1, TriggerReconcileCause::Spawn);
        assert_eq!(spawned.facts[0].kind, TriggerOverlapFactKind::Enter);

        entities
            .apply_transform(TransformCommand::Set {
                id: subject,
                transform: EntityTransform::at(Vec3::new(-2.0, 0.0, 0.0)),
            })
            .unwrap();
        rule.reconcile(&entities, 2, TriggerReconcileCause::Teleport);
        entities
            .apply_transform(TransformCommand::Set {
                id: subject,
                transform: EntityTransform::at(Vec3::new(2.0, 0.0, 0.0)),
            })
            .unwrap();
        let through = rule.reconcile(&entities, 3, TriggerReconcileCause::Teleport);
        assert!(
            through.facts.is_empty(),
            "endpoint-only teleport does not invent CCD"
        );
    }

    #[test]
    fn deactivation_reactivation_and_destruction_cannot_leave_stale_pairs() {
        let (mut entities, mut rule, trigger, subject) = fixture();
        entities
            .apply_transform(TransformCommand::Set {
                id: subject,
                transform: EntityTransform::at(Vec3::ZERO),
            })
            .unwrap();
        rule.reconcile(&entities, 1, TriggerReconcileCause::Teleport);

        entities
            .apply_capability_activation(CapabilityActivationCommand {
                entity: trigger,
                capability: core_entity::ActivatableCapabilityKind::Collision,
                action: CapabilityActivationAction::Deactivate,
            })
            .unwrap();
        let inactive = rule.reconcile(&entities, 2, TriggerReconcileCause::ActivationChanged);
        assert_eq!(inactive.facts[0].kind, TriggerOverlapFactKind::Exit);
        assert_eq!(
            inactive.diagnostics[0].code,
            TriggerVolumeDiagnosticCode::InactiveCollision
        );

        entities
            .apply_capability_activation(CapabilityActivationCommand {
                entity: trigger,
                capability: core_entity::ActivatableCapabilityKind::Collision,
                action: CapabilityActivationAction::Activate,
            })
            .unwrap();
        let active = rule.reconcile(&entities, 3, TriggerReconcileCause::ActivationChanged);
        assert_eq!(active.facts[0].kind, TriggerOverlapFactKind::Enter);

        entities
            .apply(EntityLifecycleCommand::Destroy { id: subject })
            .unwrap();
        let destroyed = rule.reconcile(&entities, 4, TriggerReconcileCause::LifecycleChanged);
        assert_eq!(destroyed.facts[0].kind, TriggerOverlapFactKind::Exit);
        assert!(destroyed.active_overlaps.is_empty());

        let (mut entities, mut rule, trigger, subject) = fixture();
        entities
            .apply_transform(TransformCommand::Set {
                id: subject,
                transform: EntityTransform::at(Vec3::ZERO),
            })
            .unwrap();
        rule.reconcile(&entities, 5, TriggerReconcileCause::Teleport);
        entities
            .apply(EntityLifecycleCommand::Destroy { id: trigger })
            .unwrap();
        let trigger_destroyed =
            rule.reconcile(&entities, 6, TriggerReconcileCause::LifecycleChanged);
        assert_eq!(
            trigger_destroyed.facts[0].kind,
            TriggerOverlapFactKind::Exit
        );
        assert_eq!(
            trigger_destroyed.diagnostics[0].code,
            TriggerVolumeDiagnosticCode::StaleEntity
        );
    }

    #[test]
    fn touching_faces_are_not_overlaps_matching_entity_collision_semantics() {
        let (mut entities, mut rule, _trigger, subject) = fixture();
        entities
            .apply_transform(TransformCommand::Set {
                id: subject,
                transform: EntityTransform::at(Vec3::new(1.0, 0.0, 0.0)),
            })
            .unwrap();
        let receipt = rule.reconcile(&entities, 1, TriggerReconcileCause::Teleport);
        assert!(receipt.facts.is_empty());
        assert!(receipt.active_overlaps.is_empty());
    }

    #[test]
    fn save_reload_preserves_pairs_and_does_not_duplicate_enter() {
        let (mut entities, mut rule, trigger, subject) = fixture();
        entities
            .apply_transform(TransformCommand::Set {
                id: subject,
                transform: EntityTransform::at(Vec3::ZERO),
            })
            .unwrap();
        rule.reconcile(&entities, 1, TriggerReconcileCause::Teleport);
        let encoded = rule.encode_snapshot();
        let mut restored = TriggerVolumeRule::decode_snapshot(&encoded).unwrap();
        assert_eq!(
            restored.current_overlaps(trigger, 1).unwrap().subjects,
            vec![20]
        );
        let receipt = restored.reconcile(&entities, 2, TriggerReconcileCause::Restore);
        assert!(receipt.facts.is_empty());
        assert_eq!(restored, rule);
    }

    #[test]
    fn malformed_definitions_stale_providers_and_read_quotas_fail_typed() {
        let invalid = TriggerVolumeRule::new([KinematicTriggerDefinition::new(
            EntityId::new(1),
            "bad scope",
            ["ok"],
        )])
        .unwrap_err();
        assert_eq!(
            invalid.diagnostics[0].code,
            TriggerVolumeDiagnosticCode::InvalidIdentifier
        );

        let mut rule = TriggerVolumeRule::new([KinematicTriggerDefinition::new(
            EntityId::new(99),
            "zone.stale",
            ["zone"],
        )])
        .unwrap();
        let receipt = rule.reconcile(&EntityStore::new(), 1, TriggerReconcileCause::Tick);
        assert_eq!(
            receipt.diagnostics[0].code,
            TriggerVolumeDiagnosticCode::StaleEntity
        );

        let (mut entities, mut rule, trigger, subject) = fixture();
        entities
            .apply_transform(TransformCommand::Set {
                id: subject,
                transform: EntityTransform::at(Vec3::ZERO),
            })
            .unwrap();
        rule.reconcile(&entities, 1, TriggerReconcileCause::Teleport);
        assert_eq!(
            rule.current_overlaps(trigger, 0).unwrap_err().diagnostics[0].code,
            TriggerVolumeDiagnosticCode::QuotaExceeded
        );
    }
}
