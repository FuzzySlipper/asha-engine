//! The hand-maintained IR that mirrors the Rust protocol crates.
//!
//! Each `*_module` function builds one generated `.ts` file's worth of schema.
//! These descriptions are kept in lockstep with the Rust border types in
//! `engine-rs/crates/protocol/*`; the committed generated files plus
//! `harness/ci/check-contracts.sh` are what catch drift between the two.
//!
//! Where a value can be sourced directly from a protocol crate (the branded ID
//! list, the replay format version) it is, so those facts have a single home.

use crate::schema::{Field, Import, Item, Module, TsPrim, TsType, Variant};

// ── Small builders to keep the descriptions readable ──────────────────────────

fn num() -> TsType {
    TsType::Prim(TsPrim::Number)
}

fn string() -> TsType {
    TsType::Prim(TsPrim::String)
}

fn boolean() -> TsType {
    TsType::Prim(TsPrim::Boolean)
}

fn r(name: &str) -> TsType {
    TsType::reference(name)
}

fn f(name: &str, ty: TsType) -> Field {
    Field::new(name, ty)
}

fn v(tag: &str, fields: Vec<Field>) -> Variant {
    Variant::new(tag, fields)
}

fn iface(doc: &str, name: &str, fields: Vec<Field>) -> Item {
    Item::Interface {
        doc: doc.to_string(),
        name: name.to_string(),
        fields,
    }
}

fn union(doc: &str, name: &str, discriminant: &str, variants: Vec<Variant>) -> Item {
    Item::Union {
        doc: doc.to_string(),
        name: name.to_string(),
        discriminant: discriminant.to_string(),
        variants,
    }
}

fn import(from: &str, names: &[&str]) -> Import {
    Import {
        names: names.iter().map(|s| s.to_string()).collect(),
        from: from.to_string(),
    }
}

// ── ids.ts — branded identifiers ──────────────────────────────────────────────

/// Human-readable doc for a branded ID, sourced from its brand name.
fn id_doc(brand: &str) -> String {
    let meaning = match brand {
        "EntityId" => "a discrete simulated entity",
        "SubjectId" => "an acting subject / authority",
        "ProcessId" => "an ongoing process",
        "ModeId" => "a state-machine mode",
        "SignalId" => "an event signal type",
        "TagId" => "a tag label",
        _ => "a border identifier",
    };
    format!("Branded identifier for {meaning} (over a 64-bit integer).")
}

pub fn ids_module() -> Module {
    let items = protocol_ids::BORDER_IDS
        .iter()
        .map(|b| Item::BrandedId {
            doc: id_doc(b.brand),
            name: b.brand.to_string(),
        })
        .collect();
    Module {
        name: "ids",
        imports: vec![],
        items,
    }
}

// ── script.ts — read-only views, commands, rejections ─────────────────────────

pub fn script_module() -> Module {
    let imports = vec![import(
        "./ids.js",
        &[
            "EntityId",
            "SubjectId",
            "ProcessId",
            "ModeId",
            "SignalId",
            "TagId",
        ],
    )];

    let items = vec![
        iface(
            "One entity as seen by a policy: its identity and current tags.",
            "EntityView",
            vec![f("id", r("EntityId")), f("tags", TsType::array(r("TagId")))],
        ),
        iface(
            "One process as seen by a policy: its identity and current mode.",
            "ProcessView",
            vec![
                f("id", r("ProcessId")),
                f("mode", TsType::nullable(r("ModeId"))),
            ],
        ),
        iface(
            "The complete read-only projection handed to a policy for one tick.",
            "ScriptView",
            vec![
                f("entities", TsType::array(r("EntityView"))),
                f("subjects", TsType::array(r("SubjectId"))),
                f("processes", TsType::array(r("ProcessView"))),
                f("modes", TsType::array(r("ModeId"))),
                f("signals", TsType::array(r("SignalId"))),
                f("tags", TsType::array(r("TagId"))),
            ],
        ),
        union(
            "Proposed commands that operate on an entity.",
            "EntityCommand",
            "kind",
            vec![
                v("create", vec![f("id", r("EntityId"))]),
                v("addTag", vec![f("id", r("EntityId")), f("tag", r("TagId"))]),
                v(
                    "removeTag",
                    vec![f("id", r("EntityId")), f("tag", r("TagId"))],
                ),
                v("delete", vec![f("id", r("EntityId"))]),
            ],
        ),
        union(
            "Proposed commands that operate on a subject.",
            "SubjectCommand",
            "kind",
            vec![
                v("create", vec![f("id", r("SubjectId"))]),
                v("delete", vec![f("id", r("SubjectId"))]),
            ],
        ),
        union(
            "Proposed commands that operate on a process.",
            "ProcessCommand",
            "kind",
            vec![
                v("start", vec![f("id", r("ProcessId"))]),
                v(
                    "setMode",
                    vec![f("id", r("ProcessId")), f("mode", r("ModeId"))],
                ),
                v("stop", vec![f("id", r("ProcessId"))]),
            ],
        ),
        union(
            "Proposed commands that define or undefine a mode.",
            "ModeCommand",
            "kind",
            vec![
                v("define", vec![f("id", r("ModeId"))]),
                v("undefine", vec![f("id", r("ModeId"))]),
            ],
        ),
        union(
            "Proposed commands that define or undefine a signal.",
            "SignalCommand",
            "kind",
            vec![
                v("define", vec![f("id", r("SignalId"))]),
                v("undefine", vec![f("id", r("SignalId"))]),
            ],
        ),
        union(
            "Proposed commands that define or undefine a tag.",
            "TagCommand",
            "kind",
            vec![
                v("define", vec![f("id", r("TagId"))]),
                v("undefine", vec![f("id", r("TagId"))]),
            ],
        ),
        union(
            "The full proposed-command union, grouped by fixture noun.",
            "Command",
            "domain",
            vec![
                v("entity", vec![f("command", r("EntityCommand"))]),
                v("subject", vec![f("command", r("SubjectCommand"))]),
                v("process", vec![f("command", r("ProcessCommand"))]),
                v("mode", vec![f("command", r("ModeCommand"))]),
                v("signal", vec![f("command", r("SignalCommand"))]),
                v("tag", vec![f("command", r("TagCommand"))]),
            ],
        ),
        Item::Alias {
            doc: "Origin category of a proposed command.".to_string(),
            name: "CommandKind".to_string(),
            ty: TsType::StringEnum(vec![
                "input".to_string(),
                "policy".to_string(),
                "system".to_string(),
            ]),
        },
        iface(
            "A proposed command paired with its origin kind.",
            "CommandEnvelope",
            vec![f("kind", r("CommandKind")), f("command", r("Command"))],
        ),
        union(
            "The border form of a command rejection.",
            "ScriptRejection",
            "reason",
            vec![
                v("entityAlreadyExists", vec![f("id", r("EntityId"))]),
                v("entityNotFound", vec![f("id", r("EntityId"))]),
                v("tagNotFound", vec![f("id", r("TagId"))]),
                v(
                    "tagAlreadyOnEntity",
                    vec![f("id", r("EntityId")), f("tag", r("TagId"))],
                ),
                v(
                    "tagNotOnEntity",
                    vec![f("id", r("EntityId")), f("tag", r("TagId"))],
                ),
                v("subjectAlreadyExists", vec![f("id", r("SubjectId"))]),
                v("subjectNotFound", vec![f("id", r("SubjectId"))]),
                v("processAlreadyExists", vec![f("id", r("ProcessId"))]),
                v("processNotFound", vec![f("id", r("ProcessId"))]),
                v("modeAlreadyExists", vec![f("id", r("ModeId"))]),
                v("modeNotFound", vec![f("id", r("ModeId"))]),
                v("signalAlreadyExists", vec![f("id", r("SignalId"))]),
                v("signalNotFound", vec![f("id", r("SignalId"))]),
                v("tagAlreadyDefined", vec![f("id", r("TagId"))]),
                v("tagDefinitionNotFound", vec![f("id", r("TagId"))]),
            ],
        ),
        union(
            "The outcome the authority core reports for one proposed command.",
            "ScriptOutcome",
            "status",
            vec![
                v("accepted", vec![]),
                v("rejected", vec![f("rejection", r("ScriptRejection"))]),
            ],
        ),
    ];

    Module {
        name: "script",
        imports,
        items,
    }
}

// ── render.ts — retained-mode diff shapes ─────────────────────────────────────

pub fn render_module() -> Module {
    let imports = vec![import("./ids.js", &["EntityId", "TagId"])];

    let tuple3 = || TsType::Tuple(vec![num(), num(), num()]);
    let tuple4 = || TsType::Tuple(vec![num(), num(), num(), num()]);

    let items = vec![
        Item::BrandedId {
            doc: "Stable identifier for a node in the retained render scene.".to_string(),
            name: "RenderHandle".to_string(),
        },
        iface(
            "Minimal affine transform for a render node.",
            "Transform",
            vec![
                f("translation", tuple3()),
                f("rotation", tuple4()),
                f("scale", tuple3()),
            ],
        ),
        union(
            "An abstract primitive shape; extents come from the node transform.",
            "Geometry",
            "shape",
            vec![
                v("cube", vec![]),
                v("sphere", vec![]),
                v("quad", vec![]),
                v("point", vec![]),
                v("line", vec![f("a", tuple3()), f("b", tuple3())]),
            ],
        ),
        iface(
            "Placeholder appearance: flat linear-RGBA colour and a wireframe flag.",
            "Material",
            vec![f("color", tuple4()), f("wireframe", boolean())],
        ),
        Item::Alias {
            doc: "Which retained layer a node belongs to.".to_string(),
            name: "RenderLayer".to_string(),
            ty: TsType::StringEnum(vec!["scene".to_string(), "debug".to_string()]),
        },
        iface(
            "Descriptive metadata carried on a render node.",
            "RenderMetadata",
            vec![
                f("source", TsType::nullable(r("EntityId"))),
                f("tags", TsType::array(r("TagId"))),
                f("label", TsType::nullable(string())),
            ],
        ),
        iface(
            "The full description of a node at creation time.",
            "RenderNode",
            vec![
                f("geometry", r("Geometry")),
                f("material", r("Material")),
                f("transform", r("Transform")),
                f("visible", boolean()),
                f("layer", r("RenderLayer")),
                f("metadata", r("RenderMetadata")),
            ],
        ),
        union(
            "A single retained-mode change against the render scene.",
            "RenderDiff",
            "op",
            vec![
                v(
                    "create",
                    vec![
                        f("handle", r("RenderHandle")),
                        f("parent", TsType::nullable(r("RenderHandle"))),
                        f("node", r("RenderNode")),
                    ],
                ),
                v(
                    "update",
                    vec![
                        f("handle", r("RenderHandle")),
                        f("transform", TsType::nullable(r("Transform"))),
                        f("material", TsType::nullable(r("Material"))),
                        f("visible", TsType::nullable(boolean())),
                        f("metadata", TsType::nullable(r("RenderMetadata"))),
                    ],
                ),
                v("destroy", vec![f("handle", r("RenderHandle"))]),
            ],
        ),
        iface(
            "All retained-mode changes emitted for a single tick, in apply order.",
            "RenderFrameDiff",
            vec![f("ops", TsType::array(r("RenderDiff")))],
        ),
    ];

    Module {
        name: "render",
        imports,
        items,
    }
}

// ── replay.ts — record/step/hash/snapshot shapes ──────────────────────────────

pub fn replay_module() -> Module {
    let imports = vec![
        import(
            "./ids.js",
            &[
                "EntityId",
                "SubjectId",
                "ProcessId",
                "ModeId",
                "SignalId",
                "TagId",
            ],
        ),
        import("./script.js", &["CommandEnvelope"]),
    ];

    let items = vec![
        Item::BrandedId {
            doc: "Zero-based position of a step within a replay record.".to_string(),
            name: "StepIndex".to_string(),
        },
        Item::BrandedId {
            doc: "A deterministic state fingerprint at a point in a replay.".to_string(),
            name: "ReplayHash".to_string(),
        },
        Item::Const {
            doc: "Compatibility marker for the replay record wire format.".to_string(),
            name: "REPLAY_FORMAT_VERSION".to_string(),
            value: protocol_replay::REPLAY_FORMAT_VERSION.to_string(),
        },
        union(
            "Authoritative record of an accepted state change.",
            "DomainEvent",
            "event",
            vec![
                v("entityCreated", vec![f("id", r("EntityId"))]),
                v(
                    "entityTagAdded",
                    vec![f("id", r("EntityId")), f("tag", r("TagId"))],
                ),
                v(
                    "entityTagRemoved",
                    vec![f("id", r("EntityId")), f("tag", r("TagId"))],
                ),
                v("entityDeleted", vec![f("id", r("EntityId"))]),
                v("subjectCreated", vec![f("id", r("SubjectId"))]),
                v("subjectDeleted", vec![f("id", r("SubjectId"))]),
                v("processStarted", vec![f("id", r("ProcessId"))]),
                v(
                    "processModeSet",
                    vec![f("id", r("ProcessId")), f("mode", r("ModeId"))],
                ),
                v("processStopped", vec![f("id", r("ProcessId"))]),
                v("modeDefined", vec![f("id", r("ModeId"))]),
                v("modeUndefined", vec![f("id", r("ModeId"))]),
                v("signalDefined", vec![f("id", r("SignalId"))]),
                v("signalUndefined", vec![f("id", r("SignalId"))]),
                v("tagDefined", vec![f("id", r("TagId"))]),
                v("tagUndefined", vec![f("id", r("TagId"))]),
            ],
        ),
        union(
            "What the authority core decided about a proposed command.",
            "StepOutcome",
            "status",
            vec![
                v(
                    "accepted",
                    vec![f("events", TsType::array(r("DomainEvent")))],
                ),
                v("rejected", vec![f("summary", string())]),
            ],
        ),
        iface(
            "One recorded step: input command, the authority outcome, and post-step hash.",
            "ReplayStep",
            vec![
                f("index", r("StepIndex")),
                f("command", r("CommandEnvelope")),
                f("outcome", r("StepOutcome")),
                f("postHash", r("ReplayHash")),
            ],
        ),
        iface(
            "Marks that a full state snapshot was captured at a given step.",
            "SnapshotMeta",
            vec![
                f("step", r("StepIndex")),
                f("hash", r("ReplayHash")),
                f("snapshotVersion", num()),
            ],
        ),
        iface(
            "A complete recorded run: initial state plus ordered steps and snapshots.",
            "ReplayRecord",
            vec![
                f("formatVersion", num()),
                f("initialHash", r("ReplayHash")),
                f("steps", TsType::array(r("ReplayStep"))),
                f("snapshots", TsType::array(r("SnapshotMeta"))),
            ],
        ),
    ];

    Module {
        name: "replay",
        imports,
        items,
    }
}

// ── voxel.ts — voxel edit/generation border shapes ────────────────────────────
//
// Mirrors `core_commands::VoxelCommand` / `core_events::VoxelEditEvent` and the
// `rule_voxel_edit::VoxelEditRejection` authority surface (voxel-capability-05).
// Coordinates and ids are plain numbers at the border (grid id, i64 coords,
// u16 material), matching the rest of the contract surface.

pub fn voxel_module() -> Module {
    let coord =
        |name: &str, doc: &str| iface(doc, name, vec![f("x", num()), f("y", num()), f("z", num())]);
    let items = vec![
        coord(
            "VoxelCoord",
            "An integer voxel cell coordinate within a grid.",
        ),
        coord("ChunkCoord", "An integer chunk coordinate."),
        union(
            "The value of a voxel cell: empty space or a solid of some material.",
            "VoxelValue",
            "kind",
            vec![v("empty", vec![]), v("solid", vec![f("material", num())])],
        ),
        union(
            "A proposed voxel edit/generation command (authority-owned).",
            "VoxelCommand",
            "op",
            vec![
                v(
                    "setVoxel",
                    vec![
                        f("grid", num()),
                        f("coord", r("VoxelCoord")),
                        f("value", r("VoxelValue")),
                    ],
                ),
                v(
                    "fillRegion",
                    vec![
                        f("grid", num()),
                        f("min", r("VoxelCoord")),
                        f("max", r("VoxelCoord")),
                        f("value", r("VoxelValue")),
                    ],
                ),
                v(
                    "generateChunk",
                    vec![
                        f("grid", num()),
                        f("chunk", r("ChunkCoord")),
                        f("seed", num()),
                        f("generatorVersion", num()),
                    ],
                ),
            ],
        ),
        union(
            "An accepted, authoritative voxel change.",
            "VoxelEditEvent",
            "event",
            vec![
                v(
                    "voxelSet",
                    vec![
                        f("grid", num()),
                        f("coord", r("VoxelCoord")),
                        f("value", r("VoxelValue")),
                    ],
                ),
                v(
                    "voxelRegionFilled",
                    vec![
                        f("grid", num()),
                        f("min", r("VoxelCoord")),
                        f("max", r("VoxelCoord")),
                        f("value", r("VoxelValue")),
                    ],
                ),
                v(
                    "chunkGenerated",
                    vec![
                        f("grid", num()),
                        f("chunk", r("ChunkCoord")),
                        f("seed", num()),
                        f("generatorVersion", num()),
                        f("hash", num()),
                    ],
                ),
            ],
        ),
        union(
            "Why a proposed voxel edit was refused.",
            "VoxelEditRejection",
            "reason",
            vec![
                v("unknownMaterial", vec![f("material", num())]),
                v(
                    "emptyRegion",
                    vec![f("min", r("VoxelCoord")), f("max", r("VoxelCoord"))],
                ),
                v("chunkNotResident", vec![f("chunk", r("ChunkCoord"))]),
                v(
                    "generationDivergence",
                    vec![
                        f("chunk", r("ChunkCoord")),
                        f("expected", num()),
                        f("actual", num()),
                    ],
                ),
            ],
        ),
    ];
    Module {
        name: "voxel",
        imports: vec![],
        items,
    }
}

// ── index.ts — barrel ─────────────────────────────────────────────────────────

pub fn index_module() -> Module {
    Module {
        name: "index",
        imports: vec![],
        items: vec![
            Item::ReExport {
                from: "./ids.js".to_string(),
            },
            Item::ReExport {
                from: "./script.js".to_string(),
            },
            Item::ReExport {
                from: "./render.js".to_string(),
            },
            Item::ReExport {
                from: "./replay.js".to_string(),
            },
            Item::ReExport {
                from: "./voxel.js".to_string(),
            },
        ],
    }
}

/// All generated modules, in deterministic file order.
pub fn all_modules() -> Vec<Module> {
    vec![
        ids_module(),
        script_module(),
        render_module(),
        replay_module(),
        voxel_module(),
        index_module(),
    ]
}
