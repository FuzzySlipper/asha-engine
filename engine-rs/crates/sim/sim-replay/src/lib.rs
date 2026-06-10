//! Deterministic text encoding of the replay audit record.
//!
//! # Lane
//!
//! `rust-state` — owns the on-disk *encoding* of the replay record whose
//! *shape* is defined by `protocol-replay`. May depend on `core-ids`,
//! `core-commands`, `core-events`, and `protocol-replay`.
//!
//! # Why a hand-written text format
//!
//! The engine workspace has zero external dependencies, so there is no serde.
//! Replay artifacts are audit bureaucracy: they must be small, deterministic,
//! and reviewable in a diff. A line-oriented text format satisfies all three and
//! is trivial to encode and parse with `std` alone.
//!
//! # Format
//!
//! ```text
//! replay <format_version>
//! init <hash>
//! step <index>
//! cmd <origin> <domain>.<kind> <args...>
//! event <noun>.<verb> <args...>      (zero or more, in order)
//! post <hash>
//! ...                                 (more step blocks)
//! snapshot <step> <hash> <snapshot_version>   (zero or more, after all steps)
//! ```
//!
//! Hashes are fixed-width 16-digit lowercase hex so the column is diff-stable.
//! Proposed commands, accepted events, and hash checkpoints are kept on separate
//! lines and never collapsed into a generic event stream.

#![forbid(unsafe_code)]

use core_commands::{
    Command, CommandEnvelope, CommandKind, EntityCommand, ModeCommand, ProcessCommand,
    SignalCommand, SubjectCommand, TagCommand,
};
use core_events::DomainEvent;
use core_ids::{EntityId, ModeId, ProcessId, SignalId, SubjectId, TagId};

// Re-export the record shapes so downstream sim crates (e.g. `sim-runner`, which
// is not permitted a direct `protocol-replay` dependency) can build records
// through `sim-replay`.
pub use protocol_replay::{
    ReplayHash, ReplayRecord, ReplayStep, SnapshotMeta, StepIndex, StepOutcome,
    REPLAY_FORMAT_VERSION,
};

// ── Checkpoint policy ─────────────────────────────────────────────────────────

/// How often a replay recorder captures a state-hash checkpoint
/// ([`SnapshotMeta`]) as it drives a store forward.
///
/// Every step already carries a `post_hash`; checkpoints are the coarser markers
/// a divergence report or resume uses. The record's `initial_hash` is always
/// present, and a recorder always captures a final checkpoint for the last step,
/// regardless of interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointInterval {
    /// Only the final step (plus the always-present initial hash).
    FinalOnly,
    /// After every step.
    EveryStep,
    /// After every `n` steps (and the final step). `n` is treated as at least 1.
    EverySteps(u64),
}

impl CheckpointInterval {
    /// Whether a mid-run checkpoint should be captured after the step at the
    /// zero-based `index`. The final-step checkpoint is handled separately by the
    /// recorder, so this returns `false` for [`CheckpointInterval::FinalOnly`].
    pub fn captures_after(&self, index: u64) -> bool {
        match self {
            CheckpointInterval::FinalOnly => false,
            CheckpointInterval::EveryStep => true,
            CheckpointInterval::EverySteps(n) => (index + 1).is_multiple_of((*n).max(1)),
        }
    }
}

// ── Encoding ──────────────────────────────────────────────────────────────────

/// Encode a replay record to its deterministic text form (with trailing newline).
pub fn encode(record: &ReplayRecord) -> String {
    let mut out = String::new();
    out.push_str(&format!("replay {}\n", record.format_version));
    out.push_str(&format!("init {}\n", encode_hash(record.initial_hash)));

    for step in &record.steps {
        out.push_str(&format!("step {}\n", step.index.raw()));
        out.push_str(&format!("cmd {}\n", encode_command(&step.command)));
        match &step.outcome {
            StepOutcome::Accepted { events } => {
                for event in events {
                    out.push_str(&format!("event {}\n", encode_event(event)));
                }
            }
            StepOutcome::Rejected { summary } => {
                out.push_str(&format!("reject {summary}\n"));
            }
        }
        out.push_str(&format!("post {}\n", encode_hash(step.post_hash)));
    }

    for snap in &record.snapshots {
        out.push_str(&format!(
            "snapshot {} {} {}\n",
            snap.step.raw(),
            encode_hash(snap.hash),
            snap.snapshot_version
        ));
    }

    out
}

fn encode_hash(hash: ReplayHash) -> String {
    format!("{:016x}", hash.raw())
}

fn encode_command(env: &CommandEnvelope) -> String {
    let origin = match env.kind {
        CommandKind::Input => "input",
        CommandKind::Policy => "policy",
        CommandKind::System => "system",
    };
    let body = match &env.command {
        Command::Entity(c) => match c {
            EntityCommand::Create { id } => format!("entity.create {}", id.raw()),
            EntityCommand::Delete { id } => format!("entity.delete {}", id.raw()),
            EntityCommand::AddTag { id, tag } => {
                format!("entity.addTag {} {}", id.raw(), tag.raw())
            }
            EntityCommand::RemoveTag { id, tag } => {
                format!("entity.removeTag {} {}", id.raw(), tag.raw())
            }
        },
        Command::Subject(c) => match c {
            SubjectCommand::Create { id } => format!("subject.create {}", id.raw()),
            SubjectCommand::Delete { id } => format!("subject.delete {}", id.raw()),
        },
        Command::Process(c) => match c {
            ProcessCommand::Start { id } => format!("process.start {}", id.raw()),
            ProcessCommand::SetMode { id, mode } => {
                format!("process.setMode {} {}", id.raw(), mode.raw())
            }
            ProcessCommand::Stop { id } => format!("process.stop {}", id.raw()),
        },
        Command::Mode(c) => match c {
            ModeCommand::Define { id } => format!("mode.define {}", id.raw()),
            ModeCommand::Undefine { id } => format!("mode.undefine {}", id.raw()),
        },
        Command::Signal(c) => match c {
            SignalCommand::Define { id } => format!("signal.define {}", id.raw()),
            SignalCommand::Undefine { id } => format!("signal.undefine {}", id.raw()),
        },
        Command::Tag(c) => match c {
            TagCommand::Define { id } => format!("tag.define {}", id.raw()),
            TagCommand::Undefine { id } => format!("tag.undefine {}", id.raw()),
        },
    };
    format!("{origin} {body}")
}

fn encode_event(event: &DomainEvent) -> String {
    match event {
        DomainEvent::EntityCreated { id } => format!("entity.created {}", id.raw()),
        DomainEvent::EntityTagAdded { id, tag } => {
            format!("entity.tagAdded {} {}", id.raw(), tag.raw())
        }
        DomainEvent::EntityTagRemoved { id, tag } => {
            format!("entity.tagRemoved {} {}", id.raw(), tag.raw())
        }
        DomainEvent::EntityDeleted { id } => format!("entity.deleted {}", id.raw()),
        DomainEvent::SubjectCreated { id } => format!("subject.created {}", id.raw()),
        DomainEvent::SubjectDeleted { id } => format!("subject.deleted {}", id.raw()),
        DomainEvent::ProcessStarted { id } => format!("process.started {}", id.raw()),
        DomainEvent::ProcessModeSet { id, mode } => {
            format!("process.modeSet {} {}", id.raw(), mode.raw())
        }
        DomainEvent::ProcessStopped { id } => format!("process.stopped {}", id.raw()),
        DomainEvent::ModeDefined { id } => format!("mode.defined {}", id.raw()),
        DomainEvent::ModeUndefined { id } => format!("mode.undefined {}", id.raw()),
        DomainEvent::SignalDefined { id } => format!("signal.defined {}", id.raw()),
        DomainEvent::SignalUndefined { id } => format!("signal.undefined {}", id.raw()),
        DomainEvent::TagDefined { id } => format!("tag.defined {}", id.raw()),
        DomainEvent::TagUndefined { id } => format!("tag.undefined {}", id.raw()),
    }
}

// ── Decoding ──────────────────────────────────────────────────────────────────

/// Why a replay record failed to decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// A required header line was missing or malformed.
    BadHeader { line: usize, detail: String },
    /// A line's leading keyword was not recognized in context.
    UnexpectedLine { line: usize, content: String },
    /// A token could not be parsed (hash, id, or version).
    BadToken { line: usize, detail: String },
    /// A command or event tag was not recognized.
    UnknownVariant { line: usize, tag: String },
    /// A step block was malformed (e.g. missing `post`).
    IncompleteStep { line: usize },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::BadHeader { line, detail } => {
                write!(f, "line {line}: bad header: {detail}")
            }
            DecodeError::UnexpectedLine { line, content } => {
                write!(f, "line {line}: unexpected line: {content:?}")
            }
            DecodeError::BadToken { line, detail } => {
                write!(f, "line {line}: bad token: {detail}")
            }
            DecodeError::UnknownVariant { line, tag } => {
                write!(f, "line {line}: unknown variant {tag:?}")
            }
            DecodeError::IncompleteStep { line } => {
                write!(f, "line {line}: incomplete step block")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

/// Decode a replay record from its text form. Round-trips with [`encode`].
pub fn decode(text: &str) -> Result<ReplayRecord, DecodeError> {
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0usize;

    let format_version = parse_header_u32(&lines, &mut i, "replay")?;
    let initial_hash = parse_header_hash(&lines, &mut i, "init")?;

    let mut steps = Vec::new();
    let mut snapshots = Vec::new();

    while i < lines.len() {
        let line = lines[i];
        if line.is_empty() {
            i += 1;
            continue;
        }
        match first_word(line) {
            "step" => steps.push(parse_step(&lines, &mut i)?),
            "snapshot" => {
                snapshots.push(parse_snapshot(line, i)?);
                i += 1;
            }
            _ => {
                return Err(DecodeError::UnexpectedLine {
                    line: i + 1,
                    content: line.to_string(),
                })
            }
        }
    }

    Ok(ReplayRecord {
        format_version,
        initial_hash,
        steps,
        snapshots,
    })
}

fn parse_step(lines: &[&str], i: &mut usize) -> Result<ReplayStep, DecodeError> {
    let step_line = lines[*i];
    let index = StepIndex::new(parse_tail_u64(step_line, "step", *i + 1)?);
    *i += 1;

    let cmd_line = lines
        .get(*i)
        .ok_or(DecodeError::IncompleteStep { line: *i })?;
    let command = decode_command(strip_keyword(cmd_line, "cmd", *i + 1)?, *i + 1)?;
    *i += 1;

    // Outcome: a single `reject` line, or zero or more `event` lines.
    let outcome_line = lines
        .get(*i)
        .ok_or(DecodeError::IncompleteStep { line: *i })?;
    let outcome = if first_word(outcome_line) == "reject" {
        let summary = strip_keyword(outcome_line, "reject", *i + 1)?.to_string();
        *i += 1;
        StepOutcome::Rejected { summary }
    } else {
        let mut events = Vec::new();
        while let Some(line) = lines.get(*i) {
            if first_word(line) != "event" {
                break;
            }
            events.push(decode_event(strip_keyword(line, "event", *i + 1)?, *i + 1)?);
            *i += 1;
        }
        StepOutcome::Accepted { events }
    };

    let post_line = lines
        .get(*i)
        .ok_or(DecodeError::IncompleteStep { line: *i })?;
    let post_hash = ReplayHash::new(parse_hash(
        strip_keyword(post_line, "post", *i + 1)?,
        *i + 1,
    )?);
    *i += 1;

    Ok(ReplayStep {
        index,
        command,
        outcome,
        post_hash,
    })
}

fn parse_snapshot(line: &str, idx: usize) -> Result<SnapshotMeta, DecodeError> {
    let mut it = line.split_whitespace();
    it.next(); // "snapshot"
    let step = StepIndex::new(next_u64(&mut it, idx + 1)?);
    let hash = ReplayHash::new(next_hash(&mut it, idx + 1)?);
    let snapshot_version = next_u32(&mut it, idx + 1)?;
    Ok(SnapshotMeta {
        step,
        hash,
        snapshot_version,
    })
}

fn decode_command(body: &str, line: usize) -> Result<CommandEnvelope, DecodeError> {
    let mut it = body.split_whitespace();
    let origin = it.next().ok_or(DecodeError::BadToken {
        line,
        detail: "missing command origin".to_string(),
    })?;
    let kind = match origin {
        "input" => CommandKind::Input,
        "policy" => CommandKind::Policy,
        "system" => CommandKind::System,
        other => {
            return Err(DecodeError::BadToken {
                line,
                detail: format!("unknown origin {other:?}"),
            })
        }
    };
    let tag = it.next().ok_or(DecodeError::BadToken {
        line,
        detail: "missing command tag".to_string(),
    })?;
    let args = collect_u64(it, line)?;
    let a = |n: usize| arg(&args, n, line);

    let command = match tag {
        "entity.create" => Command::Entity(EntityCommand::Create {
            id: EntityId::new(a(0)?),
        }),
        "entity.delete" => Command::Entity(EntityCommand::Delete {
            id: EntityId::new(a(0)?),
        }),
        "entity.addTag" => Command::Entity(EntityCommand::AddTag {
            id: EntityId::new(a(0)?),
            tag: TagId::new(a(1)?),
        }),
        "entity.removeTag" => Command::Entity(EntityCommand::RemoveTag {
            id: EntityId::new(a(0)?),
            tag: TagId::new(a(1)?),
        }),
        "subject.create" => Command::Subject(SubjectCommand::Create {
            id: SubjectId::new(a(0)?),
        }),
        "subject.delete" => Command::Subject(SubjectCommand::Delete {
            id: SubjectId::new(a(0)?),
        }),
        "process.start" => Command::Process(ProcessCommand::Start {
            id: ProcessId::new(a(0)?),
        }),
        "process.setMode" => Command::Process(ProcessCommand::SetMode {
            id: ProcessId::new(a(0)?),
            mode: ModeId::new(a(1)?),
        }),
        "process.stop" => Command::Process(ProcessCommand::Stop {
            id: ProcessId::new(a(0)?),
        }),
        "mode.define" => Command::Mode(ModeCommand::Define {
            id: ModeId::new(a(0)?),
        }),
        "mode.undefine" => Command::Mode(ModeCommand::Undefine {
            id: ModeId::new(a(0)?),
        }),
        "signal.define" => Command::Signal(SignalCommand::Define {
            id: SignalId::new(a(0)?),
        }),
        "signal.undefine" => Command::Signal(SignalCommand::Undefine {
            id: SignalId::new(a(0)?),
        }),
        "tag.define" => Command::Tag(TagCommand::Define {
            id: TagId::new(a(0)?),
        }),
        "tag.undefine" => Command::Tag(TagCommand::Undefine {
            id: TagId::new(a(0)?),
        }),
        other => {
            return Err(DecodeError::UnknownVariant {
                line,
                tag: other.to_string(),
            })
        }
    };
    Ok(CommandEnvelope::new(kind, command))
}

fn decode_event(body: &str, line: usize) -> Result<DomainEvent, DecodeError> {
    let mut it = body.split_whitespace();
    let tag = it.next().ok_or(DecodeError::BadToken {
        line,
        detail: "missing event tag".to_string(),
    })?;
    let args = collect_u64(it, line)?;
    let a = |n: usize| arg(&args, n, line);

    let event = match tag {
        "entity.created" => DomainEvent::EntityCreated {
            id: EntityId::new(a(0)?),
        },
        "entity.tagAdded" => DomainEvent::EntityTagAdded {
            id: EntityId::new(a(0)?),
            tag: TagId::new(a(1)?),
        },
        "entity.tagRemoved" => DomainEvent::EntityTagRemoved {
            id: EntityId::new(a(0)?),
            tag: TagId::new(a(1)?),
        },
        "entity.deleted" => DomainEvent::EntityDeleted {
            id: EntityId::new(a(0)?),
        },
        "subject.created" => DomainEvent::SubjectCreated {
            id: SubjectId::new(a(0)?),
        },
        "subject.deleted" => DomainEvent::SubjectDeleted {
            id: SubjectId::new(a(0)?),
        },
        "process.started" => DomainEvent::ProcessStarted {
            id: ProcessId::new(a(0)?),
        },
        "process.modeSet" => DomainEvent::ProcessModeSet {
            id: ProcessId::new(a(0)?),
            mode: ModeId::new(a(1)?),
        },
        "process.stopped" => DomainEvent::ProcessStopped {
            id: ProcessId::new(a(0)?),
        },
        "mode.defined" => DomainEvent::ModeDefined {
            id: ModeId::new(a(0)?),
        },
        "mode.undefined" => DomainEvent::ModeUndefined {
            id: ModeId::new(a(0)?),
        },
        "signal.defined" => DomainEvent::SignalDefined {
            id: SignalId::new(a(0)?),
        },
        "signal.undefined" => DomainEvent::SignalUndefined {
            id: SignalId::new(a(0)?),
        },
        "tag.defined" => DomainEvent::TagDefined {
            id: TagId::new(a(0)?),
        },
        "tag.undefined" => DomainEvent::TagUndefined {
            id: TagId::new(a(0)?),
        },
        other => {
            return Err(DecodeError::UnknownVariant {
                line,
                tag: other.to_string(),
            })
        }
    };
    Ok(event)
}

// ── Parsing helpers ───────────────────────────────────────────────────────────

fn first_word(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}

fn strip_keyword<'a>(line: &'a str, keyword: &str, idx: usize) -> Result<&'a str, DecodeError> {
    line.strip_prefix(keyword)
        .and_then(|rest| rest.strip_prefix(' '))
        .ok_or_else(|| DecodeError::UnexpectedLine {
            line: idx,
            content: line.to_string(),
        })
}

fn parse_header_u32(lines: &[&str], i: &mut usize, keyword: &str) -> Result<u32, DecodeError> {
    let line = lines.get(*i).ok_or(DecodeError::BadHeader {
        line: *i + 1,
        detail: format!("missing {keyword} line"),
    })?;
    let value = parse_tail_u64(line, keyword, *i + 1)?;
    *i += 1;
    u32::try_from(value).map_err(|_| DecodeError::BadToken {
        line: *i,
        detail: format!("{keyword} value out of range"),
    })
}

fn parse_header_hash(
    lines: &[&str],
    i: &mut usize,
    keyword: &str,
) -> Result<ReplayHash, DecodeError> {
    let line = lines.get(*i).ok_or(DecodeError::BadHeader {
        line: *i + 1,
        detail: format!("missing {keyword} line"),
    })?;
    let hash = ReplayHash::new(parse_hash(strip_keyword(line, keyword, *i + 1)?, *i + 1)?);
    *i += 1;
    Ok(hash)
}

fn parse_tail_u64(line: &str, keyword: &str, idx: usize) -> Result<u64, DecodeError> {
    let rest = strip_keyword(line, keyword, idx)?;
    rest.trim()
        .parse::<u64>()
        .map_err(|e| DecodeError::BadToken {
            line: idx,
            detail: format!("{keyword}: {e}"),
        })
}

fn parse_hash(token: &str, idx: usize) -> Result<u64, DecodeError> {
    u64::from_str_radix(token.trim(), 16).map_err(|e| DecodeError::BadToken {
        line: idx,
        detail: format!("hash {token:?}: {e}"),
    })
}

fn collect_u64<'a>(
    it: impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<Vec<u64>, DecodeError> {
    it.map(|t| {
        t.parse::<u64>().map_err(|e| DecodeError::BadToken {
            line,
            detail: format!("id {t:?}: {e}"),
        })
    })
    .collect()
}

fn arg(args: &[u64], n: usize, line: usize) -> Result<u64, DecodeError> {
    args.get(n).copied().ok_or(DecodeError::BadToken {
        line,
        detail: format!("missing argument {n}"),
    })
}

fn next_u64<'a>(it: &mut impl Iterator<Item = &'a str>, line: usize) -> Result<u64, DecodeError> {
    it.next()
        .ok_or(DecodeError::BadToken {
            line,
            detail: "missing integer".to_string(),
        })?
        .parse::<u64>()
        .map_err(|e| DecodeError::BadToken {
            line,
            detail: e.to_string(),
        })
}

fn next_u32<'a>(it: &mut impl Iterator<Item = &'a str>, line: usize) -> Result<u32, DecodeError> {
    let v = next_u64(it, line)?;
    u32::try_from(v).map_err(|_| DecodeError::BadToken {
        line,
        detail: "value out of u32 range".to_string(),
    })
}

fn next_hash<'a>(it: &mut impl Iterator<Item = &'a str>, line: usize) -> Result<u64, DecodeError> {
    let token = it.next().ok_or(DecodeError::BadToken {
        line,
        detail: "missing hash".to_string(),
    })?;
    parse_hash(token, line)
}

// ── Divergence report ─────────────────────────────────────────────────────────

/// The kind of divergence found between an expected (golden) replay and an
/// actual one, used to route a repair agent to the likely responsible crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivergenceClass {
    /// The proposed command at a step differs.
    CommandMismatch,
    /// An accepted command produced a different event sequence (or a step that
    /// was rejected is now accepted).
    AcceptedEventMismatch,
    /// A command's rejection differs (now rejected when it was accepted, or a
    /// different rejection summary).
    RejectionMismatch,
    /// A state hash (per-step post hash, initial hash, or checkpoint) differs.
    HashCheckpointMismatch,
    /// The two records have different shape (step or checkpoint counts).
    StructuralMismatch,
    /// The replay artifact could not be parsed.
    MalformedArtifact,
}

impl DivergenceClass {
    /// A short, stable label for logs.
    pub fn label(&self) -> &'static str {
        match self {
            DivergenceClass::CommandMismatch => "command-mismatch",
            DivergenceClass::AcceptedEventMismatch => "accepted-event-mismatch",
            DivergenceClass::RejectionMismatch => "rejection-mismatch",
            DivergenceClass::HashCheckpointMismatch => "hash-checkpoint-mismatch",
            DivergenceClass::StructuralMismatch => "structural-mismatch",
            DivergenceClass::MalformedArtifact => "malformed-artifact",
        }
    }

    /// The crate(s)/lane a repair agent should look at first for this class.
    pub fn likely_owner(&self) -> &'static str {
        match self {
            DivergenceClass::CommandMismatch => {
                "rust-state · core-commands (command shape) or sim-replay (its encoding)"
            }
            DivergenceClass::AcceptedEventMismatch => {
                "rust-state · sim-validator + sim-applier + core-events (the validate→apply path)"
            }
            DivergenceClass::RejectionMismatch => {
                "rust-state · sim-validator (the accept/reject decision or its reason)"
            }
            DivergenceClass::HashCheckpointMismatch => {
                "rust-state · core-snapshot (state hashing) or an upstream state change"
            }
            DivergenceClass::StructuralMismatch => {
                "rust-state · sim-runner (recording) or sim-replay (record assembly)"
            }
            DivergenceClass::MalformedArtifact => {
                "contract-steward/rust-state · sim-replay (encoder/decoder) or a corrupted artifact"
            }
        }
    }
}

/// A single, first-encountered divergence between two replay records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    pub class: DivergenceClass,
    /// The step index where divergence was found, or `None` for whole-record
    /// issues (initial hash, malformed artifact, trailing structural mismatch).
    pub step: Option<u64>,
    pub expected: String,
    pub actual: String,
}

impl Divergence {
    fn at(class: DivergenceClass, step: Option<u64>, expected: String, actual: String) -> Self {
        Self {
            class,
            step,
            expected,
            actual,
        }
    }

    /// Build a malformed-artifact divergence from a decode failure.
    pub fn malformed(error: &DecodeError) -> Self {
        Self {
            class: DivergenceClass::MalformedArtifact,
            step: None,
            expected: "a well-formed replay artifact".to_string(),
            actual: error.to_string(),
        }
    }

    /// A deterministic, multi-line report suitable for CI logs, naming the
    /// replay and routing the repair to a likely owner. Never a bare
    /// "replay failed".
    pub fn report(&self, replay_name: &str) -> String {
        let step = self
            .step
            .map(|s| s.to_string())
            .unwrap_or_else(|| "-".to_string());
        format!(
            "replay divergence: {replay_name}\n  \
             class:    {}\n  \
             step:     {step}\n  \
             expected: {}\n  \
             actual:   {}\n  \
             likely:   {}",
            self.class.label(),
            self.expected,
            self.actual,
            self.class.likely_owner(),
        )
    }
}

impl std::fmt::Display for Divergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.report("<replay>"))
    }
}

/// Compare an `expected` (golden) record against an `actual` one and return the
/// first divergence, or `None` if they match. The comparison order is stable so
/// the reported divergence is deterministic.
pub fn diff(expected: &ReplayRecord, actual: &ReplayRecord) -> Option<Divergence> {
    if expected.initial_hash != actual.initial_hash {
        return Some(Divergence::at(
            DivergenceClass::HashCheckpointMismatch,
            None,
            format!("initial hash {}", encode_hash(expected.initial_hash)),
            format!("initial hash {}", encode_hash(actual.initial_hash)),
        ));
    }

    for (i, exp) in expected.steps.iter().enumerate() {
        let idx = exp.index.raw();
        let Some(act) = actual.steps.get(i) else {
            return Some(Divergence::at(
                DivergenceClass::StructuralMismatch,
                Some(idx),
                format!("a step at index {idx}"),
                "no step (record is shorter than golden)".to_string(),
            ));
        };

        if exp.command != act.command {
            return Some(Divergence::at(
                DivergenceClass::CommandMismatch,
                Some(idx),
                encode_command(&exp.command),
                encode_command(&act.command),
            ));
        }

        if let Some(div) = diff_outcome(idx, &exp.outcome, &act.outcome) {
            return Some(div);
        }

        if exp.post_hash != act.post_hash {
            return Some(Divergence::at(
                DivergenceClass::HashCheckpointMismatch,
                Some(idx),
                format!("post hash {}", encode_hash(exp.post_hash)),
                format!("post hash {}", encode_hash(act.post_hash)),
            ));
        }
    }

    if actual.steps.len() > expected.steps.len() {
        let idx = actual.steps[expected.steps.len()].index.raw();
        return Some(Divergence::at(
            DivergenceClass::StructuralMismatch,
            Some(idx),
            "no further steps".to_string(),
            format!("an extra step at index {idx}"),
        ));
    }

    diff_snapshots(expected, actual)
}

fn diff_outcome(idx: u64, exp: &StepOutcome, act: &StepOutcome) -> Option<Divergence> {
    match (exp, act) {
        (StepOutcome::Accepted { events: e }, StepOutcome::Accepted { events: a }) if e != a => {
            Some(Divergence::at(
                DivergenceClass::AcceptedEventMismatch,
                Some(idx),
                render_events(e),
                render_events(a),
            ))
        }
        (StepOutcome::Rejected { summary: e }, StepOutcome::Rejected { summary: a }) if e != a => {
            Some(Divergence::at(
                DivergenceClass::RejectionMismatch,
                Some(idx),
                format!("rejected: {e}"),
                format!("rejected: {a}"),
            ))
        }
        // Accept/reject flip: classify by what the *actual* now is.
        (StepOutcome::Accepted { events: e }, StepOutcome::Rejected { summary: a }) => {
            Some(Divergence::at(
                DivergenceClass::RejectionMismatch,
                Some(idx),
                format!("accepted: {}", render_events(e)),
                format!("rejected: {a}"),
            ))
        }
        (StepOutcome::Rejected { summary: e }, StepOutcome::Accepted { events: a }) => {
            Some(Divergence::at(
                DivergenceClass::AcceptedEventMismatch,
                Some(idx),
                format!("rejected: {e}"),
                format!("accepted: {}", render_events(a)),
            ))
        }
        _ => None,
    }
}

fn diff_snapshots(expected: &ReplayRecord, actual: &ReplayRecord) -> Option<Divergence> {
    if expected.snapshots.len() != actual.snapshots.len() {
        return Some(Divergence::at(
            DivergenceClass::StructuralMismatch,
            None,
            format!("{} checkpoint(s)", expected.snapshots.len()),
            format!("{} checkpoint(s)", actual.snapshots.len()),
        ));
    }
    for (exp, act) in expected.snapshots.iter().zip(&actual.snapshots) {
        if exp.step != act.step || exp.hash != act.hash {
            return Some(Divergence::at(
                DivergenceClass::HashCheckpointMismatch,
                Some(exp.step.raw()),
                format!(
                    "checkpoint step {} hash {}",
                    exp.step.raw(),
                    encode_hash(exp.hash)
                ),
                format!(
                    "checkpoint step {} hash {}",
                    act.step.raw(),
                    encode_hash(act.hash)
                ),
            ));
        }
    }
    None
}

fn render_events(events: &[DomainEvent]) -> String {
    if events.is_empty() {
        return "(no events)".to_string();
    }
    events
        .iter()
        .map(encode_event)
        .collect::<Vec<_>>()
        .join("; ")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record() -> ReplayRecord {
        let mut record = ReplayRecord::new(ReplayHash::new(0xabc));
        record.steps.push(ReplayStep {
            index: StepIndex::new(0),
            command: CommandEnvelope::new(
                CommandKind::Input,
                Command::Entity(EntityCommand::Create {
                    id: EntityId::new(5),
                }),
            ),
            outcome: StepOutcome::Accepted {
                events: vec![DomainEvent::EntityCreated {
                    id: EntityId::new(5),
                }],
            },
            post_hash: ReplayHash::new(0x11),
        });
        record.steps.push(ReplayStep {
            index: StepIndex::new(1),
            command: CommandEnvelope::new(
                CommandKind::Policy,
                Command::Signal(SignalCommand::Define {
                    id: SignalId::new(1),
                }),
            ),
            outcome: StepOutcome::Accepted {
                events: vec![DomainEvent::SignalDefined {
                    id: SignalId::new(1),
                }],
            },
            post_hash: ReplayHash::new(0x22),
        });
        record.snapshots.push(SnapshotMeta {
            step: StepIndex::new(1),
            hash: ReplayHash::new(0x22),
            snapshot_version: 1,
        });
        record
    }

    const SAMPLE_TEXT: &str = "\
replay 1
init 0000000000000abc
step 0
cmd input entity.create 5
event entity.created 5
post 0000000000000011
step 1
cmd policy signal.define 1
event signal.defined 1
post 0000000000000022
snapshot 1 0000000000000022 1
";

    #[test]
    fn encode_matches_expected_text() {
        assert_eq!(encode(&sample_record()), SAMPLE_TEXT);
    }

    #[test]
    fn decode_matches_expected_record() {
        assert_eq!(decode(SAMPLE_TEXT).unwrap(), sample_record());
    }

    #[test]
    fn round_trip_is_stable() {
        let record = sample_record();
        let text = encode(&record);
        let decoded = decode(&text).unwrap();
        assert_eq!(decoded, record);
        assert_eq!(encode(&decoded), text);
    }

    #[test]
    fn round_trip_covers_every_command_and_event_variant() {
        let mut record = ReplayRecord::new(ReplayHash::new(0));
        let cmds = [
            Command::Entity(EntityCommand::Create {
                id: EntityId::new(1),
            }),
            Command::Entity(EntityCommand::AddTag {
                id: EntityId::new(1),
                tag: TagId::new(2),
            }),
            Command::Entity(EntityCommand::RemoveTag {
                id: EntityId::new(1),
                tag: TagId::new(2),
            }),
            Command::Entity(EntityCommand::Delete {
                id: EntityId::new(1),
            }),
            Command::Subject(SubjectCommand::Create {
                id: SubjectId::new(3),
            }),
            Command::Subject(SubjectCommand::Delete {
                id: SubjectId::new(3),
            }),
            Command::Process(ProcessCommand::Start {
                id: ProcessId::new(4),
            }),
            Command::Process(ProcessCommand::SetMode {
                id: ProcessId::new(4),
                mode: ModeId::new(5),
            }),
            Command::Process(ProcessCommand::Stop {
                id: ProcessId::new(4),
            }),
            Command::Mode(ModeCommand::Define { id: ModeId::new(5) }),
            Command::Mode(ModeCommand::Undefine { id: ModeId::new(5) }),
            Command::Signal(SignalCommand::Define {
                id: SignalId::new(6),
            }),
            Command::Signal(SignalCommand::Undefine {
                id: SignalId::new(6),
            }),
            Command::Tag(TagCommand::Define { id: TagId::new(7) }),
            Command::Tag(TagCommand::Undefine { id: TagId::new(7) }),
        ];
        let events = [
            DomainEvent::EntityCreated {
                id: EntityId::new(1),
            },
            DomainEvent::EntityTagAdded {
                id: EntityId::new(1),
                tag: TagId::new(2),
            },
            DomainEvent::EntityTagRemoved {
                id: EntityId::new(1),
                tag: TagId::new(2),
            },
            DomainEvent::EntityDeleted {
                id: EntityId::new(1),
            },
            DomainEvent::SubjectCreated {
                id: SubjectId::new(3),
            },
            DomainEvent::SubjectDeleted {
                id: SubjectId::new(3),
            },
            DomainEvent::ProcessStarted {
                id: ProcessId::new(4),
            },
            DomainEvent::ProcessModeSet {
                id: ProcessId::new(4),
                mode: ModeId::new(5),
            },
            DomainEvent::ProcessStopped {
                id: ProcessId::new(4),
            },
            DomainEvent::ModeDefined { id: ModeId::new(5) },
            DomainEvent::ModeUndefined { id: ModeId::new(5) },
            DomainEvent::SignalDefined {
                id: SignalId::new(6),
            },
            DomainEvent::SignalUndefined {
                id: SignalId::new(6),
            },
            DomainEvent::TagDefined { id: TagId::new(7) },
            DomainEvent::TagUndefined { id: TagId::new(7) },
        ];
        for (n, cmd) in cmds.iter().enumerate() {
            record.steps.push(ReplayStep {
                index: StepIndex::new(n as u64),
                command: CommandEnvelope::new(CommandKind::System, cmd.clone()),
                outcome: StepOutcome::Accepted {
                    events: events.to_vec(),
                },
                post_hash: ReplayHash::new(n as u64),
            });
        }

        let decoded = decode(&encode(&record)).unwrap();
        assert_eq!(decoded, record);
    }

    #[test]
    fn rejected_step_round_trips_without_events() {
        let mut record = ReplayRecord::new(ReplayHash::new(0x1));
        record.steps.push(ReplayStep {
            index: StepIndex::new(0),
            command: CommandEnvelope::new(
                CommandKind::Policy,
                Command::Entity(EntityCommand::Delete {
                    id: EntityId::new(99),
                }),
            ),
            outcome: StepOutcome::Rejected {
                summary: "EntityNotFound { id: EntityId(99) }".to_string(),
            },
            // Rejected: state unchanged, so post hash equals the prior hash.
            post_hash: ReplayHash::new(0x1),
        });

        let text = encode(&record);
        assert!(text.contains("reject EntityNotFound { id: EntityId(99) }\n"));
        assert!(!text.contains("event "));
        assert_eq!(decode(&text).unwrap(), record);
    }

    // ── Divergence diff ───────────────────────────────────────────────────

    fn accepted_step(idx: u64, cmd: Command, ev: DomainEvent, hash: u64) -> ReplayStep {
        ReplayStep {
            index: StepIndex::new(idx),
            command: CommandEnvelope::new(CommandKind::System, cmd),
            outcome: StepOutcome::Accepted { events: vec![ev] },
            post_hash: ReplayHash::new(hash),
        }
    }

    fn one_step_record(step: ReplayStep) -> ReplayRecord {
        let mut r = ReplayRecord::new(ReplayHash::new(0));
        r.steps.push(step);
        r
    }

    #[test]
    fn diff_identical_records_is_none() {
        let r = sample_record();
        assert_eq!(diff(&r, &r), None);
    }

    #[test]
    fn diff_classifies_command_mismatch() {
        let a = one_step_record(accepted_step(
            0,
            Command::Entity(EntityCommand::Create {
                id: EntityId::new(1),
            }),
            DomainEvent::EntityCreated {
                id: EntityId::new(1),
            },
            10,
        ));
        let b = one_step_record(accepted_step(
            0,
            Command::Entity(EntityCommand::Create {
                id: EntityId::new(2),
            }),
            DomainEvent::EntityCreated {
                id: EntityId::new(1),
            },
            10,
        ));
        let d = diff(&a, &b).unwrap();
        assert_eq!(d.class, DivergenceClass::CommandMismatch);
        assert_eq!(d.step, Some(0));
    }

    #[test]
    fn diff_classifies_accepted_event_mismatch() {
        let a = one_step_record(accepted_step(
            0,
            Command::Entity(EntityCommand::Create {
                id: EntityId::new(1),
            }),
            DomainEvent::EntityCreated {
                id: EntityId::new(1),
            },
            10,
        ));
        let b = one_step_record(accepted_step(
            0,
            Command::Entity(EntityCommand::Create {
                id: EntityId::new(1),
            }),
            DomainEvent::EntityCreated {
                id: EntityId::new(2),
            },
            10,
        ));
        assert_eq!(
            diff(&a, &b).unwrap().class,
            DivergenceClass::AcceptedEventMismatch
        );
    }

    #[test]
    fn diff_classifies_rejection_and_flip() {
        let mut rejected = ReplayRecord::new(ReplayHash::new(0));
        rejected.steps.push(ReplayStep {
            index: StepIndex::new(0),
            command: CommandEnvelope::new(
                CommandKind::System,
                Command::Entity(EntityCommand::Delete {
                    id: EntityId::new(9),
                }),
            ),
            outcome: StepOutcome::Rejected {
                summary: "EntityNotFound".to_string(),
            },
            post_hash: ReplayHash::new(0),
        });

        // Different rejection summary → RejectionMismatch.
        let mut other = rejected.clone();
        if let StepOutcome::Rejected { summary } = &mut other.steps[0].outcome {
            *summary = "SomethingElse".to_string();
        }
        assert_eq!(
            diff(&rejected, &other).unwrap().class,
            DivergenceClass::RejectionMismatch
        );

        // Golden rejected but actual accepted → AcceptedEventMismatch (flip).
        let accepted = one_step_record(accepted_step(
            0,
            Command::Entity(EntityCommand::Delete {
                id: EntityId::new(9),
            }),
            DomainEvent::EntityDeleted {
                id: EntityId::new(9),
            },
            0,
        ));
        assert_eq!(
            diff(&rejected, &accepted).unwrap().class,
            DivergenceClass::AcceptedEventMismatch
        );
    }

    #[test]
    fn diff_classifies_hash_and_initial_mismatch() {
        let a = one_step_record(accepted_step(
            0,
            Command::Tag(TagCommand::Define { id: TagId::new(1) }),
            DomainEvent::TagDefined { id: TagId::new(1) },
            10,
        ));
        let mut b = a.clone();
        b.steps[0].post_hash = ReplayHash::new(11);
        assert_eq!(
            diff(&a, &b).unwrap().class,
            DivergenceClass::HashCheckpointMismatch
        );

        let mut c = a.clone();
        c.initial_hash = ReplayHash::new(99);
        let d = diff(&a, &c).unwrap();
        assert_eq!(d.class, DivergenceClass::HashCheckpointMismatch);
        assert_eq!(d.step, None);
    }

    #[test]
    fn diff_classifies_structural_mismatch() {
        let a = sample_record();
        let mut b = a.clone();
        b.steps.pop();
        assert_eq!(
            diff(&a, &b).unwrap().class,
            DivergenceClass::StructuralMismatch
        );
    }

    #[test]
    fn malformed_artifact_divergence_from_decode_error() {
        let err = decode("not a replay").unwrap_err();
        let d = Divergence::malformed(&err);
        assert_eq!(d.class, DivergenceClass::MalformedArtifact);
    }

    #[test]
    fn report_is_actionable_and_names_the_replay() {
        let a = one_step_record(accepted_step(
            0,
            Command::Tag(TagCommand::Define { id: TagId::new(1) }),
            DomainEvent::TagDefined { id: TagId::new(1) },
            10,
        ));
        let mut b = a.clone();
        b.steps[0].post_hash = ReplayHash::new(11);
        let report = diff(&a, &b).unwrap().report("my-replay");
        assert!(report.contains("replay divergence: my-replay"));
        assert!(report.contains("hash-checkpoint-mismatch"));
        assert!(report.contains("core-snapshot"));
        assert!(report.contains("expected:"));
        assert!(report.contains("actual:"));
    }

    #[test]
    fn checkpoint_interval_decisions() {
        assert!(!CheckpointInterval::FinalOnly.captures_after(0));
        assert!(!CheckpointInterval::FinalOnly.captures_after(9));

        assert!(CheckpointInterval::EveryStep.captures_after(0));
        assert!(CheckpointInterval::EveryStep.captures_after(5));

        let every3 = CheckpointInterval::EverySteps(3);
        // Steps 0,1,2 -> capture after index 2 (the 3rd step), then 5, 8, ...
        assert!(!every3.captures_after(0));
        assert!(!every3.captures_after(1));
        assert!(every3.captures_after(2));
        assert!(every3.captures_after(5));

        // n == 0 is treated as 1 (capture after every step).
        assert!(CheckpointInterval::EverySteps(0).captures_after(0));
    }

    #[test]
    fn decode_rejects_unknown_variant() {
        let bad =
            "replay 1\ninit 0000000000000000\nstep 0\ncmd input bogus.kind 1\npost 0000000000000000\n";
        let err = decode(bad).unwrap_err();
        assert!(matches!(err, DecodeError::UnknownVariant { .. }));
    }

    #[test]
    fn decode_rejects_missing_header() {
        let err = decode("init 0000000000000000\n").unwrap_err();
        assert!(matches!(
            err,
            DecodeError::BadToken { .. } | DecodeError::UnexpectedLine { .. }
        ));
    }

    /// The committed golden artifact under `harness/goldens/replays` must stay
    /// in lockstep with the encoder, so a format change forces a reviewable diff.
    #[test]
    fn golden_file_matches_encoder() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .expect("repo root")
            .join("harness/fixtures/replays/format-sample.replay");
        let golden = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert_eq!(golden, SAMPLE_TEXT, "golden replay drifted from encoder");
        assert_eq!(decode(&golden).unwrap(), sample_record());
    }
}
