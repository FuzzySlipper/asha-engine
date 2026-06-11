//! Proposed command types for the ASHA authority core.
//!
//! # Lane
//!
//! `rust-state` — may depend on `core-ids`, `core-error`, `core-state`.
//! Must not reference protocol, render, UI, or any TypeScript package.
//!
//! # Design
//!
//! Commands represent *proposed* state changes that have not yet been
//! validated or applied. They are categorised by origin:
//!
//! - [`CommandKind::Input`] — comes from a subject / external input
//! - [`CommandKind::Policy`] — proposed by a constrained policy script
//! - [`CommandKind::System`] — emitted by the sim kernel itself
//!
//! Each variant carries only the abstract fixture vocabulary defined in
//! `core-ids`. No render, telemetry, or protocol concepts appear here.
//! Validated commands produce `DomainEvent`s in `core-events`; the two
//! type hierarchies remain separate.

#![forbid(unsafe_code)]

pub mod voxel;
pub use voxel::VoxelCommand;

use core_ids::{EntityId, ModeId, ProcessId, SignalId, SubjectId, TagId};

// ── Per-noun command enums ────────────────────────────────────────────────────

/// Commands that operate on [`EntityId`] fixtures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityCommand {
    Create { id: EntityId },
    AddTag { id: EntityId, tag: TagId },
    RemoveTag { id: EntityId, tag: TagId },
    Delete { id: EntityId },
}

/// Commands that operate on [`SubjectId`] fixtures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectCommand {
    Create { id: SubjectId },
    Delete { id: SubjectId },
}

/// Commands that operate on [`ProcessId`] fixtures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessCommand {
    Start { id: ProcessId },
    SetMode { id: ProcessId, mode: ModeId },
    Stop { id: ProcessId },
}

/// Commands that define or undefine [`ModeId`] fixtures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeCommand {
    Define { id: ModeId },
    Undefine { id: ModeId },
}

/// Commands that define or undefine [`SignalId`] fixtures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalCommand {
    Define { id: SignalId },
    Undefine { id: SignalId },
}

/// Commands that define or undefine [`TagId`] fixtures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagCommand {
    Define { id: TagId },
    Undefine { id: TagId },
}

// ── Top-level command union ───────────────────────────────────────────────────

/// All Phase 1 command variants, grouped by fixture noun.
///
/// A validator receives a `Command` and produces either an error or a batch
/// of `DomainEvent`s. Keeping the enum explicit (rather than a stringly-typed
/// bus) means the compiler enforces exhaustive handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Entity(EntityCommand),
    Subject(SubjectCommand),
    Process(ProcessCommand),
    Mode(ModeCommand),
    Signal(SignalCommand),
    Tag(TagCommand),
}

// ── Command envelope ──────────────────────────────────────────────────────────

/// Origin category of a proposed command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    /// Comes from an external subject / player input.
    Input,
    /// Proposed by a constrained policy script.
    Policy,
    /// Emitted by the sim kernel itself.
    System,
}

/// A [`Command`] with its origin kind attached.
///
/// The envelope is what the validator receives; it can apply different
/// validation rules depending on [`CommandKind`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandEnvelope {
    pub kind: CommandKind,
    pub command: Command,
}

impl CommandEnvelope {
    pub fn new(kind: CommandKind, command: Command) -> Self {
        Self { kind, command }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core_ids::{EntityId, ModeId, ProcessId, TagId};

    #[test]
    fn command_shape_entity_create() {
        let cmd = Command::Entity(EntityCommand::Create {
            id: EntityId::new(1),
        });
        assert!(matches!(cmd, Command::Entity(EntityCommand::Create { .. })));
    }

    #[test]
    fn command_shape_entity_add_tag() {
        let cmd = Command::Entity(EntityCommand::AddTag {
            id: EntityId::new(2),
            tag: TagId::new(9),
        });
        if let Command::Entity(EntityCommand::AddTag { id, tag }) = cmd {
            assert_eq!(id, EntityId::new(2));
            assert_eq!(tag, TagId::new(9));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn command_shape_process_set_mode() {
        let cmd = Command::Process(ProcessCommand::SetMode {
            id: ProcessId::new(3),
            mode: ModeId::new(7),
        });
        assert!(matches!(
            cmd,
            Command::Process(ProcessCommand::SetMode { .. })
        ));
    }

    #[test]
    fn command_envelope_preserves_kind() {
        let env = CommandEnvelope::new(
            CommandKind::Policy,
            Command::Entity(EntityCommand::Delete {
                id: EntityId::new(42),
            }),
        );
        assert_eq!(env.kind, CommandKind::Policy);
        assert!(matches!(
            env.command,
            Command::Entity(EntityCommand::Delete { .. })
        ));
    }

    #[test]
    fn command_kinds_are_distinct() {
        assert_ne!(CommandKind::Input, CommandKind::Policy);
        assert_ne!(CommandKind::Policy, CommandKind::System);
        assert_ne!(CommandKind::Input, CommandKind::System);
    }
}
