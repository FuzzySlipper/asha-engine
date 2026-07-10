//! A tiny TypeScript schema IR and its deterministic emitter.
//!
//! The IR is intentionally small: it covers exactly the shapes the ASHA
//! protocol border uses (branded IDs, interfaces, discriminated unions, string
//! enums, maps, and simple constants) and nothing more. Codegen derives this IR
//! from Rust declarations through [`crate::source`] and then emits it here.
//!
//! Determinism is the whole point: emission depends only on the IR, uses fixed
//! two-space indentation and LF newlines, preserves declaration order, and ends
//! every file with a single trailing newline. The same IR always produces
//! byte-identical output.

/// A TypeScript primitive that maps onto a Rust scalar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TsPrim {
    Number,
    String,
    Boolean,
}

impl TsPrim {
    fn render(&self) -> &'static str {
        match self {
            TsPrim::Number => "number",
            TsPrim::String => "string",
            TsPrim::Boolean => "boolean",
        }
    }
}

/// A TypeScript type expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TsType {
    Prim(TsPrim),
    /// A reference to a named type, e.g. `EntityId` or `Command`.
    Ref(String),
    /// `readonly T[]`.
    Array(Box<TsType>),
    /// `readonly [A, B, C]`.
    Tuple(Vec<TsType>),
    /// `T | null` — the border form of a Rust `Option<T>`.
    Nullable(Box<TsType>),
    /// `Readonly<Record<K, V>>` — the border form of a Rust map.
    Map(Box<TsType>, Box<TsType>),
    /// A union of string literals, e.g. `'input' | 'policy' | 'system'`.
    StringEnum(Vec<String>),
}

impl TsType {
    pub fn reference(name: impl Into<String>) -> TsType {
        TsType::Ref(name.into())
    }

    pub fn array(inner: TsType) -> TsType {
        TsType::Array(Box::new(inner))
    }

    pub fn nullable(inner: TsType) -> TsType {
        TsType::Nullable(Box::new(inner))
    }

    fn render(&self) -> String {
        match self {
            TsType::Prim(p) => p.render().to_string(),
            TsType::Ref(name) => name.clone(),
            TsType::Array(inner) => match inner.as_ref() {
                TsType::Tuple(_) | TsType::StringEnum(_) => {
                    format!("readonly ({})[]", inner.render())
                }
                _ => format!("readonly {}[]", inner.render()),
            },
            TsType::Tuple(items) => {
                let inner = items
                    .iter()
                    .map(TsType::render)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("readonly [{inner}]")
            }
            TsType::Nullable(inner) => format!("{} | null", inner.render()),
            TsType::Map(key, value) => {
                format!("Readonly<Record<{}, {}>>", key.render(), value.render())
            }
            TsType::StringEnum(values) => values
                .iter()
                .map(|v| format!("'{v}'"))
                .collect::<Vec<_>>()
                .join(" | "),
        }
    }
}

/// One field of an interface or union variant. Always emitted `readonly`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: TsType,
}

impl Field {
    pub fn new(name: impl Into<String>, ty: TsType) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }

    fn render(&self) -> String {
        format!("readonly {}: {}", self.name, self.ty.render())
    }
}

/// One member of a discriminated union: a discriminant literal plus fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// The literal value of the union's discriminant field for this member.
    pub tag: String,
    pub fields: Vec<Field>,
}

impl Variant {
    pub fn new(tag: impl Into<String>, fields: Vec<Field>) -> Self {
        Self {
            tag: tag.into(),
            fields,
        }
    }
}

/// An import of type names from another generated module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub names: Vec<String>,
    /// Module specifier including the `.js` extension, e.g. `./ids.js`.
    pub from: String,
}

/// A top-level declaration in a generated module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// `export type Name = number & { readonly __brand: 'Name' };` plus a
    /// lowercase smart constructor `export const name = (raw): Name => ...`.
    BrandedId { doc: String, name: String },
    /// `export type Name = <ty>;`
    Alias {
        doc: String,
        name: String,
        ty: TsType,
    },
    /// `export interface Name { ... }`
    Interface {
        doc: String,
        name: String,
        fields: Vec<Field>,
    },
    /// `export type Name = | { kind: 'a'; ... } | ...;` keyed on `discriminant`.
    Union {
        doc: String,
        name: String,
        discriminant: String,
        variants: Vec<Variant>,
    },
    /// `export const NAME = <value>;`
    Const {
        doc: String,
        name: String,
        value: String,
    },
    /// `export * from '<from>';`
    ReExport { from: String },
}

/// A generated module: a single `.ts` file's worth of imports and items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// File stem (without extension), e.g. `ids` → `ids.ts`.
    pub name: &'static str,
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
}

/// The do-not-edit banner prepended to every generated file.
pub const BANNER: &str = "\
// @generated by protocol-codegen — DO NOT EDIT.
//
// Source of truth: engine-rs/crates/protocol/*. Regenerate with:
//   cargo run -p protocol-codegen
//
// Manual edits will be overwritten and are rejected by CI
// (harness/ci/check-contracts.sh).";

/// Render a whole module to a deterministic TypeScript source string.
pub fn render_module(module: &Module) -> String {
    let mut out = String::new();
    out.push_str(BANNER);
    out.push_str("\n\n");

    if !module.imports.is_empty() {
        for import in &module.imports {
            out.push_str(&format!(
                "import type {{ {} }} from '{}';\n",
                import.names.join(", "),
                import.from
            ));
        }
        out.push('\n');
    }

    let rendered: Vec<String> = module.items.iter().map(render_item).collect();
    out.push_str(&rendered.join("\n\n"));
    out.push('\n');
    out
}

fn render_item(item: &Item) -> String {
    match item {
        Item::BrandedId { doc, name } => {
            let ctor = lower_first(name);
            format!(
                "{}export type {name} = number & {{ readonly __brand: '{name}' }};\n\
                 export const {ctor} = (raw: number): {name} => raw as {name};",
                doc_prefix(doc)
            )
        }
        Item::Alias { doc, name, ty } => {
            format!("{}export type {name} = {};", doc_prefix(doc), ty.render())
        }
        Item::Interface { doc, name, fields } => {
            let body: String = fields
                .iter()
                .map(|f| format!("  {};\n", f.render()))
                .collect();
            format!("{}export interface {name} {{\n{body}}}", doc_prefix(doc))
        }
        Item::Union {
            doc,
            name,
            discriminant,
            variants,
        } => {
            let arms: Vec<String> = variants
                .iter()
                .map(|v| render_variant(discriminant, v))
                .collect();
            format!(
                "{}export type {name} =\n{};",
                doc_prefix(doc),
                arms.join("\n")
            )
        }
        Item::Const { doc, name, value } => {
            format!("{}export const {name} = {value};", doc_prefix(doc))
        }
        Item::ReExport { from } => format!("export * from '{from}';"),
    }
}

fn doc_prefix(doc: &str) -> String {
    if doc.is_empty() {
        String::new()
    } else {
        format!("// {doc}\n")
    }
}

fn render_variant(discriminant: &str, variant: &Variant) -> String {
    let mut parts = vec![format!("readonly {}: '{}'", discriminant, variant.tag)];
    parts.extend(variant.fields.iter().map(Field::render));
    format!("  | {{ {} }}", parts.join("; "))
}

fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_first_handles_brand_names() {
        assert_eq!(lower_first("EntityId"), "entityId");
        assert_eq!(lower_first("RenderHandle"), "renderHandle");
        assert_eq!(lower_first(""), "");
    }

    #[test]
    fn nullable_and_array_render() {
        assert_eq!(
            TsType::nullable(TsType::reference("ModeId")).render(),
            "ModeId | null"
        );
        assert_eq!(
            TsType::array(TsType::reference("TagId")).render(),
            "readonly TagId[]"
        );
        assert_eq!(
            TsType::Tuple(vec![TsType::Prim(TsPrim::Number); 3]).render(),
            "readonly [number, number, number]"
        );
    }

    #[test]
    fn render_module_starts_with_banner_and_ends_with_single_newline() {
        let module = Module {
            name: "demo",
            imports: vec![],
            items: vec![Item::BrandedId {
                doc: "An id.".to_string(),
                name: "DemoId".to_string(),
            }],
        };
        let out = render_module(&module);
        assert!(out.starts_with("// @generated by protocol-codegen"));
        assert!(out.ends_with("\n"));
        assert!(!out.ends_with("\n\n"));
        assert!(out.contains("export type DemoId = number & { readonly __brand: 'DemoId' };"));
        assert!(out.contains("export const demoId = (raw: number): DemoId => raw as DemoId;"));
    }
}
