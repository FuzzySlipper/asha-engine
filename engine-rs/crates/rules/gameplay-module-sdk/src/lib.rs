//! Public Rust gameplay-module authoring and static composition substrate.
//!
//! Downstream crates should normally import the `asha-gameplay-module-sdk`
//! facade under `public-rust/`, not this engine ownership cell directly.

#![forbid(unsafe_code)]

mod authoring;
mod binding;
mod composition;
mod ergonomics;
mod facade;
mod legacy_weapon;

pub use authoring::*;
pub use binding::*;
pub use composition::*;
pub use ergonomics::*;
pub use facade::*;
pub use legacy_weapon::*;
