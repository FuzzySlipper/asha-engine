//! Std-only canonical JSON codec for the runtime session-state snapshot
//! (post-launchable-02, Den task #2484).
//!
//! [`EntityStore::snapshot`](crate::store::EntityStore::snapshot) already produces
//! a deterministic, in-memory [`EntitySnapshot`]; this module gives that snapshot a
//! durable on-disk form so runtime-created entities, runtime-diverged transforms,
//! capability tables, relations (transform attachment, containment, source
//! ancestry), and source traces survive a world-bundle save → reload.
//!
//! Like `core-scene`'s scene-document codec, the codec is hand-written std-only
//! JSON (the workspace carries no serde dependency). [`encode_snapshot`] emits a
//! deterministic document — records in ascending id order, fixed field order, one
//! capability/relation field per slot — and [`decode_snapshot`] parses it back,
//! **failing closed** on an unsupported schema version, a malformed structure, or
//! an unknown enum discriminant rather than guessing. Encode∘decode reproduces the
//! [`EntityStore::hash`](crate::store::EntityStore::hash) exactly, which the
//! round-trip tests pin.

use core_assets::{AssetHash, AssetId, AssetReference, AssetVersionReq};
use core_ids::{EntityId, ProcessId, SceneNodeId, SubjectId, TagId};
use core_math::Vec3;

use crate::capability::{
    AssetBindingCapability, BoundsCapability, CollisionCapability, ContainmentCapability,
    ControllerCapability, RenderProjectionCapability, TransformCapability,
};
use crate::core::{EntityCore, EntityLifecycle, EntitySource};
use crate::store::{EntityRecord, EntitySnapshot};
use crate::value::{Aabb, EntityTransform, Quat};

/// Compatibility marker for the on-disk session-state snapshot. A snapshot whose
/// schema version is newer than this build understands fails closed at decode.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

// ── Encode ──────────────────────────────────────────────────────────────────--

/// Encode a snapshot as canonical JSON (LF newlines, trailing newline). Records
/// are emitted in the snapshot's existing ascending-id order with a fixed field
/// order, so equivalent stores encode byte-identically.
pub fn encode_snapshot(snapshot: &EntitySnapshot) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"schemaVersion\": {SNAPSHOT_SCHEMA_VERSION},\n"
    ));
    out.push_str("  \"records\": [");
    if snapshot.records.is_empty() {
        out.push(']');
    } else {
        out.push('\n');
        for (i, record) in snapshot.records.iter().enumerate() {
            out.push_str("    ");
            encode_record(&mut out, record);
            if i + 1 < snapshot.records.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]");
    }
    out.push('\n');
    out.push_str("}\n");
    out
}

fn encode_record(out: &mut String, record: &EntityRecord) {
    out.push_str("{ \"id\": ");
    out.push_str(&record.core.id.raw().to_string());
    out.push_str(", \"lifecycle\": ");
    out.push_str(&format!("\"{}\"", record.core.lifecycle.label()));
    out.push_str(", \"source\": ");
    encode_source(out, &record.core.source);
    out.push_str(", \"labels\": ");
    encode_u64_array(out, record.core.labels.iter().map(|t| t.raw()));

    out.push_str(", \"transform\": ");
    match &record.transform {
        Some(t) => encode_transform(out, &t.transform),
        None => out.push_str("null"),
    }
    out.push_str(", \"bounds\": ");
    match &record.bounds {
        Some(b) => encode_aabb(out, &b.bounds),
        None => out.push_str("null"),
    }
    out.push_str(", \"render\": ");
    match &record.render {
        Some(r) => out.push_str(&format!("{{ \"visible\": {} }}", r.visible)),
        None => out.push_str("null"),
    }
    out.push_str(", \"collision\": ");
    match &record.collision {
        Some(c) => out.push_str(&format!("{{ \"staticCollider\": {} }}", c.static_collider)),
        None => out.push_str("null"),
    }
    out.push_str(", \"containment\": ");
    match &record.containment {
        Some(c) => out.push_str(&format!("{{ \"container\": {} }}", c.container.raw())),
        None => out.push_str("null"),
    }
    out.push_str(", \"controller\": ");
    match &record.controller {
        Some(ControllerCapability::Process(p)) => {
            out.push_str(&format!("{{ \"kind\": \"process\", \"id\": {} }}", p.raw()))
        }
        Some(ControllerCapability::Subject(s)) => {
            out.push_str(&format!("{{ \"kind\": \"subject\", \"id\": {} }}", s.raw()))
        }
        None => out.push_str("null"),
    }
    out.push_str(", \"assetBinding\": ");
    match &record.asset_binding {
        Some(a) => {
            out.push_str("{ \"asset\": ");
            encode_asset_ref(out, &a.asset);
            out.push_str(" }");
        }
        None => out.push_str("null"),
    }
    out.push_str(", \"transformParent\": ");
    encode_opt_id(out, record.transform_parent);
    out.push_str(", \"derivedFrom\": ");
    encode_opt_id(out, record.derived_from);
    out.push_str(" }");
}

fn encode_source(out: &mut String, source: &EntitySource) {
    match source {
        EntitySource::SceneBootstrap { node } => out.push_str(&format!(
            "{{ \"kind\": \"sceneBootstrap\", \"node\": {} }}",
            node.raw()
        )),
        EntitySource::RuntimeCreated { by } => {
            out.push_str("{ \"kind\": \"runtimeCreated\", \"by\": ");
            match by {
                Some(p) => out.push_str(&p.raw().to_string()),
                None => out.push_str("null"),
            }
            out.push_str(" }");
        }
        EntitySource::Imported { asset } => {
            out.push_str("{ \"kind\": \"imported\", \"asset\": ");
            encode_asset_ref(out, asset);
            out.push_str(" }");
        }
        EntitySource::DiagnosticTooling => out.push_str("{ \"kind\": \"diagnosticTooling\" }"),
        EntitySource::PolicyProposed { by } => out.push_str(&format!(
            "{{ \"kind\": \"policyProposed\", \"by\": {} }}",
            by.raw()
        )),
    }
}

fn encode_asset_ref(out: &mut String, r: &AssetReference) {
    out.push_str(&format!(
        "{{ \"id\": \"{}\", \"version\": ",
        r.id().as_str()
    ));
    match r.version() {
        AssetVersionReq::Any => out.push_str("{ \"req\": \"any\" }"),
        AssetVersionReq::Exact(v) => {
            out.push_str(&format!("{{ \"req\": \"exact\", \"value\": {v} }}"))
        }
        AssetVersionReq::AtLeast(v) => {
            out.push_str(&format!("{{ \"req\": \"atLeast\", \"value\": {v} }}"))
        }
    }
    out.push_str(", \"hash\": ");
    match r.hash() {
        Some(h) => out.push_str(&format!("\"{}\"", h.as_str())),
        None => out.push_str("null"),
    }
    out.push_str(" }");
}

fn encode_transform(out: &mut String, t: &EntityTransform) {
    out.push_str("{ \"translation\": ");
    encode_vec3(out, t.translation);
    out.push_str(&format!(
        ", \"rotation\": [{}, {}, {}, {}]",
        fmt_f32(t.rotation.x),
        fmt_f32(t.rotation.y),
        fmt_f32(t.rotation.z),
        fmt_f32(t.rotation.w)
    ));
    out.push_str(", \"scale\": ");
    encode_vec3(out, t.scale);
    out.push_str(" }");
}

fn encode_aabb(out: &mut String, a: &Aabb) {
    out.push_str("{ \"min\": ");
    encode_vec3(out, a.min);
    out.push_str(", \"max\": ");
    encode_vec3(out, a.max);
    out.push_str(" }");
}

fn encode_vec3(out: &mut String, v: Vec3) {
    out.push_str(&format!(
        "[{}, {}, {}]",
        fmt_f32(v.x),
        fmt_f32(v.y),
        fmt_f32(v.z)
    ));
}

fn encode_u64_array(out: &mut String, items: impl Iterator<Item = u64>) {
    out.push('[');
    for (i, n) in items.enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&n.to_string());
    }
    out.push(']');
}

fn encode_opt_id(out: &mut String, id: Option<EntityId>) {
    match id {
        Some(id) => out.push_str(&id.raw().to_string()),
        None => out.push_str("null"),
    }
}

/// Shortest round-trippable rendering of an `f32` (Rust's `Display` is
/// deterministic), so canonical output is stable across runs/platforms.
fn fmt_f32(v: f32) -> String {
    format!("{v}")
}

// ── Decode ──────────────────────────────────────────────────────────────────--

/// Why decoding a session-state snapshot failed. Every variant is fail-closed: a
/// rejected snapshot never partially mutates a store.
#[derive(Debug, Clone, PartialEq)]
pub enum SnapshotDecodeError {
    /// The bytes were not valid JSON.
    Json(String),
    /// The schema version is newer than this build understands (fail closed).
    UnsupportedSchema { found: u32, supported: u32 },
    /// A required field was missing or had the wrong type.
    Field(String),
    /// An asset id/hash string in the snapshot was malformed.
    Asset(String),
    /// A closed-enum discriminant (lifecycle / source kind / controller / version
    /// req) was not recognized.
    UnknownVariant(String),
}

impl core::fmt::Display for SnapshotDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SnapshotDecodeError::Json(e) => write!(f, "snapshot is not valid JSON: {e}"),
            SnapshotDecodeError::UnsupportedSchema { found, supported } => write!(
                f,
                "snapshot schema version {found} is newer than supported {supported}"
            ),
            SnapshotDecodeError::Field(e) => write!(f, "snapshot field error: {e}"),
            SnapshotDecodeError::Asset(e) => write!(f, "snapshot asset reference invalid: {e}"),
            SnapshotDecodeError::UnknownVariant(e) => {
                write!(f, "snapshot has unknown discriminant: {e}")
            }
        }
    }
}

impl std::error::Error for SnapshotDecodeError {}

/// Decode canonical spatial-session-state-snapshot JSON into an [`EntitySnapshot`]. The
/// result is suitable for [`EntityStore::from_snapshot`](crate::store::EntityStore::from_snapshot).
/// Fails closed on schema mismatch, malformed structure, or unknown discriminants.
pub fn decode_snapshot(input: &str) -> Result<EntitySnapshot, SnapshotDecodeError> {
    let json = Json::parse(input).map_err(SnapshotDecodeError::Json)?;
    let schema_version = field_u64(&json, "schemaVersion")? as u32;
    if schema_version > SNAPSHOT_SCHEMA_VERSION {
        return Err(SnapshotDecodeError::UnsupportedSchema {
            found: schema_version,
            supported: SNAPSHOT_SCHEMA_VERSION,
        });
    }
    let records_json = field(&json, "records")?
        .as_array()
        .ok_or_else(|| SnapshotDecodeError::Field("records must be an array".into()))?;
    let mut records = Vec::with_capacity(records_json.len());
    for r in records_json {
        records.push(decode_record(r)?);
    }
    Ok(EntitySnapshot { records })
}

fn decode_record(j: &Json) -> Result<EntityRecord, SnapshotDecodeError> {
    let id = EntityId::new(field_u64(j, "id")?);
    let lifecycle = decode_lifecycle(field_str(j, "lifecycle")?)?;
    let source = decode_source(field(j, "source")?)?;
    let labels = decode_u64_array(j.get("labels"))?
        .into_iter()
        .map(TagId::new)
        .collect();

    let core = EntityCore {
        id,
        lifecycle,
        source,
        labels,
    };

    let transform = match opt_obj(j, "transform")? {
        Some(t) => Some(TransformCapability {
            transform: decode_transform(t)?,
        }),
        None => None,
    };
    let bounds = match opt_obj(j, "bounds")? {
        Some(b) => Some(BoundsCapability {
            bounds: decode_aabb(b)?,
        }),
        None => None,
    };
    let render = match opt_obj(j, "render")? {
        Some(r) => Some(RenderProjectionCapability {
            visible: field_bool(r, "visible")?,
        }),
        None => None,
    };
    let collision = match opt_obj(j, "collision")? {
        Some(c) => Some(CollisionCapability {
            static_collider: field_bool(c, "staticCollider")?,
        }),
        None => None,
    };
    let containment = match opt_obj(j, "containment")? {
        Some(c) => Some(ContainmentCapability {
            container: EntityId::new(field_u64(c, "container")?),
        }),
        None => None,
    };
    let controller = match opt_obj(j, "controller")? {
        Some(c) => Some(decode_controller(c)?),
        None => None,
    };
    let asset_binding = match opt_obj(j, "assetBinding")? {
        Some(a) => Some(AssetBindingCapability {
            asset: decode_asset_ref(field(a, "asset")?)?,
        }),
        None => None,
    };
    let transform_parent = decode_opt_id(j, "transformParent")?;
    let derived_from = decode_opt_id(j, "derivedFrom")?;

    Ok(EntityRecord {
        core,
        transform,
        bounds,
        render,
        collision,
        containment,
        controller,
        asset_binding,
        transform_parent,
        derived_from,
    })
}

fn decode_lifecycle(tag: &str) -> Result<EntityLifecycle, SnapshotDecodeError> {
    match tag {
        "active" => Ok(EntityLifecycle::Active),
        "disabled" => Ok(EntityLifecycle::Disabled),
        "tombstoned" => Ok(EntityLifecycle::Tombstoned),
        other => Err(SnapshotDecodeError::UnknownVariant(format!(
            "lifecycle `{other}`"
        ))),
    }
}

fn decode_source(j: &Json) -> Result<EntitySource, SnapshotDecodeError> {
    let kind = field_str(j, "kind")?;
    match kind {
        "sceneBootstrap" => Ok(EntitySource::SceneBootstrap {
            node: SceneNodeId::new(field_u64(j, "node")?),
        }),
        "runtimeCreated" => {
            let by = match j.get("by") {
                None | Some(Json::Null) => None,
                Some(Json::Num(_)) => Some(ProcessId::new(field_u64(j, "by")?)),
                Some(_) => {
                    return Err(SnapshotDecodeError::Field(
                        "source.by must be a number or null".into(),
                    ))
                }
            };
            Ok(EntitySource::RuntimeCreated { by })
        }
        "imported" => Ok(EntitySource::Imported {
            asset: decode_asset_ref(field(j, "asset")?)?,
        }),
        "diagnosticTooling" => Ok(EntitySource::DiagnosticTooling),
        "policyProposed" => Ok(EntitySource::PolicyProposed {
            by: SubjectId::new(field_u64(j, "by")?),
        }),
        other => Err(SnapshotDecodeError::UnknownVariant(format!(
            "source kind `{other}`"
        ))),
    }
}

fn decode_controller(j: &Json) -> Result<ControllerCapability, SnapshotDecodeError> {
    let kind = field_str(j, "kind")?;
    match kind {
        "process" => Ok(ControllerCapability::Process(ProcessId::new(field_u64(
            j, "id",
        )?))),
        "subject" => Ok(ControllerCapability::Subject(SubjectId::new(field_u64(
            j, "id",
        )?))),
        other => Err(SnapshotDecodeError::UnknownVariant(format!(
            "controller kind `{other}`"
        ))),
    }
}

fn decode_asset_ref(j: &Json) -> Result<AssetReference, SnapshotDecodeError> {
    let id_str = field_str(j, "id")?;
    let id = AssetId::parse(id_str).map_err(|e| SnapshotDecodeError::Asset(e.to_string()))?;

    let version = match j.get("version") {
        None | Some(Json::Null) => AssetVersionReq::Any,
        Some(v) => {
            let req = field_str(v, "req")?;
            match req {
                "any" => AssetVersionReq::Any,
                "exact" => AssetVersionReq::Exact(field_u64(v, "value")? as u32),
                "atLeast" => AssetVersionReq::AtLeast(field_u64(v, "value")? as u32),
                other => {
                    return Err(SnapshotDecodeError::UnknownVariant(format!(
                        "version req `{other}`"
                    )))
                }
            }
        }
    };

    let hash = match j.get("hash") {
        None | Some(Json::Null) => None,
        Some(Json::Str(s)) => {
            Some(AssetHash::parse(s).map_err(|e| SnapshotDecodeError::Asset(e.to_string()))?)
        }
        Some(_) => {
            return Err(SnapshotDecodeError::Field(
                "asset.hash must be a string or null".into(),
            ))
        }
    };
    Ok(AssetReference::new(id, version, hash))
}

fn decode_transform(j: &Json) -> Result<EntityTransform, SnapshotDecodeError> {
    let translation = decode_vec3(field(j, "translation")?)?;
    let rot = field(j, "rotation")?
        .as_array()
        .filter(|a| a.len() == 4)
        .ok_or_else(|| SnapshotDecodeError::Field("rotation must be a 4-array".into()))?;
    let rotation = Quat {
        x: num(&rot[0])?,
        y: num(&rot[1])?,
        z: num(&rot[2])?,
        w: num(&rot[3])?,
    };
    let scale = decode_vec3(field(j, "scale")?)?;
    Ok(EntityTransform {
        translation,
        rotation,
        scale,
    })
}

fn decode_aabb(j: &Json) -> Result<Aabb, SnapshotDecodeError> {
    Ok(Aabb {
        min: decode_vec3(field(j, "min")?)?,
        max: decode_vec3(field(j, "max")?)?,
    })
}

fn decode_vec3(j: &Json) -> Result<Vec3, SnapshotDecodeError> {
    let a = j
        .as_array()
        .filter(|a| a.len() == 3)
        .ok_or_else(|| SnapshotDecodeError::Field("vec3 must be a 3-array".into()))?;
    Ok(Vec3::new(num(&a[0])?, num(&a[1])?, num(&a[2])?))
}

fn decode_u64_array(j: Option<&Json>) -> Result<Vec<u64>, SnapshotDecodeError> {
    match j {
        None | Some(Json::Null) => Ok(Vec::new()),
        Some(Json::Arr(items)) => items
            .iter()
            .map(|i| {
                i.as_u64().ok_or_else(|| {
                    SnapshotDecodeError::Field("expected a u64 array element".into())
                })
            })
            .collect(),
        Some(_) => Err(SnapshotDecodeError::Field("expected an array".into())),
    }
}

fn decode_opt_id(j: &Json, key: &str) -> Result<Option<EntityId>, SnapshotDecodeError> {
    match j.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Num(_)) => Ok(Some(EntityId::new(field_u64(j, key)?))),
        Some(_) => Err(SnapshotDecodeError::Field(format!(
            "field `{key}` must be a number or null"
        ))),
    }
}

// ── Small typed-field helpers over `Json` ─────────────────────────────────────

fn field<'a>(j: &'a Json, key: &str) -> Result<&'a Json, SnapshotDecodeError> {
    j.get(key)
        .ok_or_else(|| SnapshotDecodeError::Field(format!("missing field `{key}`")))
}

fn field_u64(j: &Json, key: &str) -> Result<u64, SnapshotDecodeError> {
    field(j, key)?.as_u64().ok_or_else(|| {
        SnapshotDecodeError::Field(format!("field `{key}` must be a non-negative integer"))
    })
}

fn field_str<'a>(j: &'a Json, key: &str) -> Result<&'a str, SnapshotDecodeError> {
    field(j, key)?
        .as_str()
        .ok_or_else(|| SnapshotDecodeError::Field(format!("field `{key}` must be a string")))
}

fn field_bool(j: &Json, key: &str) -> Result<bool, SnapshotDecodeError> {
    match field(j, key)? {
        Json::Bool(b) => Ok(*b),
        _ => Err(SnapshotDecodeError::Field(format!(
            "field `{key}` must be a boolean"
        ))),
    }
}

/// Resolve a field that is either a JSON object or explicitly `null`/absent.
/// `Ok(None)` for null/absent; `Err` if present but not an object.
fn opt_obj<'a>(j: &'a Json, key: &str) -> Result<Option<&'a Json>, SnapshotDecodeError> {
    match j.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(o @ Json::Obj(_)) => Ok(Some(o)),
        Some(_) => Err(SnapshotDecodeError::Field(format!(
            "field `{key}` must be an object or null"
        ))),
    }
}

fn num(j: &Json) -> Result<f32, SnapshotDecodeError> {
    match j {
        Json::Num(n) => Ok(*n as f32),
        _ => Err(SnapshotDecodeError::Field("expected a number".into())),
    }
}

// ── Minimal JSON value + parser (std-only) ────────────────────────────────────
//
// Mirrors `core-scene`'s scene-document parser: the workspace has no serde
// dependency, so each contract surface hand-writes the JSON subset it needs.

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn parse(input: &str) -> Result<Json, String> {
        let chars: Vec<char> = input.chars().collect();
        let mut p = Parser { chars, pos: 0 };
        p.skip_ws();
        let v = p.value()?;
        p.skip_ws();
        if p.pos != p.chars.len() {
            return Err(format!("trailing input at position {}", p.pos));
        }
        Ok(v)
    }

    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            Json::Num(n) if n.fract() == 0.0 && *n >= 0.0 => Some(*n as u64),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.pos += 1;
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(Json::Str(self.string()?)),
            Some('t') | Some('f') => self.boolean(),
            Some('n') => self.null(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            other => Err(format!("unexpected {other:?} at {}", self.pos)),
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(Json::Obj(entries));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(':')?;
            let val = self.value()?;
            entries.push((key, val));
            self.skip_ws();
            match self.bump() {
                Some(',') => continue,
                Some('}') => break,
                other => return Err(format!("expected ',' or '}}', got {other:?}")),
            }
        }
        Ok(Json::Obj(entries))
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.bump() {
                Some(',') => continue,
                Some(']') => break,
                other => return Err(format!("expected ',' or ']', got {other:?}")),
            }
        }
        Ok(Json::Arr(items))
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    other => return Err(format!("bad escape {other:?}")),
                },
                Some(c) => out.push(c),
                None => return Err("unterminated string".into()),
            }
        }
        Ok(out)
    }

    fn boolean(&mut self) -> Result<Json, String> {
        if self.consume("true") {
            Ok(Json::Bool(true))
        } else if self.consume("false") {
            Ok(Json::Bool(false))
        } else {
            Err(format!("bad literal at {}", self.pos))
        }
    }

    fn null(&mut self) -> Result<Json, String> {
        if self.consume("null") {
            Ok(Json::Null)
        } else {
            Err(format!("bad literal at {}", self.pos))
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-')
        {
            self.pos += 1;
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        s.parse::<f64>()
            .map(Json::Num)
            .map_err(|_| format!("bad number `{s}`"))
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.bump() == Some(c) {
            Ok(())
        } else {
            Err(format!("expected '{c}' at {}", self.pos))
        }
    }

    fn consume(&mut self, lit: &str) -> bool {
        let end = self.pos + lit.len();
        if end <= self.chars.len() && self.chars[self.pos..end].iter().collect::<String>() == lit {
            self.pos = end;
            true
        } else {
            false
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::EntityLifecycleCommand;
    use crate::relation::RelationCommand;
    use crate::store::EntityStore;
    use core_assets::AssetId;
    use core_math::Vec3;

    /// Build a mixed-world store exercising every fixture vocabulary class the
    /// session-state snapshot must persist (Den task #2484): a runtime-created
    /// spatial rendered entity, a spatial non-rendered collider, a non-spatial
    /// logical entity, a containment relation, a transform attachment, and an
    /// asset-bound import plus a diverged transform.
    fn mixed_world() -> EntityStore {
        let mut store = EntityStore::new();

        // 1. scene-sourced spatial rendered entity, transform diverged from origin.
        let scene = EntityId::new(1);
        store
            .apply(EntityLifecycleCommand::Create {
                id: scene,
                source: EntitySource::SceneBootstrap {
                    node: SceneNodeId::new(10),
                },
                labels: vec![TagId::new(3), TagId::new(7)],
            })
            .unwrap();
        store.attach_transform(scene, EntityTransform::at(Vec3::new(4.0, 0.5, -2.0)));
        store.attach_render_projection(scene, true);

        // 2. runtime-created spatial non-rendered collider.
        let collider = EntityId::new(2);
        store
            .apply(EntityLifecycleCommand::Create {
                id: collider,
                source: EntitySource::RuntimeCreated {
                    by: Some(ProcessId::new(99)),
                },
                labels: vec![],
            })
            .unwrap();
        store.attach_transform(collider, EntityTransform::IDENTITY);
        store.attach_bounds(
            collider,
            Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)),
        );
        store.attach_collision(collider, true);

        // 3. non-spatial logical entity (no transform), controller association.
        let logical = EntityId::new(3);
        store
            .apply(EntityLifecycleCommand::Create {
                id: logical,
                source: EntitySource::PolicyProposed {
                    by: SubjectId::new(5),
                },
                labels: vec![TagId::new(1)],
            })
            .unwrap();
        store.attach_controller(logical, ControllerCapability::Subject(SubjectId::new(5)));

        // 4. contained member (containment relation into the collider).
        let member = EntityId::new(4);
        store
            .apply(EntityLifecycleCommand::Create {
                id: member,
                source: EntitySource::RuntimeCreated { by: None },
                labels: vec![],
            })
            .unwrap();
        store.attach_containment(member, collider);

        // 5. attached child (transform parent = scene) + asset binding + source trace.
        let child = EntityId::new(5);
        store
            .apply(EntityLifecycleCommand::Create {
                id: child,
                source: EntitySource::Imported {
                    asset: AssetReference::new(
                        AssetId::parse("mesh/crate").unwrap(),
                        AssetVersionReq::Exact(2),
                        None,
                    ),
                },
                labels: vec![],
            })
            .unwrap();
        store.attach_transform(child, EntityTransform::at(Vec3::new(0.0, 1.0, 0.0)));
        store.attach_asset_binding(
            child,
            AssetReference::new(
                AssetId::parse("mesh/crate").unwrap(),
                AssetVersionReq::Any,
                None,
            ),
        );
        // transform attachment child → scene, and a source-ancestry trace child → member.
        store
            .apply_relation(RelationCommand::AttachTransformParent {
                child,
                parent: scene,
            })
            .unwrap();
        store
            .apply_relation(RelationCommand::SetDerivedFrom {
                derived: child,
                origin: member,
            })
            .unwrap();

        store
    }

    #[test]
    fn round_trip_preserves_hash_for_mixed_world() {
        let store = mixed_world();
        let snapshot = store.snapshot();
        let text = encode_snapshot(&snapshot);
        let decoded = decode_snapshot(&text).expect("decode");
        let restored = EntityStore::from_snapshot(decoded);
        assert_eq!(
            store.hash(),
            restored.hash(),
            "save→reload must reproduce the exact entity-store fingerprint"
        );
    }

    #[test]
    fn encode_is_a_fixed_point() {
        let store = mixed_world();
        let text = encode_snapshot(&store.snapshot());
        let decoded = decode_snapshot(&text).unwrap();
        let restored = EntityStore::from_snapshot(decoded);
        let reencoded = encode_snapshot(&restored.snapshot());
        assert_eq!(text, reencoded, "encode∘decode is a fixed point");
    }

    #[test]
    fn empty_store_round_trips() {
        let store = EntityStore::new();
        let text = encode_snapshot(&store.snapshot());
        let decoded = decode_snapshot(&text).unwrap();
        assert_eq!(EntityStore::from_snapshot(decoded).hash(), store.hash());
    }

    #[test]
    fn newer_schema_fails_closed() {
        let text = encode_snapshot(&EntityStore::new().snapshot())
            .replace("\"schemaVersion\": 1", "\"schemaVersion\": 2");
        assert!(matches!(
            decode_snapshot(&text),
            Err(SnapshotDecodeError::UnsupportedSchema {
                found: 2,
                supported: 1
            })
        ));
    }

    #[test]
    fn unknown_source_kind_fails_closed() {
        let store = mixed_world();
        let text = encode_snapshot(&store.snapshot()).replace("sceneBootstrap", "mysteryKind");
        assert!(matches!(
            decode_snapshot(&text),
            Err(SnapshotDecodeError::UnknownVariant(_))
        ));
    }

    #[test]
    fn malformed_json_fails_closed() {
        assert!(matches!(
            decode_snapshot("{ not json"),
            Err(SnapshotDecodeError::Json(_))
        ));
    }

    #[test]
    fn tombstoned_entity_survives_round_trip() {
        let mut store = EntityStore::new();
        let id = EntityId::new(1);
        store
            .apply(EntityLifecycleCommand::Create {
                id,
                source: EntitySource::RuntimeCreated { by: None },
                labels: vec![],
            })
            .unwrap();
        store.apply(EntityLifecycleCommand::Destroy { id }).unwrap();
        let decoded = decode_snapshot(&encode_snapshot(&store.snapshot())).unwrap();
        let restored = EntityStore::from_snapshot(decoded);
        assert_eq!(restored.hash(), store.hash());
        assert_eq!(restored.lifecycle(id), Some(EntityLifecycle::Tombstoned));
    }
}
