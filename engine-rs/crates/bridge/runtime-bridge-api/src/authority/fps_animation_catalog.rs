use super::*;

pub(super) fn animation_authority_error(
    error: rule_animation_controller::AnimationAuthorityError,
) -> RuntimeBridgeError {
    RuntimeBridgeError::new(
        RuntimeBridgeErrorKind::Internal,
        format!("built-in animation authority rejected input: {error}"),
    )
}

pub(super) fn animation_projection_error(
    error: render_animation::AnimationProjectionError,
) -> RuntimeBridgeError {
    RuntimeBridgeError::new(
        RuntimeBridgeErrorKind::Internal,
        format!("built-in animation projection rejected input: {error}"),
    )
}

pub(super) fn primary_fire_animation_catalog(
    asset_id: &str,
    clip_ids: &[String],
) -> rule_animation_controller::AnimationCatalog {
    use rule_animation_controller::{
        AnimationCatalog, AnimationClipAsset, AnimationCondition, AnimationGraphDefinition,
        AnimationMotionDefinition, AnimationParameterDefinition, AnimationParameterKind,
        AnimationParameterValue, AnimationStateDefinition, AnimationTransitionDefinition,
    };

    AnimationCatalog {
        schema_version: rule_animation_controller::ANIMATION_CATALOG_SCHEMA_VERSION,
        catalog_id: "asha.fps.animation".to_string(),
        assets: vec![AnimationClipAsset {
            asset_id: asset_id.to_string(),
            clips: clip_ids.to_vec(),
        }],
        graphs: vec![AnimationGraphDefinition {
            graph_id: "fps.primary-fire".to_string(),
            version: 1,
            asset_id: asset_id.to_string(),
            initial_state_id: "ready".to_string(),
            parameters: vec![
                AnimationParameterDefinition {
                    parameter_id: "active".to_string(),
                    kind: AnimationParameterKind::Bool,
                    default_value: AnimationParameterValue::Bool(false),
                },
                AnimationParameterDefinition {
                    parameter_id: "intensity".to_string(),
                    kind: AnimationParameterKind::Float,
                    default_value: AnimationParameterValue::Float(0),
                },
            ],
            states: vec![
                AnimationStateDefinition {
                    state_id: "ready".to_string(),
                    motion: AnimationMotionDefinition::Clip {
                        clip_id: "idle".to_string(),
                        speed_milli: 1_000,
                    },
                },
                AnimationStateDefinition {
                    state_id: "primary_fire".to_string(),
                    motion: AnimationMotionDefinition::LinearBlend {
                        parameter_id: "intensity".to_string(),
                        low_clip_id: "run".to_string(),
                        high_clip_id: "jump".to_string(),
                        minimum_milli: 0,
                        maximum_milli: 1_000,
                        speed_milli: 1_000,
                    },
                },
            ],
            transitions: vec![AnimationTransitionDefinition {
                transition_id: "ready.primary_fire".to_string(),
                from_state_id: "ready".to_string(),
                to_state_id: "primary_fire".to_string(),
                priority: 0,
                duration_ticks: 4,
                conditions: vec![AnimationCondition::BoolEquals {
                    parameter_id: "active".to_string(),
                    value: true,
                }],
            }],
        }],
    }
}
