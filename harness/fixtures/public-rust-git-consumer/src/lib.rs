//! Clean-checkout consumer proof for the governed public Rust distribution.

#![forbid(unsafe_code)]

use asha_gameplay_module_sdk::GameplayStaticCompositionBuilder;
use asha_runtime_session_composition::compatibility::StaticRuntimeSessionBuilder;

pub fn gameplay_composition_builder() -> GameplayStaticCompositionBuilder {
    GameplayStaticCompositionBuilder::new()
}

pub fn runtime_session_builder_type_name() -> &'static str {
    core::any::type_name::<StaticRuntimeSessionBuilder>()
}
