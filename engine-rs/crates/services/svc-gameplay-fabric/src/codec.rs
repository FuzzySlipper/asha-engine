use protocol_game_extension::{GameplayContractRef, GameplayEventSchemaDeclaration};
use std::any::{Any, TypeId};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayCodecError {
    UnknownContract { contract: String },
    WrongPayloadType { contract: String },
    Encode { contract: String, message: String },
    Decode { contract: String, message: String },
    SchemaDescriptorMismatch { contract: String },
    NonCanonical { contract: String },
    PayloadHashMismatch { contract: String },
    InvalidFilter { contract: String, message: String },
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
            Self::InvalidFilter { contract, message } => {
                write!(f, "filter for `{contract}` is invalid: {message}")
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
    filter: Option<TypedGameplayEventFilter<T>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GameplayEventFilterValueKind {
    Entity,
    PrefabPart,
    Text,
    Boolean,
    Integer,
    Number,
    Vector3,
}

impl GameplayEventFilterValueKind {
    fn identity(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::PrefabPart => "prefabPart",
            Self::Text => "text",
            Self::Boolean => "boolean",
            Self::Integer => "integer",
            Self::Number => "number",
            Self::Vector3 => "vector3",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameplayEventFilterValue {
    Entity(u64),
    PrefabPart { instance: u64, role: String },
    Text(String),
    Boolean(bool),
    Integer(i64),
    Number(f64),
    Vector3([f32; 3]),
}

impl GameplayEventFilterValue {
    pub fn kind(&self) -> GameplayEventFilterValueKind {
        match self {
            Self::Entity(_) => GameplayEventFilterValueKind::Entity,
            Self::PrefabPart { .. } => GameplayEventFilterValueKind::PrefabPart,
            Self::Text(_) => GameplayEventFilterValueKind::Text,
            Self::Boolean(_) => GameplayEventFilterValueKind::Boolean,
            Self::Integer(_) => GameplayEventFilterValueKind::Integer,
            Self::Number(_) => GameplayEventFilterValueKind::Number,
            Self::Vector3(_) => GameplayEventFilterValueKind::Vector3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayEventFilterFieldShape {
    pub name: String,
    pub value_kind: GameplayEventFilterValueKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameplayEventFilterField {
    pub name: String,
    pub value: GameplayEventFilterValue,
}

impl GameplayEventFilterField {
    pub fn shape(&self) -> GameplayEventFilterFieldShape {
        GameplayEventFilterFieldShape {
            name: self.name.clone(),
            value_kind: self.value.kind(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayEventFilterFieldDescriptor {
    pub name: String,
    pub value_kind: GameplayEventFilterValueKind,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayEventFilterDescriptor {
    pub fields: Vec<GameplayEventFilterFieldDescriptor>,
}

impl GameplayEventFilterDescriptor {
    pub fn identity(&self) -> String {
        let mut canonical = String::from("asha.gameplay-event-filter.v1");
        let mut fields = self.fields.iter().collect::<Vec<_>>();
        fields.sort_by(|left, right| left.name.cmp(&right.name));
        for field in fields {
            canonical.push('|');
            canonical.push_str(&field.name);
            canonical.push(':');
            canonical.push_str(field.value_kind.identity());
            canonical.push(':');
            canonical.push_str(if field.required {
                "required"
            } else {
                "optional"
            });
        }
        stable_identity([canonical.as_str()])
    }

    pub(crate) fn validates(&self, fields: &[GameplayEventFilterFieldShape]) -> Result<(), String> {
        let mut names = BTreeSet::new();
        for field in fields {
            if !names.insert(field.name.as_str()) {
                return Err(format!("filter field `{}` is duplicated", field.name));
            }
            let Some(descriptor) = self
                .fields
                .iter()
                .find(|descriptor| descriptor.name == field.name)
            else {
                return Err(format!("filter field `{}` is not published", field.name));
            };
            if descriptor.value_kind != field.value_kind {
                return Err(format!(
                    "filter field `{}` requires `{}` rather than `{}`",
                    field.name,
                    descriptor.value_kind.identity(),
                    field.value_kind.identity()
                ));
            }
        }
        for descriptor in self.fields.iter().filter(|field| field.required) {
            if !names.contains(descriptor.name.as_str()) {
                return Err(format!(
                    "required filter field `{}` is missing",
                    descriptor.name
                ));
            }
        }
        Ok(())
    }

    fn is_well_formed(&self) -> bool {
        let mut names = BTreeSet::new();
        !self.fields.is_empty()
            && self
                .fields
                .iter()
                .all(|field| !field.name.trim().is_empty() && names.insert(field.name.as_str()))
    }
}

#[derive(Clone)]
struct TypedGameplayEventFilter<T: 'static> {
    descriptor: GameplayEventFilterDescriptor,
    matches: fn(&T, &GameplayEventFilterField) -> bool,
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
            filter: None,
        }
    }

    pub fn with_filter(
        mut self,
        descriptor: GameplayEventFilterDescriptor,
        matches: fn(&T, &GameplayEventFilterField) -> bool,
    ) -> Self {
        self.filter = Some(TypedGameplayEventFilter {
            descriptor,
            matches,
        });
        self
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
    fn filter_descriptor(&self) -> Option<&GameplayEventFilterDescriptor>;
    fn matches_filter(
        &self,
        bytes: &[u8],
        fields: &[GameplayEventFilterField],
    ) -> Result<bool, GameplayCodecError>;
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
            && self
                .filter
                .as_ref()
                .is_none_or(|filter| filter.descriptor.is_well_formed())
    }

    fn filter_descriptor(&self) -> Option<&GameplayEventFilterDescriptor> {
        self.filter.as_ref().map(|filter| &filter.descriptor)
    }

    fn matches_filter(
        &self,
        bytes: &[u8],
        fields: &[GameplayEventFilterField],
    ) -> Result<bool, GameplayCodecError> {
        let contract = self.declaration.event.key();
        let Some(filter) = &self.filter else {
            return if fields.is_empty() {
                Ok(true)
            } else {
                Err(GameplayCodecError::InvalidFilter {
                    contract,
                    message: "the published event does not expose filter fields".to_owned(),
                })
            };
        };
        let shapes = fields
            .iter()
            .map(GameplayEventFilterField::shape)
            .collect::<Vec<_>>();
        filter.descriptor.validates(&shapes).map_err(|message| {
            GameplayCodecError::InvalidFilter {
                contract: contract.clone(),
                message,
            }
        })?;
        let payload = (self.decode)(bytes)
            .map_err(|message| GameplayCodecError::Decode { contract, message })?;
        Ok(fields.iter().all(|field| (filter.matches)(&payload, field)))
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
