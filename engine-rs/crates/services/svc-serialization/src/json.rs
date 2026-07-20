//! Std-only canonical JSON encode/decode for the project-bundle manifest.
//!
//! The workspace has zero external dependencies, so — like `core-scene`'s
//! `json` module — this hand-writes the exact manifest shape. [`encode`] emits a
//! deterministic, canonicalized manifest (artifact table sorted by path, fixed
//! field order); [`decode`] parses it back. Encode∘decode is a fixed point on a
//! canonicalized manifest, pinned by the golden-fixture test.

use core_ids::{ProjectId, SceneId};

use crate::artifact::{ArtifactClass, ArtifactEntry, ArtifactRole};
use crate::hash::BundleHash;
use crate::manifest::{
    AssetLockSection, GeneratorMetadata, ProjectBundleManifest, ProjectSection, SceneSection,
    BUNDLE_SCHEMA_VERSION,
};

// ── Encode ──────────────────────────────────────────────────────────────────

/// Encode a manifest as canonical JSON (LF newlines, trailing newline). The input
/// is canonicalized first, so equivalent manifests encode byte-identically.
pub fn encode(manifest: &ProjectBundleManifest) -> String {
    let m = manifest.canonical();
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"bundleSchemaVersion\": {},\n",
        m.bundle_schema_version
    ));
    out.push_str(&format!("  \"protocolVersion\": {},\n", m.protocol_version));

    out.push_str("  \"project\": { \"id\": ");
    out.push_str(&m.project.id.raw().to_string());
    out.push_str(", \"name\": ");
    encode_opt_str(&mut out, m.project.name.as_deref());
    out.push_str(" },\n");

    out.push_str(&format!("  \"entryScene\": {},\n", m.entry_scene.raw()));
    out.push_str("  \"scenes\": [");
    if m.scenes.is_empty() {
        out.push(']');
    } else {
        out.push('\n');
        for (index, scene) in m.scenes.iter().enumerate() {
            out.push_str(&format!(
                "    {{ \"id\": {}, \"schemaVersion\": {}, \"artifact\": ",
                scene.id.raw(),
                scene.schema_version
            ));
            encode_str(&mut out, &scene.artifact);
            out.push_str(" }");
            if index + 1 < m.scenes.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]");
    }
    out.push_str(",\n");

    out.push_str("  \"assetLock\": { \"artifact\": ");
    encode_str(&mut out, &m.asset_lock.artifact);
    out.push_str(&format!(
        ", \"assetCount\": {} }},\n",
        m.asset_lock.asset_count
    ));

    out.push_str("  \"generationProvenance\": ");
    match &m.generation_provenance {
        Some(provenance) => {
            out.push_str("{ \"provider\": ");
            encode_str(&mut out, &provenance.provider);
            out.push_str(&format!(
                ", \"seed\": {}, \"version\": {}, \"params\": ",
                provenance.seed, provenance.version
            ));
            encode_str(&mut out, &provenance.params);
            out.push_str(" },\n");
        }
        None => out.push_str("null,\n"),
    }

    out.push_str("  \"artifacts\": [");
    if m.artifacts.is_empty() {
        out.push(']');
    } else {
        out.push('\n');
        for (i, a) in m.artifacts.iter().enumerate() {
            out.push_str("    ");
            encode_artifact(&mut out, a);
            if i + 1 < m.artifacts.len() {
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

fn encode_artifact(out: &mut String, a: &ArtifactEntry) {
    out.push_str("{ \"path\": ");
    encode_str(out, &a.path);
    out.push_str(&format!(", \"class\": \"{}\"", a.class.tag()));
    out.push_str(", \"role\": ");
    encode_str(out, a.role.tag());
    out.push_str(", \"contentHash\": ");
    match a.content_hash {
        Some(h) => encode_str(out, &h.to_hex()),
        None => out.push_str("null"),
    }
    out.push_str(" }");
}

fn encode_str(out: &mut String, s: &str) {
    out.push('"');
    out.push_str(&escape(s));
    out.push('"');
}

fn encode_opt_str(out: &mut String, s: Option<&str>) {
    match s {
        Some(s) => encode_str(out, s),
        None => out.push_str("null"),
    }
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

// ── Decode ──────────────────────────────────────────────────────────────────

/// Why decoding a manifest failed structurally (before [`ProjectBundleManifest::validate`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestDecodeError {
    /// The bytes were not valid JSON.
    Json(String),
    /// A required field was missing or had the wrong type.
    Field(String),
    /// An artifact `class` discriminant was not recognized.
    UnknownClass(String),
    /// A content-hash string was not 16-digit lowercase hex.
    BadHash(String),
    /// A non-negative integer could not be represented by the manifest field's
    /// declared wire type.
    IntegerOutOfRange {
        field: String,
        found: u64,
        maximum: u32,
    },
    /// The JSON names a manifest schema for which this strict codec has no
    /// understood closed field set.
    UnsupportedSchema { found: u32, supported: u32 },
}

impl core::fmt::Display for ManifestDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ManifestDecodeError::Json(s) => write!(f, "invalid JSON: {s}"),
            ManifestDecodeError::Field(s) => write!(f, "bad field: {s}"),
            ManifestDecodeError::UnknownClass(s) => write!(f, "unknown artifact class `{s}`"),
            ManifestDecodeError::BadHash(s) => write!(f, "bad content hash `{s}`"),
            ManifestDecodeError::IntegerOutOfRange {
                field,
                found,
                maximum,
            } => write!(f, "field `{field}` value {found} exceeds maximum {maximum}"),
            ManifestDecodeError::UnsupportedSchema { found, supported } => write!(
                f,
                "unsupported bundle schema version {found}; supported version is {supported}"
            ),
        }
    }
}

impl std::error::Error for ManifestDecodeError {}

/// Decode canonical/authored manifest JSON. The result is **not** validated; call
/// [`ProjectBundleManifest::validate`].
pub fn decode(input: &str) -> Result<ProjectBundleManifest, ManifestDecodeError> {
    let json = Json::parse(input).map_err(ManifestDecodeError::Json)?;
    let bundle_schema_version = field_u32(&json, "bundleSchemaVersion")?;
    if bundle_schema_version != BUNDLE_SCHEMA_VERSION {
        return Err(ManifestDecodeError::UnsupportedSchema {
            found: bundle_schema_version,
            supported: BUNDLE_SCHEMA_VERSION,
        });
    }
    require_object_fields(
        &json,
        &[
            "bundleSchemaVersion",
            "protocolVersion",
            "project",
            "entryScene",
            "scenes",
            "assetLock",
            "generationProvenance",
            "artifacts",
        ],
        "manifest",
    )?;
    let protocol_version = field_u32(&json, "protocolVersion")?;

    let project_j = field(&json, "project")?;
    require_object_fields(project_j, &["id", "name"], "project")?;
    let project = ProjectSection {
        id: ProjectId::new(field_u64(project_j, "id")?),
        name: opt_str(project_j, "name")?,
    };

    let entry_scene = SceneId::new(field_u64(&json, "entryScene")?);
    let scene_values = field(&json, "scenes")?
        .as_array()
        .ok_or_else(|| ManifestDecodeError::Field("scenes must be an array".into()))?;
    let scenes = scene_values
        .iter()
        .map(decode_scene)
        .collect::<Result<Vec<_>, _>>()?;
    let generation_provenance = match field(&json, "generationProvenance")? {
        Json::Null => None,
        value => Some(decode_generation_provenance(value)?),
    };

    let lock_j = field(&json, "assetLock")?;
    require_object_fields(lock_j, &["artifact", "assetCount"], "assetLock")?;
    let asset_lock = AssetLockSection {
        artifact: req_str(lock_j, "artifact")?,
        asset_count: field_u32(lock_j, "assetCount")?,
    };

    let arr = field(&json, "artifacts")?
        .as_array()
        .ok_or_else(|| ManifestDecodeError::Field("artifacts must be an array".into()))?;
    let mut artifacts = Vec::with_capacity(arr.len());
    for a in arr {
        artifacts.push(decode_artifact(a)?);
    }

    Ok(ProjectBundleManifest {
        bundle_schema_version,
        protocol_version,
        project,
        entry_scene,
        scenes,
        asset_lock,
        generation_provenance,
        artifacts,
    })
}

fn decode_scene(j: &Json) -> Result<SceneSection, ManifestDecodeError> {
    require_object_fields(j, &["id", "schemaVersion", "artifact"], "scene")?;
    Ok(SceneSection {
        id: SceneId::new(field_u64(j, "id")?),
        schema_version: field_u32(j, "schemaVersion")?,
        artifact: req_str(j, "artifact")?,
    })
}

fn decode_generation_provenance(j: &Json) -> Result<GeneratorMetadata, ManifestDecodeError> {
    require_object_fields(
        j,
        &["provider", "seed", "version", "params"],
        "generationProvenance",
    )?;
    Ok(GeneratorMetadata {
        provider: req_str(j, "provider")?,
        seed: field_u64(j, "seed")?,
        version: field_u32(j, "version")?,
        params: req_str(j, "params")?,
    })
}

fn decode_artifact(j: &Json) -> Result<ArtifactEntry, ManifestDecodeError> {
    require_object_fields(j, &["path", "class", "role", "contentHash"], "artifact")?;
    let path = req_str(j, "path")?;
    let class_tag = req_str(j, "class")?;
    let class =
        ArtifactClass::from_tag(&class_tag).ok_or(ManifestDecodeError::UnknownClass(class_tag))?;
    let role = ArtifactRole::from_tag(&req_str(j, "role")?);
    let content_hash = match j.get("contentHash") {
        None | Some(Json::Null) => None,
        Some(Json::Str(s)) => {
            Some(BundleHash::parse_hex(s).ok_or_else(|| ManifestDecodeError::BadHash(s.clone()))?)
        }
        Some(_) => {
            return Err(ManifestDecodeError::Field(
                "contentHash must be a string or null".into(),
            ))
        }
    };
    Ok(ArtifactEntry {
        path,
        class,
        role,
        content_hash,
    })
}

fn require_object_fields(
    j: &Json,
    allowed: &[&str],
    context: &str,
) -> Result<(), ManifestDecodeError> {
    let Json::Obj(fields) = j else {
        return Err(ManifestDecodeError::Field(format!(
            "{context} must be an object"
        )));
    };
    let mut seen: Vec<&str> = Vec::with_capacity(fields.len());
    for (key, _) in fields {
        if seen.contains(&key.as_str()) {
            return Err(ManifestDecodeError::Field(format!(
                "duplicate field `{key}` in {context}"
            )));
        }
        seen.push(key);
        if !allowed.contains(&key.as_str()) {
            return Err(ManifestDecodeError::Field(format!(
                "unknown field `{key}` in {context}"
            )));
        }
    }
    Ok(())
}

// ── typed-field helpers ───────────────────────────────────────────────────────

fn field<'a>(j: &'a Json, key: &str) -> Result<&'a Json, ManifestDecodeError> {
    j.get(key)
        .ok_or_else(|| ManifestDecodeError::Field(format!("missing field `{key}`")))
}

fn field_u64(j: &Json, key: &str) -> Result<u64, ManifestDecodeError> {
    field(j, key)?.as_u64().ok_or_else(|| {
        ManifestDecodeError::Field(format!("field `{key}` must be a non-negative integer"))
    })
}

fn field_u32(j: &Json, key: &str) -> Result<u32, ManifestDecodeError> {
    let found = field_u64(j, key)?;
    u32::try_from(found).map_err(|_| ManifestDecodeError::IntegerOutOfRange {
        field: key.to_string(),
        found,
        maximum: u32::MAX,
    })
}

fn req_str(j: &Json, key: &str) -> Result<String, ManifestDecodeError> {
    field(j, key)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| ManifestDecodeError::Field(format!("field `{key}` must be a string")))
}

fn opt_str(j: &Json, key: &str) -> Result<Option<String>, ManifestDecodeError> {
    match j.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Str(s)) => Ok(Some(s.clone())),
        Some(_) => Err(ManifestDecodeError::Field(format!(
            "field `{key}` must be a string or null"
        ))),
    }
}

// ── Minimal JSON value + parser (std-only) ────────────────────────────────────

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
