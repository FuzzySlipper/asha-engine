//! Source-derived TypeScript schema metadata.
//!
//! Rust declarations are parsed from the repository at generation time. The
//! module plans below retain only border-specific ordering, imports, branded
//! constructors, and names for intentionally synthetic aliases. Field and
//! variant shapes always come from Rust.

use crate::schema::{Field, Import, Item, Module, TsPrim, TsType, Variant};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{
    parse::{Parse, ParseStream},
    Attribute, Expr, ExprLit, Fields, GenericArgument, Ident, ItemConst, ItemEnum, ItemStruct,
    ItemType, Lit, PathArguments, Type, Visibility,
};

#[derive(Debug, Clone)]
enum Declaration {
    Struct(ItemStruct),
    Enum(ItemEnum),
    Alias(ItemType),
    Const(ItemConst),
    MacroId,
}

#[derive(Debug, Clone)]
struct MacroIdDeclaration {
    ident: Ident,
}

impl Parse for MacroIdDeclaration {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let _attrs = Attribute::parse_outer(input)?;
        let ident = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens in id_type! declaration"));
        }
        Ok(Self { ident })
    }
}

#[derive(Debug, Clone)]
struct LocatedDeclaration {
    path: PathBuf,
    declaration: Declaration,
}

#[derive(Debug, Default)]
struct SourceIndex {
    declarations: BTreeMap<String, Vec<LocatedDeclaration>>,
}

impl SourceIndex {
    fn load() -> Result<Self, String> {
        let root = crate::repo_root().join("engine-rs/crates");
        let mut rust_files = Vec::new();
        collect_rust_files(&root, &mut rust_files)?;
        rust_files.sort();

        let mut index = Self::default();
        for path in rust_files {
            if path.components().any(|part| part.as_os_str() == "target")
                || path.ends_with("protocol-codegen/src/model.rs")
                || path.ends_with("protocol-codegen/src/source.rs")
            {
                continue;
            }
            let source = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            let syntax = syn::parse_file(&source)
                .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
            for item in syntax.items {
                let (name, declaration) = match item {
                    syn::Item::Struct(item) if is_public(&item.vis) => {
                        (item.ident.to_string(), Declaration::Struct(item))
                    }
                    syn::Item::Enum(item) if is_public(&item.vis) => {
                        (item.ident.to_string(), Declaration::Enum(item))
                    }
                    syn::Item::Type(item) if is_public(&item.vis) => {
                        (item.ident.to_string(), Declaration::Alias(item))
                    }
                    syn::Item::Const(item) if is_public(&item.vis) => {
                        (item.ident.to_string(), Declaration::Const(item))
                    }
                    syn::Item::Macro(item) if item.mac.path.is_ident("id_type") => {
                        let declaration = syn::parse2::<MacroIdDeclaration>(item.mac.tokens)
                            .map_err(|error| {
                                format!(
                                    "failed to parse id_type! declaration in {}: {error}",
                                    path.display()
                                )
                            })?;
                        (declaration.ident.to_string(), Declaration::MacroId)
                    }
                    _ => continue,
                };
                index
                    .declarations
                    .entry(name)
                    .or_default()
                    .push(LocatedDeclaration {
                        path: path.clone(),
                        declaration,
                    });
            }
        }
        Ok(index)
    }

    fn resolve<'a>(
        &'a self,
        rust_name: &str,
        preferred_paths: &[&str],
    ) -> Result<&'a LocatedDeclaration, String> {
        let candidates = self
            .declarations
            .get(rust_name)
            .ok_or_else(|| format!("no public Rust declaration named `{rust_name}`"))?;
        for preferred in preferred_paths {
            let matches = candidates
                .iter()
                .filter(|candidate| candidate.path.to_string_lossy().contains(preferred))
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [candidate] => return Ok(candidate),
                [] => {}
                _ => {
                    return Err(format!(
                        "ambiguous Rust declaration `{rust_name}` under `{preferred}`: {}",
                        matches
                            .iter()
                            .map(|candidate| candidate.path.display().to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
        }
        match candidates.as_slice() {
            [candidate] => Ok(candidate),
            _ => Err(format!(
                "ambiguous Rust declaration `{rust_name}`; add a preferred source path: {}",
                candidates
                    .iter()
                    .map(|candidate| candidate.path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}

fn collect_rust_files(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to scan {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect an entry under {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, output)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
    Ok(())
}

fn is_public(visibility: &Visibility) -> bool {
    matches!(visibility, Visibility::Public(_))
}

#[derive(Debug, Clone, Copy, Default)]
struct SerdeOptions {
    rename_all: Option<RenameRule>,
    rename_all_fields: Option<RenameRule>,
    tag: Option<&'static str>,
    rename: Option<&'static str>,
    skip: bool,
    flatten: bool,
}

#[derive(Debug, Clone, Copy)]
enum RenameRule {
    CamelCase,
    SnakeCase,
    Lowercase,
    PascalCase,
}

fn serde_options(attributes: &[Attribute]) -> Result<SerdeOptions, String> {
    let mut options = SerdeOptions::default();
    for attribute in attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("serde"))
    {
        attribute
            .parse_nested_meta(|meta| {
                if meta.path.is_ident("rename_all") {
                    let value = meta.value()?.parse::<syn::LitStr>()?;
                    options.rename_all = Some(parse_rename_rule(&value.value(), value.span())?);
                } else if meta.path.is_ident("rename_all_fields") {
                    let value = meta.value()?.parse::<syn::LitStr>()?;
                    options.rename_all_fields =
                        Some(parse_rename_rule(&value.value(), value.span())?);
                } else if meta.path.is_ident("tag") {
                    let value = meta.value()?.parse::<syn::LitStr>()?;
                    options.tag = Some(leak(value.value()));
                } else if meta.path.is_ident("rename") {
                    let value = meta.value()?.parse::<syn::LitStr>()?;
                    options.rename = Some(leak(value.value()));
                } else if meta.path.is_ident("skip") || meta.path.is_ident("skip_serializing") {
                    options.skip = true;
                } else if meta.path.is_ident("flatten") {
                    options.flatten = true;
                } else if meta.input.peek(syn::Token![=]) {
                    let _ignored = meta.value()?.parse::<Expr>()?;
                }
                Ok(())
            })
            .map_err(|error| format!("unsupported serde attribute: {error}"))?;
    }
    Ok(options)
}

fn leak(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn parse_rename_rule(value: &str, span: proc_macro2::Span) -> syn::Result<RenameRule> {
    match value {
        "camelCase" => Ok(RenameRule::CamelCase),
        "snake_case" => Ok(RenameRule::SnakeCase),
        "lowercase" => Ok(RenameRule::Lowercase),
        "PascalCase" => Ok(RenameRule::PascalCase),
        _ => Err(syn::Error::new(
            span,
            format!("unsupported serde rename rule `{value}`"),
        )),
    }
}

fn renamed(name: &str, explicit: Option<&str>, rule: Option<RenameRule>) -> String {
    if let Some(explicit) = explicit {
        return explicit.to_string();
    }
    match rule {
        None => name.to_string(),
        Some(RenameRule::CamelCase) => to_camel_case(name),
        Some(RenameRule::SnakeCase) => to_snake_case(name),
        Some(RenameRule::Lowercase) => name.to_ascii_lowercase(),
        Some(RenameRule::PascalCase) => to_pascal_case(name),
    }
}

fn words(name: &str) -> Vec<String> {
    if name.contains('_') {
        return name
            .split('_')
            .filter(|word| !word.is_empty())
            .map(str::to_ascii_lowercase)
            .collect();
    }
    let mut output = Vec::new();
    let mut current = String::new();
    for character in name.chars() {
        if character.is_ascii_uppercase() && !current.is_empty() {
            output.push(current.to_ascii_lowercase());
            current.clear();
        }
        current.push(character);
    }
    if !current.is_empty() {
        output.push(current.to_ascii_lowercase());
    }
    output
}

fn to_camel_case(name: &str) -> String {
    let mut parts = words(name).into_iter();
    let Some(mut output) = parts.next() else {
        return String::new();
    };
    for part in parts {
        output.push_str(&upper_first(&part));
    }
    output
}

fn to_pascal_case(name: &str) -> String {
    words(name)
        .into_iter()
        .map(|word| upper_first(&word))
        .collect()
}

fn to_snake_case(name: &str) -> String {
    words(name).join("_")
}

fn upper_first(value: &str) -> String {
    let mut characters = value.chars();
    characters
        .next()
        .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
        .unwrap_or_default()
}

fn lower_first(value: &str) -> String {
    let mut characters = value.chars();
    characters
        .next()
        .map(|first| first.to_ascii_lowercase().to_string() + characters.as_str())
        .unwrap_or_default()
}

fn documentation(attributes: &[Attribute]) -> String {
    attributes
        .iter()
        .filter_map(|attribute| {
            if !attribute.path().is_ident("doc") {
                return None;
            }
            let syn::Meta::NameValue(meta) = &attribute.meta else {
                return None;
            };
            let Expr::Lit(ExprLit {
                lit: Lit::Str(value),
                ..
            }) = &meta.value
            else {
                return None;
            };
            Some(value.value().trim().to_string())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn ts_type(rust_type: &Type) -> Result<TsType, String> {
    match rust_type {
        Type::Reference(reference) => ts_type(&reference.elem),
        Type::Paren(parenthesized) => ts_type(&parenthesized.elem),
        Type::Array(array) => {
            let length = match &array.len {
                Expr::Lit(ExprLit {
                    lit: Lit::Int(length),
                    ..
                }) => length
                    .base10_parse::<usize>()
                    .map_err(|error| format!("invalid array length: {error}"))?,
                other => return Err(format!("unsupported non-literal array length `{other:?}`")),
            };
            let item = ts_type(&array.elem)?;
            Ok(TsType::Tuple(vec![item; length]))
        }
        Type::Tuple(tuple) => tuple
            .elems
            .iter()
            .map(ts_type)
            .collect::<Result<Vec<_>, _>>()
            .map(TsType::Tuple),
        Type::Path(path) if path.qself.is_none() => {
            let segment = path
                .path
                .segments
                .last()
                .ok_or_else(|| "empty Rust type path".to_string())?;
            let name = segment.ident.to_string();
            match name.as_str() {
                "String" | "str" => Ok(TsType::Prim(TsPrim::String)),
                "bool" => Ok(TsType::Prim(TsPrim::Boolean)),
                "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64"
                | "i128" | "isize" | "f32" | "f64" => Ok(TsType::Prim(TsPrim::Number)),
                "Option" => Ok(TsType::nullable(single_type_argument(segment)?)),
                "Vec" | "VecDeque" | "BTreeSet" | "HashSet" => {
                    Ok(TsType::array(single_type_argument(segment)?))
                }
                "Box" | "Arc" | "Rc" => single_type_argument(segment),
                "BTreeMap" | "HashMap" => {
                    let arguments = type_arguments(segment)?;
                    if arguments.len() != 2 {
                        return Err(format!("map `{name}` requires two type arguments"));
                    }
                    Ok(TsType::Map(
                        Box::new(arguments[0].clone()),
                        Box::new(arguments[1].clone()),
                    ))
                }
                _ if matches!(segment.arguments, PathArguments::None) => match name.as_str() {
                    "VoxelMaterialId" | "GridId" | "SceneObjectSnapshotHash" => {
                        Ok(TsType::Prim(TsPrim::Number))
                    }
                    "AssetId" | "AssetHash" => Ok(TsType::Prim(TsPrim::String)),
                    "Vec3" => Ok(TsType::Tuple(vec![TsType::Prim(TsPrim::Number); 3])),
                    "Quat" => Ok(TsType::Tuple(vec![TsType::Prim(TsPrim::Number); 4])),
                    "VoxelConversionCoord" => Ok(TsType::reference("VoxelCoord")),
                    "ProjectBundleVoxelCoord" => Ok(TsType::reference("VoxelCoord")),
                    "ProjectBundleVoxelValue" => Ok(TsType::reference("VoxelValue")),
                    _ => Ok(TsType::reference(name.strip_suffix("Dto").unwrap_or(&name))),
                },
                _ => Err(format!("unsupported Rust type `{rust_type:?}`")),
            }
        }
        other => Err(format!("unsupported Rust type `{other:?}`")),
    }
}

fn type_arguments(segment: &syn::PathSegment) -> Result<Vec<TsType>, String> {
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(format!("`{}` requires type arguments", segment.ident));
    };
    arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            GenericArgument::Type(rust_type) => Some(ts_type(rust_type)),
            _ => None,
        })
        .collect()
}

fn single_type_argument(segment: &syn::PathSegment) -> Result<TsType, String> {
    let arguments = type_arguments(segment)?;
    match arguments.as_slice() {
        [argument] => Ok(argument.clone()),
        _ => Err(format!("`{}` requires one type argument", segment.ident)),
    }
}

fn fields(fields: &Fields, rename_rule: Option<RenameRule>) -> Result<Vec<Field>, String> {
    match fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .filter_map(|field| {
                let options = match serde_options(&field.attrs) {
                    Ok(options) => options,
                    Err(error) => return Some(Err(error)),
                };
                if options.skip {
                    return None;
                }
                if options.flatten {
                    return Some(Err(format!(
                        "serde(flatten) is unsupported on `{}`",
                        field
                            .ident
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default()
                    )));
                }
                let rust_name = field
                    .ident
                    .as_ref()?
                    .to_string()
                    .trim_start_matches("r#")
                    .to_string();
                Some(
                    ts_type(&field.ty)
                        .map(|ty| Field::new(renamed(&rust_name, options.rename, rename_rule), ty)),
                )
            })
            .collect(),
        Fields::Unit => Ok(Vec::new()),
        Fields::Unnamed(_) => Err("tuple fields require variant/newtype handling".to_string()),
    }
}

fn tuple_payload_field(fields: &syn::FieldsUnnamed) -> Result<Vec<Field>, String> {
    if fields.unnamed.len() != 1 {
        return Err("only single-value tuple variants are supported".to_string());
    }
    let rust_type = &fields.unnamed[0].ty;
    let ty = ts_type(rust_type)?;
    let name = match &ty {
        TsType::Ref(name) if name.ends_with("Rejection") => "rejection".to_string(),
        TsType::Ref(name) if name.ends_with("Outcome") => "outcome".to_string(),
        TsType::Ref(name) if name == "VoxelHit" => "hit".to_string(),
        TsType::Ref(name) => lower_first(name),
        _ => "value".to_string(),
    };
    Ok(vec![Field::new(name, ty)])
}

fn apply_border_field_policy(parent: &str, fields: &mut Vec<Field>) {
    for field in fields.iter_mut() {
        let reference = match (parent, field.name.as_str()) {
            ("RenderMaterial", "uvStrategy") => Some("UvStrategy"),
            ("CollisionMaterial", "structuralClass") => Some("StructuralClass"),
            ("CatalogEntry" | "AssetLockEntry", "kind") => Some("AssetKind"),
            ("CatalogValidationError", "code") => Some("CatalogValidationCode"),
            ("CatalogValidationError", "kind" | "expected" | "actual") => {
                field.ty = TsType::nullable(TsType::reference("AssetKind"));
                None
            }
            ("LockFinding", "code") => Some("LockIssueCode"),
            ("LockFinding", "lockedKind" | "currentKind") => {
                field.ty = TsType::nullable(TsType::reference("AssetKind"));
                None
            }
            ("FallbackDecision", "visual") => Some("FallbackVisual"),
            ("AudioEmitter", "entity") => Some("EntityId"),
            _ => None,
        };
        if parent == "Command" && field.name.ends_with("Command") {
            field.name = "command".to_string();
        } else if parent == "SceneNodeKind" && field.name == "assetReference" {
            field.name = "asset".to_string();
        } else if parent == "ScriptOutcome" && field.name == "script" {
            field.name = "rejection".to_string();
        } else if parent == "VoxelEditRejection"
            && matches!(field.name.as_str(), "voxelMaterialId" | "value")
        {
            field.name = "material".to_string();
            field.ty = TsType::Prim(TsPrim::Number);
        } else if parent == "PickResult" && field.name == "voxelHit" {
            field.name = "hit".to_string();
        } else if parent == "PickResult" && field.name == "pick" {
            field.name = "rejection".to_string();
        }
        if let Some(reference) = reference {
            field.ty = TsType::reference(reference);
        }
    }
    if parent == "PickRejection" {
        if let Some(index) = fields
            .iter()
            .position(|field| field.name == "authoritative")
        {
            fields.splice(
                index..=index,
                [
                    Field::new("authoritativeVoxel", TsType::reference("VoxelCoord")),
                    Field::new("authoritativeFace", TsType::reference("Face")),
                ],
            );
        }
    }
}

fn declaration_item(
    declaration: &LocatedDeclaration,
    output_name: &str,
    discriminant_override: Option<&str>,
) -> Result<Item, String> {
    let result = match &declaration.declaration {
        Declaration::Struct(item) => {
            let options = serde_options(&item.attrs)?;
            match &item.fields {
                Fields::Named(item_fields) => {
                    let mut derived_fields = fields(
                        &Fields::Named(item_fields.clone()),
                        options.rename_all.or(Some(RenameRule::CamelCase)),
                    )?;
                    apply_border_field_policy(output_name, &mut derived_fields);
                    if output_name == "ModelMaterialPreviewSnapshot" {
                        let classification = derived_fields
                            .iter_mut()
                            .find(|field| field.name == "rendererClassification")
                            .expect("preview snapshot owns rendererClassification");
                        classification.ty = TsType::StringEnum(vec![
                            "reference_preview".to_string(),
                            "runtime_readback".to_string(),
                        ]);
                    }
                    Item::Interface {
                        doc: documentation(&item.attrs),
                        name: output_name.to_string(),
                        fields: derived_fields,
                    }
                }
                Fields::Unnamed(item_fields) if item_fields.unnamed.len() == 1 => Item::Alias {
                    doc: documentation(&item.attrs),
                    name: output_name.to_string(),
                    ty: ts_type(&item_fields.unnamed[0].ty)?,
                },
                Fields::Unnamed(_) => {
                    return Err(format!(
                        "unsupported tuple struct `{}` in {}",
                        item.ident,
                        declaration.path.display()
                    ));
                }
                Fields::Unit => {
                    return Err(format!(
                        "unsupported unit struct `{}` in {}",
                        item.ident,
                        declaration.path.display()
                    ));
                }
            }
        }
        Declaration::Enum(item) => {
            let options = serde_options(&item.attrs)?;
            let unit_only = item
                .variants
                .iter()
                .all(|variant| matches!(variant.fields, Fields::Unit));
            if unit_only {
                let enum_rule = match output_name {
                    "CameraCollisionPolicyMode" | "ScreenPointSpace" => Some(RenameRule::SnakeCase),
                    _ => options.rename_all.or(Some(RenameRule::CamelCase)),
                };
                let values = item
                    .variants
                    .iter()
                    .map(|variant| {
                        let variant_options = serde_options(&variant.attrs).map_err(|error| {
                            format!("failed to derive enum variant `{}`: {error}", variant.ident)
                        })?;
                        Ok((!variant_options.skip).then(|| {
                            if output_name == "ScreenPointSpace" && variant.ident == "Normalized01"
                            {
                                "normalized_0_1".to_string()
                            } else {
                                renamed(
                                    &variant.ident.to_string(),
                                    variant_options.rename,
                                    enum_rule,
                                )
                            }
                        }))
                    })
                    .collect::<Result<Vec<_>, String>>()?
                    .into_iter()
                    .flatten()
                    .collect();
                Item::Alias {
                    doc: documentation(&item.attrs),
                    name: output_name.to_string(),
                    ty: TsType::StringEnum(values),
                }
            } else {
                let discriminant = discriminant_override
                    .or(options.tag)
                    .ok_or_else(|| {
                        format!(
                            "enum `{}` needs a serde tag or manifest discriminant",
                            item.ident
                        )
                    })?
                    .to_string();
                let field_rule = options
                    .rename_all_fields
                    .or(options.rename_all)
                    .or(Some(RenameRule::CamelCase));
                let variants = item
                    .variants
                    .iter()
                    .filter_map(|variant| {
                        let variant_options = match serde_options(&variant.attrs) {
                            Ok(options) => options,
                            Err(error) => return Some(Err(error)),
                        };
                        if variant_options.skip {
                            return None;
                        }
                        let tag = renamed(
                            &variant.ident.to_string(),
                            variant_options.rename,
                            options.rename_all.or(Some(RenameRule::CamelCase)),
                        );
                        let variant_fields = match &variant.fields {
                            Fields::Named(named) => {
                                fields(&Fields::Named(named.clone()), field_rule)
                            }
                            Fields::Unnamed(unnamed) => tuple_payload_field(unnamed),
                            Fields::Unit => Ok(Vec::new()),
                        };
                        Some(variant_fields.map(|mut fields| {
                            apply_border_field_policy(output_name, &mut fields);
                            Variant::new(tag, fields)
                        }))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Item::Union {
                    doc: documentation(&item.attrs),
                    name: output_name.to_string(),
                    discriminant,
                    variants,
                }
            }
        }
        Declaration::Alias(item) => Item::Alias {
            doc: documentation(&item.attrs),
            name: output_name.to_string(),
            ty: ts_type(&item.ty)?,
        },
        Declaration::Const(_) => {
            return Err(format!("`{output_name}` is a const, not a type"));
        }
        Declaration::MacroId => {
            return Err(format!(
                "`{output_name}` is a macro-declared ID; use a brand plan"
            ));
        }
    };
    Ok(result)
}

fn branded_id_item(declaration: &LocatedDeclaration, output_name: &str) -> Result<Item, String> {
    let attrs = match &declaration.declaration {
        Declaration::Struct(item) => match &item.fields {
            Fields::Unnamed(fields)
                if fields.unnamed.len() == 1
                    && ts_type(&fields.unnamed[0].ty) == Ok(TsType::Prim(TsPrim::Number)) =>
            {
                &item.attrs
            }
            _ => {
                return Err(format!(
                    "brand source `{}` must be a one-field numeric tuple struct",
                    item.ident
                ));
            }
        },
        Declaration::MacroId => {
            return Ok(Item::BrandedId {
                doc: format!("Branded `{output_name}` border identifier."),
                name: output_name.to_string(),
            });
        }
        Declaration::Enum(_) | Declaration::Alias(_) | Declaration::Const(_) => {
            return Err(format!(
                "brand source `{output_name}` must be a one-field numeric tuple struct"
            ));
        }
    };
    Ok(Item::BrandedId {
        doc: documentation(attrs),
        name: output_name.to_string(),
    })
}

fn constant_item(declaration: &LocatedDeclaration, output_name: &str) -> Result<Item, String> {
    let Declaration::Const(item) = &declaration.declaration else {
        return Err(format!("`{output_name}` is not a Rust const"));
    };
    let value = match item.expr.as_ref() {
        Expr::Lit(ExprLit {
            lit: Lit::Int(value),
            ..
        }) => value.base10_digits().to_string(),
        Expr::Lit(ExprLit {
            lit: Lit::Str(value),
            ..
        }) => format!("{:?}", value.value()),
        other => {
            return Err(format!(
                "unsupported const expression `{other:?}` for `{output_name}`"
            ))
        }
    };
    Ok(Item::Const {
        doc: documentation(&item.attrs),
        name: output_name.to_string(),
        value,
    })
}

fn string_enum_item(declaration: &LocatedDeclaration, output_name: &str) -> Result<Item, String> {
    let Declaration::Const(item) = &declaration.declaration else {
        return Err(format!("`{output_name}` is not a Rust const"));
    };
    let expression = match item.expr.as_ref() {
        Expr::Reference(reference) => reference.expr.as_ref(),
        expression => expression,
    };
    let Expr::Array(array) = expression else {
        return Err(format!(
            "string-enum source `{}` must be an array literal",
            item.ident
        ));
    };
    let values = array
        .elems
        .iter()
        .map(|element| match element {
            Expr::Lit(ExprLit {
                lit: Lit::Str(value),
                ..
            }) => Ok(value.value()),
            other => Err(format!(
                "string-enum source `{}` contains unsupported element `{other:?}`",
                item.ident
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Item::Alias {
        doc: documentation(&item.attrs),
        name: output_name.to_string(),
        ty: TsType::StringEnum(values),
    })
}

fn import(from: &str, names: &str) -> Import {
    Import {
        names: names.split_whitespace().map(str::to_string).collect(),
        from: from.to_string(),
    }
}

#[derive(Debug, Clone, Copy)]
struct ModulePlan {
    name: &'static str,
    preferred_paths: &'static [&'static str],
    imports: &'static [(&'static str, &'static str)],
    declarations: &'static str,
}

fn source_module(index: &SourceIndex, plan: ModulePlan) -> Result<Module, String> {
    let mut items = Vec::new();
    for declaration_plan in plan.declarations.split_whitespace() {
        let mut parts = declaration_plan.split(':');
        let kind = parts.next().unwrap_or_default();
        let output_name = parts.next().unwrap_or_default();
        let rust_name = parts.next().unwrap_or(output_name);
        let discriminant = parts.next();
        if output_name.is_empty() || parts.next().is_some() {
            return Err(format!("invalid declaration plan `{declaration_plan}`"));
        }
        if kind == "brand" {
            let declaration = index.resolve(rust_name, plan.preferred_paths)?;
            let item = branded_id_item(declaration, output_name).map_err(|error| {
                format!(
                    "failed to derive `{output_name}` from {}: {error}",
                    declaration.path.display()
                )
            })?;
            items.push(item);
            continue;
        }
        let declaration = index.resolve(rust_name, plan.preferred_paths)?;
        let item = match kind {
            "type" => declaration_item(declaration, output_name, discriminant),
            "const" => constant_item(declaration, output_name),
            "enumconst" => string_enum_item(declaration, output_name),
            _ => return Err(format!("unsupported declaration plan kind `{kind}`")),
        }
        .map_err(|error| {
            format!(
                "failed to derive `{output_name}` from {}: {error}",
                declaration.path.display()
            )
        })?;
        items.push(item);
    }
    Ok(Module {
        name: plan.name,
        imports: plan
            .imports
            .iter()
            .map(|(from, names)| import(from, names))
            .collect(),
        items,
    })
}

// Module plans are filled below. Keeping this as data makes declaration order
// inspectable while every field and variant remains Rust-derived.
const MODULE_PLANS: &[ModulePlan] = &[
    ModulePlan {
        name: "script",
        preferred_paths: &["protocol/protocol-script", "state/core-commands"],
        imports: &[(
            "./ids.js",
            "EntityId SubjectId ProcessId ModeId SignalId TagId",
        )],
        declarations: "type:EntityView type:ProcessView type:ScriptView type:EntityCommand:EntityCommand:kind type:SubjectCommand:SubjectCommand:kind type:ProcessCommand:ProcessCommand:kind type:ModeCommand:ModeCommand:kind type:SignalCommand:SignalCommand:kind type:TagCommand:TagCommand:kind type:Command:Command:domain type:CommandKind type:CommandEnvelope type:ScriptRejection:ScriptRejection:reason type:ScriptOutcome:ScriptOutcome:status",
    },
    ModulePlan {
        name: "replay",
        preferred_paths: &["protocol/protocol-replay", "state/core-events"],
        imports: &[
            ("./ids.js", "EntityId SubjectId ProcessId ModeId SignalId TagId"),
            ("./script.js", "CommandEnvelope"),
        ],
        declarations: "brand:StepIndex brand:ReplayHash const:REPLAY_FORMAT_VERSION type:DomainEvent:DomainEvent:event type:StepOutcome:StepOutcome:status type:ReplayStep type:SnapshotMeta type:ReplayRecord",
    },
    ModulePlan {
        name: "scene",
        preferred_paths: &[
            "protocol/protocol-scene",
            "foundation/core-assets",
        ],
        imports: &[("./ids.js", "EntityId")],
        declarations: "brand:ProjectId brand:SceneId brand:RuntimeSessionId brand:SceneNodeId type:SceneNodeKindTag enumconst:SceneValidationCode:SCENE_VALIDATION_CODES enumconst:SceneObjectCommandRejectionCode:SCENE_OBJECT_COMMAND_REJECTION_CODES type:AssetVersionReq:AssetVersionReqDto:req type:AssetReference:AssetReferenceDto type:SceneTransform:SceneTransformDto type:SceneNodeKind:SceneNodeKindDto:kind type:SceneNodeRecord:SceneNodeRecordDto type:SceneMetadata:SceneMetadataDto type:FlatSceneDocument:FlatSceneDocumentDto type:SceneValidationError:SceneValidationErrorDto type:SceneValidationReport:SceneValidationReportDto type:SceneObjectRecord:SceneObjectRecordDto type:SceneObjectSnapshot:SceneObjectSnapshotDto type:SceneObjectCommand:SceneObjectCommandDto:kind type:SceneObjectCommandRejection:SceneObjectCommandRejectionDto type:SceneObjectCommandOutcome:SceneObjectCommandOutcomeDto type:SceneObjectCommandRequest:SceneObjectCommandRequestDto type:SceneObjectCommandResult:SceneObjectCommandResultDto type:SceneSourceTrace:SceneSourceTraceDto type:BootstrapRecord:BootstrapRecordDto",
    },
    ModulePlan {
        name: "voxel",
        preferred_paths: &[
            "foundation/core-space/src/voxel.rs",
            "state/core-voxel",
            "state/core-commands/src/voxel.rs",
            "state/core-events/src/voxel.rs",
            "rules/rule-voxel-edit",
            "bridge/runtime-bridge-api/src/payloads.rs",
            "foundation/core-space/src/direction.rs",
        ],
        imports: &[],
        declarations: "type:VoxelCoord type:ChunkCoord type:VoxelValue:VoxelValue:kind type:VoxelCommand:VoxelCommand:op type:VoxelEditEvent:VoxelEditEvent:event type:VoxelEditRejection:VoxelEditRejection:reason type:CommandBatch type:CommandResult type:Face:Direction6 type:PickRejection:PickRejection:reason type:PickRay type:VoxelHit type:PickResult:PickResult:outcome",
    },
    ModulePlan {
        name: "projectBundle",
        preferred_paths: &[
            "protocol/protocol-project-bundle",
            "services/svc-serialization",
            "rules/rule-project-bundle",
            "bridge/runtime-bridge-api/src/payloads.rs",
        ],
        imports: &[
            ("./scene.js", "ProjectId RuntimeSessionId SceneId"),
            ("./voxel.js", "VoxelCoord VoxelValue"),
        ],
        declarations: "brand:PrefabId brand:PrefabPartId brand:PrefabInstanceId type:ArtifactClass enumconst:KnownArtifactRole:KNOWN_ARTIFACT_ROLES type:LoadStage type:SuggestedAction type:ArtifactEntry type:GeneratorMetadata type:ProjectSection type:SceneSection type:AssetLockSection type:ProjectBundleManifest const:GAMEPLAY_TRIGGER_DEFINITION_SCHEMA_VERSION type:GameplayTriggerDefinition const:PREFAB_REGISTRY_SCHEMA_VERSION const:PREFAB_DEFINITION_SCHEMA_VERSION enumconst:PrefabDiagnosticCode:PREFAB_DIAGNOSTIC_CODES type:PrefabTransform type:PrefabPartSource:PrefabPartSource:kind type:PrefabPart type:PrefabPartRoleBinding type:PrefabOverrideValue:PrefabOverrideValue:field type:PrefabOverride type:PrefabVariantDelta type:PrefabDefinition type:PrefabRegistry type:PrefabInstanceRecord type:PrefabPartReference type:PrefabDiagnostic type:PrefabValidationOutcome:PrefabValidationOutcome:status type:ManifestError:ManifestError:code type:ManifestValidationReport type:LoadStep:LoadStep:step type:LoadPlan type:LoadPlanError:LoadPlanError:code type:CompactionSummary type:SaveSummary type:GeneratorMismatch type:EditConflict type:RegenConflictReport",
    },
    ModulePlan {
        name: "assets",
        preferred_paths: &[
            "protocol/protocol-assets",
            "state/core-catalog",
            "foundation/core-assets",
        ],
        imports: &[("./scene.js", "AssetReference")],
        declarations: "enumconst:AssetKind:ASSET_KINDS enumconst:StructuralClass:STRUCTURAL_CLASSES enumconst:UvStrategy:UV_STRATEGIES enumconst:CatalogValidationCode:CATALOG_VALIDATION_CODES enumconst:LockIssueCode:LOCK_ISSUE_CODES enumconst:FallbackContext:FALLBACK_CONTEXTS type:FallbackVisual type:Rgba type:RenderMaterial type:CollisionMaterial type:MaterialProjection type:CatalogEntry type:Catalog type:CatalogValidationError type:CatalogValidationReport type:AssetLockEntry type:AssetLock type:LockFinding type:LockValidationReport type:FallbackDecision:FallbackDecision:outcome",
    },
    ModulePlan {
        name: "diagnostics",
        preferred_paths: &["protocol/protocol-diagnostics"],
        imports: &[],
        declarations: "type:DiagnosticSeverity type:DiagnosticScope type:DiagnosticCode type:RemedyAction type:SuggestedRemedy type:DiagnosticSourceRef type:DiagnosticReport type:DiagnosticReportSet type:SourceTrace type:RendererResourceReport",
    },
    ModulePlan {
        name: "entityAuthoring",
        preferred_paths: &["protocol/protocol-entity-authoring"],
        imports: &[
            ("./ids.js", "EntityId TagId ProcessId SubjectId"),
            ("./scene.js", "SceneNodeId"),
        ],
        declarations: "type:AuthoringTransform type:AuthoringSource:AuthoringSource:kind type:AuthoringCapability:AuthoringCapability:kind enumconst:ActivatableCapabilityKind:ACTIVATABLE_CAPABILITY_KINDS enumconst:CapabilityActivationAction:CAPABILITY_ACTIVATION_ACTIONS enumconst:CapabilityActivationPresence:CAPABILITY_ACTIVATION_PRESENCE_VALUES enumconst:CapabilityActivationEntityLifecycle:CAPABILITY_ACTIVATION_ENTITY_LIFECYCLES enumconst:CapabilityActivationDiagnosticCode:CAPABILITY_ACTIVATION_DIAGNOSTIC_CODES type:CapabilityActivationRequest type:CapabilityActivationEvent type:CapabilityActivationReadout type:CapabilityActivationDiagnostic type:CapabilityActivationOutcome:CapabilityActivationOutcome:status type:EntityDefinitionSourceTrace type:EntityDefinitionMetadataEntry type:EntityDefinitionCapability:EntityDefinitionCapability:kind type:EntityDefinition type:EntityDefinitionDiagnosticCode type:EntityDefinitionDiagnostic type:EntityDefinitionValidationOutcome:EntityDefinitionValidationOutcome:status type:EntityAuthoringCommand:EntityAuthoringCommand:kind type:AuthoringEventKind type:EntityAuthoringEvent type:AuthoringRejectionReason type:EntityAuthoringRejection type:EntityAuthoringOutcome:EntityAuthoringOutcome:status",
    },
    ModulePlan {
        name: "gameExtension",
        preferred_paths: &["protocol/protocol-game-extension"],
        imports: &[
            ("./ids.js", "EntityId"),
            ("./diagnostics.js", "DiagnosticSeverity"),
            ("./projectBundle.js", "PrefabId PrefabInstanceId PrefabPartReference"),
        ],
        declarations: "type:GameExtensionHookKind enumconst:GameExtensionProposalKind:GAME_EXTENSION_PROPOSAL_KINDS type:GameExtensionReceiptStatus type:GameExtensionDiagnosticCode type:GameRuleModuleRef type:GameRuleHookDeclaration type:GameRuleModuleManifest type:GameExtensionDiagnostic type:WeaponEffectHookRequest type:GameExtensionProposal:GameExtensionProposal:kind type:GameExtensionTraceEntry type:GameExtensionHookReceipt type:GameExtensionReplayEvidence enumconst:GameplayInvocationFamily:GAMEPLAY_INVOCATION_FAMILIES enumconst:GameplayEventPhase:GAMEPLAY_EVENT_PHASES enumconst:GameplayReadViewKind:GAMEPLAY_READ_VIEW_KINDS enumconst:GameplayReadSelectorCapability:GAMEPLAY_READ_SELECTOR_CAPABILITIES enumconst:GameplayRegistryDiagnosticCode:GAMEPLAY_REGISTRY_DIAGNOSTIC_CODES const:GAMEPLAY_MODULE_BINDING_SCHEMA_VERSION enumconst:GameplayModuleBindingDiagnosticCode:GAMEPLAY_MODULE_BINDING_DIAGNOSTIC_CODES type:GameplayContractRef type:GameplayModuleRef type:GameplayModuleConfiguration type:GameplayModuleBindingTarget:GameplayModuleBindingTarget:kind type:GameplayModuleBinding type:GameplayModuleBindingOverride type:GameplayModuleBindingRegistry type:GameplayModuleBindingDiagnostic type:GameplayModuleBindingReadout type:GameplayModuleBindingActivationReceipt type:GameplayOwnerRef type:GameplayEventSchemaDeclaration type:GameplayEntityRef type:GameplayEmitterRef:GameplayEmitterRef:kind type:GameplayCausationRef type:GameplayEventEnvelope type:GameplayHeaderSelector type:GameplaySubscriptionDeclaration type:GameplayInvocationReadRequirement type:GameplayInvocationDescriptor type:GameplayProposalDeclaration type:GameplayProposalEnvelope type:GameplayReadViewRequirement type:GameplayOwnedSchemaDeclaration type:GameplayOrderingConstraint type:GameplayExecutionBudget type:GameplayModuleManifest type:GameplayRegistryDiagnostic type:GameplayTopologyEdge type:GameplayReadViewProviderReadout type:GameplayRegistryReadout type:GameplayRegistryValidationOutcome:GameplayRegistryValidationOutcome:status",
    },
    ModulePlan {
        name: "gameRules",
        preferred_paths: &["protocol/protocol-game-rules"],
        imports: &[
            ("./ids.js", "EntityId"),
            ("./diagnostics.js", "DiagnosticSeverity"),
        ],
        declarations: "enumconst:GameRuleEffectOpKind:GAME_RULE_EFFECT_OP_KINDS enumconst:GameRuleStackPolicyKind:GAME_RULE_STACK_POLICIES type:GameRuleDiagnosticCode type:GameRuleEvidenceKind type:GameRuleCatalogRef type:GameRuleValueChannelRef type:GameRuleBoundedValue type:GameRuleValueDelta type:GameRuleDuration:GameRuleDuration:kind type:GameRuleTickCadence type:GameRuleStackPolicy:GameRuleStackPolicy:kind type:GameRuleEffectOp:GameRuleEffectOp:kind type:GameRuleModifierDefinition type:GameRuleEffectBundle type:GameRuleCatalog type:GameRuleDiagnostic type:GameRuleEvidenceRef type:GameRuleTraceRef type:GameRuleTraceEntry type:GameRuleModifierState type:GameRuleResolutionRequest type:GameRuleResolutionReceipt",
    },
    ModulePlan {
        name: "policyView",
        preferred_paths: &["protocol/protocol-policy-view"],
        imports: &[("./ids.js", "EntityId TagId")],
        declarations: "type:PolicyTransform type:PolicyEntityLifecycle type:PolicyEntitySource:PolicyEntitySource:kind type:PolicyAssetStatus type:PolicyAssetView type:PolicyEntityView type:PolicyWorldSummary type:PolicyWorldView type:PolicyWorldCommand:PolicyWorldCommand:kind type:PolicyWorldEvent:PolicyWorldEvent:kind type:PolicyWorldRejection type:PolicyWorldOutcome:PolicyWorldOutcome:status",
    },
    ModulePlan {
        name: "render",
        preferred_paths: &["protocol/protocol-render", "bridge/runtime-bridge-api"],
        imports: &[
            ("./ids.js", "EntityId TagId"),
            ("./assets.js", "CatalogEntry MaterialProjection"),
        ],
        declarations: "brand:RenderHandle type:Transform type:Geometry:Geometry:shape type:Material type:RenderLayer type:RenderMetadata type:RenderNode type:MeshAttributeKind type:MeshAttributeName type:MeshAttribute type:MeshIndexWidth type:MeshBufferLayout type:MeshGroupDescriptor type:MeshBoundsDescriptor type:MeshProvenance type:MeshPayloadSource:MeshPayloadSource:kind type:MeshPayloadDescriptor type:MeshMaterialSlot type:MeshCollisionPolicy:MeshCollisionPolicy:kind type:StaticMeshAsset type:StaticMeshInstanceDescriptor type:AnimatedMeshRuntimeFormat type:AnimationLoopMode type:AnimationClipDescriptor type:AnimatedMeshAsset type:AnimatedMeshPlaybackCommand:AnimatedMeshPlaybackCommand:action type:AnimatedMeshInstanceDescriptor type:SpriteSizeMode type:BillboardMode type:SpriteDepthPolicy type:SpriteShading type:SpriteAttachment type:SpriteInstanceDescriptor type:SpritePickHit type:MeshPickHit type:TextureFilter type:TextureWrap type:TextureDescriptor type:SpriteFrameRect type:SpriteAtlasDescriptor type:MaterialUvStrategy type:RenderMaterialDescriptor type:MaterialInstanceParameters type:RenderDiff:RenderDiff:op type:ModelMaterialPreviewRequest type:ModelMaterialPreviewSnapshot type:RenderFrameDiff",
    },
    ModulePlan {
        name: "presentation",
        preferred_paths: &["protocol/protocol-presentation"],
        imports: &[
            ("./ids.js", "EntityId"),
            ("./render.js", "RenderFrameDiff RenderHandle"),
        ],
        declarations: "const:RUNTIME_PROJECTION_SCHEMA_VERSION brand:AudioHandle brand:BillboardHandle brand:ParticleEmitterHandle brand:TelemetryOverlayHandle brand:AnimationProjectionHandle type:ProjectionReplayScope type:PresentationOriginKind type:PresentationOriginRef type:PresentationOpMeta type:AudioBus type:AudioEmitter:AudioEmitter:kind type:AudioClipRef type:AudioSourceDescriptor type:AudioSourcePatch type:AudioProjectionOp:AudioProjectionOp:op type:AudioProjectionDiagnosticCode type:AudioProjectionDiagnostic type:AudioProjectionReadout type:BillboardAnchor:BillboardAnchor:kind type:BillboardTemplateArgument type:BillboardTextureRef type:BillboardContent:BillboardContent:kind type:BillboardFontRef:BillboardFontRef:kind type:BillboardLayer type:BillboardDescriptor type:BillboardPatch type:BillboardProjectionOp:BillboardProjectionOp:op type:BillboardProjectionDiagnosticCode type:BillboardProjectionDiagnostic type:BillboardProjectionReadout type:ParticleAnchor:ParticleAnchor:kind type:ParticleSpriteRef type:ParticleScalarKey type:ParticleColorKey type:ParticleEmitterDescriptor type:ParticleEmitterPatch type:ParticleProjectionOp:ParticleProjectionOp:op type:ParticleProjectionDiagnosticCode type:ParticleProjectionDiagnostic type:ParticleProjectionReadout type:TelemetryOverlayCorner type:TelemetryOverlayDescriptor type:TelemetryOverlayPatch type:TelemetryOverlayProjectionOp:TelemetryOverlayProjectionOp:op type:TelemetryOverlayDiagnosticCode type:TelemetryOverlayDiagnostic type:TelemetryOverlayReadout type:AnimationResolvedMotion type:AnimationTransitionProjection type:AnimationTransitionFactMoment type:AnimationTransitionFactRef type:AnimationControllerProjectionState type:AnimationProjectionDescriptor type:AnimationProjectionOp:AnimationProjectionOp:op type:AnimationProjectionDiagnosticCode type:AnimationProjectionDiagnostic type:AnimationProjectionReadout type:PresentationOp:PresentationOp:domain type:PresentationFrameDiff type:RuntimeProjectionFrame",
    },
    ModulePlan {
        name: "telemetry",
        preferred_paths: &["protocol/protocol-telemetry"],
        imports: &[],
        declarations: "type:TelemetrySource type:TelemetryLevel type:TelemetryMetricKind type:TelemetryMetric type:TelemetryEvent:TelemetryEvent:kind type:TelemetryEnvelope type:LiveTelemetryCounter type:LiveTelemetryMetric type:LiveTelemetryDiagnosticCode type:LiveTelemetryDiagnostic type:LiveTelemetrySnapshot",
    },
    ModulePlan {
        name: "input",
        preferred_paths: &["protocol/protocol-input"],
        imports: &[],
        declarations: "const:INPUT_BINDING_CATALOG_SCHEMA_VERSION const:INPUT_CONTEXT_STATE_SCHEMA_VERSION const:INPUT_ACTION_RECORD_SCHEMA_VERSION type:InputActionId type:InputContextId type:InputBindingId type:InputValueKind type:InputActionPhase type:PlatformInputKind type:InputValue:InputValue:kind type:InputActionDefinition type:InputContextDefinition type:InputBindingExtension type:InputBindingRecord type:InputBindingCatalog type:InputSessionConfigureRequest type:ActiveInputContext type:InputContextStackState type:InputContextCommand:InputContextCommand:operation type:InputContextChangeReceipt type:InputSessionSnapshot type:RawInputSample type:ResolvedInputAction type:RecordedInputAction type:InputDiagnosticCode type:InputDiagnostic type:InputResolutionReceipt type:InputActionReplayReceipt",
    },
    ModulePlan {
        name: "timeControl",
        preferred_paths: &["protocol/protocol-time-control"],
        imports: &[],
        declarations: "const:TIME_CONTROL_STATE_SCHEMA_VERSION enumconst:TimeControlMode:TIME_CONTROL_MODES type:TimeControlCommand:TimeControlCommand:operation enumconst:TimeControlRejection:TIME_CONTROL_REJECTIONS type:TimeControlState type:TimeControlReceipt",
    },
    ModulePlan {
        name: "view",
        preferred_paths: &["protocol/protocol-view"],
        imports: &[("./voxel.js", "Face VoxelCoord")],
        declarations: "brand:CameraHandle type:CameraPose type:CameraBasis type:PerspectiveProjection type:ViewportSize type:CameraCreateRequest type:FirstPersonCameraInput type:FirstPersonCameraInputEnvelope type:CameraProjectionRequest type:CameraSnapshot const:CAMERA_CONTROLLER_STATE_SCHEMA_VERSION type:CameraMode type:CameraTransitionEasing type:CameraTransitionSpec type:CameraModeTarget:CameraModeTarget:mode type:CameraModeCommand type:CameraControllerState type:CameraTransitionReadout type:CameraControllerRejection type:CameraModeChangeReceipt type:CameraNavigationInput type:CameraNavigationInputEnvelope type:CameraNavigationReceipt type:CameraControllerReadRequest type:CameraProjectionSnapshot type:CameraCollisionShape type:CameraCollisionPolicyMode type:CameraCollisionPolicy type:FirstPersonMovementMode type:GeneratedTunnelPreset type:GeneratedTunnelRuntimeApplyRequest type:GeneratedTunnelRuntimeFrame type:GeneratedTunnelRuntimeApplyReceipt type:CollisionConstrainedCameraInputEnvelope type:CollisionAabbEvidence type:CollisionAxis type:CameraCollisionEvidence type:CameraCollisionSnapshot type:ScreenPointSpace type:ScreenPoint type:ScreenPointToPickRayRequest type:PickRaySnapshot type:VoxelSelectionOutcome type:VoxelSelectionSnapshot",
    },
    ModulePlan {
        name: "voxelAnnotation",
        preferred_paths: &["protocol/protocol-voxel-annotation"],
        imports: &[("./diagnostics.js", "DiagnosticSeverity")],
        declarations: "const:VOXEL_ANNOTATION_SCHEMA_VERSION const:VOXEL_ANNOTATION_MEDIA_TYPE const:VOXEL_ANNOTATION_EXTENSION type:VoxelAnnotationKind type:VoxelAnnotationProvenanceKind type:VoxelAnnotationDiagnosticCode type:VoxelAnnotationEditOperation type:VoxelAnnotationQueryMode type:VoxelAnnotationCoord type:VoxelAnnotationBounds type:VoxelAnnotationSparseRun type:VoxelAnnotationSelection type:VoxelAnnotationProvenanceRef type:VoxelAnnotationContentHashes type:VoxelAnnotationDiagnostic type:VoxelAnnotationRegion type:VoxelAnnotationLayerDraft type:VoxelAnnotationLayer type:VoxelAnnotationLayerValidationInput:VoxelAnnotationLayerValidationInput:kind type:VoxelAnnotationLayerValidationRequest type:VoxelAnnotationLayerValidationReport type:VoxelAnnotationLayerLoadRequest type:VoxelAnnotationLayerLoadReceipt type:VoxelAnnotationQueryRequest type:VoxelAnnotationRegionReadout type:VoxelAnnotationQueryReadout type:VoxelAnnotationEditRequest type:VoxelAnnotationEditReceipt type:VoxelAnnotationLayerExportRequest type:VoxelAnnotationLayerExportReceipt",
    },
    ModulePlan {
        name: "voxelAsset",
        preferred_paths: &["protocol/protocol-voxel-asset"],
        imports: &[("./diagnostics.js", "DiagnosticSeverity")],
        declarations: "const:VOXEL_ASSET_SCHEMA_VERSION const:VOXEL_ASSET_MEDIA_TYPE const:VOXEL_ASSET_EXTENSION const:VOXEL_PALETTE_UPDATE_MAX_REQUEST_BYTES const:VOXEL_PALETTE_UPDATE_MAX_SPARSE_RUNS const:VOXEL_PALETTE_UPDATE_MAX_REPRESENTED_VOXELS const:VOXEL_PALETTE_UPDATE_MAX_MATERIAL_BINDINGS const:VOXEL_PALETTE_UPDATE_MAX_PROVENANCE_REFS const:VOXEL_PALETTE_UPDATE_MAX_EMBEDDED_DIAGNOSTICS const:VOXEL_PALETTE_UPDATE_MAX_STRING_BYTES type:VoxelAssetRepresentationKind type:VoxelAssetProvenanceKind type:VoxelAssetDiagnosticCode type:VoxelAssetCoord type:VoxelAssetBounds type:VoxelAssetGrid type:VoxelAssetMaterialBinding type:VoxelAssetSparseRun type:VoxelAssetRepresentation type:VoxelAssetProvenanceRef type:VoxelAssetAuthoringMetadata type:VoxelAssetContentHashes type:VoxelAssetDiagnostic type:VoxelAssetMaterialCount type:VoxelVolumeAsset type:VoxelVolumeAssetExportRequest type:VoxelVolumeAssetExportReceipt type:VoxelVolumeAssetSaveRequest type:VoxelVolumeAssetStoredDiff type:VoxelVolumeAssetSaveReceipt type:VoxelVolumeAssetPaletteUpdateRequest type:VoxelVolumeAssetPaletteStoredDiff type:VoxelVolumeAssetPaletteUpdateReceipt type:VoxelVolumeAuthoringInitializeRequest type:VoxelVolumeAuthoringInitializeReceipt type:VoxelVolumeAssetLoadRequest type:VoxelVolumeAssetLoadReceipt type:VoxelVolumeAssetUnloadRequest type:VoxelVolumeAssetUnloadReceipt",
    },
    ModulePlan {
        name: "voxelConversion",
        preferred_paths: &["protocol/protocol-voxel-conversion"],
        imports: &[
            ("./diagnostics.js", "DiagnosticSeverity"),
            ("./voxel.js", "VoxelCoord"),
        ],
        declarations: "const:VOXEL_CONVERSION_MESH_IMPORT_MAX_SOURCE_BYTES const:VOXEL_CONVERSION_MESH_IMPORT_MAX_VERTICES const:VOXEL_CONVERSION_MESH_IMPORT_MAX_INDICES type:VoxelConversionMode type:VoxelConversionMeshSourceFormat type:VoxelConversionFitPolicy type:VoxelConversionOriginPolicy type:VoxelConversionDiagnosticCode type:VoxelConversionEvidenceKind type:VoxelConversionSourceRef type:VoxelConversionSourceTriangle type:VoxelConversionSourceMaterialSlot type:VoxelConversionSourceRegistrationRequest type:VoxelConversionMeshAssetGroup type:VoxelConversionMeshAsset type:VoxelConversionMeshAssetRegistrationRequest type:VoxelConversionMeshSourceImportRequest type:VoxelConversionMeshSourceImportReceipt type:VoxelConversionSourceMetadataRequest type:VoxelConversionSourceBounds type:VoxelConversionSourceGroupMetadata type:VoxelConversionSourceMetadataReadout type:VoxelConversionSourceRegistration type:VoxelConversionTargetRef type:VoxelConversionBounds type:VoxelConversionMaterialMapEntry type:VoxelConversionUvAttributeRef type:VoxelConversionTextureSourceRef type:VoxelConversionTextureSampleAsset type:VoxelConversionTextureBinding type:VoxelConversionMaterialMap type:VoxelConversionSettings type:VoxelConversionPlanRequest type:VoxelConversionDiagnostic type:VoxelConversionEvidenceRef type:VoxelConversionPlan type:VoxelConversionPreviewRequest type:VoxelConversionPreviewVoxel type:VoxelConversionPreview type:VoxelConversionApplyRequest type:VoxelConversionReceipt type:VoxelModelInfoRequest type:VoxelModelMaterialCount type:VoxelModelInfoReadout type:VoxelModelWindowRequest type:VoxelModelWindowSample type:VoxelModelWindowReadout",
    },
    ModulePlan {
        name: "voxelEditHistory",
        preferred_paths: &["protocol/protocol-voxel-edit-history"],
        imports: &[("./diagnostics.js", "DiagnosticSeverity")],
        declarations: "const:VOXEL_EDIT_HISTORY_SCHEMA_VERSION const:VOXEL_EDIT_HISTORY_MEDIA_TYPE const:VOXEL_EDIT_HISTORY_EXTENSION type:VoxelEditHistoryEntryKind type:VoxelEditHistoryCursorKind type:VoxelEditHistoryRevertMode type:VoxelEditHistoryDiffLevel type:VoxelEditHistoryDiagnosticCode type:VoxelEditHistoryCoord type:VoxelEditHistoryBounds type:VoxelEditHistoryMaterialDelta type:VoxelEditHistoryCheckpointRef type:VoxelEditHistoryDiagnostic type:VoxelEditHistoryDiffSummary type:VoxelEditHistoryEntry type:VoxelEditHistoryCursor type:VoxelEditHistorySummary type:VoxelEditHistoryReadRequest type:VoxelEditHistoryRevertTarget type:VoxelEditHistoryRevertRequest type:VoxelEditHistoryUndoRequest type:VoxelEditHistoryRedoRequest type:VoxelEditHistoryPreviewEvidence type:VoxelEditHistoryRevertReceipt type:VoxelEditHistoryUndoReceipt type:VoxelEditHistoryRedoReceipt",
    },
];

pub fn try_all_modules() -> Result<Vec<Module>, String> {
    let index = SourceIndex::load()?;
    let derived = MODULE_PLANS
        .iter()
        .copied()
        .map(|plan| source_module(&index, plan))
        .collect::<Result<Vec<_>, _>>()?;
    let mut by_name = derived
        .into_iter()
        .map(|module| (module.name, module))
        .collect::<BTreeMap<_, _>>();
    let mut modules = Vec::new();
    modules.push(ids_module());
    for name in [
        "script",
        "render",
        "presentation",
        "replay",
        "voxel",
        "voxelConversion",
        "voxelAsset",
        "voxelAnnotation",
        "voxelEditHistory",
        "gameRules",
        "gameExtension",
        "scene",
        "projectBundle",
        "assets",
        "diagnostics",
        "policyView",
        "telemetry",
        "input",
        "timeControl",
        "view",
        "entityAuthoring",
    ] {
        modules.push(
            by_name
                .remove(name)
                .ok_or_else(|| format!("missing module plan `{name}`"))?,
        );
    }
    if !by_name.is_empty() {
        return Err(format!(
            "module plans are not in the generated file order: {}",
            by_name.keys().copied().collect::<Vec<_>>().join(", ")
        ));
    }
    modules.push(index_module());
    Ok(modules)
}

fn ids_module() -> Module {
    let items = protocol_ids::BORDER_IDS
        .iter()
        .map(|border_id| Item::BrandedId {
            doc: format!(
                "Branded identifier for {} (over a 64-bit integer).",
                match border_id.brand {
                    "EntityId" => "a discrete simulated entity",
                    "SubjectId" => "an acting subject / authority",
                    "ProcessId" => "an ongoing process",
                    "ModeId" => "a state-machine mode",
                    "SignalId" => "an event signal type",
                    "TagId" => "a tag label",
                    _ => "a border identifier",
                }
            ),
            name: border_id.brand.to_string(),
        })
        .collect();
    Module {
        name: "ids",
        imports: Vec::new(),
        items,
    }
}

fn index_module() -> Module {
    Module {
        name: "index",
        imports: Vec::new(),
        items: [
            "ids",
            "script",
            "render",
            "presentation",
            "replay",
            "voxel",
            "voxelConversion",
            "voxelAsset",
            "voxelAnnotation",
            "voxelEditHistory",
            "gameRules",
            "gameExtension",
            "scene",
            "projectBundle",
            "assets",
            "diagnostics",
            "policyView",
            "telemetry",
            "input",
            "timeControl",
            "view",
            "entityAuthoring",
        ]
        .into_iter()
        .map(|name| Item::ReExport {
            from: format!("./{name}.js"),
        })
        .collect(),
    }
}

pub fn all_modules() -> Vec<Module> {
    try_all_modules().unwrap_or_else(|error| panic!("protocol source derivation failed: {error}"))
}

pub fn policy_view_module() -> Module {
    let index = SourceIndex::load()
        .unwrap_or_else(|error| panic!("protocol source derivation failed: {error}"));
    let plan = MODULE_PLANS
        .iter()
        .find(|plan| plan.name == "policyView")
        .copied()
        .expect("policyView module plan exists");
    source_module(&index, plan)
        .unwrap_or_else(|error| panic!("policyView source derivation failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn source_index_with(name: &str, path: &str, declaration: Declaration) -> SourceIndex {
        let mut index = SourceIndex::default();
        index
            .declarations
            .entry(name.to_string())
            .or_default()
            .push(LocatedDeclaration {
                path: PathBuf::from(path),
                declaration,
            });
        index
    }

    fn test_plan(declarations: &'static str) -> ModulePlan {
        ModulePlan {
            name: "test",
            preferred_paths: &[],
            imports: &[],
            declarations,
        }
    }

    #[test]
    fn naming_rules_are_deterministic() {
        assert_eq!(to_camel_case("source_tool"), "sourceTool");
        assert_eq!(to_camel_case("XML_asset"), "xmlAsset");
        assert_eq!(to_snake_case("RuntimeExport"), "runtime_export");
        assert_eq!(to_pascal_case("runtime_export"), "RuntimeExport");
    }

    #[test]
    fn nested_types_and_maps_are_derived() {
        let rust_type: Type = syn::parse_str("Option<Vec<BTreeMap<String, [u32; 3]>>>")
            .expect("representative nested Rust type parses");
        assert_eq!(
            ts_type(&rust_type),
            Ok(TsType::nullable(TsType::array(TsType::Map(
                Box::new(TsType::Prim(TsPrim::String)),
                Box::new(TsType::Tuple(vec![TsType::Prim(TsPrim::Number); 3])),
            ))))
        );
    }

    #[test]
    fn unsupported_shapes_fail_closed() {
        let rust_type: Type = syn::parse_str("Result<String, Error>")
            .expect("representative unsupported Rust type parses");
        let error = ts_type(&rust_type).expect_err("unsupported generic must fail");
        assert!(error.contains("unsupported Rust type"), "{error}");
    }

    #[test]
    fn brand_plans_require_one_numeric_rust_newtype() {
        let missing = source_module(&SourceIndex::default(), test_plan("brand:MissingBrand"))
            .expect_err("missing brand authority must fail");
        assert!(
            missing.contains("no public Rust declaration named `MissingBrand`"),
            "{missing}"
        );

        let wrong_kind: ItemEnum =
            syn::parse_str("pub enum NotAnId { Value }").expect("enum parses");
        let index = source_index_with(
            "NotAnId",
            "authority/not_an_id.rs",
            Declaration::Enum(wrong_kind),
        );
        let wrong_kind = source_module(&index, test_plan("brand:NotAnId"))
            .expect_err("non-newtype brand authority must fail");
        assert!(
            wrong_kind.contains("authority/not_an_id.rs"),
            "{wrong_kind}"
        );
        assert!(
            wrong_kind.contains("must be a one-field numeric tuple struct"),
            "{wrong_kind}"
        );

        let first: ItemStruct =
            syn::parse_str("pub struct DuplicateId(pub u64);").expect("first newtype parses");
        let second = first.clone();
        let mut index = source_index_with(
            "DuplicateId",
            "authority/first.rs",
            Declaration::Struct(first),
        );
        index
            .declarations
            .get_mut("DuplicateId")
            .expect("first declaration exists")
            .push(LocatedDeclaration {
                path: PathBuf::from("authority/second.rs"),
                declaration: Declaration::Struct(second),
            });
        let ambiguous = source_module(&index, test_plan("brand:DuplicateId"))
            .expect_err("ambiguous brand authority must fail");
        assert!(
            ambiguous.contains("ambiguous Rust declaration `DuplicateId`"),
            "{ambiguous}"
        );
    }

    #[test]
    fn unit_enum_variant_serde_errors_are_not_treated_as_skips() {
        let enumeration: ItemEnum = syn::parse_str(
            r#"
                pub enum BrokenMode {
                    Accepted,
                    #[serde(rename_all = "kebab-case")]
                    Malformed,
                }
            "#,
        )
        .expect("representative enum parses");
        let index = source_index_with(
            "BrokenMode",
            "authority/broken_mode.rs",
            Declaration::Enum(enumeration),
        );
        let error = source_module(&index, test_plan("type:BrokenMode"))
            .expect_err("malformed variant serde metadata must fail");
        assert!(error.contains("authority/broken_mode.rs"), "{error}");
        assert!(error.contains("enum variant `Malformed`"), "{error}");
        assert!(
            error.contains("unsupported serde rename rule `kebab-case`"),
            "{error}"
        );
    }

    #[test]
    fn explicitly_skipped_unit_enum_variants_are_omitted() {
        let enumeration: ItemEnum = syn::parse_str(
            r#"
                pub enum SupportedMode {
                    Accepted,
                    #[serde(skip)]
                    IntentionallySkipped,
                }
            "#,
        )
        .expect("representative enum parses");
        let located = LocatedDeclaration {
            path: PathBuf::from("authority/supported_mode.rs"),
            declaration: Declaration::Enum(enumeration),
        };
        let Item::Alias {
            ty: TsType::StringEnum(values),
            ..
        } = declaration_item(&located, "SupportedMode", None).expect("enum derives")
        else {
            panic!("unit enum must derive a string union");
        };
        assert_eq!(values, ["accepted"]);
    }

    #[test]
    fn struct_fields_and_enum_variants_come_from_rust_syntax() {
        let structure: ItemStruct = syn::parse_str(
            "pub struct Sample { pub first_value: u32, pub added_later: Option<String> }",
        )
        .expect("representative struct parses");
        let located = LocatedDeclaration {
            path: PathBuf::from("sample.rs"),
            declaration: Declaration::Struct(structure),
        };
        let Item::Interface { fields, .. } =
            declaration_item(&located, "Sample", None).expect("struct derives")
        else {
            panic!("struct must derive an interface");
        };
        assert_eq!(
            fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            ["firstValue", "addedLater"]
        );

        let enumeration: ItemEnum =
            syn::parse_str("pub enum SampleOutcome { Accepted, AddedLater { receipt_id: u64 } }")
                .expect("representative enum parses");
        let located = LocatedDeclaration {
            path: PathBuf::from("sample.rs"),
            declaration: Declaration::Enum(enumeration),
        };
        let Item::Union { variants, .. } =
            declaration_item(&located, "SampleOutcome", Some("status")).expect("enum derives")
        else {
            panic!("enum must derive a union");
        };
        assert_eq!(
            variants
                .iter()
                .map(|variant| variant.tag.as_str())
                .collect::<Vec<_>>(),
            ["accepted", "addedLater"]
        );
    }

    #[test]
    fn every_public_protocol_type_is_planned_or_exempted() {
        const EXEMPTIONS: &[(&str, &str)] = &[
            (
                "AnimatedMeshAssetError",
                "Rust validation error, not a wire DTO",
            ),
            (
                "AssetReference",
                "catalog-local source shape; TS imports the scene asset reference",
            ),
            (
                "BorderId",
                "protocol-ids codegen descriptor, not a wire DTO",
            ),
            (
                "CollisionResolution",
                "renderer validation detail, not a wire DTO",
            ),
            ("IdRepr", "protocol-ids codegen descriptor, not a wire DTO"),
            (
                "MeshDescriptorError",
                "Rust validation error, not a wire DTO",
            ),
            (
                "SceneObjectCommandRejectionCode",
                "wire values derive from its canonical const table",
            ),
            (
                "SceneValidationCode",
                "wire values derive from its canonical const table",
            ),
            (
                "PrefabDiagnosticCode",
                "wire values derive from its canonical const table",
            ),
            (
                "GameplayInvocationFamily",
                "wire values derive from its canonical const table",
            ),
            (
                "GameplayEventPhase",
                "wire values derive from its canonical const table",
            ),
            (
                "GameplayReadViewKind",
                "wire values derive from its canonical const table",
            ),
            (
                "GameplayReadSelectorCapability",
                "wire values derive from its canonical const table",
            ),
            (
                "GameplayRegistryDiagnosticCode",
                "wire values derive from its canonical const table",
            ),
            (
                "GameplayModuleBindingDiagnosticCode",
                "wire values derive from its canonical const table",
            ),
            (
                "ActivatableCapabilityKind",
                "wire values derive from its canonical const table",
            ),
            (
                "CapabilityActivationAction",
                "wire values derive from its canonical const table",
            ),
            (
                "CapabilityActivationPresence",
                "wire values derive from its canonical const table",
            ),
            (
                "CapabilityActivationDiagnosticCode",
                "wire values derive from its canonical const table",
            ),
            (
                "CapabilityActivationEntityLifecycle",
                "wire values derive from its canonical const table",
            ),
            (
                "TimeControlMode",
                "wire values derive from its canonical const table",
            ),
            (
                "TimeControlRejection",
                "wire values derive from its canonical const table",
            ),
            ("SpriteAtlasError", "Rust validation error, not a wire DTO"),
            ("SpriteError", "Rust validation error, not a wire DTO"),
            ("StaticMeshError", "Rust validation error, not a wire DTO"),
            ("TextureError", "Rust validation error, not a wire DTO"),
            (
                "VoxelConversionCoord",
                "wire contract reuses the shared VoxelCoord shape",
            ),
            (
                "ProjectBundleVoxelCoord",
                "project-bundle fields reuse the shared generated VoxelCoord shape",
            ),
            (
                "ProjectBundleVoxelValue",
                "project-bundle fields reuse the shared generated VoxelValue shape",
            ),
        ];
        let index = SourceIndex::load().expect("Rust source index loads");
        let planned = MODULE_PLANS
            .iter()
            .flat_map(|plan| plan.declarations.split_whitespace())
            .filter_map(|declaration| {
                let parts = declaration.split(':').collect::<Vec<_>>();
                matches!(parts.first(), Some(&"type") | Some(&"brand"))
                    .then(|| parts.get(2).copied().unwrap_or(parts[1]).to_string())
            })
            .collect::<BTreeSet<_>>();
        let exempted = EXEMPTIONS
            .iter()
            .map(|(name, reason)| {
                assert!(
                    !reason.trim().is_empty(),
                    "coverage exemption `{name}` needs a reason"
                );
                (*name).to_string()
            })
            .collect::<BTreeSet<_>>();
        let mut missing = Vec::new();
        for (name, declarations) in &index.declarations {
            for declaration in declarations {
                let path = declaration.path.to_string_lossy();
                if !path.contains("/crates/protocol/protocol-")
                    || path.contains("/protocol-codegen/")
                    || matches!(declaration.declaration, Declaration::Const(_))
                {
                    continue;
                }
                if !planned.contains(name) && !exempted.contains(name) {
                    missing.push(format!("{} ({})", name, declaration.path.display()));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "public protocol types need a generated declaration or a reasoned exemption:\n{}",
            missing.join("\n")
        );
    }
}
