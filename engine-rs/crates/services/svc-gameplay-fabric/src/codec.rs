use protocol_game_extension::{GameplayContractRef, GameplayEventSchemaDeclaration};
use std::any::{Any, TypeId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayCodecError {
    UnknownContract { contract: String },
    WrongPayloadType { contract: String },
    Encode { contract: String, message: String },
    Decode { contract: String, message: String },
    SchemaDescriptorMismatch { contract: String },
    NonCanonical { contract: String },
    PayloadHashMismatch { contract: String },
}

impl core::fmt::Display for GameplayCodecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownContract { contract } => {
                write!(f, "no codec for contract `{contract}`")
            }
            Self::WrongPayloadType { contract } => {
                write!(f, "payload type does not match codec for `{contract}`")
            }
            Self::Encode { contract, message } => {
                write!(f, "codec for `{contract}` could not encode: {message}")
            }
            Self::Decode { contract, message } => {
                write!(f, "codec for `{contract}` could not decode: {message}")
            }
            Self::SchemaDescriptorMismatch { contract } => {
                write!(f, "codec schema descriptor does not match `{contract}`")
            }
            Self::NonCanonical { contract } => {
                write!(f, "payload bytes are not canonical for `{contract}`")
            }
            Self::PayloadHashMismatch { contract } => {
                write!(
                    f,
                    "payload hash does not match canonical bytes for `{contract}`"
                )
            }
        }
    }
}

impl std::error::Error for GameplayCodecError {}

#[derive(Clone)]
pub struct TypedGameplayEventCodec<T: 'static> {
    declaration: GameplayEventSchemaDeclaration,
    schema_descriptor: String,
    encode: fn(&T) -> Result<Vec<u8>, String>,
    decode: fn(&[u8]) -> Result<T, String>,
}

/// Canonical payload hashing shared by registry admission, authoring helpers,
/// routing evidence, and replay. Keeping it below the Rule layer prevents a
/// caller from choosing a parallel hash convention for the same bytes.
pub fn gameplay_canonical_payload_hash(payload: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in (payload.len() as u64)
        .to_le_bytes()
        .into_iter()
        .chain(payload.iter().copied())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}

/// Derives a schema identity from a committed canonical descriptor. The
/// descriptor names fields, value shapes, optionality, and canonical codec
/// semantics; contract constructors do not accept handwritten hash labels.
pub fn gameplay_schema_hash(schema_descriptor: &str) -> String {
    stable_identity(["asha.gameplay-schema.v1", schema_descriptor])
}

pub fn gameplay_canonical_codec_id(schema_hash: &str) -> String {
    format!("asha.canonical-codec.{schema_hash}")
}

pub fn gameplay_contract(
    namespace: impl Into<String>,
    name: impl Into<String>,
    version: u32,
    schema_descriptor: &str,
) -> GameplayContractRef {
    GameplayContractRef {
        namespace: namespace.into(),
        name: name.into(),
        version,
        schema_hash: gameplay_schema_hash(schema_descriptor),
    }
}

pub fn stable_identity<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    stable_bytes_identity(parts.into_iter().map(str::as_bytes))
}

pub fn stable_bytes_identity<'a>(parts: impl IntoIterator<Item = &'a [u8]>) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for part in parts {
        for byte in (part.len() as u64)
            .to_le_bytes()
            .into_iter()
            .chain(part.iter().copied())
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("fnv1a64:{hash:016x}")
}

/// Opaque, heterogeneous codec token for static provider composition. A
/// downstream provider constructs it from one concrete typed codec; only the
/// closed registry can erase and invoke the codec.
pub struct GameplayEventCodecRegistration {
    pub(crate) codec: RegisteredCodec,
}

impl GameplayEventCodecRegistration {
    pub fn typed<T: 'static>(codec: TypedGameplayEventCodec<T>) -> Self {
        Self {
            codec: codec.into(),
        }
    }
}

impl<T: 'static> TypedGameplayEventCodec<T> {
    pub fn new(
        declaration: GameplayEventSchemaDeclaration,
        schema_descriptor: impl Into<String>,
        encode: fn(&T) -> Result<Vec<u8>, String>,
        decode: fn(&[u8]) -> Result<T, String>,
    ) -> Self {
        Self {
            declaration,
            schema_descriptor: schema_descriptor.into(),
            encode,
            decode,
        }
    }

    pub fn declaration(&self) -> &GameplayEventSchemaDeclaration {
        &self.declaration
    }

    pub fn contract(&self) -> &GameplayContractRef {
        &self.declaration.event
    }

    pub fn schema_descriptor(&self) -> &str {
        &self.schema_descriptor
    }

    pub fn encode(&self, payload: &T) -> Result<Vec<u8>, GameplayCodecError> {
        let contract = self.declaration.event.key();
        (self.encode)(payload).map_err(|message| GameplayCodecError::Encode { contract, message })
    }
}

pub(crate) trait ErasedGameplayEventCodec: Send + Sync {
    fn declaration(&self) -> &GameplayEventSchemaDeclaration;
    fn payload_type_id(&self) -> TypeId;
    fn encode_any(&self, payload: &dyn Any) -> Result<Vec<u8>, GameplayCodecError>;
    fn decode_any(&self, bytes: &[u8]) -> Result<Box<dyn Any>, GameplayCodecError>;
    fn canonicalize(&self, bytes: &[u8]) -> Result<Vec<u8>, GameplayCodecError>;
    fn descriptor_matches_contract(&self) -> bool;
}

impl<T: 'static> ErasedGameplayEventCodec for TypedGameplayEventCodec<T> {
    fn declaration(&self) -> &GameplayEventSchemaDeclaration {
        &self.declaration
    }

    fn payload_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn encode_any(&self, payload: &dyn Any) -> Result<Vec<u8>, GameplayCodecError> {
        let contract = self.declaration.event.key();
        let typed =
            payload
                .downcast_ref::<T>()
                .ok_or_else(|| GameplayCodecError::WrongPayloadType {
                    contract: contract.clone(),
                })?;
        (self.encode)(typed).map_err(|message| GameplayCodecError::Encode { contract, message })
    }

    fn decode_any(&self, bytes: &[u8]) -> Result<Box<dyn Any>, GameplayCodecError> {
        let contract = self.declaration.event.key();
        (self.decode)(bytes)
            .map(|payload| Box::new(payload) as Box<dyn Any>)
            .map_err(|message| GameplayCodecError::Decode { contract, message })
    }

    fn canonicalize(&self, bytes: &[u8]) -> Result<Vec<u8>, GameplayCodecError> {
        let contract = self.declaration.event.key();
        let decoded = (self.decode)(bytes).map_err(|message| GameplayCodecError::Decode {
            contract: contract.clone(),
            message,
        })?;
        (self.encode)(&decoded).map_err(|message| GameplayCodecError::Encode { contract, message })
    }

    fn descriptor_matches_contract(&self) -> bool {
        self.declaration.event.schema_hash == gameplay_schema_hash(&self.schema_descriptor)
            && self.declaration.codec_id
                == gameplay_canonical_codec_id(&self.declaration.event.schema_hash)
    }
}

pub(crate) struct RegisteredCodec {
    pub event: GameplayContractRef,
    pub codec_id: String,
    pub codec: Box<dyn ErasedGameplayEventCodec>,
}

impl<T: 'static> From<TypedGameplayEventCodec<T>> for RegisteredCodec {
    fn from(codec: TypedGameplayEventCodec<T>) -> Self {
        Self {
            event: codec.declaration.event.clone(),
            codec_id: codec.declaration.codec_id.clone(),
            codec: Box::new(codec),
        }
    }
}
