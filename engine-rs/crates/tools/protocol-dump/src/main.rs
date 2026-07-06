//! `protocol-dump` — deterministic inspection for the generated protocol border.
//!
//! Commands:
//!   protocol-dump list
//!   protocol-dump show <module> <item>
//!   protocol-dump show <module/item>
//!   protocol-dump verify-generated
//!   protocol-dump --help
//!
//! Exit codes: 0 = ok, 1 = missing item / generated drift, 2 = usage error.

use std::io::Write;
use std::process::ExitCode;

use protocol_codegen::schema::{Field, Item, Module, TsPrim, TsType, Variant};

const USAGE: &str = "\
protocol-dump — inspect ASHA generated protocol metadata

USAGE:
    protocol-dump list
    protocol-dump show <module> <item>
    protocol-dump show <module/item>
    protocol-dump verify-generated
    protocol-dump --help

COMMANDS:
    list              Print every generated module and exported item in
                      deterministic codegen order.

    show              Print one generated protocol item from the Rust
                      protocol-codegen IR. This does not read committed TS, so
                      it remains anchored to Rust protocol authority.

    verify-generated  Compare protocol-codegen output with committed generated
                      TypeScript contracts. Exits 0 when in sync, 1 on drift.

EXIT CODES:
    0 ok
    1 missing item or generated contract drift
    2 usage error
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&args, &mut std::io::stdout(), &mut std::io::stderr());
    ExitCode::from(code)
}

fn run<O: Write, E: Write>(args: &[String], out: &mut O, err: &mut E) -> u8 {
    match args.first().map(String::as_str) {
        None | Some("--help") | Some("-h") | Some("help") => {
            let _ = write!(out, "{USAGE}");
            if args.is_empty() {
                2
            } else {
                0
            }
        }
        Some("list") => cmd_list(&args[1..], out, err),
        Some("show") => cmd_show(&args[1..], out, err),
        Some("verify-generated") => cmd_verify_generated(&args[1..], out, err),
        Some(other) => {
            let _ = writeln!(err, "error: unknown command '{other}'\n");
            let _ = write!(err, "{USAGE}");
            2
        }
    }
}

fn cmd_list<O: Write, E: Write>(args: &[String], out: &mut O, err: &mut E) -> u8 {
    if !args.is_empty() {
        let _ = writeln!(err, "error: `list` takes no arguments");
        return 2;
    }

    let modules = protocol_codegen::model::all_modules();
    let total_items: usize = modules.iter().map(|module| module.items.len()).sum();
    let _ = writeln!(
        out,
        "protocol-modules: {} modules, {} items",
        modules.len(),
        total_items
    );
    for module in modules {
        let _ = writeln!(
            out,
            "module {} generated={}/{}.ts imports={} items={}",
            module.name,
            protocol_codegen::OUTPUT_DIR,
            module.name,
            module.imports.len(),
            module.items.len()
        );
        for item in &module.items {
            let _ = writeln!(out, "  {} {}", item_kind(item), item_name(item));
        }
    }
    0
}

fn cmd_show<O: Write, E: Write>(args: &[String], out: &mut O, err: &mut E) -> u8 {
    let (module_name, requested_item_name) = match parse_show_target(args) {
        Ok(target) => target,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            return 2;
        }
    };

    let modules = protocol_codegen::model::all_modules();
    let Some(module) = modules.iter().find(|module| module.name == module_name) else {
        let _ = writeln!(err, "missing module: {module_name}");
        return 1;
    };

    let Some(item) = module
        .items
        .iter()
        .find(|item| item_name(item) == requested_item_name)
    else {
        let _ = writeln!(err, "missing item: {}/{}", module.name, requested_item_name);
        return 1;
    };

    write_item(module, item, out);
    0
}

fn cmd_verify_generated<O: Write, E: Write>(args: &[String], out: &mut O, err: &mut E) -> u8 {
    if !args.is_empty() {
        let _ = writeln!(err, "error: `verify-generated` takes no arguments");
        return 2;
    }

    let root = protocol_codegen::repo_root();
    let drifts = protocol_codegen::check_against(&root);
    if drifts.is_empty() {
        let _ = writeln!(
            out,
            "ok: generated contracts match protocol-codegen output under {}",
            protocol_codegen::OUTPUT_DIR
        );
        0
    } else {
        let _ = writeln!(
            err,
            "generated contract drift: {} file(s) differ from protocol-codegen output",
            drifts.len()
        );
        for drift in drifts {
            let _ = writeln!(err, "{}", drift.describe());
        }
        1
    }
}

fn parse_show_target(args: &[String]) -> Result<(&str, &str), &'static str> {
    match args {
        [target] => target
            .split_once('/')
            .filter(|(module, item)| !module.is_empty() && !item.is_empty())
            .ok_or("`show` requires <module/item> or <module> <item>"),
        [module, item] if !module.is_empty() && !item.is_empty() => {
            Ok((module.as_str(), item.as_str()))
        }
        _ => Err("`show` requires <module/item> or <module> <item>"),
    }
}

fn write_item<O: Write>(module: &Module, item: &Item, out: &mut O) {
    let _ = writeln!(out, "module: {}", module.name);
    let _ = writeln!(out, "item: {}", item_name(item));
    let _ = writeln!(out, "kind: {}", item_kind(item));

    match item {
        Item::BrandedId { doc, name } => {
            let _ = writeln!(out, "doc: {doc}");
            let _ = writeln!(out, "ts: export type {name} = branded number");
        }
        Item::Alias { doc, ty, .. } => {
            let _ = writeln!(out, "doc: {doc}");
            let _ = writeln!(out, "type: {}", format_type(ty));
        }
        Item::Interface { doc, fields, .. } => {
            let _ = writeln!(out, "doc: {doc}");
            let _ = writeln!(out, "fields: {}", fields.len());
            for field in fields {
                let _ = writeln!(out, "  {}", format_field(field));
            }
        }
        Item::Union {
            doc,
            discriminant,
            variants,
            ..
        } => {
            let _ = writeln!(out, "doc: {doc}");
            let _ = writeln!(out, "discriminant: {discriminant}");
            let _ = writeln!(out, "variants: {}", variants.len());
            for variant in variants {
                let _ = writeln!(out, "  {}", format_variant(variant));
            }
        }
        Item::Const { doc, value, .. } => {
            let _ = writeln!(out, "doc: {doc}");
            let _ = writeln!(out, "value: {value}");
        }
        Item::ReExport { from } => {
            let _ = writeln!(out, "from: {from}");
        }
    }
}

fn item_name(item: &Item) -> &str {
    match item {
        Item::BrandedId { name, .. }
        | Item::Alias { name, .. }
        | Item::Interface { name, .. }
        | Item::Union { name, .. }
        | Item::Const { name, .. } => name,
        Item::ReExport { from } => from,
    }
}

fn item_kind(item: &Item) -> &'static str {
    match item {
        Item::BrandedId { .. } => "branded-id",
        Item::Alias { .. } => "alias",
        Item::Interface { .. } => "interface",
        Item::Union { .. } => "union",
        Item::Const { .. } => "const",
        Item::ReExport { .. } => "re-export",
    }
}

fn format_field(field: &Field) -> String {
    format!("{}: {}", field.name, format_type(&field.ty))
}

fn format_variant(variant: &Variant) -> String {
    let fields = if variant.fields.is_empty() {
        "no fields".to_string()
    } else {
        variant
            .fields
            .iter()
            .map(format_field)
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!("{} ({fields})", variant.tag)
}

fn format_type(ty: &TsType) -> String {
    match ty {
        TsType::Prim(prim) => match prim {
            TsPrim::Number => "number".to_string(),
            TsPrim::String => "string".to_string(),
            TsPrim::Boolean => "boolean".to_string(),
        },
        TsType::Ref(name) => name.clone(),
        TsType::Array(inner) => format!("readonly {}[]", format_type(inner)),
        TsType::Tuple(items) => {
            let inner = items.iter().map(format_type).collect::<Vec<_>>().join(", ");
            format!("readonly [{inner}]")
        }
        TsType::Nullable(inner) => format!("{} | null", format_type(inner)),
        TsType::StringEnum(values) => values
            .iter()
            .map(|value| format!("'{value}'"))
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_str(args: &[&str]) -> (u8, String, String) {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run(&owned, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn help_and_usage_are_stable() {
        let (code, out, err) = run_str(&["--help"]);
        assert_eq!(code, 0);
        assert!(err.is_empty());
        assert!(out.contains("USAGE:"));
        assert!(out.contains("verify-generated"));

        let (code, _out, err) = run_str(&[]);
        assert_eq!(code, 2);
        assert!(err.is_empty());
    }

    #[test]
    fn list_prints_modules_and_items_in_codegen_order() {
        let (code, out, err) = run_str(&["list"]);
        assert_eq!(code, 0);
        assert!(err.is_empty());
        assert!(out.starts_with("protocol-modules: 15 modules"));
        assert!(out.contains("module ids generated=ts/packages/contracts/src/generated/ids.ts"));
        assert!(out.contains("  branded-id EntityId"));
    }

    #[test]
    fn show_prints_one_item_from_the_codegen_ir() {
        let (code, out, err) = run_str(&["show", "ids", "EntityId"]);
        assert_eq!(code, 0);
        assert!(err.is_empty());
        assert!(out.contains("module: ids"));
        assert!(out.contains("item: EntityId"));
        assert!(out.contains("kind: branded-id"));
    }

    #[test]
    fn show_accepts_slash_target() {
        let (code, out, err) = run_str(&["show", "script/CommandEnvelope"]);
        assert_eq!(code, 0);
        assert!(err.is_empty());
        assert!(out.contains("module: script"));
        assert!(out.contains("item: CommandEnvelope"));
        assert!(out.contains("kind: interface"));
        assert!(out.contains("fields:"));
    }

    #[test]
    fn show_missing_item_exits_one() {
        let (code, out, err) = run_str(&["show", "ids", "NoSuchItem"]);
        assert_eq!(code, 1);
        assert!(out.is_empty());
        assert!(err.contains("missing item: ids/NoSuchItem"));
    }

    #[test]
    fn verify_generated_succeeds_when_contracts_are_current() {
        let (code, out, err) = run_str(&["verify-generated"]);
        assert_eq!(code, 0);
        assert!(err.is_empty());
        assert!(out.contains("ok: generated contracts match"));
    }
}
