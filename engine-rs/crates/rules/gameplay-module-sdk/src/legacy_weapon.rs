use crate::{
    GameRuleModule, GameplayModuleActions, GameplayModuleBehavior, GameplayModuleContext,
    GameplayModuleError, WeaponEffectHookRequest,
};

/// Named compatibility edge for the legacy weapon-effect trait. It executes
/// the downstream module's real range-sensitive behavior inside the common
/// Transform family. Delete after downstream providers migrate to native
/// gameplay-module contracts and #5634 compatibility consumers are retired.
pub struct LegacyWeaponEffectTransformBehavior<M> {
    module: M,
}

impl<M> LegacyWeaponEffectTransformBehavior<M> {
    pub fn new(module: M) -> Self {
        Self { module }
    }
}

impl<M: GameRuleModule + Send> GameplayModuleBehavior for LegacyWeaponEffectTransformBehavior<M> {
    fn invoke(
        &self,
        context: &GameplayModuleContext<'_>,
    ) -> Result<GameplayModuleActions, GameplayModuleError> {
        let request: WeaponEffectHookRequest = context.decision_workspace()?;
        let proposal = self
            .module
            .evaluate_weapon_effect(&request)
            .map_err(|diagnostic| GameplayModuleError {
                code: format!("legacyWeapon.{:?}", diagnostic.code),
                message: diagnostic.message,
            })?;
        let contract = context
            .decision_workspace_contract()
            .cloned()
            .ok_or_else(|| GameplayModuleError {
                code: "legacyWeapon.notTransform".to_owned(),
                message: "legacy weapon behavior requires a decision Workspace".to_owned(),
            })?;
        let workspace_hash = context
            .decision_workspace_hash()
            .ok_or_else(|| GameplayModuleError {
                code: "legacyWeapon.missingWorkspaceHash".to_owned(),
                message: "legacy weapon Workspace hash is unavailable".to_owned(),
            })?
            .to_owned();
        let mut actions = context.actions();
        actions.transform_workspace_json(contract, workspace_hash, &proposal)?;
        actions.trace("legacyWeapon.transformCompatibility");
        Ok(actions)
    }
}
