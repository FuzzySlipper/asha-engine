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

/// Render runtime wire validators from the same IR that emits TypeScript DTOs.
///
/// The output is intentionally a small data-driven structural validator. It
/// checks the serializable contract shape only; semantic validation remains in
/// the owning Rust lanes.
pub fn render_wire_module(modules: &[Module]) -> Result<String, String> {
    use std::collections::{BTreeMap, BTreeSet};

    let contract_modules = modules
        .iter()
        .filter(|module| module.name != "index")
        .collect::<Vec<_>>();
    let mut module_names = BTreeSet::new();
    for module in &contract_modules {
        module_names.insert(module.name);
    }

    let mut schemas = Vec::new();
    for module in contract_modules {
        let local_names = module
            .items
            .iter()
            .filter_map(item_name)
            .collect::<BTreeSet<_>>();
        let mut imported_names = BTreeMap::new();
        for import in &module.imports {
            let imported_module = import
                .from
                .strip_prefix("./")
                .and_then(|value| value.strip_suffix(".js"))
                .ok_or_else(|| format!("unsupported generated import {}", import.from))?;
            if !module_names.contains(imported_module) {
                return Err(format!(
                    "generated import {} points at unknown module {imported_module}",
                    import.from
                ));
            }
            for name in &import.names {
                imported_names.insert(name.as_str(), imported_module);
            }
        }
        for item in &module.items {
            let Some(name) = item_name(item) else {
                continue;
            };
            let schema = render_item_schema(item, module.name, &local_names, &imported_names)?;
            schemas.push((format!("{}.{}", module.name, name), schema));
        }
    }

    let mut out = String::from(BANNER);
    out.push_str(
        "\n\nexport type GeneratedWireValue =\n\
         \x20 | null\n\
         \x20 | boolean\n\
         \x20 | number\n\
         \x20 | string\n\
         \x20 | readonly GeneratedWireValue[]\n\
         \x20 | { readonly [key: string]: GeneratedWireValue };\n\n\
         export type GeneratedWireIssueCode =\n\
         \x20 | 'missing_field'\n\
         \x20 | 'noncanonical_number'\n\
         \x20 | 'unknown_field'\n\
         \x20 | 'unknown_type'\n\
         \x20 | 'unknown_variant'\n\
         \x20 | 'wrong_type';\n\n\
         export interface GeneratedWireValidationIssue {\n\
         \x20 readonly code: GeneratedWireIssueCode;\n\
         \x20 readonly path: string;\n\
         \x20 readonly message: string;\n\
         }\n\n\
         export type GeneratedWireValidationResult =\n\
         \x20 | { readonly valid: true }\n\
         \x20 | { readonly valid: false; readonly issue: GeneratedWireValidationIssue };\n\n\
         type WireSchema =\n\
         \x20 | { readonly kind: 'array'; readonly item: WireSchema }\n\
         \x20 | { readonly kind: 'boolean' }\n\
         \x20 | { readonly kind: 'enum'; readonly values: readonly string[] }\n\
         \x20 | { readonly kind: 'map'; readonly key: WireSchema; readonly value: WireSchema }\n\
         \x20 | { readonly kind: 'nullable'; readonly value: WireSchema }\n\
         \x20 | { readonly kind: 'number'; readonly integer: boolean }\n\
         \x20 | { readonly kind: 'object'; readonly fields: Readonly<Record<string, WireSchema>> }\n\
         \x20 | { readonly kind: 'ref'; readonly name: string }\n\
         \x20 | { readonly kind: 'string' }\n\
         \x20 | { readonly kind: 'tuple'; readonly items: readonly WireSchema[] }\n\
         \x20 | {\n\
         \x20     readonly kind: 'union';\n\
         \x20     readonly discriminant: string;\n\
         \x20     readonly variants: Readonly<Record<string, Readonly<Record<string, WireSchema>>>>;\n\
         \x20   };\n\n\
         const GENERATED_WIRE_SCHEMAS: Readonly<Record<string, WireSchema>> = {\n",
    );
    for (name, schema) in &schemas {
        out.push_str(&format!("  '{}': {},\n", escape_ts(name), schema));
    }
    out.push_str("};\n\nconst GENERATED_WIRE_TYPE_NAME_VALUES = [\n");
    for (name, _) in &schemas {
        out.push_str(&format!("  '{}',\n", escape_ts(name)));
    }
    out.push_str(
        "] as const;\n\n\
         export type GeneratedWireTypeName = (typeof GENERATED_WIRE_TYPE_NAME_VALUES)[number];\n\
         export const GENERATED_WIRE_TYPE_NAMES: readonly string[] = GENERATED_WIRE_TYPE_NAME_VALUES;\n\n\
         function issue(\n\
         \x20 code: GeneratedWireIssueCode,\n\
         \x20 path: string,\n\
         \x20 message: string,\n\
         ): GeneratedWireValidationResult {\n\
         \x20 return { valid: false, issue: { code, path, message } };\n\
         }\n\n\
         function isObject(value: GeneratedWireValue): value is { readonly [key: string]: GeneratedWireValue } {\n\
         \x20 return typeof value === 'object' && value !== null && !Array.isArray(value);\n\
         }\n\n\
         function childPath(path: string, field: string): string {\n\
         \x20 return /^[A-Za-z_$][A-Za-z0-9_$]*$/u.test(field)\n\
         \x20   ? `${path}.${field}`\n\
         \x20   : `${path}[${JSON.stringify(field)}]`;\n\
         }\n\n\
         function validateSchema(\n\
         \x20 schema: WireSchema,\n\
         \x20 value: GeneratedWireValue,\n\
         \x20 path: string,\n\
         ): GeneratedWireValidationResult {\n\
         \x20 switch (schema.kind) {\n\
         \x20   case 'boolean':\n\
         \x20     return typeof value === 'boolean' ? { valid: true } : issue('wrong_type', path, 'expected boolean');\n\
         \x20   case 'string':\n\
         \x20     return typeof value === 'string' ? { valid: true } : issue('wrong_type', path, 'expected string');\n\
         \x20   case 'number':\n\
         \x20     if (typeof value !== 'number' || !Number.isFinite(value)) {\n\
         \x20       return issue('wrong_type', path, 'expected finite number');\n\
         \x20     }\n\
         \x20     if (schema.integer && (!Number.isSafeInteger(value) || value < 0)) {\n\
         \x20       return issue('noncanonical_number', path, 'expected non-negative safe integer');\n\
         \x20     }\n\
         \x20     return { valid: true };\n\
         \x20   case 'enum':\n\
         \x20     return typeof value === 'string' && schema.values.includes(value)\n\
         \x20       ? { valid: true }\n\
         \x20       : issue('unknown_variant', path, `expected one of ${schema.values.join(', ')}`);\n\
         \x20   case 'nullable':\n\
         \x20     return value === null ? { valid: true } : validateSchema(schema.value, value, path);\n\
         \x20   case 'ref': {\n\
         \x20     const target = GENERATED_WIRE_SCHEMAS[schema.name];\n\
         \x20     return target === undefined\n\
         \x20       ? issue('unknown_type', path, `unknown generated wire type ${schema.name}`)\n\
         \x20       : validateSchema(target, value, path);\n\
         \x20   }\n\
         \x20   case 'array':\n\
         \x20     if (!Array.isArray(value)) return issue('wrong_type', path, 'expected array');\n\
         \x20     const arrayValue = value as readonly GeneratedWireValue[];\n\
         \x20     for (let index = 0; index < arrayValue.length; index += 1) {\n\
         \x20       const result = validateSchema(schema.item, arrayValue[index] ?? null, `${path}[${index}]`);\n\
         \x20       if (!result.valid) return result;\n\
         \x20     }\n\
         \x20     return { valid: true };\n\
         \x20   case 'tuple':\n\
         \x20     if (!Array.isArray(value) || value.length !== schema.items.length) {\n\
         \x20       return issue('wrong_type', path, `expected tuple of length ${schema.items.length}`);\n\
         \x20     }\n\
         \x20     const tupleValue = value as readonly GeneratedWireValue[];\n\
         \x20     for (let index = 0; index < schema.items.length; index += 1) {\n\
         \x20       const itemSchema = schema.items[index];\n\
         \x20       if (itemSchema === undefined) return issue('unknown_type', path, 'missing tuple schema');\n\
         \x20       const result = validateSchema(itemSchema, tupleValue[index] ?? null, `${path}[${index}]`);\n\
         \x20       if (!result.valid) return result;\n\
         \x20     }\n\
         \x20     return { valid: true };\n\
         \x20   case 'map':\n\
         \x20     if (!isObject(value)) return issue('wrong_type', path, 'expected object map');\n\
         \x20     for (const [key, entry] of Object.entries(value)) {\n\
         \x20       const keyResult = validateSchema(schema.key, key, `${path}{key}`);\n\
         \x20       if (!keyResult.valid) return keyResult;\n\
         \x20       const valueResult = validateSchema(schema.value, entry, childPath(path, key));\n\
         \x20       if (!valueResult.valid) return valueResult;\n\
         \x20     }\n\
         \x20     return { valid: true };\n\
         \x20   case 'object':\n\
         \x20     return validateObject(schema.fields, value, path);\n\
         \x20   case 'union': {\n\
         \x20     if (!isObject(value)) return issue('wrong_type', path, 'expected tagged object');\n\
         \x20     const tag = value[schema.discriminant];\n\
         \x20     if (typeof tag !== 'string' || schema.variants[tag] === undefined) {\n\
         \x20       return issue('unknown_variant', childPath(path, schema.discriminant), 'unknown tagged-union variant');\n\
         \x20     }\n\
         \x20     return validateObject(\n\
         \x20       { [schema.discriminant]: { kind: 'enum', values: [tag] }, ...schema.variants[tag] },\n\
         \x20       value,\n\
         \x20       path,\n\
         \x20     );\n\
         \x20   }\n\
         \x20 }\n\
         }\n\n\
         function validateObject(\n\
         \x20 fields: Readonly<Record<string, WireSchema>>,\n\
         \x20 value: GeneratedWireValue,\n\
         \x20 path: string,\n\
         ): GeneratedWireValidationResult {\n\
         \x20 if (!isObject(value)) return issue('wrong_type', path, 'expected object');\n\
         \x20 for (const key of Object.keys(value)) {\n\
         \x20   if (fields[key] === undefined) return issue('unknown_field', childPath(path, key), 'unknown field');\n\
         \x20 }\n\
         \x20 for (const [field, fieldSchema] of Object.entries(fields)) {\n\
         \x20   if (!(field in value)) return issue('missing_field', childPath(path, field), 'missing required field');\n\
         \x20   const result = validateSchema(fieldSchema, value[field] ?? null, childPath(path, field));\n\
         \x20   if (!result.valid) return result;\n\
         \x20 }\n\
         \x20 return { valid: true };\n\
         }\n\n\
         export function validateGeneratedWireValue(\n\
         \x20 typeName: string,\n\
         \x20 value: GeneratedWireValue,\n\
         \x20 path = '$',\n\
         ): GeneratedWireValidationResult {\n\
         \x20 const schema = GENERATED_WIRE_SCHEMAS[typeName];\n\
         \x20 return schema === undefined\n\
         \x20   ? issue('unknown_type', path, `unknown generated wire type ${typeName}`)\n\
         \x20   : validateSchema(schema, value, path);\n\
         }\n",
    );
    Ok(out)
}

fn item_name(item: &Item) -> Option<&str> {
    match item {
        Item::BrandedId { name, .. }
        | Item::Alias { name, .. }
        | Item::Interface { name, .. }
        | Item::Union { name, .. } => Some(name),
        Item::Const { .. } | Item::ReExport { .. } => None,
    }
}

fn render_item_schema(
    item: &Item,
    module_name: &str,
    local_names: &std::collections::BTreeSet<&str>,
    imported_names: &std::collections::BTreeMap<&str, &str>,
) -> Result<String, String> {
    match item {
        Item::BrandedId { .. } => Ok("{ kind: 'number', integer: true }".to_string()),
        Item::Alias { ty, .. } => render_type_schema(ty, module_name, local_names, imported_names),
        Item::Interface { fields, .. } => Ok(format!(
            "{{ kind: 'object', fields: {} }}",
            render_fields_schema(fields, module_name, local_names, imported_names)?
        )),
        Item::Union {
            discriminant,
            variants,
            ..
        } => {
            let mut rendered = Vec::new();
            for variant in variants {
                rendered.push(format!(
                    "'{}': {}",
                    escape_ts(&variant.tag),
                    render_fields_schema(
                        &variant.fields,
                        module_name,
                        local_names,
                        imported_names,
                    )?
                ));
            }
            Ok(format!(
                "{{ kind: 'union', discriminant: '{}', variants: {{ {} }} }}",
                escape_ts(discriminant),
                rendered.join(", ")
            ))
        }
        Item::Const { .. } | Item::ReExport { .. } => {
            Err("constants and re-exports have no wire schema".to_string())
        }
    }
}

fn render_fields_schema(
    fields: &[Field],
    module_name: &str,
    local_names: &std::collections::BTreeSet<&str>,
    imported_names: &std::collections::BTreeMap<&str, &str>,
) -> Result<String, String> {
    let rendered = fields
        .iter()
        .map(|field| {
            Ok(format!(
                "'{}': {}",
                escape_ts(&field.name),
                render_type_schema(&field.ty, module_name, local_names, imported_names)?
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(format!("{{ {} }}", rendered.join(", ")))
}

fn render_type_schema(
    ty: &TsType,
    module_name: &str,
    local_names: &std::collections::BTreeSet<&str>,
    imported_names: &std::collections::BTreeMap<&str, &str>,
) -> Result<String, String> {
    match ty {
        TsType::Prim(TsPrim::Number) => Ok("{ kind: 'number', integer: false }".to_string()),
        TsType::Prim(TsPrim::String) => Ok("{ kind: 'string' }".to_string()),
        TsType::Prim(TsPrim::Boolean) => Ok("{ kind: 'boolean' }".to_string()),
        TsType::Ref(name) => {
            let owner = if local_names.contains(name.as_str()) {
                module_name
            } else {
                imported_names.get(name.as_str()).copied().ok_or_else(|| {
                    format!("unresolved generated wire reference {module_name}.{name}")
                })?
            };
            Ok(format!(
                "{{ kind: 'ref', name: '{}.{}' }}",
                escape_ts(owner),
                escape_ts(name)
            ))
        }
        TsType::Array(item) => Ok(format!(
            "{{ kind: 'array', item: {} }}",
            render_type_schema(item, module_name, local_names, imported_names)?
        )),
        TsType::Tuple(items) => Ok(format!(
            "{{ kind: 'tuple', items: [{}] }}",
            items
                .iter()
                .map(|item| render_type_schema(item, module_name, local_names, imported_names))
                .collect::<Result<Vec<_>, String>>()?
                .join(", ")
        )),
        TsType::Nullable(value) => Ok(format!(
            "{{ kind: 'nullable', value: {} }}",
            render_type_schema(value, module_name, local_names, imported_names)?
        )),
        TsType::Map(key, value) => Ok(format!(
            "{{ kind: 'map', key: {}, value: {} }}",
            render_type_schema(key, module_name, local_names, imported_names)?,
            render_type_schema(value, module_name, local_names, imported_names)?
        )),
        TsType::StringEnum(values) => Ok(format!(
            "{{ kind: 'enum', values: [{}] }}",
            values
                .iter()
                .map(|value| format!("'{}'", escape_ts(value)))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn escape_ts(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
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
