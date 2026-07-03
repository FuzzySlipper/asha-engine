//! Minimal combat/health/raycast authority substrate.
//!
//! # Lane
//!
//! `rust-service` — validates fire intent, resolves ray hits against
//! authoritative collision projections and target bounds, mutates health state,
//! and emits deterministic events/readouts. It owns no policy, AI, UI, demo
//! weapon vocabulary, or render state.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use core_ids::EntityId;
use core_space::{WorldPos, WorldVec};
use svc_collision::{CollisionProjection, Ray};

/// Health capability/state for a combat participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthState {
    pub current: u32,
    pub max: u32,
}

impl HealthState {
    pub const fn new(current: u32, max: u32) -> Self {
        Self { current, max }
    }

    pub const fn is_defeated(self) -> bool {
        self.current == 0
    }
}

/// Deterministic health table keyed by entity id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CombatState {
    health: BTreeMap<EntityId, HealthState>,
}

impl CombatState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attach_health(&mut self, entity: EntityId, health: HealthState) -> CombatOutcome {
        if health.max == 0 || health.current > health.max {
            return CombatOutcome::Rejected {
                reason: CombatRejectionReason::InvalidHealth,
            };
        }
        self.health.insert(entity, health);
        CombatOutcome::Accepted
    }

    pub fn health(&self, entity: EntityId) -> Option<HealthState> {
        self.health.get(&entity).copied()
    }

    pub fn health_hash(&self) -> u64 {
        let mut h = fnv_offset();
        for (entity, health) in &self.health {
            feed_u64(&mut h, entity.raw());
            feed_u32(&mut h, health.current);
            feed_u32(&mut h, health.max);
        }
        h
    }
}

/// Axis-aligned target bounds for fire resolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CombatTarget {
    pub entity: EntityId,
    pub min: WorldPos,
    pub max: WorldPos,
}

/// Deterministic weapon/fire validation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireControlState {
    pub ammo: u32,
    pub cooldown_ticks_remaining: u32,
    pub cooldown_ticks_after_fire: u32,
}

impl FireControlState {
    pub const fn ready(ammo: u32, cooldown_ticks_after_fire: u32) -> Self {
        Self {
            ammo,
            cooldown_ticks_remaining: 0,
            cooldown_ticks_after_fire,
        }
    }
}

/// Proposed fire intent command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireIntentCommand {
    pub shooter: EntityId,
    pub ray: Ray,
    pub max_distance: f64,
    pub damage: u32,
    pub fire_control: FireControlState,
    pub tick: u64,
}

/// Accepted combat event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CombatEvent {
    FireHit {
        shooter: EntityId,
        target: EntityId,
        distance: f64,
        tick: u64,
    },
    FireMissed {
        shooter: EntityId,
        reason: FireMissReason,
        tick: u64,
    },
    DamageApplied {
        target: EntityId,
        amount: u32,
        before: u32,
        after: u32,
    },
    EntityDefeated {
        target: EntityId,
    },
}

/// Why an accepted fire command did not damage a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireMissReason {
    NoTarget,
    GeometryBlocked,
}

/// Fire resolution outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatOutcome {
    Accepted,
    Rejected { reason: CombatRejectionReason },
}

/// Why combat validation rejected a proposed command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatRejectionReason {
    InvalidHealth,
    InvalidRay,
    InvalidDamage,
    NoAmmo,
    Cooldown,
    UnknownTargetHealth,
}

impl CombatRejectionReason {
    pub const fn label(self) -> &'static str {
        match self {
            CombatRejectionReason::InvalidHealth => "invalidHealth",
            CombatRejectionReason::InvalidRay => "invalidRay",
            CombatRejectionReason::InvalidDamage => "invalidDamage",
            CombatRejectionReason::NoAmmo => "noAmmo",
            CombatRejectionReason::Cooldown => "cooldown",
            CombatRejectionReason::UnknownTargetHealth => "unknownTargetHealth",
        }
    }
}

/// Deterministic readout for one fire command.
#[derive(Debug, Clone, PartialEq)]
pub struct CombatReadout {
    pub outcome: CombatFireOutcome,
    pub events: Vec<CombatEvent>,
    pub next_fire_control: FireControlState,
    pub health_hash: u64,
    pub replay_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CombatFireOutcome {
    Hit {
        target: EntityId,
        distance: f64,
        defeated: bool,
    },
    Miss {
        reason: FireMissReason,
    },
}

/// Validate, resolve, and apply a fire command atomically.
pub fn apply_fire_intent(
    state: &mut CombatState,
    projection: &CollisionProjection,
    targets: &[CombatTarget],
    command: FireIntentCommand,
) -> Result<CombatReadout, CombatRejectionReason> {
    validate_fire_command(command)?;

    let geometry_hit = projection
        .raycast(command.ray, command.max_distance)
        .map(|hit| hit.distance);
    let target_hit = nearest_target_hit(command.ray, command.max_distance, targets);

    let mut next_fire_control = command.fire_control;
    next_fire_control.ammo -= 1;
    next_fire_control.cooldown_ticks_remaining = next_fire_control.cooldown_ticks_after_fire;

    let mut events = Vec::new();
    let outcome = match (target_hit, geometry_hit) {
        (Some(hit), blocker) if blocker.is_none_or(|distance| hit.distance < distance) => {
            let before = state
                .health(hit.entity)
                .ok_or(CombatRejectionReason::UnknownTargetHealth)?;
            let damage = command.damage.min(before.current);
            let after = before.current - damage;
            state.health.insert(
                hit.entity,
                HealthState {
                    current: after,
                    max: before.max,
                },
            );
            events.push(CombatEvent::FireHit {
                shooter: command.shooter,
                target: hit.entity,
                distance: hit.distance,
                tick: command.tick,
            });
            events.push(CombatEvent::DamageApplied {
                target: hit.entity,
                amount: damage,
                before: before.current,
                after,
            });
            if after == 0 {
                events.push(CombatEvent::EntityDefeated { target: hit.entity });
            }
            CombatFireOutcome::Hit {
                target: hit.entity,
                distance: hit.distance,
                defeated: after == 0,
            }
        }
        (Some(_), Some(_)) => {
            events.push(CombatEvent::FireMissed {
                shooter: command.shooter,
                reason: FireMissReason::GeometryBlocked,
                tick: command.tick,
            });
            CombatFireOutcome::Miss {
                reason: FireMissReason::GeometryBlocked,
            }
        }
        _ => {
            events.push(CombatEvent::FireMissed {
                shooter: command.shooter,
                reason: FireMissReason::NoTarget,
                tick: command.tick,
            });
            CombatFireOutcome::Miss {
                reason: FireMissReason::NoTarget,
            }
        }
    };

    let health_hash = state.health_hash();
    let replay_hash = hash_readout(&outcome, &events, next_fire_control, health_hash);
    Ok(CombatReadout {
        outcome,
        events,
        next_fire_control,
        health_hash,
        replay_hash,
    })
}

fn validate_fire_command(command: FireIntentCommand) -> Result<(), CombatRejectionReason> {
    let dir = command.ray.dir;
    let origin = command.ray.origin;
    if !origin.x.is_finite()
        || !origin.y.is_finite()
        || !origin.z.is_finite()
        || !dir.x.is_finite()
        || !dir.y.is_finite()
        || !dir.z.is_finite()
        || dir.length() <= 0.0
        || !command.max_distance.is_finite()
        || command.max_distance <= 0.0
    {
        return Err(CombatRejectionReason::InvalidRay);
    }
    if command.damage == 0 {
        return Err(CombatRejectionReason::InvalidDamage);
    }
    if command.fire_control.ammo == 0 {
        return Err(CombatRejectionReason::NoAmmo);
    }
    if command.fire_control.cooldown_ticks_remaining > 0 {
        return Err(CombatRejectionReason::Cooldown);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TargetHit {
    entity: EntityId,
    distance: f64,
}

fn nearest_target_hit(ray: Ray, max_distance: f64, targets: &[CombatTarget]) -> Option<TargetHit> {
    let dir_len = ray.dir.length();
    if dir_len <= 0.0 || !dir_len.is_finite() {
        return None;
    }
    let inv = 1.0 / dir_len;
    let dir = WorldVec::new(ray.dir.x * inv, ray.dir.y * inv, ray.dir.z * inv);
    let mut ordered = targets.to_vec();
    ordered.sort_by_key(|target| target.entity);
    let mut best: Option<TargetHit> = None;
    for target in ordered {
        let Some(distance) = ray_aabb_distance(ray.origin, dir, target.min, target.max) else {
            continue;
        };
        if distance > max_distance {
            continue;
        }
        if best.is_none_or(|hit| distance < hit.distance) {
            best = Some(TargetHit {
                entity: target.entity,
                distance,
            });
        }
    }
    best
}

fn ray_aabb_distance(origin: WorldPos, dir: WorldVec, min: WorldPos, max: WorldPos) -> Option<f64> {
    let lo = WorldPos::new(min.x.min(max.x), min.y.min(max.y), min.z.min(max.z));
    let hi = WorldPos::new(min.x.max(max.x), min.y.max(max.y), min.z.max(max.z));
    let mut t_min = 0.0f64;
    let mut t_max = f64::INFINITY;
    for (o, d, a, b) in [
        (origin.x, dir.x, lo.x, hi.x),
        (origin.y, dir.y, lo.y, hi.y),
        (origin.z, dir.z, lo.z, hi.z),
    ] {
        if d.abs() < f64::EPSILON {
            if o < a || o > b {
                return None;
            }
            continue;
        }
        let inv = 1.0 / d;
        let mut near = (a - o) * inv;
        let mut far = (b - o) * inv;
        if near > far {
            std::mem::swap(&mut near, &mut far);
        }
        t_min = t_min.max(near);
        t_max = t_max.min(far);
        if t_min > t_max {
            return None;
        }
    }
    if t_max < 0.0 {
        None
    } else {
        Some(t_min.max(0.0))
    }
}

/// Human-reviewable deterministic summary used by committed fixtures.
pub fn describe_combat_readout(readout: &CombatReadout) -> String {
    let mut out = String::new();
    out.push_str("combat-fire 1\n");
    out.push_str(&format!("outcome={}\n", outcome_label(readout.outcome)));
    out.push_str(&format!("events={}\n", readout.events.len()));
    for event in &readout.events {
        describe_event(&mut out, *event);
    }
    out.push_str(&format!(
        "next_fire_control=ammo:{} cooldown:{} after:{}\n",
        readout.next_fire_control.ammo,
        readout.next_fire_control.cooldown_ticks_remaining,
        readout.next_fire_control.cooldown_ticks_after_fire
    ));
    out.push_str(&format!("health_hash={:016x}\n", readout.health_hash));
    out.push_str(&format!("replay_hash={:016x}\n", readout.replay_hash));
    out
}

fn outcome_label(outcome: CombatFireOutcome) -> String {
    match outcome {
        CombatFireOutcome::Hit {
            target,
            distance,
            defeated,
        } => format!(
            "hit target={} distance={:.3} defeated={}",
            target.raw(),
            distance,
            defeated
        ),
        CombatFireOutcome::Miss { reason } => format!("miss reason={}", miss_label(reason)),
    }
}

fn describe_event(out: &mut String, event: CombatEvent) {
    match event {
        CombatEvent::FireHit {
            shooter,
            target,
            distance,
            tick,
        } => out.push_str(&format!(
            "event=fire_hit shooter={} target={} distance={:.3} tick={}\n",
            shooter.raw(),
            target.raw(),
            distance,
            tick
        )),
        CombatEvent::FireMissed {
            shooter,
            reason,
            tick,
        } => out.push_str(&format!(
            "event=fire_missed shooter={} reason={} tick={}\n",
            shooter.raw(),
            miss_label(reason),
            tick
        )),
        CombatEvent::DamageApplied {
            target,
            amount,
            before,
            after,
        } => out.push_str(&format!(
            "event=damage_applied target={} amount={} before={} after={}\n",
            target.raw(),
            amount,
            before,
            after
        )),
        CombatEvent::EntityDefeated { target } => {
            out.push_str(&format!("event=entity_defeated target={}\n", target.raw()))
        }
    }
}

fn miss_label(reason: FireMissReason) -> &'static str {
    match reason {
        FireMissReason::NoTarget => "noTarget",
        FireMissReason::GeometryBlocked => "geometryBlocked",
    }
}

fn hash_readout(
    outcome: &CombatFireOutcome,
    events: &[CombatEvent],
    fire_control: FireControlState,
    health_hash: u64,
) -> u64 {
    let mut h = fnv_offset();
    feed_str(&mut h, &outcome_label(*outcome));
    for event in events {
        let mut line = String::new();
        describe_event(&mut line, *event);
        feed_str(&mut h, &line);
    }
    feed_u32(&mut h, fire_control.ammo);
    feed_u32(&mut h, fire_control.cooldown_ticks_remaining);
    feed_u32(&mut h, fire_control.cooldown_ticks_after_fire);
    feed_u64(&mut h, health_hash);
    h
}

fn fnv_offset() -> u64 {
    0xcbf2_9ce4_8422_2325
}

fn feed_byte(h: &mut u64, b: u8) {
    *h ^= b as u64;
    *h = h.wrapping_mul(0x0000_0100_0000_01b3);
}

fn feed_u32(h: &mut u64, value: u32) {
    for b in value.to_le_bytes() {
        feed_byte(h, b);
    }
}

fn feed_u64(h: &mut u64, value: u64) {
    for b in value.to_le_bytes() {
        feed_byte(h, b);
    }
}

fn feed_str(h: &mut u64, value: &str) {
    for b in value.as_bytes() {
        feed_byte(h, *b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_ids::EntityId;
    use core_space::{WorldPos, WorldVec};
    use svc_collision::CollisionProjection;
    use svc_levelgen::{generate_tunnel, TunnelGeneratorConfig};

    fn tunnel_projection() -> CollisionProjection {
        let tunnel = generate_tunnel(TunnelGeneratorConfig::tiny_enclosed(17)).expect("tunnel");
        CollisionProjection::build(&tunnel.world)
    }

    fn target() -> CombatTarget {
        CombatTarget {
            entity: EntityId::new(20),
            min: WorldPos::new(2.2, 1.0, 5.0),
            max: WorldPos::new(2.8, 2.0, 5.8),
        }
    }

    fn fire(damage: u32) -> FireIntentCommand {
        FireIntentCommand {
            shooter: EntityId::new(10),
            ray: Ray::new(WorldPos::new(2.5, 1.5, 1.5), WorldVec::new(0.0, 0.0, 1.0)),
            max_distance: 16.0,
            damage,
            fire_control: FireControlState::ready(3, 4),
            tick: 7,
        }
    }

    fn state(health: u32) -> CombatState {
        let mut state = CombatState::new();
        assert_eq!(
            state.attach_health(EntityId::new(20), HealthState::new(health, health)),
            CombatOutcome::Accepted
        );
        state
    }

    #[test]
    fn fire_hits_target_and_applies_damage_before_geometry() {
        let projection = tunnel_projection();
        let mut state = state(100);
        let readout =
            apply_fire_intent(&mut state, &projection, &[target()], fire(35)).expect("fire");
        assert!(matches!(readout.outcome, CombatFireOutcome::Hit { .. }));
        assert_eq!(state.health(EntityId::new(20)).unwrap().current, 65);
        assert_eq!(readout.events.len(), 2);
    }

    #[test]
    fn lethal_damage_emits_defeat_event() {
        let projection = tunnel_projection();
        let mut state = state(40);
        let readout =
            apply_fire_intent(&mut state, &projection, &[target()], fire(100)).expect("fire");
        assert!(matches!(
            readout.outcome,
            CombatFireOutcome::Hit { defeated: true, .. }
        ));
        assert_eq!(state.health(EntityId::new(20)).unwrap().current, 0);
        assert!(matches!(
            readout.events.last(),
            Some(CombatEvent::EntityDefeated { target }) if *target == EntityId::new(20)
        ));
    }

    #[test]
    fn geometry_blocks_target_behind_wall() {
        let projection = tunnel_projection();
        let mut state = state(100);
        let blocked = CombatTarget {
            entity: EntityId::new(20),
            min: WorldPos::new(-2.0, 1.0, 1.0),
            max: WorldPos::new(-1.2, 2.0, 2.0),
        };
        let mut command = fire(20);
        command.ray = Ray::new(WorldPos::new(2.5, 1.5, 1.5), WorldVec::new(-1.0, 0.0, 0.0));
        let readout =
            apply_fire_intent(&mut state, &projection, &[blocked], command).expect("fire");
        assert_eq!(
            readout.outcome,
            CombatFireOutcome::Miss {
                reason: FireMissReason::GeometryBlocked
            }
        );
        assert_eq!(state.health(EntityId::new(20)).unwrap().current, 100);
    }

    #[test]
    fn invalid_fire_command_rejects_without_mutation() {
        let projection = tunnel_projection();
        let mut state = state(100);
        let mut command = fire(20);
        command.fire_control.ammo = 0;
        assert_eq!(
            apply_fire_intent(&mut state, &projection, &[target()], command),
            Err(CombatRejectionReason::NoAmmo)
        );
        assert_eq!(state.health(EntityId::new(20)).unwrap().current, 100);
    }

    #[test]
    fn fire_readout_matches_committed_golden() {
        let projection = tunnel_projection();
        let mut state = state(40);
        let readout =
            apply_fire_intent(&mut state, &projection, &[target()], fire(100)).expect("fire");
        assert_eq!(
            describe_combat_readout(&readout),
            include_str!(
                "../../../../../harness/fixtures/combat/generated-tunnel-fire.snapshot.txt"
            )
        );
    }
}
