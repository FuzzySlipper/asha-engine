//! Phase 3 boundary smoke (Rust side): a command *proposed in TypeScript*
//! reaches Rust validation and is accepted or rejected there.
//!
//! The boundary is fixture-based: the TypeScript policy/host emits a command
//! whose JSON shape is the generated contract (proven equal to these fixtures
//! by `ts/packages/policy-examples/src/boundary.test.ts`). Here the Rust
//! authority core reads the *same* fixture files, decodes them into the
//! authoritative `core_commands::Command`, and runs the real `sim_validator`.
//!
//! There is no parallel validator and no hand-maintained command type: the
//! decoder maps the generated contract discriminants (`domain` / `kind`) onto
//! the authority `Command`, and validation is the production `validate` fn.

use std::path::PathBuf;

use core_commands::{Command, CommandEnvelope, CommandKind};
use core_events::DomainEvent;
use core_ids::{EntityId, ModeId, ProcessId, SignalId, SubjectId, TagId};
use core_state::StateStore;
use sim_validator::{validate, ValidationError};

mod json;
use json::Json;

/// Repo root, derived from this crate's location:
/// `<repo>/engine-rs/crates/sim/sim-validator` → up four components.
fn repo_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("sim-validator is nested four levels under the repo root")
        .to_path_buf()
}

/// Load a shared command fixture and decode it into an authority `Command`.
fn load_command(name: &str) -> Command {
    let path = repo_root()
        .join("harness/fixtures/commands")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()));
    let value = Json::parse(&text).expect("fixture is valid JSON");
    decode_command(&value).expect("fixture decodes to a Command")
}

/// Map a generated-contract command object onto the authority `Command`.
///
/// Mirrors the generated discriminants exactly: outer `domain`, inner `kind`.
fn decode_command(value: &Json) -> Result<Command, String> {
    let domain = value
        .get("domain")
        .and_then(Json::as_str)
        .ok_or("missing domain")?;
    let inner = value.get("command").ok_or("missing command")?;
    let kind = inner
        .get("kind")
        .and_then(Json::as_str)
        .ok_or("missing kind")?;

    let id = || {
        inner
            .get("id")
            .and_then(Json::as_u64)
            .ok_or("missing id".to_string())
    };
    let tag = || {
        inner
            .get("tag")
            .and_then(Json::as_u64)
            .ok_or("missing tag".to_string())
    };
    let mode = || {
        inner
            .get("mode")
            .and_then(Json::as_u64)
            .ok_or("missing mode".to_string())
    };

    use core_commands::{
        EntityCommand, ModeCommand, ProcessCommand, SignalCommand, SubjectCommand, TagCommand,
    };

    let command = match (domain, kind) {
        ("entity", "create") => Command::Entity(EntityCommand::Create {
            id: EntityId::new(id()?),
        }),
        ("entity", "delete") => Command::Entity(EntityCommand::Delete {
            id: EntityId::new(id()?),
        }),
        ("entity", "addTag") => Command::Entity(EntityCommand::AddTag {
            id: EntityId::new(id()?),
            tag: TagId::new(tag()?),
        }),
        ("entity", "removeTag") => Command::Entity(EntityCommand::RemoveTag {
            id: EntityId::new(id()?),
            tag: TagId::new(tag()?),
        }),
        ("subject", "create") => Command::Subject(SubjectCommand::Create {
            id: SubjectId::new(id()?),
        }),
        ("subject", "delete") => Command::Subject(SubjectCommand::Delete {
            id: SubjectId::new(id()?),
        }),
        ("process", "start") => Command::Process(ProcessCommand::Start {
            id: ProcessId::new(id()?),
        }),
        ("process", "setMode") => Command::Process(ProcessCommand::SetMode {
            id: ProcessId::new(id()?),
            mode: ModeId::new(mode()?),
        }),
        ("process", "stop") => Command::Process(ProcessCommand::Stop {
            id: ProcessId::new(id()?),
        }),
        ("mode", "define") => Command::Mode(ModeCommand::Define {
            id: ModeId::new(id()?),
        }),
        ("mode", "undefine") => Command::Mode(ModeCommand::Undefine {
            id: ModeId::new(id()?),
        }),
        ("signal", "define") => Command::Signal(SignalCommand::Define {
            id: SignalId::new(id()?),
        }),
        ("signal", "undefine") => Command::Signal(SignalCommand::Undefine {
            id: SignalId::new(id()?),
        }),
        ("tag", "define") => Command::Tag(TagCommand::Define {
            id: TagId::new(id()?),
        }),
        ("tag", "undefine") => Command::Tag(TagCommand::Undefine {
            id: TagId::new(id()?),
        }),
        (d, k) => return Err(format!("unknown command {d}.{k}")),
    };
    Ok(command)
}

fn input(command: Command) -> CommandEnvelope {
    CommandEnvelope::new(CommandKind::Policy, command)
}

#[test]
fn typescript_proposed_command_is_accepted_by_rust_validation() {
    // The threshold policy proposed `signal.define 1`. Against an abstract state
    // where signal 1 is not yet defined, the authority core accepts it.
    let command = load_command("threshold-accepted");
    let store = StateStore::new();

    let batch = validate(&store, &input(command)).expect("command must be accepted");

    assert_eq!(batch.len(), 1);
    assert!(matches!(
        batch.events()[0],
        DomainEvent::SignalDefined { id } if id == SignalId::new(1)
    ));
}

#[test]
fn typescript_proposed_command_is_rejected_with_structured_reason() {
    // A structurally-valid command authored in TypeScript (delete entity 99)
    // that is stale against an empty state: the authority core rejects it with a
    // structured reason — TypeScript never makes this call.
    let command = load_command("stale-rejected");
    let store = StateStore::new();

    let err = validate(&store, &input(command)).expect_err("stale command must be rejected");

    assert_eq!(
        err,
        ValidationError::EntityNotFound {
            id: EntityId::new(99)
        }
    );
}
