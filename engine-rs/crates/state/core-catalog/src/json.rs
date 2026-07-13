//! Std-only canonical JSON encode/decode for catalogs (subtask #2322).
//!
//! TS may author catalog data; this module is the seam where authored JSON
//! crosses into Rust for validation. [`encode`] emits a deterministic,
//! canonicalized catalog (entries sorted by id, fixed field order); [`decode`]
//! parses it back into a [`Catalog`]. The result is **not** validated — call
//! [`crate::validate`]. Mirrors `core-scene`'s hand-written JSON posture (zero
//! external dependencies).

use core_assets::{AssetHash, AssetId, AssetReference, AssetVersionReq};

use crate::entry::{Catalog, CatalogEntry};
use crate::material::{
    MaterialAuthority, MaterialDef, MaterialStyle, Rgba, StructuralClass, UvStrategy,
};

// ── Encode ──────────────────────────────────────────────────────────────────

/// Encode a catalog as canonical JSON (LF newlines, trailing newline).
pub fn encode(catalog: &Catalog) -> String {
    let c = catalog.canonical();
    let mut out = String::new();
    out.push_str("{\n  \"entries\": [");
    if c.entries.is_empty() {
        out.push_str("]\n}\n");
        return out;
    }
    out.push('\n');
    for (i, e) in c.entries.iter().enumerate() {
        encode_entry(&mut out, e);
        if i + 1 < c.entries.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

fn encode_entry(out: &mut String, e: &CatalogEntry) {
    out.push_str("    {\n");
    out.push_str(&format!("      \"id\": \"{}\",\n", e.id.as_str()));
    out.push_str(&format!("      \"version\": {},\n", e.version));
    out.push_str("      \"hash\": ");
    match &e.hash {
        Some(h) => out.push_str(&format!("\"{}\"", h.as_str())),
        None => out.push_str("null"),
    }
    out.push_str(",\n      \"sourcePath\": ");
    encode_opt_str(out, e.source_path.as_deref());
    out.push_str(",\n      \"label\": ");
    encode_opt_str(out, e.label.as_deref());
    out.push_str(",\n      \"dependencies\": ");
    encode_deps(out, &e.dependencies);
    out.push_str(",\n      \"material\": ");
    match &e.material {
        Some(m) => encode_material(out, m),
        None => out.push_str("null"),
    }
    out.push_str("\n    }");
}

fn encode_deps(out: &mut String, deps: &[AssetReference]) {
    if deps.is_empty() {
        out.push_str("[]");
        return;
    }
    // Canonical: sort dependency references by id.
    let mut sorted: Vec<&AssetReference> = deps.iter().collect();
    sorted.sort_by(|a, b| a.id().as_str().cmp(b.id().as_str()));
    out.push('[');
    for (i, d) in sorted.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        encode_ref(out, d);
    }
    out.push(']');
}

fn encode_ref(out: &mut String, r: &AssetReference) {
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

fn encode_material(out: &mut String, m: &MaterialDef) {
    out.push_str("{ \"authority\": ");
    out.push_str(&format!(
        "{{ \"solid\": {}, \"collidable\": {}, \"occludes\": {}, \"structuralClass\": \"{}\" }}",
        m.authority.solid,
        m.authority.collidable,
        m.authority.occludes,
        structural_tag(m.authority.structural_class),
    ));
    out.push_str(", \"style\": { \"color\": ");
    out.push_str(&format!(
        "[{}, {}, {}, {}]",
        fmt_f32(m.style.color.r),
        fmt_f32(m.style.color.g),
        fmt_f32(m.style.color.b),
        fmt_f32(m.style.color.a),
    ));
    out.push_str(", \"texture\": ");
    match &m.style.texture {
        Some(t) => encode_ref(out, t),
        None => out.push_str("null"),
    }
    out.push_str(", \"textureTint\": ");
    encode_rgba(out, m.style.texture_tint);
    out.push_str(", \"emissionColor\": ");
    encode_rgba(out, m.style.emission_color);
    out.push_str(&format!(
        ", \"roughness\": {}, \"emissive\": {}, \"uvStrategy\": \"{}\" }} }}",
        fmt_f32(m.style.roughness),
        fmt_f32(m.style.emissive),
        uv_tag(m.style.uv_strategy),
    ));
}

fn structural_tag(c: StructuralClass) -> &'static str {
    match c {
        StructuralClass::Decorative => "decorative",
        StructuralClass::Solid => "solid",
        StructuralClass::Structural => "structural",
    }
}

fn uv_tag(u: UvStrategy) -> &'static str {
    match u {
        UvStrategy::Flat => "flat",
        UvStrategy::Planar => "planar",
        UvStrategy::Atlas => "atlas",
    }
}

fn encode_opt_str(out: &mut String, s: Option<&str>) {
    match s {
        Some(s) => out.push_str(&format!("\"{}\"", escape(s))),
        None => out.push_str("null"),
    }
}

fn fmt_f32(v: f32) -> String {
    format!("{v}")
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

/// Why decoding catalog JSON failed structurally (before [`crate::validate`]).
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogDecodeError {
    Json(String),
    Field(String),
    Asset(String),
    UnknownEnum { field: String, value: String },
}

impl core::fmt::Display for CatalogDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CatalogDecodeError::Json(s) => write!(f, "invalid JSON: {s}"),
            CatalogDecodeError::Field(s) => write!(f, "bad field: {s}"),
            CatalogDecodeError::Asset(s) => write!(f, "bad asset reference: {s}"),
            CatalogDecodeError::UnknownEnum { field, value } => {
                write!(f, "unknown {field} value `{value}`")
            }
        }
    }
}

impl std::error::Error for CatalogDecodeError {}

/// Decode authored catalog JSON. Not validated; call [`crate::validate`].
pub fn decode(input: &str) -> Result<Catalog, CatalogDecodeError> {
    let json = Json::parse(input).map_err(CatalogDecodeError::Json)?;
    let arr = field(&json, "entries")?
        .as_array()
        .ok_or_else(|| CatalogDecodeError::Field("entries must be an array".into()))?;
    let mut entries = Vec::with_capacity(arr.len());
    for e in arr {
        entries.push(decode_entry(e)?);
    }
    Ok(Catalog { entries })
}

fn decode_entry(j: &Json) -> Result<CatalogEntry, CatalogDecodeError> {
    let id = asset_id(&req_str(j, "id")?)?;
    let version = field_u64(j, "version")? as u32;
    let hash = decode_opt_hash(j.get("hash"))?;
    let source_path = opt_str(j, "sourcePath")?;
    let label = opt_str(j, "label")?;
    let dependencies = match j.get("dependencies") {
        None | Some(Json::Null) => Vec::new(),
        Some(arr) => {
            let items = arr
                .as_array()
                .ok_or_else(|| CatalogDecodeError::Field("dependencies must be an array".into()))?;
            items.iter().map(decode_ref).collect::<Result<_, _>>()?
        }
    };
    let material = match j.get("material") {
        None | Some(Json::Null) => None,
        Some(m) => Some(decode_material(m)?),
    };
    Ok(CatalogEntry {
        id,
        version,
        hash,
        source_path,
        label,
        dependencies,
        material,
    })
}

fn decode_ref(j: &Json) -> Result<AssetReference, CatalogDecodeError> {
    let id = asset_id(
        j.get("id")
            .and_then(Json::as_str)
            .ok_or_else(|| CatalogDecodeError::Field("ref.id must be a string".into()))?,
    )?;
    let version = match j.get("version") {
        None | Some(Json::Null) => AssetVersionReq::Any,
        Some(v) => {
            let req = v
                .get("req")
                .and_then(Json::as_str)
                .ok_or_else(|| CatalogDecodeError::Field("version.req must be a string".into()))?;
            match req {
                "any" => AssetVersionReq::Any,
                "exact" => AssetVersionReq::Exact(field_u64(v, "value")? as u32),
                "atLeast" => AssetVersionReq::AtLeast(field_u64(v, "value")? as u32),
                other => {
                    return Err(CatalogDecodeError::UnknownEnum {
                        field: "version.req".into(),
                        value: other.into(),
                    })
                }
            }
        }
    };
    let hash = decode_opt_hash(j.get("hash"))?;
    Ok(AssetReference::new(id, version, hash))
}

fn decode_material(j: &Json) -> Result<MaterialDef, CatalogDecodeError> {
    let a = field(j, "authority")?;
    let authority = MaterialAuthority {
        solid: field_bool(a, "solid")?,
        collidable: field_bool(a, "collidable")?,
        occludes: field_bool(a, "occludes")?,
        structural_class: match req_str(a, "structuralClass")?.as_str() {
            "decorative" => StructuralClass::Decorative,
            "solid" => StructuralClass::Solid,
            "structural" => StructuralClass::Structural,
            other => {
                return Err(CatalogDecodeError::UnknownEnum {
                    field: "structuralClass".into(),
                    value: other.into(),
                })
            }
        },
    };
    let s = field(j, "style")?;
    let color = decode_rgba(field(s, "color")?)?;
    let texture = match s.get("texture") {
        None | Some(Json::Null) => None,
        Some(t) => Some(decode_ref(t)?),
    };
    let texture_tint = match s.get("textureTint") {
        None => Rgba::WHITE,
        Some(value) => decode_rgba(value)?,
    };
    let emission_color = match s.get("emissionColor") {
        None => color,
        Some(value) => decode_rgba(value)?,
    };
    let style = MaterialStyle {
        color,
        texture,
        roughness: field_f32(s, "roughness")?,
        texture_tint,
        emission_color,
        emissive: field_f32(s, "emissive")?,
        uv_strategy: match req_str(s, "uvStrategy")?.as_str() {
            "flat" => UvStrategy::Flat,
            "planar" => UvStrategy::Planar,
            "atlas" => UvStrategy::Atlas,
            other => {
                return Err(CatalogDecodeError::UnknownEnum {
                    field: "uvStrategy".into(),
                    value: other.into(),
                })
            }
        },
    };
    Ok(MaterialDef { authority, style })
}

fn encode_rgba(out: &mut String, color: Rgba) {
    out.push_str(&format!(
        "[{}, {}, {}, {}]",
        fmt_f32(color.r),
        fmt_f32(color.g),
        fmt_f32(color.b),
        fmt_f32(color.a),
    ));
}

fn decode_rgba(j: &Json) -> Result<Rgba, CatalogDecodeError> {
    let a = j
        .as_array()
        .filter(|a| a.len() == 4)
        .ok_or_else(|| CatalogDecodeError::Field("color must be a 4-array".into()))?;
    Ok(Rgba {
        r: num(&a[0])?,
        g: num(&a[1])?,
        b: num(&a[2])?,
        a: num(&a[3])?,
    })
}

fn asset_id(s: &str) -> Result<AssetId, CatalogDecodeError> {
    AssetId::parse(s).map_err(|e| CatalogDecodeError::Asset(e.to_string()))
}

fn decode_opt_hash(j: Option<&Json>) -> Result<Option<AssetHash>, CatalogDecodeError> {
    match j {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Str(s)) => {
            Some(AssetHash::parse(s).map_err(|e| CatalogDecodeError::Asset(e.to_string())))
                .transpose()
        }
        Some(_) => Err(CatalogDecodeError::Field(
            "hash must be a string or null".into(),
        )),
    }
}

// ── typed-field helpers ───────────────────────────────────────────────────────

fn field<'a>(j: &'a Json, key: &str) -> Result<&'a Json, CatalogDecodeError> {
    j.get(key)
        .ok_or_else(|| CatalogDecodeError::Field(format!("missing field `{key}`")))
}

fn field_u64(j: &Json, key: &str) -> Result<u64, CatalogDecodeError> {
    field(j, key)?.as_u64().ok_or_else(|| {
        CatalogDecodeError::Field(format!("field `{key}` must be a non-negative integer"))
    })
}

fn field_bool(j: &Json, key: &str) -> Result<bool, CatalogDecodeError> {
    match field(j, key)? {
        Json::Bool(b) => Ok(*b),
        _ => Err(CatalogDecodeError::Field(format!(
            "field `{key}` must be a bool"
        ))),
    }
}

fn field_f32(j: &Json, key: &str) -> Result<f32, CatalogDecodeError> {
    num(field(j, key)?)
}

fn req_str(j: &Json, key: &str) -> Result<String, CatalogDecodeError> {
    field(j, key)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| CatalogDecodeError::Field(format!("field `{key}` must be a string")))
}

fn opt_str(j: &Json, key: &str) -> Result<Option<String>, CatalogDecodeError> {
    match j.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Str(s)) => Ok(Some(s.clone())),
        Some(_) => Err(CatalogDecodeError::Field(format!(
            "field `{key}` must be a string or null"
        ))),
    }
}

fn num(j: &Json) -> Result<f32, CatalogDecodeError> {
    match j {
        Json::Num(n) => Ok(*n as f32),
        _ => Err(CatalogDecodeError::Field("expected a number".into())),
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
