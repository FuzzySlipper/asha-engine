//! Immutable gameplay-fabric contracts, typed codecs, and Session topology.
//!
//! # Lane
//!
//! `rust-service` — validates a statically linked module/provider/owner graph
//! before RuntimeSession bootstrap commits. This crate does not dispatch
//! handlers, apply proposals, own persistent module state, or mutate a live
//! registry.

#![forbid(unsafe_code)]

mod codec;
mod registry;
mod topology;
mod validation;

pub use codec::{
    gameplay_canonical_codec_id, gameplay_canonical_payload_hash, gameplay_contract,
    gameplay_schema_hash, stable_bytes_identity, stable_identity, GameplayCodecError,
    GameplayEventCodecRegistration, TypedGameplayEventCodec,
};
pub use registry::{
    GameplayEventMetadata, GameplayFabricRegistry, GameplayFabricRegistryBuilder,
    GameplayLinkedProvider, GameplayProposalMetadata, GameplayProposalOwnerRegistration,
    GameplayReadViewProviderRegistration, GameplayRegistryBuildError,
    GameplayStateOwnerRegistration,
};
