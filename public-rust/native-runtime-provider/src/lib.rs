//! Public Rust facade for linking a downstream composition into ASHA's
//! generated native transport.
//!
//! A provider installs exactly one `EngineBridge` constructor during native
//! module load. The transport's complete generated operation table then acts
//! on each isolated bridge returned by that constructor; consumers do not
//! duplicate N-API verbs or register semantic callbacks.

#![forbid(unsafe_code)]

pub use native_bridge::{
    install_native_engine_bridge_factory, native_provider_module_init, NativeEngineBridgeFactory,
    NativeEngineBridgeFactoryInstallError,
};
