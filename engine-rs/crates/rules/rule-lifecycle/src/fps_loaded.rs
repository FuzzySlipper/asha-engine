use super::*;

pub struct FpsPrimaryFireAuthorityInput<'authority> {
    pub entities: &'authority mut EntityStore,
    pub projection: &'authority CollisionProjection,
    pub ray: Ray,
    pub tick: u64,
    pub shooter_role: FpsRuntimeRole,
    pub target_role: FpsRuntimeRole,
    pub damage_delta: i64,
}

impl FpsRuntimeSessionState {
    pub fn apply_primary_fire_with_entities(
        &mut self,
        entities: &mut EntityStore,
        projection: &CollisionProjection,
        ray: Ray,
        tick: u64,
    ) -> Result<FpsPrimaryFireReceipt, FpsRuntimeError> {
        self.apply_primary_fire_for_roles_with_entities(FpsPrimaryFireAuthorityInput {
            entities,
            projection,
            ray,
            tick,
            shooter_role: FpsRuntimeRole::Player,
            target_role: FpsRuntimeRole::Enemy,
            damage_delta: 0,
        })
    }

    pub fn apply_primary_fire_with_damage_delta_and_entities(
        &mut self,
        entities: &mut EntityStore,
        projection: &CollisionProjection,
        ray: Ray,
        tick: u64,
        damage_delta: i64,
    ) -> Result<FpsPrimaryFireReceipt, FpsRuntimeError> {
        self.apply_primary_fire_for_roles_with_entities(FpsPrimaryFireAuthorityInput {
            entities,
            projection,
            ray,
            tick,
            shooter_role: FpsRuntimeRole::Player,
            target_role: FpsRuntimeRole::Enemy,
            damage_delta,
        })
    }
}

/// A freshly bootstrapped FPS session plus the entity authority produced by
/// its ProjectBundle definitions.
///
/// The bridge immediately separates these values: `EngineBridge` owns the one
/// live `EntityStore`, while [`FpsRuntimeSessionState`] retains only rule-local
/// combat, lifecycle, projection, and replay state. The wrapper keeps the
/// direct rule-lane API convenient for focused tests and non-bridge tools
/// without putting a second entity authority inside the session state.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedFpsRuntimeSession {
    pub session: FpsRuntimeSessionState,
    pub entities: EntityStore,
}

impl core::ops::Deref for LoadedFpsRuntimeSession {
    type Target = FpsRuntimeSessionState;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl core::ops::DerefMut for LoadedFpsRuntimeSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}

impl LoadedFpsRuntimeSession {
    pub fn into_parts(self) -> (FpsRuntimeSessionState, EntityStore) {
        (self.session, self.entities)
    }

    pub fn apply_encounter_transition(
        &mut self,
        preset_id: &str,
        action: FpsEncounterTransitionAction,
        lifecycle: &FpsEncounterLifecycleInput,
    ) -> Result<FpsEncounterTransitionReceipt, FpsRuntimeError> {
        self.session.apply_encounter_transition_with_entities(
            &self.entities,
            preset_id,
            action,
            lifecycle,
        )
    }

    pub fn apply_primary_fire(
        &mut self,
        projection: &CollisionProjection,
        ray: Ray,
        tick: u64,
    ) -> Result<FpsPrimaryFireReceipt, FpsRuntimeError> {
        self.session
            .apply_primary_fire_with_entities(&mut self.entities, projection, ray, tick)
    }

    pub fn apply_primary_fire_with_damage_delta(
        &mut self,
        projection: &CollisionProjection,
        ray: Ray,
        tick: u64,
        damage_delta: i64,
    ) -> Result<FpsPrimaryFireReceipt, FpsRuntimeError> {
        self.session
            .apply_primary_fire_with_damage_delta_and_entities(
                &mut self.entities,
                projection,
                ray,
                tick,
                damage_delta,
            )
    }

    pub fn apply_primary_fire_for_roles(
        &mut self,
        projection: &CollisionProjection,
        ray: Ray,
        tick: u64,
        shooter_role: FpsRuntimeRole,
        target_role: FpsRuntimeRole,
        damage_delta: i64,
    ) -> Result<FpsPrimaryFireReceipt, FpsRuntimeError> {
        self.session
            .apply_primary_fire_for_roles_with_entities(FpsPrimaryFireAuthorityInput {
                entities: &mut self.entities,
                projection,
                ray,
                tick,
                shooter_role,
                target_role,
                damage_delta,
            })
    }

    pub fn entity_lifecycle(&self, entity: EntityId) -> Option<EntityLifecycle> {
        self.session
            .entity_lifecycle_with_entities(&self.entities, entity)
    }
}
