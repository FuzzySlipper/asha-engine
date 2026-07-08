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
    let imports = vec![
        import("./ids.js", &["EntityId", "TagId"]),
        import("./assets.js", &["CatalogEntry", "MaterialProjection"]),
    ];

    let tuple2 = || TsType::Tuple(vec![num(), num()]);
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
        // ── voxel mesh payload descriptors (ADR 0007) ──────────────────────────
        Item::Alias {
            doc: "Vertex attribute element type (only f32 today).".to_string(),
            name: "MeshAttributeKind".to_string(),
            ty: TsType::StringEnum(vec!["f32".to_string()]),
        },
        Item::Alias {
            doc: "Which vertex attribute a stream carries (uv/color reserved).".to_string(),
            name: "MeshAttributeName".to_string(),
            ty: TsType::StringEnum(vec![
                "position".to_string(),
                "normal".to_string(),
                "uv".to_string(),
                "color".to_string(),
            ]),
        },
        iface(
            "One declared vertex attribute stream.",
            "MeshAttribute",
            vec![
                f("name", r("MeshAttributeName")),
                f("components", num()),
                f("kind", r("MeshAttributeKind")),
            ],
        ),
        Item::Alias {
            doc: "Index buffer element width (u32 everywhere today).".to_string(),
            name: "MeshIndexWidth".to_string(),
            ty: TsType::StringEnum(vec!["u32".to_string()]),
        },
        iface(
            "Buffer layout for BufferGeometry upload without transcoding.",
            "MeshBufferLayout",
            vec![
                f("vertexCount", num()),
                f("indexCount", num()),
                f("indexWidth", r("MeshIndexWidth")),
                f("attributes", TsType::array(r("MeshAttribute"))),
            ],
        ),
        iface(
            "One material-slot draw group over a contiguous index range.",
            "MeshGroupDescriptor",
            vec![
                f("materialSlot", num()),
                f("start", num()),
                f("count", num()),
            ],
        ),
        iface(
            "Axis-aligned mesh bounds (chunk-local).",
            "MeshBoundsDescriptor",
            vec![f("min", tuple3()), f("max", tuple3())],
        ),
        Item::Alias {
            doc: "Which source produced a mesh payload (voxel chunk vs authored static asset)."
                .to_string(),
            name: "MeshProvenance".to_string(),
            ty: TsType::StringEnum(vec![
                "voxelChunk".to_string(),
                "staticAsset".to_string(),
                "generated".to_string(),
                "debug".to_string(),
            ]),
        },
        union(
            "Where the bulk vertex/index bytes live: inline (fixtures) or by handle (runtime).",
            "MeshPayloadSource",
            "kind",
            vec![
                v(
                    "inline",
                    vec![
                        f("positions", TsType::array(num())),
                        f("normals", TsType::array(num())),
                        f("indices", TsType::array(num())),
                    ],
                ),
                v(
                    "handle",
                    vec![
                        f("buffer", num()),
                        f("positionsByteOffset", num()),
                        f("normalsByteOffset", num()),
                        f("indicesByteOffset", num()),
                    ],
                ),
            ],
        ),
        iface(
            "The full mesh-payload border: layout + groups + bounds + source + provenance.",
            "MeshPayloadDescriptor",
            vec![
                f("layout", r("MeshBufferLayout")),
                f("groups", TsType::array(r("MeshGroupDescriptor"))),
                f("bounds", r("MeshBoundsDescriptor")),
                f("source", r("MeshPayloadSource")),
                f("provenance", r("MeshProvenance")),
            ],
        ),
        // ── static mesh assets + instances (render-asset-04) ───────────────────
        iface(
            "One material slot of a static mesh, bound to a catalog material asset id.",
            "MeshMaterialSlot",
            vec![f("slot", num()), f("material", string())],
        ),
        union(
            "Collision policy for a static mesh (visual-only, explicit proxy, or AABB fallback).",
            "MeshCollisionPolicy",
            "kind",
            vec![
                v("visualOnly", vec![]),
                v("proxy", vec![f("proxyAsset", string())]),
                v("aabbFallback", vec![]),
            ],
        ),
        iface(
            "An authored static mesh asset: shared geometry payload, material slots, collision.",
            "StaticMeshAsset",
            vec![
                f("asset", string()),
                f("payload", r("MeshPayloadDescriptor")),
                f("materialSlots", TsType::array(r("MeshMaterialSlot"))),
                f("collision", r("MeshCollisionPolicy")),
            ],
        ),
        iface(
            "One placed instance of a static mesh asset (shared geometry, own transform/overrides).",
            "StaticMeshInstanceDescriptor",
            vec![
                f("asset", string()),
                f("transform", r("Transform")),
                f("materialOverrides", TsType::array(r("MeshMaterialSlot"))),
                f("metadata", r("RenderMetadata")),
            ],
        ),
        // ── sprites / billboards (render-asset-05/06) ──────────────────────────
        Item::Alias {
            doc: "How a sprite size is interpreted (world units vs screen pixels).".to_string(),
            name: "SpriteSizeMode".to_string(),
            ty: TsType::StringEnum(vec!["world".to_string(), "pixel".to_string()]),
        },
        Item::Alias {
            doc: "Billboarding behaviour for a sprite plane.".to_string(),
            name: "BillboardMode".to_string(),
            ty: TsType::StringEnum(vec![
                "none".to_string(),
                "spherical".to_string(),
                "cylindrical".to_string(),
            ]),
        },
        Item::Alias {
            doc: "Depth handling for a sprite (reserves overlay/no-write modes).".to_string(),
            name: "SpriteDepthPolicy".to_string(),
            ty: TsType::StringEnum(vec![
                "default".to_string(),
                "depthTestOff".to_string(),
                "depthWriteOff".to_string(),
            ]),
        },
        Item::Alias {
            doc: "Reserved sprite shading mode (unlit implemented; lit/shadow/custom reserved)."
                .to_string(),
            name: "SpriteShading".to_string(),
            ty: TsType::StringEnum(vec![
                "unlit".to_string(),
                "lit".to_string(),
                "shadowed".to_string(),
                "custom".to_string(),
            ]),
        },
        iface(
            "Where a sprite is attached in authority terms (source ids, not render handles).",
            "SpriteAttachment",
            vec![
                f("sourceEntity", TsType::nullable(r("EntityId"))),
                f("sourceSceneNode", TsType::nullable(num())),
                f("attachmentPoint", TsType::nullable(string())),
            ],
        ),
        iface(
            "One placed plane-geometry sprite/billboard instance.",
            "SpriteInstanceDescriptor",
            vec![
                f("asset", string()),
                f("frame", num()),
                f("pivot", tuple2()),
                f("size", tuple2()),
                f("sizeMode", r("SpriteSizeMode")),
                f("billboard", r("BillboardMode")),
                f("tint", tuple4()),
                f("renderOrder", num()),
                f("depth", r("SpriteDepthPolicy")),
                f("shading", r("SpriteShading")),
                f("transform", r("Transform")),
                f("attachment", r("SpriteAttachment")),
                f("metadata", r("RenderMetadata")),
            ],
        ),
        iface(
            "A renderer-side sprite pick hit traced to authority identity (renderer never acts).",
            "SpritePickHit",
            vec![
                f("handle", r("RenderHandle")),
                f("sourceEntity", TsType::nullable(r("EntityId"))),
                f("sourceSceneNode", TsType::nullable(num())),
                f("asset", string()),
                f("attachmentPoint", TsType::nullable(string())),
            ],
        ),
        // A renderer-side mesh pick hint (launchable-voxel picking, #2437). It maps a
        // render handle back to the authority SOURCE that produced the mesh (its
        // provenance), so a hover/pick can be revalidated against authority — the
        // renderer never owns voxel coordinates, and a stale/destroyed handle yields
        // no hint at all (undefined) rather than a guess.
        iface(
            "A renderer-side mesh pick hint mapping a render handle to the authority \
             source (provenance) that produced its mesh. Only a hint — authority \
             picking (pickVoxel) revalidates before any selection/edit acts on it.",
            "MeshPickHit",
            vec![f("handle", r("RenderHandle")), f("provenance", r("MeshProvenance"))],
        ),
        // ── textures + sprite atlases (material-wiring super, epic #2353; #2374) ─
        Item::Alias {
            doc: "Texture sampling filter policy.".to_string(),
            name: "TextureFilter".to_string(),
            ty: TsType::StringEnum(vec!["nearest".to_string(), "linear".to_string()]),
        },
        Item::Alias {
            doc: "Texture wrap/addressing policy outside [0,1].".to_string(),
            name: "TextureWrap".to_string(),
            ty: TsType::StringEnum(vec!["clamp".to_string(), "repeat".to_string()]),
        },
        iface(
            "A texture asset descriptor (identity, dimensions, sampling policy, content \
             metadata). Carries no pixel bytes — those load through a renderer texture provider.",
            "TextureDescriptor",
            vec![
                f("id", string()),
                f("width", num()),
                f("height", num()),
                f("filter", r("TextureFilter")),
                f("wrap", r("TextureWrap")),
                f("contentHash", TsType::nullable(string())),
                f("version", num()),
            ],
        ),
        iface(
            "One atlas frame: its sprite frame id and normalized UV sub-rectangle in [0,1].",
            "SpriteFrameRect",
            vec![
                f("frame", num()),
                f("uvMin", tuple2()),
                f("uvMax", tuple2()),
            ],
        ),
        iface(
            "A sprite atlas descriptor: the texture it samples and its frame rects. The \
             renderer resolves a sprite frame to one of these rects deterministically.",
            "SpriteAtlasDescriptor",
            vec![
                f("id", string()),
                f("texture", string()),
                f("frames", TsType::array(r("SpriteFrameRect"))),
            ],
        ),
        // ── catalog material descriptor (material-wiring super, epic #2353) ─────
        Item::Alias {
            doc: "How a material samples colour across geometry (visual projection only)."
                .to_string(),
            name: "MaterialUvStrategy".to_string(),
            ty: TsType::StringEnum(vec![
                "flat".to_string(),
                "planar".to_string(),
                "atlas".to_string(),
            ]),
        },
        iface(
            "The renderer-facing projection of a catalog material, keyed by asset id. The \
             VISUAL projection only — no collision/authority field ever appears here.",
            "RenderMaterialDescriptor",
            vec![
                f("id", string()),
                f("color", tuple4()),
                f("texture", TsType::nullable(string())),
                f("roughness", num()),
                f("emissive", num()),
                f("uvStrategy", r("MaterialUvStrategy")),
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
                v(
                    "replaceMeshPayload",
                    vec![
                        f("handle", r("RenderHandle")),
                        f("payload", r("MeshPayloadDescriptor")),
                    ],
                ),
                v(
                    "defineMaterial",
                    vec![f("material", r("RenderMaterialDescriptor"))],
                ),
                v("defineTexture", vec![f("texture", r("TextureDescriptor"))]),
                v(
                    "defineSpriteAtlas",
                    vec![f("atlas", r("SpriteAtlasDescriptor"))],
                ),
                v("defineStaticMesh", vec![f("asset", r("StaticMeshAsset"))]),
                v(
                    "createStaticMeshInstance",
                    vec![
                        f("handle", r("RenderHandle")),
                        f("parent", TsType::nullable(r("RenderHandle"))),
                        f("instance", r("StaticMeshInstanceDescriptor")),
                    ],
                ),
                v(
                    "createSprite",
                    vec![
                        f("handle", r("RenderHandle")),
                        f("parent", TsType::nullable(r("RenderHandle"))),
                        f("sprite", r("SpriteInstanceDescriptor")),
                    ],
                ),
                v(
                    "updateSprite",
                    vec![
                        f("handle", r("RenderHandle")),
                        f("frame", TsType::nullable(num())),
                        f("tint", TsType::nullable(tuple4())),
                        f("renderOrder", TsType::nullable(num())),
                        f("visible", TsType::nullable(boolean())),
                    ],
                ),
            ],
        ),

        iface(
            "Request to derive/read a model/material preview using public catalog/material and static-mesh DTOs.",
            "ModelMaterialPreviewRequest",
            vec![
                f("catalogEntry", r("CatalogEntry")),
                f("meshAsset", r("StaticMeshAsset")),
                f("instanceHandle", r("RenderHandle")),
            ],
        ),
        iface(
            "Snapshot returned by read_model_material_preview: public material/model DTOs plus retained-mode render-diff evidence.",
            "ModelMaterialPreviewSnapshot",
            vec![
                f("catalogEntry", r("CatalogEntry")),
                f("material", r("MaterialProjection")),
                f("meshAsset", r("StaticMeshAsset")),
                f("previewDiff", r("RenderFrameDiff")),
                f("rendererClassification", TsType::StringEnum(vec!["reference_preview".to_string(), "runtime_readback".to_string()])),
                f("diagnostics", TsType::array(string())),
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
        // ── submit_commands border (launchable-voxel, #2436) ──────────────────
        // The runtime facade's `submit_commands` verb carries a batch of generated
        // `VoxelCommand`s to Rust authority and returns a classified accept/reject
        // summary. Mirrors `runtime_bridge_api::{CommandBatch, CommandResult}`.
        iface(
            "A batch of proposed voxel commands submitted to Rust authority for \
             validation + apply (the runtime facade `submitCommands` input).",
            "CommandBatch",
            vec![f("commands", TsType::array(r("VoxelCommand")))],
        ),
        iface(
            "The classified outcome of a submitted command batch: how many commands \
             authority accepted/rejected, plus one classified rejection per refused \
             command in submission order. Accepted commands have already mutated \
             authority and marked their chunks dirty.",
            "CommandResult",
            vec![
                f("accepted", num()),
                f("rejected", num()),
                f("rejections", TsType::array(r("VoxelEditRejection"))),
            ],
        ),
        Item::Alias {
            doc: "A cube face / axis-aligned outward normal direction.".to_string(),
            name: "Face".to_string(),
            ty: TsType::StringEnum(vec![
                "posX".to_string(),
                "negX".to_string(),
                "posY".to_string(),
                "negY".to_string(),
                "posZ".to_string(),
                "negZ".to_string(),
            ]),
        },
        union(
            "Why an authority-revalidated renderer pick was refused (picking).",
            "PickRejection",
            "reason",
            vec![
                v("noHit", vec![]),
                v(
                    "hitMismatch",
                    vec![
                        f("authoritativeVoxel", r("VoxelCoord")),
                        f("authoritativeFace", r("Face")),
                        f("claimedVoxel", r("VoxelCoord")),
                        f("claimedFace", r("Face")),
                    ],
                ),
            ],
        ),
        // ── pick_voxel border (launchable-voxel picking, #2437) ───────────────
        // The renderer/UI builds a world-space `PickRay` from camera + pointer and
        // hands it to the runtime facade `pick_voxel` verb; Rust authority owns the
        // voxel-grid raycast (no parallel TS DDA) and returns a classified
        // `PickResult`. Mirrors `runtime_bridge_api::{PickRay, VoxelHit, PickResult}`.
        iface(
            "A world-space pick ray built by the renderer/UI from camera + pointer. \
             The ray is plain geometry; Rust authority owns the voxel-grid raycast.",
            "PickRay",
            vec![
                f("grid", num()),
                f("origin", TsType::Tuple(vec![num(), num(), num()])),
                f("direction", TsType::Tuple(vec![num(), num(), num()])),
                f("maxDistance", num()),
            ],
        ),
        iface(
            "An authoritative voxel ray hit: the solid voxel struck, its chunk, the \
             struck face (outward normal — the anchor a place edit builds against), \
             and the world-space impact point + distance along the ray. Derived from \
             authority voxel state; a renderer pick is only a hint until revalidated.",
            "VoxelHit",
            vec![
                f("grid", num()),
                f("voxel", r("VoxelCoord")),
                f("chunk", r("ChunkCoord")),
                f("face", r("Face")),
                f("point", TsType::Tuple(vec![num(), num(), num()])),
                f("distance", num()),
            ],
        ),
        union(
            "The classified outcome of an authority voxel pick: a hit, or a \
             classified miss carrying the PickRejection reason.",
            "PickResult",
            "outcome",
            vec![
                v("hit", vec![f("hit", r("VoxelHit"))]),
                v("miss", vec![f("rejection", r("PickRejection"))]),
            ],
        ),
    ];
    Module {
        name: "voxel",
        imports: vec![],
        items,
    }
}

// ── voxelConversion.ts — mesh/source asset to voxel conversion DTOs ───────────
//
// Mirrors the border-only `protocol-voxel-conversion` crate. Authority services
// plan/validate/apply conversion; this generated surface only carries typed
// requests, receipts, diagnostics, and evidence references across the TS border.

pub fn voxel_conversion_module() -> Module {
    let imports = vec![
        import("./diagnostics.js", &["DiagnosticSeverity"]),
        import("./voxel.js", &["VoxelCoord"]),
    ];
    let matrix4 = || {
        TsType::Tuple(vec![
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
        ])
    };
    let resolution3 = || TsType::Tuple(vec![num(), num(), num()]);
    let vec3 = || TsType::Tuple(vec![num(), num(), num()]);

    let items = vec![
        string_enum(
            "How source geometry is converted into occupied voxel cells.",
            "VoxelConversionMode",
            protocol_voxel_conversion::VOXEL_CONVERSION_MODES,
        ),
        string_enum(
            "How source bounds fit into the requested target resolution.",
            "VoxelConversionFitPolicy",
            protocol_voxel_conversion::VOXEL_CONVERSION_FIT_POLICIES,
        ),
        string_enum(
            "How converted voxel coordinates are anchored.",
            "VoxelConversionOriginPolicy",
            protocol_voxel_conversion::VOXEL_CONVERSION_ORIGIN_POLICIES,
        ),
        string_enum(
            "Stable classified diagnostic/error code for voxel conversion.",
            "VoxelConversionDiagnosticCode",
            protocol_voxel_conversion::VOXEL_CONVERSION_DIAGNOSTIC_CODES,
        ),
        string_enum(
            "Role of an exported conversion evidence artifact.",
            "VoxelConversionEvidenceKind",
            protocol_voxel_conversion::VOXEL_CONVERSION_EVIDENCE_KINDS,
        ),
        iface(
            "Source asset and authority snapshot identity for conversion.",
            "VoxelConversionSourceRef",
            vec![
                f("assetId", string()),
                f("assetKind", string()),
                f("assetVersion", num()),
                f("sourceHash", string()),
                f("meshPrimitive", TsType::nullable(string())),
            ],
        ),
        iface(
            "One static-mesh triangle registered as an authority-visible conversion source.",
            "VoxelConversionSourceTriangle",
            vec![
                f("indices", TsType::Tuple(vec![num(), num(), num()])),
                f("sourceMaterialSlot", num()),
            ],
        ),
        iface(
            "One source material slot available on a registered conversion source.",
            "VoxelConversionSourceMaterialSlot",
            vec![
                f("sourceMaterialSlot", num()),
                f("sourceMaterialId", TsType::nullable(string())),
            ],
        ),
        iface(
            "Register inline static-mesh geometry as an authority-visible conversion source.",
            "VoxelConversionSourceRegistrationRequest",
            vec![
                f("source", r("VoxelConversionSourceRef")),
                f("positions", TsType::array(vec3())),
                f(
                    "triangles",
                    TsType::array(r("VoxelConversionSourceTriangle")),
                ),
                f(
                    "materialSlots",
                    TsType::array(r("VoxelConversionSourceMaterialSlot")),
                ),
            ],
        ),
        iface(
            "A material-indexed triangle group inside a project mesh asset.",
            "VoxelConversionMeshAssetGroup",
            vec![
                f("materialSlot", num()),
                f("start", num()),
                f("count", num()),
            ],
        ),
        iface(
            "Project/catalog static-mesh data accepted by Rust voxel-conversion ingestion.",
            "VoxelConversionMeshAsset",
            vec![
                f("assetId", string()),
                f("sourcePath", TsType::nullable(string())),
                f("positions", TsType::array(vec3())),
                f("normals", TsType::array(vec3())),
                f("indices", TsType::array(num())),
                f("groups", TsType::array(r("VoxelConversionMeshAssetGroup"))),
                f(
                    "materialSlots",
                    TsType::array(r("VoxelConversionSourceMaterialSlot")),
                ),
            ],
        ),
        iface(
            "Register an authored project static-mesh asset as a conversion source.",
            "VoxelConversionMeshAssetRegistrationRequest",
            vec![
                f("source", r("VoxelConversionSourceRef")),
                f("meshAsset", r("VoxelConversionMeshAsset")),
            ],
        ),
        iface(
            "Result of registering a conversion source; rejected inputs carry diagnostics.",
            "VoxelConversionSourceRegistration",
            vec![
                f("source", r("VoxelConversionSourceRef")),
                f("registered", boolean()),
                f(
                    "materialSlots",
                    TsType::array(r("VoxelConversionSourceMaterialSlot")),
                ),
                f("diagnostics", TsType::array(r("VoxelConversionDiagnostic"))),
                f("evidence", TsType::array(r("VoxelConversionEvidenceRef"))),
            ],
        ),
        iface(
            "Target voxel grid/volume identity.",
            "VoxelConversionTargetRef",
            vec![
                f("grid", num()),
                f("volumeAssetId", TsType::nullable(string())),
                f("origin", r("VoxelCoord")),
            ],
        ),
        iface(
            "Inclusive voxel-space bounds.",
            "VoxelConversionBounds",
            vec![f("min", r("VoxelCoord")), f("max", r("VoxelCoord"))],
        ),
        iface(
            "One source material slot mapped into an Asha voxel material id.",
            "VoxelConversionMaterialMapEntry",
            vec![
                f("sourceMaterialSlot", num()),
                f("sourceMaterialId", TsType::nullable(string())),
                f("voxelMaterial", num()),
            ],
        ),
        iface(
            "Material-map DTO. Default material is null when unmapped slots fail closed.",
            "VoxelConversionMaterialMap",
            vec![
                f(
                    "entries",
                    TsType::array(r("VoxelConversionMaterialMapEntry")),
                ),
                f("defaultVoxelMaterial", TsType::nullable(num())),
            ],
        ),
        iface(
            "A conversion request's tunable settings.",
            "VoxelConversionSettings",
            vec![
                f("mode", r("VoxelConversionMode")),
                f("fitPolicy", r("VoxelConversionFitPolicy")),
                f("originPolicy", r("VoxelConversionOriginPolicy")),
                f("resolution", resolution3()),
                f("voxelSize", num()),
                f("maxOutputVoxels", num()),
                f("transform", matrix4()),
                f("materialMap", r("VoxelConversionMaterialMap")),
            ],
        ),
        iface(
            "One request to plan a conversion.",
            "VoxelConversionPlanRequest",
            vec![
                f("source", r("VoxelConversionSourceRef")),
                f("target", r("VoxelConversionTargetRef")),
                f("settings", r("VoxelConversionSettings")),
            ],
        ),
        iface(
            "One classified diagnostic for a conversion operation.",
            "VoxelConversionDiagnostic",
            vec![
                f("code", r("VoxelConversionDiagnosticCode")),
                f("severity", r("DiagnosticSeverity")),
                f("reference", string()),
                f("message", string()),
            ],
        ),
        iface(
            "Reference to an inspectable artifact emitted by authority.",
            "VoxelConversionEvidenceRef",
            vec![
                f("kind", r("VoxelConversionEvidenceKind")),
                f("uri", string()),
                f("contentHash", string()),
            ],
        ),
        iface(
            "Deterministic conversion plan produced by Rust authority.",
            "VoxelConversionPlan",
            vec![
                f("planId", string()),
                f("source", r("VoxelConversionSourceRef")),
                f("target", r("VoxelConversionTargetRef")),
                f("settings", r("VoxelConversionSettings")),
                f("authorityVersion", string()),
                f("expectedSourceHash", string()),
                f("settingsHash", string()),
                f("planHash", string()),
                f("estimatedOutputVoxels", num()),
                f(
                    "estimatedBounds",
                    TsType::nullable(r("VoxelConversionBounds")),
                ),
                f("diagnostics", TsType::array(r("VoxelConversionDiagnostic"))),
                f("evidence", TsType::array(r("VoxelConversionEvidenceRef"))),
            ],
        ),
        iface(
            "Preview request for a previously produced plan.",
            "VoxelConversionPreviewRequest",
            vec![f("planId", string()), f("expectedPlanHash", string())],
        ),
        iface(
            "One sampled/previewed output voxel.",
            "VoxelConversionPreviewVoxel",
            vec![f("coord", r("VoxelCoord")), f("material", num())],
        ),
        iface(
            "Bounded preview of conversion output.",
            "VoxelConversionPreview",
            vec![
                f("planId", string()),
                f("outputHash", string()),
                f("outputVoxelCount", num()),
                f("outputBounds", TsType::nullable(r("VoxelConversionBounds"))),
                f(
                    "sampleVoxels",
                    TsType::array(r("VoxelConversionPreviewVoxel")),
                ),
                f("diagnostics", TsType::array(r("VoxelConversionDiagnostic"))),
                f("evidence", TsType::array(r("VoxelConversionEvidenceRef"))),
            ],
        ),
        iface(
            "Apply request for a planned conversion.",
            "VoxelConversionApplyRequest",
            vec![
                f("planId", string()),
                f("expectedPlanHash", string()),
                f("expectedPreviewHash", TsType::nullable(string())),
            ],
        ),
        iface(
            "Final apply receipt. Rejected requests never pretend to have applied output.",
            "VoxelConversionReceipt",
            vec![
                f("planId", string()),
                f("applied", boolean()),
                f("outputHash", TsType::nullable(string())),
                f("outputVoxelCount", num()),
                f("outputBounds", TsType::nullable(r("VoxelConversionBounds"))),
                f("diagnostics", TsType::array(r("VoxelConversionDiagnostic"))),
                f("evidence", TsType::array(r("VoxelConversionEvidenceRef"))),
            ],
        ),
        iface(
            "Request for authority-owned model/volume readback.",
            "VoxelModelInfoRequest",
            vec![
                f("grid", num()),
                f("volumeAssetId", TsType::nullable(string())),
                f("includeMaterialCounts", boolean()),
            ],
        ),
        iface(
            "Per-material voxel count derived from authority state.",
            "VoxelModelMaterialCount",
            vec![f("material", num()), f("voxelCount", num())],
        ),
        iface(
            "Rich but bounded model/volume readback for Studio and agents.",
            "VoxelModelInfoReadout",
            vec![
                f("request", r("VoxelModelInfoRequest")),
                f("resident", boolean()),
                f("modelId", string()),
                f("volumeAssetId", TsType::nullable(string())),
                f("grid", num()),
                f("bounds", TsType::nullable(r("VoxelConversionBounds"))),
                f("voxelCount", num()),
                f(
                    "materialCounts",
                    TsType::array(r("VoxelModelMaterialCount")),
                ),
                f("source", TsType::nullable(r("VoxelConversionSourceRef"))),
                f("latestPlanId", TsType::nullable(string())),
                f("latestOutputHash", TsType::nullable(string())),
                f("sessionHash", string()),
                f("replayHash", string()),
                f("evidence", TsType::array(r("VoxelConversionEvidenceRef"))),
                f("diagnostics", TsType::array(r("VoxelConversionDiagnostic"))),
            ],
        ),
    ];

    Module {
        name: "voxelConversion",
        imports,
        items,
    }
}

// ── voxelAsset.ts — Asha-native stored voxel-volume asset DTOs ───────────────
//
// Mirrors the border-only `protocol-voxel-asset` crate. Rust services own
// validation, canonical JSON, hashing, and stored/runtime transitions; this
// generated surface only carries typed ProjectBundle/catalog asset data across
// the TS border.

pub fn voxel_asset_module() -> Module {
    let imports = vec![import("./diagnostics.js", &["DiagnosticSeverity"])];
    let vec3 = || TsType::Tuple(vec![num(), num(), num()]);

    let items = vec![
        Item::Const {
            doc: "Current supported Asha voxel-volume asset schema.".to_string(),
            name: "VOXEL_ASSET_SCHEMA_VERSION".to_string(),
            value: protocol_voxel_asset::VOXEL_ASSET_SCHEMA_VERSION.to_string(),
        },
        Item::Const {
            doc: "Canonical media type for the JSON envelope.".to_string(),
            name: "VOXEL_ASSET_MEDIA_TYPE".to_string(),
            value: format!("{:?}", protocol_voxel_asset::VOXEL_ASSET_MEDIA_TYPE),
        },
        Item::Const {
            doc: "Canonical filename extension for this JSON envelope.".to_string(),
            name: "VOXEL_ASSET_EXTENSION".to_string(),
            value: format!("{:?}", protocol_voxel_asset::VOXEL_ASSET_EXTENSION),
        },
        string_enum(
            "Stored voxel representation kind.",
            "VoxelAssetRepresentationKind",
            protocol_voxel_asset::VOXEL_ASSET_REPRESENTATION_KINDS,
        ),
        string_enum(
            "Stored voxel-volume provenance kind.",
            "VoxelAssetProvenanceKind",
            protocol_voxel_asset::VOXEL_ASSET_PROVENANCE_KINDS,
        ),
        string_enum(
            "Classified stored-voxel asset diagnostic code.",
            "VoxelAssetDiagnosticCode",
            protocol_voxel_asset::VOXEL_ASSET_DIAGNOSTIC_CODES,
        ),
        iface(
            "Integer coordinate in stored voxel space.",
            "VoxelAssetCoord",
            vec![f("x", num()), f("y", num()), f("z", num())],
        ),
        iface(
            "Inclusive stored voxel-space bounds.",
            "VoxelAssetBounds",
            vec![
                f("min", r("VoxelAssetCoord")),
                f("max", r("VoxelAssetCoord")),
            ],
        ),
        iface(
            "Grid placement metadata for stored voxel cells.",
            "VoxelAssetGrid",
            vec![
                f("origin", vec3()),
                f("cellSize", num()),
                f("coordinateSystem", string()),
            ],
        ),
        iface(
            "One compact voxel-material binding to a catalog material asset.",
            "VoxelAssetMaterialBinding",
            vec![f("voxelMaterial", num()), f("materialAssetId", string())],
        ),
        iface(
            "One run of solid voxels along +X. Absence is empty space.",
            "VoxelAssetSparseRun",
            vec![
                f("start", r("VoxelAssetCoord")),
                f("length", num()),
                f("material", num()),
            ],
        ),
        iface(
            "Stored voxel representation payload.",
            "VoxelAssetRepresentation",
            vec![
                f("kind", r("VoxelAssetRepresentationKind")),
                f("sparseRuns", TsType::array(r("VoxelAssetSparseRun"))),
            ],
        ),
        iface(
            "Provenance/evidence reference for stored voxel assets.",
            "VoxelAssetProvenanceRef",
            vec![
                f("kind", r("VoxelAssetProvenanceKind")),
                f("uri", string()),
                f("contentHash", string()),
            ],
        ),
        iface(
            "Human/editor metadata that never owns runtime authority.",
            "VoxelAssetAuthoringMetadata",
            vec![
                f("label", TsType::nullable(string())),
                f("createdBy", TsType::nullable(string())),
                f("sourceTool", TsType::nullable(string())),
            ],
        ),
        iface(
            "Canonical hashes recorded with the stored asset.",
            "VoxelAssetContentHashes",
            vec![f("canonicalJson", string()), f("voxelData", string())],
        ),
        iface(
            "One classified validation diagnostic for a stored voxel-volume asset.",
            "VoxelAssetDiagnostic",
            vec![
                f("code", r("VoxelAssetDiagnosticCode")),
                f("severity", r("DiagnosticSeverity")),
                f("reference", string()),
                f("message", string()),
            ],
        ),
        iface(
            "Per-material voxel count for stored/runtime voxel asset readbacks.",
            "VoxelAssetMaterialCount",
            vec![f("material", num()), f("voxelCount", num())],
        ),
        iface(
            "A complete Asha-native stored voxel-volume asset.",
            "VoxelVolumeAsset",
            vec![
                f("assetId", string()),
                f("schemaVersion", num()),
                f("mediaType", string()),
                f("grid", r("VoxelAssetGrid")),
                f("bounds", r("VoxelAssetBounds")),
                f("representation", r("VoxelAssetRepresentation")),
                f(
                    "materialPalette",
                    TsType::array(r("VoxelAssetMaterialBinding")),
                ),
                f("provenance", TsType::array(r("VoxelAssetProvenanceRef"))),
                f("authoring", r("VoxelAssetAuthoringMetadata")),
                f(
                    "validationDiagnostics",
                    TsType::array(r("VoxelAssetDiagnostic")),
                ),
                f("contentHashes", r("VoxelAssetContentHashes")),
            ],
        ),
        iface(
            "Request to export a resident runtime voxel model into stored asset form.",
            "VoxelVolumeAssetExportRequest",
            vec![
                f("grid", num()),
                f("volumeAssetId", TsType::nullable(string())),
                f("targetAssetId", string()),
                f("label", TsType::nullable(string())),
                f("createdBy", TsType::nullable(string())),
                f("sourceTool", TsType::nullable(string())),
                f("maxSparseRuns", num()),
                f("expectedSessionHash", TsType::nullable(string())),
            ],
        ),
        iface(
            "Receipt for explicit runtime-to-stored voxel asset export.",
            "VoxelVolumeAssetExportReceipt",
            vec![
                f("request", r("VoxelVolumeAssetExportRequest")),
                f("exported", boolean()),
                f("asset", TsType::nullable(r("VoxelVolumeAsset"))),
                f("canonicalJson", TsType::nullable(string())),
                f("canonicalJsonHash", TsType::nullable(string())),
                f("voxelDataHash", TsType::nullable(string())),
                f("diagnostics", TsType::array(r("VoxelAssetDiagnostic"))),
            ],
        ),
        iface(
            "Explicit request to load a validated stored voxel-volume asset into runtime.",
            "VoxelVolumeAssetLoadRequest",
            vec![
                f("asset", r("VoxelVolumeAsset")),
                f("targetGrid", num()),
                f("targetVolumeAssetId", TsType::nullable(string())),
                f("replaceExisting", boolean()),
                f("includeMaterialCounts", boolean()),
            ],
        ),
        iface(
            "Receipt/readback for loading a stored voxel-volume asset into runtime.",
            "VoxelVolumeAssetLoadReceipt",
            vec![
                f("requestAssetId", string()),
                f("loaded", boolean()),
                f("modelId", string()),
                f("volumeAssetId", TsType::nullable(string())),
                f("grid", num()),
                f("bounds", TsType::nullable(r("VoxelAssetBounds"))),
                f("voxelCount", num()),
                f(
                    "materialCounts",
                    TsType::array(r("VoxelAssetMaterialCount")),
                ),
                f("provenance", TsType::array(r("VoxelAssetProvenanceRef"))),
                f("canonicalJsonHash", TsType::nullable(string())),
                f("voxelDataHash", TsType::nullable(string())),
                f("sessionHash", string()),
                f("replayHash", string()),
                f("diagnostics", TsType::array(r("VoxelAssetDiagnostic"))),
            ],
        ),
    ];

    Module {
        name: "voxelAsset",
        imports,
        items,
    }
}

// ── gameRules.ts — generic effect/modifier catalog DTOs ──────────────────────
//
// Mirrors the border-only `protocol-game-rules` crate. Game rules authority and
// resolution live in Rust services/rule crates; this generated surface only
// carries catalogs, requests, receipts, diagnostics, traces, and evidence refs.

pub fn game_rules_module() -> Module {
    let imports = vec![
        import("./ids.js", &["EntityId"]),
        import("./diagnostics.js", &["DiagnosticSeverity"]),
    ];

    let items = vec![
        string_enum(
            "Stable kind tags for generic game-rule effect operations.",
            "GameRuleEffectOpKind",
            protocol_game_rules::GAME_RULE_EFFECT_OP_KINDS,
        ),
        string_enum(
            "Stable kind tags for modifier stack policies.",
            "GameRuleStackPolicyKind",
            protocol_game_rules::GAME_RULE_STACK_POLICIES,
        ),
        string_enum(
            "Stable classified diagnostic/error code for game-rule catalogs and resolution.",
            "GameRuleDiagnosticCode",
            protocol_game_rules::GAME_RULE_DIAGNOSTIC_CODES,
        ),
        string_enum(
            "Role of an exported game-rule evidence artifact.",
            "GameRuleEvidenceKind",
            protocol_game_rules::GAME_RULE_EVIDENCE_KINDS,
        ),
        iface(
            "Catalog identity and immutable content/version evidence.",
            "GameRuleCatalogRef",
            vec![
                f("catalogId", string()),
                f("version", string()),
                f("contentHash", string()),
            ],
        ),
        iface(
            "A declared value channel, such as health or stamina.",
            "GameRuleValueChannelRef",
            vec![
                f("channelId", string()),
                f("displayName", TsType::nullable(string())),
            ],
        ),
        iface(
            "A bounded runtime value snapshot for one channel.",
            "GameRuleBoundedValue",
            vec![
                f("channelId", string()),
                f("min", num()),
                f("current", num()),
                f("max", num()),
            ],
        ),
        iface(
            "A pending value delta emitted by resolution.",
            "GameRuleValueDelta",
            vec![f("channelId", string()), f("amount", num())],
        ),
        union(
            "Duration policy for a modifier or effect.",
            "GameRuleDuration",
            "kind",
            vec![
                v("instant", vec![]),
                v("ticks", vec![f("ticks", num())]),
                v("infinite", vec![]),
            ],
        ),
        iface(
            "Periodic tick cadence for scheduled effects.",
            "GameRuleTickCadence",
            vec![f("periodTicks", num())],
        ),
        union(
            "How repeated applications of a modifier combine.",
            "GameRuleStackPolicy",
            "kind",
            vec![
                v("refresh", vec![]),
                v("stack", vec![f("maxStacks", num())]),
                v("rejectDuplicate", vec![]),
                v("replaceIfStronger", vec![]),
            ],
        ),
        union(
            "Generic effect operation IR for authored action/effect bundles.",
            "GameRuleEffectOp",
            "kind",
            vec![
                v(
                    "applyDelta",
                    vec![
                        f("opId", string()),
                        f("channelId", string()),
                        f("amount", num()),
                        f("tags", TsType::array(string())),
                    ],
                ),
                v(
                    "restore",
                    vec![
                        f("opId", string()),
                        f("channelId", string()),
                        f("amount", num()),
                        f("tags", TsType::array(string())),
                    ],
                ),
                v(
                    "spend",
                    vec![
                        f("opId", string()),
                        f("channelId", string()),
                        f("amount", num()),
                        f("tags", TsType::array(string())),
                    ],
                ),
                v(
                    "grant",
                    vec![
                        f("opId", string()),
                        f("channelId", string()),
                        f("amount", num()),
                        f("tags", TsType::array(string())),
                    ],
                ),
                v(
                    "applyModifier",
                    vec![
                        f("opId", string()),
                        f("modifierId", string()),
                        f("tags", TsType::array(string())),
                    ],
                ),
                v(
                    "removeModifier",
                    vec![
                        f("opId", string()),
                        f("modifierId", string()),
                        f("tags", TsType::array(string())),
                    ],
                ),
                v(
                    "schedulePeriodicEffect",
                    vec![
                        f("opId", string()),
                        f("modifierId", string()),
                        f("cadence", r("GameRuleTickCadence")),
                        f("duration", r("GameRuleDuration")),
                        f("tags", TsType::array(string())),
                    ],
                ),
                v(
                    "cancelResolution",
                    vec![
                        f("opId", string()),
                        f("reason", string()),
                        f("tags", TsType::array(string())),
                    ],
                ),
                v(
                    "emitTrace",
                    vec![
                        f("opId", string()),
                        f("code", string()),
                        f("message", string()),
                        f("tags", TsType::array(string())),
                    ],
                ),
            ],
        ),
        iface(
            "One modifier definition in a game-rule catalog.",
            "GameRuleModifierDefinition",
            vec![
                f("modifierId", string()),
                f("stackPolicy", r("GameRuleStackPolicy")),
                f("duration", r("GameRuleDuration")),
                f("tickCadence", TsType::nullable(r("GameRuleTickCadence"))),
                f("tags", TsType::array(string())),
                f("effectOpIds", TsType::array(string())),
                f("sourceHash", string()),
            ],
        ),
        iface(
            "Authored action/effect bundle expressed through generic effect operations.",
            "GameRuleEffectBundle",
            vec![
                f("bundleId", string()),
                f("effectOps", TsType::array(r("GameRuleEffectOp"))),
                f("modifiers", TsType::array(r("GameRuleModifierDefinition"))),
                f("tags", TsType::array(string())),
                f("sourceHash", string()),
            ],
        ),
        iface(
            "Complete game-rule catalog DTO.",
            "GameRuleCatalog",
            vec![
                f("catalog", r("GameRuleCatalogRef")),
                f("valueChannels", TsType::array(r("GameRuleValueChannelRef"))),
                f("bundles", TsType::array(r("GameRuleEffectBundle"))),
            ],
        ),
        iface(
            "One classified catalog or resolution diagnostic.",
            "GameRuleDiagnostic",
            vec![
                f("code", r("GameRuleDiagnosticCode")),
                f("severity", r("DiagnosticSeverity")),
                f("path", string()),
                f("message", string()),
            ],
        ),
        iface(
            "Reference to an inspectable game-rule artifact.",
            "GameRuleEvidenceRef",
            vec![
                f("kind", r("GameRuleEvidenceKind")),
                f("uri", string()),
                f("contentHash", string()),
            ],
        ),
        iface(
            "One structured trace key/value pair.",
            "GameRuleTraceRef",
            vec![f("key", string()), f("value", string())],
        ),
        iface(
            "One ordered resolution trace entry.",
            "GameRuleTraceEntry",
            vec![
                f("step", num()),
                f("code", string()),
                f("message", string()),
                f("refs", TsType::array(r("GameRuleTraceRef"))),
            ],
        ),
        iface(
            "Runtime readout for one applied modifier.",
            "GameRuleModifierState",
            vec![
                f("modifierId", string()),
                f("source", r("EntityId")),
                f("target", r("EntityId")),
                f("stacks", num()),
                f("appliedTick", num()),
                f("expiresTick", TsType::nullable(num())),
                f("nextTick", TsType::nullable(num())),
                f("sourceHash", string()),
            ],
        ),
        iface(
            "Request to resolve one authored effect bundle against source and target entities.",
            "GameRuleResolutionRequest",
            vec![
                f("catalog", r("GameRuleCatalogRef")),
                f("bundleId", string()),
                f("source", r("EntityId")),
                f("target", r("EntityId")),
                f("values", TsType::array(r("GameRuleBoundedValue"))),
                f("tick", num()),
            ],
        ),
        iface(
            "Resolution receipt carrying authority-ready deltas, modifier readouts, diagnostics, traces, and replay evidence.",
            "GameRuleResolutionReceipt",
            vec![
                f("accepted", boolean()),
                f("requestHash", string()),
                f("pendingValueDeltas", TsType::array(r("GameRuleValueDelta"))),
                f("appliedModifiers", TsType::array(r("GameRuleModifierState"))),
                f("diagnostics", TsType::array(r("GameRuleDiagnostic"))),
                f("trace", TsType::array(r("GameRuleTraceEntry"))),
                f("evidence", TsType::array(r("GameRuleEvidenceRef"))),
                f("replayHash", string()),
            ],
        ),
    ];

    Module {
        name: "gameRules",
        imports,
        items,
    }
}

// ── gameExtension.ts — downstream Rust rule module extension DTOs ────────────
//
// Mirrors the border-only `protocol-game-extension` crate. RuntimeSession
// invocation and authority validation live in later runtime/rule tasks; this
// generated surface carries manifests, deterministic hook requests, typed
// proposals, receipts, diagnostics, and replay evidence.

pub fn game_extension_module() -> Module {
    let imports = vec![
        import("./ids.js", &["EntityId"]),
        import("./diagnostics.js", &["DiagnosticSeverity"]),
    ];

    let items = vec![
        string_enum(
            "Stable extension hook kinds a game-owned Rust module may declare.",
            "GameExtensionHookKind",
            protocol_game_extension::GAME_EXTENSION_HOOK_KINDS,
        ),
        string_enum(
            "Stable proposal kinds returned by game-owned Rust modules.",
            "GameExtensionProposalKind",
            protocol_game_extension::GAME_EXTENSION_PROPOSAL_KINDS,
        ),
        string_enum(
            "Status of a game-owned Rust hook receipt before RuntimeSession validation.",
            "GameExtensionReceiptStatus",
            protocol_game_extension::GAME_EXTENSION_RECEIPT_STATUSES,
        ),
        string_enum(
            "Stable diagnostic code for game-owned Rust extension manifests and hook output.",
            "GameExtensionDiagnosticCode",
            protocol_game_extension::GAME_EXTENSION_DIAGNOSTIC_CODES,
        ),
        iface(
            "Compiled game rule module identity and compatibility evidence.",
            "GameRuleModuleRef",
            vec![
                f("moduleId", string()),
                f("version", string()),
                f("contractHash", string()),
            ],
        ),
        iface(
            "One deterministic hook declared by a compiled game rule module.",
            "GameRuleHookDeclaration",
            vec![
                f("hookId", string()),
                f("kind", r("GameExtensionHookKind")),
                f("inputContract", string()),
                f("outputContract", string()),
                f("requiredCapabilities", TsType::array(string())),
            ],
        ),
        iface(
            "Manifest compiled with a downstream game-owned Rust rule module.",
            "GameRuleModuleManifest",
            vec![
                f("moduleRef", r("GameRuleModuleRef")),
                f("declaredHooks", TsType::array(r("GameRuleHookDeclaration"))),
                f("deterministicRequirements", TsType::array(string())),
                f("sourceHash", string()),
            ],
        ),
        iface(
            "One classified extension manifest or proposal diagnostic.",
            "GameExtensionDiagnostic",
            vec![
                f("code", r("GameExtensionDiagnosticCode")),
                f("severity", r("DiagnosticSeverity")),
                f("path", string()),
                f("message", string()),
            ],
        ),
        iface(
            "Deterministic weapon-effect hook input supplied by ASHA RuntimeSession.",
            "WeaponEffectHookRequest",
            vec![
                f("moduleRef", r("GameRuleModuleRef")),
                f("hookId", string()),
                f("requestId", string()),
                f("tick", num()),
                f("source", r("EntityId")),
                f("target", TsType::nullable(r("EntityId"))),
                f("baseDamage", num()),
                f("rangeMillimeters", num()),
                f("tags", TsType::array(string())),
                f("inputHash", string()),
            ],
        ),
        union(
            "Typed pending proposal returned by a game-owned Rust rule module.",
            "GameExtensionProposal",
            "kind",
            vec![
                v(
                    "damageModifier",
                    vec![
                        f("proposalId", string()),
                        f("target", r("EntityId")),
                        f("channelId", string()),
                        f("amountDelta", num()),
                        f("tags", TsType::array(string())),
                        f("proposalHash", string()),
                    ],
                ),
                v(
                    "effectBundle",
                    vec![
                        f("proposalId", string()),
                        f("bundleId", string()),
                        f("tags", TsType::array(string())),
                        f("proposalHash", string()),
                    ],
                ),
                v(
                    "reject",
                    vec![
                        f("proposalId", string()),
                        f("code", r("GameExtensionDiagnosticCode")),
                        f("message", string()),
                        f("proposalHash", string()),
                    ],
                ),
                v(
                    "noop",
                    vec![f("proposalId", string()), f("proposalHash", string())],
                ),
            ],
        ),
        iface(
            "One deterministic trace entry emitted by a game-owned Rust hook.",
            "GameExtensionTraceEntry",
            vec![
                f("step", num()),
                f("code", string()),
                f("message", string()),
                f("refs", TsType::array(string())),
            ],
        ),
        iface(
            "Hook receipt before RuntimeSession validates/applies any proposal.",
            "GameExtensionHookReceipt",
            vec![
                f("moduleRef", r("GameRuleModuleRef")),
                f("hookId", string()),
                f("requestId", string()),
                f("status", r("GameExtensionReceiptStatus")),
                f("inputHash", string()),
                f("proposal", TsType::nullable(r("GameExtensionProposal"))),
                f("diagnostics", TsType::array(r("GameExtensionDiagnostic"))),
                f("trace", TsType::array(r("GameExtensionTraceEntry"))),
                f("proposalHash", string()),
            ],
        ),
        iface(
            "Replay evidence tying a hook invocation to validation and authority output hashes.",
            "GameExtensionReplayEvidence",
            vec![
                f("moduleRef", r("GameRuleModuleRef")),
                f("hookId", string()),
                f("requestId", string()),
                f("inputHash", string()),
                f("proposalHash", string()),
                f("validationStatus", string()),
                f("eventHashes", TsType::array(string())),
                f("rejectionHashes", TsType::array(string())),
                f("replayHash", string()),
            ],
        ),
    ];

    Module {
        name: "gameExtension",
        imports,
        items,
    }
}

// ── diagnostics.ts — scene/asset/bundle/render diagnostic reports ─────────────
//
// Mirrors `protocol-diagnostics`. The string-enum members are sourced directly
// from that crate's canonical tables (`DIAGNOSTIC_*`/`REMEDY_ACTIONS`) so the
// codes have a single home and drift is impossible; the report/trace/resource
// shapes are described by hand to match the Rust structs.

fn string_enum(doc: &str, name: &str, values: &[&str]) -> Item {
    Item::Alias {
        doc: doc.to_string(),
        name: name.to_string(),
        ty: TsType::StringEnum(values.iter().map(|s| s.to_string()).collect()),
    }
}

pub fn diagnostics_module() -> Module {
    let tuple3 = || TsType::Tuple(vec![num(), num(), num()]);

    let items = vec![
        string_enum(
            "How serious a diagnostic is, and which recovery path applies (only 'fatal' blocks a load).",
            "DiagnosticSeverity",
            protocol_diagnostics::DIAGNOSTIC_SEVERITIES,
        ),
        string_enum(
            "Which subsystem / lane a diagnostic belongs to.",
            "DiagnosticScope",
            protocol_diagnostics::DIAGNOSTIC_SCOPES,
        ),
        string_enum(
            "A stable, machine-routable diagnostic code. The string form is a contract.",
            "DiagnosticCode",
            protocol_diagnostics::DIAGNOSTIC_CODES,
        ),
        string_enum(
            "A suggested next action (advisory only — diagnostics never authorize mutation).",
            "RemedyAction",
            protocol_diagnostics::REMEDY_ACTIONS,
        ),
        iface(
            "A suggested remedy: a categorized action plus a human-readable detail.",
            "SuggestedRemedy",
            vec![f("action", r("RemedyAction")), f("detail", string())],
        ),
        iface(
            "Where a diagnostic points in authority terms; absent hops are null.",
            "DiagnosticSourceRef",
            vec![
                f("sceneNodeId", TsType::nullable(num())),
                f("runtimeEntityId", TsType::nullable(num())),
                f("assetId", TsType::nullable(string())),
                f("chunkCoord", TsType::nullable(tuple3())),
                f("renderHandle", TsType::nullable(num())),
                f("bundlePath", TsType::nullable(string())),
            ],
        ),
        iface(
            "One structured diagnostic: scope + severity + stable code + locus + remedy.",
            "DiagnosticReport",
            vec![
                f("scope", r("DiagnosticScope")),
                f("severity", r("DiagnosticSeverity")),
                f("code", r("DiagnosticCode")),
                f("reference", string()),
                f("source", r("DiagnosticSourceRef")),
                f("message", string()),
                f("remedy", TsType::nullable(r("SuggestedRemedy"))),
            ],
        ),
        iface(
            "A collection of diagnostic reports.",
            "DiagnosticReportSet",
            vec![f("reports", TsType::array(r("DiagnosticReport")))],
        ),
        iface(
            "A render-handle to scene-node to entity to asset trace; absent hops are null.",
            "SourceTrace",
            vec![
                f("renderHandle", num()),
                f("sceneNodeId", TsType::nullable(num())),
                f("runtimeEntityId", TsType::nullable(num())),
                f("assetId", TsType::nullable(string())),
                f("assetResolved", boolean()),
            ],
        ),
        iface(
            "An observational snapshot of renderer resource usage (counts only).",
            "RendererResourceReport",
            vec![
                f("liveHandles", num()),
                f("geometries", num()),
                f("materials", num()),
                f("spriteInstances", num()),
                f("spritesUpdatedLastTick", num()),
                f("resourcesCreated", num()),
                f("resourcesDisposed", num()),
                f("fallbackMaterials", num()),
            ],
        ),
    ];

    Module {
        name: "diagnostics",
        imports: vec![],
        items,
    }
}

pub fn scene_module() -> Module {
    let imports = vec![import("./ids.js", &["EntityId"])];

    let tuple3 = || TsType::Tuple(vec![num(), num(), num()]);
    let tuple4 = || TsType::Tuple(vec![num(), num(), num(), num()]);

    let items = vec![
        Item::BrandedId {
            doc: "Stable identifier for an authored, loadable scene document.".to_string(),
            name: "SceneId".to_string(),
        },
        Item::BrandedId {
            doc: "Stable identifier for a live runtime world bootstrapped from a scene.".to_string(),
            name: "WorldId".to_string(),
        },
        Item::BrandedId {
            doc: "Stable identifier for one node within a scene document (never a render handle)."
                .to_string(),
            name: "SceneNodeId".to_string(),
        },
        string_enum(
            "Stable tag for a scene-node kind. Asset-backed kinds carry an AssetReference.",
            "SceneNodeKindTag",
            protocol_scene::SCENE_NODE_KIND_TAGS,
        ),
        string_enum(
            "Stable classified scene-validation code. The string form is a contract.",
            "SceneValidationCode",
            protocol_scene::SCENE_VALIDATION_CODES,
        ),
        string_enum(
            "Stable classified scene-object command rejection code. The string form is a contract.",
            "SceneObjectCommandRejectionCode",
            protocol_scene::SCENE_OBJECT_COMMAND_REJECTION_CODES,
        ),
        union(
            "An asset version requirement.",
            "AssetVersionReq",
            "req",
            vec![
                v("any", vec![]),
                v("exact", vec![f("value", num())]),
                v("atLeast", vec![f("value", num())]),
            ],
        ),
        iface(
            "A kind-erased reference to an authored asset.",
            "AssetReference",
            vec![
                f("id", string()),
                f("version", r("AssetVersionReq")),
                f("hash", TsType::nullable(string())),
            ],
        ),
        iface(
            "A scene node's initial transform (authority owns runtime transforms after bootstrap).",
            "SceneTransform",
            vec![
                f("translation", tuple3()),
                f("rotation", tuple4()),
                f("scale", tuple3()),
            ],
        ),
        union(
            "A scene node's kind. Only asset-backed kinds carry an AssetReference; \
             the discriminant values are the SceneNodeKindTag vocabulary.",
            "SceneNodeKind",
            "kind",
            vec![
                v("emptyGroup", vec![]),
                v("staticMesh", vec![f("asset", r("AssetReference"))]),
                v("sprite", vec![f("asset", r("AssetReference"))]),
                v("voxelVolume", vec![f("asset", r("AssetReference"))]),
            ],
        ),
        iface(
            "One node in the canonical flat scene document.",
            "SceneNodeRecord",
            vec![
                f("id", r("SceneNodeId")),
                f("parent", TsType::nullable(r("SceneNodeId"))),
                f("childOrder", num()),
                f("label", TsType::nullable(string())),
                f("tags", TsType::array(string())),
                f("transform", r("SceneTransform")),
                f("kind", r("SceneNodeKind")),
            ],
        ),
        iface(
            "Document-level scene metadata (never affects authority semantics).",
            "SceneMetadata",
            vec![
                f("name", TsType::nullable(string())),
                f("authoringFormatVersion", num()),
            ],
        ),
        iface(
            "The canonical flat scene document: the form TS authors and Rust validates.",
            "FlatSceneDocument",
            vec![
                f("schemaVersion", num()),
                f("id", r("SceneId")),
                f("metadata", r("SceneMetadata")),
                f("dependencies", TsType::array(r("AssetReference"))),
                f("nodes", TsType::array(r("SceneNodeRecord"))),
            ],
        ),
        iface(
            "One classified scene-validation failure; absent loci are null.",
            "SceneValidationError",
            vec![
                f("code", r("SceneValidationCode")),
                f("node", TsType::nullable(r("SceneNodeId"))),
                f("parent", TsType::nullable(r("SceneNodeId"))),
                f("expectedKind", TsType::nullable(string())),
                f("actualKind", TsType::nullable(string())),
                f("transformReason", TsType::nullable(string())),
                f("cyclePath", TsType::array(r("SceneNodeId"))),
            ],
        ),
        iface(
            "A full scene-validation report: every classified error.",
            "SceneValidationReport",
            vec![f("errors", TsType::array(r("SceneValidationError")))],
        ),
        iface(
            "One canonical scene object projected from a flat scene document.",
            "SceneObjectRecord",
            vec![
                f("id", r("SceneNodeId")),
                f("parent", TsType::nullable(r("SceneNodeId"))),
                f("childOrder", num()),
                f("label", TsType::nullable(string())),
                f("kind", r("SceneNodeKindTag")),
                f("hasRenderableAsset", boolean()),
            ],
        ),
        iface(
            "A deterministic scene-object hierarchy snapshot.",
            "SceneObjectSnapshot",
            vec![
                f("documentHash", num()),
                f("objects", TsType::array(r("SceneObjectRecord"))),
            ],
        ),
        union(
            "Explicit scene-object hierarchy command.",
            "SceneObjectCommand",
            "kind",
            vec![
                v("create", vec![f("record", r("SceneNodeRecord"))]),
                v("delete", vec![f("id", r("SceneNodeId"))]),
                v(
                    "rename",
                    vec![
                        f("id", r("SceneNodeId")),
                        f("label", TsType::nullable(string())),
                    ],
                ),
                v(
                    "reparent",
                    vec![
                        f("id", r("SceneNodeId")),
                        f("parent", TsType::nullable(r("SceneNodeId"))),
                        f("childOrder", num()),
                    ],
                ),
                v(
                    "translate",
                    vec![
                        f("id", r("SceneNodeId")),
                        f("delta", tuple3()),
                    ],
                ),
                v(
                    "rotate",
                    vec![
                        f("id", r("SceneNodeId")),
                        f("rotation", tuple4()),
                    ],
                ),
                v("select", vec![f("id", TsType::nullable(r("SceneNodeId")))]),
            ],
        ),
        iface(
            "A classified scene-object command rejection.",
            "SceneObjectCommandRejection",
            vec![
                f("code", r("SceneObjectCommandRejectionCode")),
                f("id", TsType::nullable(r("SceneNodeId"))),
                f("parent", TsType::nullable(r("SceneNodeId"))),
                f("expectedHash", TsType::nullable(num())),
                f("actualHash", TsType::nullable(num())),
                f("validationErrors", TsType::array(r("SceneValidationError"))),
            ],
        ),
        iface(
            "A successful scene-object command application.",
            "SceneObjectCommandOutcome",
            vec![
                f("document", r("FlatSceneDocument")),
                f("snapshot", r("SceneObjectSnapshot")),
                f("selected", TsType::nullable(r("SceneNodeId"))),
            ],
        ),
        iface(
            "One-in request envelope for applying a scene-object command.",
            "SceneObjectCommandRequest",
            vec![
                f("expectedDocumentHash", num()),
                f("command", r("SceneObjectCommand")),
            ],
        ),
        iface(
            "One-out result envelope for applying a scene-object command.",
            "SceneObjectCommandResult",
            vec![
                f("accepted", boolean()),
                f("outcome", TsType::nullable(r("SceneObjectCommandOutcome"))),
                f("rejection", TsType::nullable(r("SceneObjectCommandRejection"))),
            ],
        ),
        iface(
            "One hop in the scene-node to runtime-entity source trace.",
            "SceneSourceTrace",
            vec![
                f("sceneNodeId", r("SceneNodeId")),
                f("runtimeEntityId", r("EntityId")),
            ],
        ),
        iface(
            "The atomic bootstrap record: the single replay/audit unit of a scene to authority init.",
            "BootstrapRecord",
            vec![
                f("sceneId", r("SceneId")),
                f("worldId", r("WorldId")),
                f("schemaVersion", num()),
                f("nodeCount", num()),
                f("entityCount", num()),
                f("worldHash", num()),
                f("sourceTrace", TsType::array(r("SceneSourceTrace"))),
            ],
        ),
    ];

    Module {
        name: "scene",
        imports,
        items,
    }
}

pub fn world_bundle_module() -> Module {
    let imports = vec![
        import("./scene.js", &["SceneId", "WorldId"]),
        import("./voxel.js", &["VoxelCoord", "VoxelValue"]),
    ];

    let items = vec![
        string_enum(
            "An artifact's persistence guarantee.",
            "ArtifactClass",
            protocol_world_bundle::ARTIFACT_CLASSES,
        ),
        string_enum(
            "The artifact roles this build names. The wire role is an open string; \
             unknown roles are carried verbatim. This is the known vocabulary for display.",
            "KnownArtifactRole",
            protocol_world_bundle::KNOWN_ARTIFACT_ROLES,
        ),
        string_enum(
            "A stage in the canonical, ordered authority load sequence.",
            "LoadStage",
            protocol_world_bundle::LOAD_STAGES,
        ),
        string_enum(
            "What to do about an edit whose generated context changed under a new generator.",
            "SuggestedAction",
            protocol_world_bundle::SUGGESTED_ACTIONS,
        ),
        iface(
            "One row of the manifest artifact table. `role` is an open string (see KnownArtifactRole).",
            "ArtifactEntry",
            vec![
                f("path", string()),
                f("class", r("ArtifactClass")),
                f("role", string()),
                f("contentHash", TsType::nullable(string())),
            ],
        ),
        iface(
            "Terrain generator provenance.",
            "GeneratorMetadata",
            vec![
                f("seed", num()),
                f("version", num()),
                f("params", string()),
            ],
        ),
        iface(
            "The world section of a bundle manifest.",
            "WorldSection",
            vec![f("id", r("WorldId")), f("name", TsType::nullable(string()))],
        ),
        iface(
            "The scene section of a bundle manifest.",
            "SceneSection",
            vec![
                f("id", r("SceneId")),
                f("schemaVersion", num()),
                f("artifact", string()),
            ],
        ),
        iface(
            "The asset-lock section of a bundle manifest.",
            "AssetLockSection",
            vec![f("artifact", string()), f("assetCount", num())],
        ),
        iface(
            "The inspectable world-bundle manifest: identity, versions, and the artifact table.",
            "WorldBundleManifest",
            vec![
                f("bundleSchemaVersion", num()),
                f("protocolVersion", num()),
                f("world", r("WorldSection")),
                f("scene", r("SceneSection")),
                f("assetLock", r("AssetLockSection")),
                f("generator", r("GeneratorMetadata")),
                f("artifacts", TsType::array(r("ArtifactEntry"))),
            ],
        ),
        union(
            "One classified manifest-validation / version-compatibility failure.",
            "ManifestError",
            "code",
            vec![
                v(
                    "unsupportedSchema",
                    vec![f("found", num()), f("supported", num())],
                ),
                v(
                    "unsupportedProtocol",
                    vec![f("found", num()), f("supported", num())],
                ),
                v("duplicateArtifact", vec![f("path", string())]),
                v("missingArtifact", vec![f("role", string()), f("path", string())]),
                v("durableMissingHash", vec![f("path", string())]),
            ],
        ),
        iface(
            "A manifest validation / version-compatibility report: every classified error.",
            "ManifestValidationReport",
            vec![f("errors", TsType::array(r("ManifestError")))],
        ),
        union(
            "One ordered step of a load plan, carrying the typed inputs it consumes.",
            "LoadStep",
            "step",
            vec![
                v(
                    "validateVersions",
                    vec![f("bundleSchemaVersion", num()), f("protocolVersion", num())],
                ),
                v(
                    "loadAssetLock",
                    vec![f("artifact", string()), f("assetCount", num())],
                ),
                v(
                    "loadSceneDocument",
                    vec![f("artifact", string()), f("scene", r("SceneId"))],
                ),
                v(
                    "generateTerrain",
                    vec![f("seed", num()), f("version", num()), f("params", string())],
                ),
                v(
                    "applyVoxelEdits",
                    vec![
                        f("editLogs", TsType::array(string())),
                        f("snapshots", TsType::array(string())),
                    ],
                ),
                v(
                    "bootstrapScene",
                    vec![f("scene", r("SceneId")), f("world", r("WorldId"))],
                ),
                v("restoreWorldState", vec![f("artifact", string())]),
                v("validateFinalState", vec![]),
            ],
        ),
        iface(
            "A deterministic, ordered authority load plan.",
            "LoadPlan",
            vec![f("steps", TsType::array(r("LoadStep")))],
        ),
        union(
            "Why a load plan could not be built or verified.",
            "LoadPlanError",
            "code",
            vec![
                v("manifest", vec![f("error", r("ManifestError"))]),
                v("missingPrerequisiteArtifact", vec![f("role", string())]),
                v(
                    "outOfOrder",
                    vec![f("step", r("LoadStage")), f("after", r("LoadStage"))],
                ),
                v("missingStage", vec![f("stage", r("LoadStage"))]),
            ],
        ),
        iface(
            "Save-time compaction summary: how many edits were folded vs retained.",
            "CompactionSummary",
            vec![
                f("compactedEdits", num()),
                f("retainedEdits", num()),
                f("snapshotChunks", TsType::array(string())),
            ],
        ),
        iface(
            "A save summary: the artifacts written plus the compaction outcome.",
            "SaveSummary",
            vec![
                f("writes", TsType::array(r("ArtifactEntry"))),
                f("compaction", r("CompactionSummary")),
            ],
        ),
        iface(
            "A fail-closed generator version mismatch between a save and the current build.",
            "GeneratorMismatch",
            vec![f("savedVersion", num()), f("currentVersion", num())],
        ),
        iface(
            "One edit whose authored generated context changed under a new generator.",
            "EditConflict",
            vec![
                f("eventId", num()),
                f("coord", r("VoxelCoord")),
                f("oldGenerated", r("VoxelValue")),
                f("newGenerated", r("VoxelValue")),
                f("editValue", r("VoxelValue")),
                f("suggested", r("SuggestedAction")),
            ],
        ),
        iface(
            "The outcome of a regenerate-and-replay diagnostic (never rewrites a save).",
            "RegenConflictReport",
            vec![
                f("savedVersion", num()),
                f("newVersion", num()),
                f("conflicts", TsType::array(r("EditConflict"))),
                f("replayedEdits", num()),
                f("stagingWorldHash", num()),
            ],
        ),
    ];

    Module {
        name: "worldBundle",
        imports,
        items,
    }
}

pub fn assets_module() -> Module {
    let imports = vec![import("./scene.js", &["AssetReference"])];

    let items = vec![
        string_enum(
            "Stable kind-prefix tag for an asset kind.",
            "AssetKind",
            protocol_assets::ASSET_KINDS,
        ),
        string_enum(
            "Structural role for authority/collision (no visual meaning).",
            "StructuralClass",
            protocol_assets::STRUCTURAL_CLASSES,
        ),
        string_enum(
            "How a material samples colour across geometry (visual only).",
            "UvStrategy",
            protocol_assets::UV_STRATEGIES,
        ),
        string_enum(
            "Stable classified catalog-validation code.",
            "CatalogValidationCode",
            protocol_assets::CATALOG_VALIDATION_CODES,
        ),
        string_enum(
            "Stable classified asset-lock issue code.",
            "LockIssueCode",
            protocol_assets::LOCK_ISSUE_CODES,
        ),
        string_enum(
            "The context a missing asset is used in (dominates fallback policy).",
            "FallbackContext",
            protocol_assets::FALLBACK_CONTEXTS,
        ),
        string_enum(
            "A concrete debug placeholder a fallback resolves to.",
            "FallbackVisual",
            protocol_assets::FALLBACK_VISUALS,
        ),
        iface(
            "A linear RGBA colour (0..=1 per channel).",
            "Rgba",
            vec![f("r", num()), f("g", num()), f("b", num()), f("a", num())],
        ),
        iface(
            "The renderer-facing projection of a material. NO collision class.",
            "RenderMaterial",
            vec![
                f("color", r("Rgba")),
                f("texture", TsType::nullable(r("AssetReference"))),
                f("roughness", num()),
                f("emissive", num()),
                f("uvStrategy", r("UvStrategy")),
            ],
        ),
        iface(
            "The collision/authority-facing projection of a material. NO texture or colour.",
            "CollisionMaterial",
            vec![
                f("solid", boolean()),
                f("collidable", boolean()),
                f("occludes", boolean()),
                f("structuralClass", r("StructuralClass")),
            ],
        ),
        iface(
            "A read-only devtools bundle of both disjoint material projections. The pure \
             render path consumes only `render`; authority consumes only `collision`.",
            "MaterialProjection",
            vec![
                f("render", r("RenderMaterial")),
                f("collision", r("CollisionMaterial")),
            ],
        ),
        iface(
            "One catalog entry. `material` is present only for material-kind assets.",
            "CatalogEntry",
            vec![
                f("id", string()),
                f("kind", r("AssetKind")),
                f("version", num()),
                f("hash", TsType::nullable(string())),
                f("sourcePath", TsType::nullable(string())),
                f("label", TsType::nullable(string())),
                f("dependencies", TsType::array(r("AssetReference"))),
                f("material", TsType::nullable(r("MaterialProjection"))),
            ],
        ),
        iface(
            "The asset registry above the asset-reference vocabulary.",
            "Catalog",
            vec![f("entries", TsType::array(r("CatalogEntry")))],
        ),
        iface(
            "One classified catalog-validation failure; absent loci are null. \
             `cyclePath` is non-empty only for dependency-cycle.",
            "CatalogValidationError",
            vec![
                f("code", r("CatalogValidationCode")),
                f("id", TsType::nullable(string())),
                f("kind", TsType::nullable(r("AssetKind"))),
                f("from", TsType::nullable(string())),
                f("slot", TsType::nullable(string())),
                f("expected", TsType::nullable(r("AssetKind"))),
                f("actual", TsType::nullable(r("AssetKind"))),
                f("reference", TsType::nullable(string())),
                f("dependency", TsType::nullable(string())),
                f("cyclePath", TsType::array(string())),
            ],
        ),
        iface(
            "A full catalog-validation report: every classified error.",
            "CatalogValidationReport",
            vec![f("errors", TsType::array(r("CatalogValidationError")))],
        ),
        iface(
            "One asset-lock entry: the resolved identity a save pinned.",
            "AssetLockEntry",
            vec![
                f("id", string()),
                f("kind", r("AssetKind")),
                f("version", num()),
                f("hash", TsType::nullable(string())),
                f("dependencies", TsType::array(string())),
            ],
        ),
        iface(
            "A world-bundle asset lock.",
            "AssetLock",
            vec![f("entries", TsType::array(r("AssetLockEntry")))],
        ),
        iface(
            "One asset's classified lock-drift finding; absent detail fields are null.",
            "LockFinding",
            vec![
                f("id", string()),
                f("code", r("LockIssueCode")),
                f("lockedKind", TsType::nullable(r("AssetKind"))),
                f("currentKind", TsType::nullable(r("AssetKind"))),
                f("lockedVersion", TsType::nullable(num())),
                f("currentVersion", TsType::nullable(num())),
                f("lockedHash", TsType::nullable(string())),
                f("currentHash", TsType::nullable(string())),
                f("addedDependencies", TsType::array(string())),
                f("removedDependencies", TsType::array(string())),
            ],
        ),
        iface(
            "A full asset-lock validation report: classified drift, never a silent re-lock.",
            "LockValidationReport",
            vec![f("findings", TsType::array(r("LockFinding")))],
        ),
        union(
            "What to do when a referenced asset is missing in a given context.",
            "FallbackDecision",
            "outcome",
            vec![
                v(
                    "useFallback",
                    vec![f("reason", string()), f("visual", r("FallbackVisual"))],
                ),
                v("failClosed", vec![f("reason", string())]),
                v("skip", vec![f("reason", string())]),
            ],
        ),
    ];

    Module {
        name: "assets",
        imports,
        items,
    }
}

// ── policyView.ts — read-only world view a constrained policy is handed ────────

pub fn policy_view_module() -> Module {
    let imports = vec![import("./ids.js", &["EntityId", "TagId"])];

    let triple = || TsType::Tuple(vec![num(), num(), num()]);
    let quad = || TsType::Tuple(vec![num(), num(), num(), num()]);

    let items = vec![
        iface(
            "A runtime transform as a policy sees it (translation, rotation xyzw, scale).",
            "PolicyTransform",
            vec![
                f("translation", triple()),
                f("rotation", quad()),
                f("scale", triple()),
            ],
        ),
        Item::Alias {
            doc: "Lifecycle states a policy may observe. Tombstoned entities are omitted, never shown."
                .to_string(),
            name: "PolicyEntityLifecycle".to_string(),
            ty: TsType::StringEnum(vec!["active".to_string(), "disabled".to_string()]),
        },
        union(
            "Where an entity came from, as a policy sees it. DiagnosticTooling is redacted entirely.",
            "PolicyEntitySource",
            "kind",
            vec![
                v("sceneNode", vec![f("node", num())]),
                v("runtime", vec![]),
                v("imported", vec![f("asset", string())]),
                v("policy", vec![]),
            ],
        ),
        Item::Alias {
            doc: "Catalog resolution status of an asset a policy may reference.".to_string(),
            name: "PolicyAssetStatus".to_string(),
            ty: TsType::StringEnum(vec![
                "resolved".to_string(),
                "missing".to_string(),
                "stale".to_string(),
            ]),
        },
        iface(
            "One asset a policy may reason about: id, kind, and resolution status.",
            "PolicyAssetView",
            vec![
                f("id", string()),
                f("kind", string()),
                f("status", r("PolicyAssetStatus")),
            ],
        ),
        iface(
            "One entity as a policy sees it: identity, lifecycle, optional transform, source, labels, spatiality.",
            "PolicyEntityView",
            vec![
                f("id", r("EntityId")),
                f("lifecycle", r("PolicyEntityLifecycle")),
                f("transform", TsType::nullable(r("PolicyTransform"))),
                f("source", r("PolicyEntitySource")),
                f("labels", TsType::array(r("TagId"))),
                f("spatial", boolean()),
            ],
        ),
        iface(
            "Cheap aggregate counts so a policy can branch without scanning the whole view.",
            "PolicyWorldSummary",
            vec![
                f("tick", num()),
                f("activeEntities", num()),
                f("spatialEntities", num()),
                f("assetCount", num()),
                f("missingAssets", num()),
            ],
        ),
        iface(
            "The complete read-only world projection handed to a policy for one tick.",
            "PolicyWorldView",
            vec![
                f("tick", num()),
                f("entities", TsType::array(r("PolicyEntityView"))),
                f("assets", TsType::array(r("PolicyAssetView"))),
                f("summary", r("PolicyWorldSummary")),
            ],
        ),
        union(
            "The narrow, safe set of world/entity actions a policy may propose. Each is a request; authority validates and applies or rejects.",
            "PolicyWorldCommand",
            "kind",
            vec![
                v(
                    "requestSetTransform",
                    vec![f("entity", r("EntityId")), f("transform", r("PolicyTransform"))],
                ),
                v(
                    "requestAddLabel",
                    vec![f("entity", r("EntityId")), f("label", r("TagId"))],
                ),
                v("requestDisable", vec![f("entity", r("EntityId"))]),
                v("noopMarker", vec![f("note", string())]),
            ],
        ),
        union(
            "The accepted domain event a validated command becomes. Distinct from the command and the rejection.",
            "PolicyWorldEvent",
            "kind",
            vec![
                v(
                    "transformSet",
                    vec![f("entity", r("EntityId")), f("transform", r("PolicyTransform"))],
                ),
                v(
                    "labelAdded",
                    vec![f("entity", r("EntityId")), f("label", r("TagId"))],
                ),
                v("disabled", vec![f("entity", r("EntityId"))]),
                v("noopRecorded", vec![f("note", string())]),
            ],
        ),
        Item::Alias {
            doc: "The classified reason authority refused a proposed command. A policy reflects this; it never decides acceptance."
                .to_string(),
            name: "PolicyWorldRejection".to_string(),
            ty: TsType::StringEnum(vec![
                "unknownEntity".to_string(),
                "entityDisabled".to_string(),
                "notSpatial".to_string(),
                "immovable".to_string(),
                "invalidTransform".to_string(),
                "labelAlreadyPresent".to_string(),
                "alreadyDisabled".to_string(),
            ]),
        },
        union(
            "The outcome authority reports for one proposed command: accepted (with its event) or rejected (with the reason).",
            "PolicyWorldOutcome",
            "status",
            vec![
                v("accepted", vec![f("event", r("PolicyWorldEvent"))]),
                v("rejected", vec![f("rejection", r("PolicyWorldRejection"))]),
            ],
        ),
    ];

    Module {
        name: "policyView",
        imports,
        items,
    }
}

// ── telemetry.ts — observational telemetry events ────────────────────────────

pub fn telemetry_module() -> Module {
    let items = vec![
        Item::Alias {
            doc: "Component that produced an observational telemetry event.".to_string(),
            name: "TelemetrySource".to_string(),
            ty: TsType::StringEnum(
                protocol_telemetry::TELEMETRY_SOURCES
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
            ),
        },
        Item::Alias {
            doc: "Severity of an observational telemetry event.".to_string(),
            name: "TelemetryLevel".to_string(),
            ty: TsType::StringEnum(
                protocol_telemetry::TELEMETRY_LEVELS
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
            ),
        },
        Item::Alias {
            doc: "Metric value category.".to_string(),
            name: "TelemetryMetricKind".to_string(),
            ty: TsType::StringEnum(
                protocol_telemetry::TELEMETRY_METRIC_KINDS
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
            ),
        },
        iface(
            "One numeric telemetry sample.",
            "TelemetryMetric",
            vec![
                f("name", string()),
                f("kind", r("TelemetryMetricKind")),
                f("value", num()),
                f("unit", TsType::nullable(string())),
            ],
        ),
        union(
            "One observational telemetry event.",
            "TelemetryEvent",
            "kind",
            vec![
                v(
                    "metric",
                    vec![
                        f("source", r("TelemetrySource")),
                        f("level", r("TelemetryLevel")),
                        f("sequence", num()),
                        f("metric", r("TelemetryMetric")),
                    ],
                ),
                v(
                    "trace",
                    vec![
                        f("source", r("TelemetrySource")),
                        f("level", r("TelemetryLevel")),
                        f("sequence", num()),
                        f("span", string()),
                        f("message", string()),
                    ],
                ),
            ],
        ),
        iface(
            "A batch of telemetry events emitted for one observation point.",
            "TelemetryEnvelope",
            vec![
                f("protocolVersion", num()),
                f("emittedAtTick", num()),
                f("events", TsType::array(r("TelemetryEvent"))),
            ],
        ),
    ];

    Module {
        name: "telemetry",
        imports: vec![],
        items,
    }
}

pub fn entity_authoring_module() -> Module {
    let imports = vec![
        import("./ids.js", &["EntityId", "TagId", "ProcessId", "SubjectId"]),
        import("./scene.js", &["SceneNodeId"]),
    ];

    let triple = || TsType::Tuple(vec![num(), num(), num()]);
    let quad = || TsType::Tuple(vec![num(), num(), num(), num()]);

    let items = vec![
        iface(
            "A runtime transform on the authoring border (translation, rotation xyzw, scale).",
            "AuthoringTransform",
            vec![
                f("translation", triple()),
                f("rotation", quad()),
                f("scale", triple()),
            ],
        ),
        union(
            "Where an authored entity comes from. Mirrors core-entity's EntitySource on the wire.",
            "AuthoringSource",
            "kind",
            vec![
                v("sceneBootstrap", vec![f("node", r("SceneNodeId"))]),
                v("runtimeCreated", vec![f("by", TsType::nullable(r("ProcessId")))]),
                v("imported", vec![f("asset", string())]),
                v("diagnosticTooling", vec![]),
                v("policyProposed", vec![f("by", r("SubjectId"))]),
            ],
        ),
        union(
            "A capability an attachCapability command establishes on a live entity.",
            "AuthoringCapability",
            "kind",
            vec![
                v("transform", vec![f("transform", r("AuthoringTransform"))]),
                v("render", vec![f("visible", boolean())]),
                v("collision", vec![f("staticCollider", boolean())]),
                v("bounds", vec![f("min", triple()), f("max", triple())]),
            ],
        ),
        iface(
            "Where a stored entity definition was read from inside a durable ProjectBundle.",
            "EntityDefinitionSourceTrace",
            vec![
                f("projectBundle", string()),
                f("relativePath", string()),
            ],
        ),
        iface(
            "Small string metadata entry for Studio/project readout.",
            "EntityDefinitionMetadataEntry",
            vec![f("key", string()), f("value", string())],
        ),
        union(
            "A stored capability declaration with an initial value.",
            "EntityDefinitionCapability",
            "kind",
            vec![
                v("transform", vec![f("transform", r("AuthoringTransform"))]),
                v("render", vec![f("visible", boolean())]),
                v("collision", vec![f("staticCollider", boolean())]),
                v("bounds", vec![f("min", triple()), f("max", triple())]),
                v("unknown", vec![f("capabilityKind", string())]),
            ],
        ),
        iface(
            "Durable stored entity definition authored in a ProjectBundle/catalog.",
            "EntityDefinition",
            vec![
                f("stableId", string()),
                f("displayName", string()),
                f("source", r("EntityDefinitionSourceTrace")),
                f("tags", TsType::array(r("TagId"))),
                f("metadata", TsType::array(r("EntityDefinitionMetadataEntry"))),
                f("capabilities", TsType::array(r("EntityDefinitionCapability"))),
            ],
        ),
        string_enum(
            "Classified validation diagnostic for stored entity definitions.",
            "EntityDefinitionDiagnosticCode",
            protocol_entity_authoring::ENTITY_DEFINITION_DIAGNOSTIC_CODES,
        ),
        iface(
            "One stored EntityDefinition validation diagnostic.",
            "EntityDefinitionDiagnostic",
            vec![
                f("code", r("EntityDefinitionDiagnosticCode")),
                f("path", string()),
                f("message", string()),
            ],
        ),
        union(
            "Validation outcome for stored EntityDefinitions.",
            "EntityDefinitionValidationOutcome",
            "status",
            vec![
                v("valid", vec![]),
                v("invalid", vec![f("diagnostics", TsType::array(r("EntityDefinitionDiagnostic")))]),
            ],
        ),
        union(
            "A proposed generic entity authoring change. Proposal-only: authority validates and applies or rejects.",
            "EntityAuthoringCommand",
            "kind",
            vec![
                v(
                    "create",
                    vec![
                        f("id", r("EntityId")),
                        f("source", r("AuthoringSource")),
                        f("labels", TsType::array(r("TagId"))),
                    ],
                ),
                v("destroy", vec![f("id", r("EntityId"))]),
                v("disable", vec![f("id", r("EntityId"))]),
                v("enable", vec![f("id", r("EntityId"))]),
                v("addLabel", vec![f("id", r("EntityId")), f("tag", r("TagId"))]),
                v("removeLabel", vec![f("id", r("EntityId")), f("tag", r("TagId"))]),
                v(
                    "attachCapability",
                    vec![f("id", r("EntityId")), f("capability", r("AuthoringCapability"))],
                ),
                v(
                    "setTransform",
                    vec![f("id", r("EntityId")), f("transform", r("AuthoringTransform"))],
                ),
                v("move", vec![f("id", r("EntityId")), f("delta", triple())]),
                v(
                    "attachTransformParent",
                    vec![f("child", r("EntityId")), f("parent", r("EntityId"))],
                ),
                v("detachTransformParent", vec![f("child", r("EntityId"))]),
                v(
                    "setContainment",
                    vec![f("member", r("EntityId")), f("container", r("EntityId"))],
                ),
                v("clearContainment", vec![f("member", r("EntityId"))]),
                v(
                    "setDerivedFrom",
                    vec![f("derived", r("EntityId")), f("origin", r("EntityId"))],
                ),
            ],
        ),
        string_enum(
            "The kind of accepted authoring change (compact; re-read the snapshot for full detail).",
            "AuthoringEventKind",
            protocol_entity_authoring::EVENT_KINDS,
        ),
        iface(
            "The accepted authoring event: what happened, to which entity.",
            "EntityAuthoringEvent",
            vec![f("kind", r("AuthoringEventKind")), f("entity", r("EntityId"))],
        ),
        string_enum(
            "The classified reason authority refused a proposed authoring command. A UI reflects this; it never decides acceptance.",
            "AuthoringRejectionReason",
            protocol_entity_authoring::REJECTION_REASONS,
        ),
        iface(
            "The classified refusal: a reason plus the primary entity it concerns.",
            "EntityAuthoringRejection",
            vec![
                f("reason", r("AuthoringRejectionReason")),
                f("entity", r("EntityId")),
            ],
        ),
        union(
            "The outcome authority reports for one proposed authoring command.",
            "EntityAuthoringOutcome",
            "status",
            vec![
                v("accepted", vec![f("event", r("EntityAuthoringEvent"))]),
                v("rejected", vec![f("rejection", r("EntityAuthoringRejection"))]),
            ],
        ),
    ];

    Module {
        name: "entityAuthoring",
        imports,
        items,
    }
}

// ── view.ts — public camera/view projection surface ──────────────────────────

pub fn view_module() -> Module {
    let tuple3 = || TsType::Tuple(vec![num(), num(), num()]);
    let matrix4 = || {
        TsType::Tuple(vec![
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
            num(),
        ])
    };

    let imports = vec![import("./voxel.js", &["Face", "VoxelCoord"])];

    let items = vec![
        Item::BrandedId {
            doc: "Opaque bridge-owned camera handle for runtime view/projection state.".to_string(),
            name: "CameraHandle".to_string(),
        },
        iface(
            "Camera pose in world units/degrees; deterministic view state, not gameplay authority.",
            "CameraPose",
            vec![
                f("position", tuple3()),
                f("yawDegrees", num()),
                f("pitchDegrees", num()),
            ],
        ),
        iface(
            "Orthogonal basis vectors derived from a camera pose.",
            "CameraBasis",
            vec![f("forward", tuple3()), f("right", tuple3()), f("up", tuple3())],
        ),
        iface(
            "Perspective projection parameters for runtime view evidence.",
            "PerspectiveProjection",
            vec![
                f("fovYDegrees", num()),
                f("near", num()),
                f("far", num()),
            ],
        ),
        iface(
            "Pixel viewport dimensions for projection evidence.",
            "ViewportSize",
            vec![f("width", num()), f("height", num())],
        ),
        iface(
            "Request to create a bridge-owned runtime view camera.",
            "CameraCreateRequest",
            vec![
                f("initialPose", r("CameraPose")),
                f("projection", r("PerspectiveProjection")),
                f("viewport", r("ViewportSize")),
            ],
        ),
        iface(
            "Bounded first-person input for deterministic camera movement evidence.",
            "FirstPersonCameraInput",
            vec![
                f("moveForward", num()),
                f("moveRight", num()),
                f("moveUp", num()),
                f("yawDeltaDegrees", num()),
                f("pitchDeltaDegrees", num()),
                f("dtSeconds", num()),
                f("moveSpeedUnitsPerSecond", num()),
            ],
        ),
        iface(
            "One camera input proposal for a specific deterministic tick.",
            "FirstPersonCameraInputEnvelope",
            vec![
                f("camera", r("CameraHandle")),
                f("input", r("FirstPersonCameraInput")),
                f("tick", num()),
            ],
        ),
        iface(
            "Request to read current projection evidence for a camera. Null viewport means use the camera viewport.",
            "CameraProjectionRequest",
            vec![
                f("camera", r("CameraHandle")),
                f("viewport", TsType::nullable(r("ViewportSize"))),
            ],
        ),
        iface(
            "Camera pose/basis snapshot after create or input application.",
            "CameraSnapshot",
            vec![
                f("camera", r("CameraHandle")),
                f("tick", num()),
                f("pose", r("CameraPose")),
                f("basis", r("CameraBasis")),
                f("projection", r("PerspectiveProjection")),
                f("viewport", r("ViewportSize")),
            ],
        ),
        iface(
            "Camera pose plus deterministic column-major 4x4 projection matrices.",
            "CameraProjectionSnapshot",
            vec![
                f("camera", r("CameraHandle")),
                f("tick", num()),
                f("pose", r("CameraPose")),
                f("basis", r("CameraBasis")),
                f("projection", r("PerspectiveProjection")),
                f("viewport", r("ViewportSize")),
                f("viewMatrix", matrix4()),
                f("projectionMatrix", matrix4()),
                f("viewProjectionMatrix", matrix4()),
                f("projectionHash", string()),
            ],
        ),
        iface(
            "Explicit V1 editor/testbench camera collision shape.",
            "CameraCollisionShape",
            vec![f("halfExtents", tuple3())],
        ),
        Item::Alias {
            doc: "The intentionally simple collision policy for V1 camera movement.".to_string(),
            name: "CameraCollisionPolicyMode".to_string(),
            ty: TsType::StringEnum(vec!["axis_separable_slide".to_string()]),
        },
        iface(
            "Bounded collision policy evidence.",
            "CameraCollisionPolicy",
            vec![
                f("mode", r("CameraCollisionPolicyMode")),
                f("maxIterations", num()),
            ],
        ),
        iface(
            "One constrained camera input proposal for a specific tick/grid.",
            "CollisionConstrainedCameraInputEnvelope",
            vec![
                f("camera", r("CameraHandle")),
                f("grid", num()),
                f("input", r("FirstPersonCameraInput")),
                f("tick", num()),
                f("shape", r("CameraCollisionShape")),
                f("policy", r("CameraCollisionPolicy")),
            ],
        ),
        iface(
            "Axis-aligned world AABB queried against voxel collision.",
            "CollisionAabbEvidence",
            vec![f("min", tuple3()), f("max", tuple3())],
        ),
        Item::Alias {
            doc: "Axis blocked by the V1 axis-separable collision policy.".to_string(),
            name: "CollisionAxis".to_string(),
            ty: TsType::StringEnum(vec!["x".to_string(), "y".to_string(), "z".to_string()]),
        },
        iface(
            "Collision details for an attempted camera move.",
            "CameraCollisionEvidence",
            vec![
                f("grid", num()),
                f("shape", r("CameraCollisionShape")),
                f("policy", r("CameraCollisionPolicy")),
                f("collided", boolean()),
                f("blockedAxes", TsType::array(r("CollisionAxis"))),
                f("correction", tuple3()),
                f("queriedAabb", r("CollisionAabbEvidence")),
                f("worldHash", string()),
                f("collisionProjectionHash", string()),
            ],
        ),
        iface(
            "Before/attempted/after camera evidence for constrained movement.",
            "CameraCollisionSnapshot",
            vec![
                f("camera", r("CameraHandle")),
                f("tick", num()),
                f("before", r("CameraSnapshot")),
                f("attempted", r("CameraSnapshot")),
                f("after", r("CameraSnapshot")),
                f("collision", r("CameraCollisionEvidence")),
                f("movementHash", string()),
            ],
        ),
        Item::Alias {
            doc: "Screen-point coordinate convention.".to_string(),
            name: "ScreenPointSpace".to_string(),
            ty: TsType::StringEnum(vec!["normalized_0_1".to_string(), "pixel".to_string()]),
        },
        iface(
            "Screen/crosshair point used to derive a camera ray.",
            "ScreenPoint",
            vec![
                f("x", num()),
                f("y", num()),
                f("space", r("ScreenPointSpace")),
            ],
        ),
        iface(
            "Request to derive a pick ray from bridge-owned camera/projection evidence.",
            "ScreenPointToPickRayRequest",
            vec![
                f("camera", r("CameraHandle")),
                f("grid", num()),
                f("viewport", TsType::nullable(r("ViewportSize"))),
                f("screenPoint", r("ScreenPoint")),
                f("maxDistance", num()),
            ],
        ),
        iface(
            "Camera-derived world-space ray plus source projection hash.",
            "PickRaySnapshot",
            vec![
                f("camera", r("CameraHandle")),
                f("tick", num()),
                f("grid", num()),
                f("screenPoint", r("ScreenPoint")),
                f("origin", tuple3()),
                f("direction", tuple3()),
                f("maxDistance", num()),
                f("cameraProjectionHash", string()),
                f("rayHash", string()),
            ],
        ),
        Item::Alias {
            doc: "Classified selection outcome.".to_string(),
            name: "VoxelSelectionOutcome".to_string(),
            ty: TsType::StringEnum(vec!["hit".to_string(), "miss".to_string()]),
        },
        iface(
            "Combined camera-to-ray plus authority raycast selection evidence.",
            "VoxelSelectionSnapshot",
            vec![
                f("pickRay", r("PickRaySnapshot")),
                f("outcome", r("VoxelSelectionOutcome")),
                f("selectedVoxel", TsType::nullable(r("VoxelCoord"))),
                f("selectedFace", TsType::nullable(r("Face"))),
                f("editAnchor", TsType::nullable(r("VoxelCoord"))),
                f("selectionHash", string()),
            ],
        ),
    ];

    Module {
        name: "view",
        imports,
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
            Item::ReExport {
                from: "./voxelConversion.js".to_string(),
            },
            Item::ReExport {
                from: "./voxelAsset.js".to_string(),
            },
            Item::ReExport {
                from: "./gameRules.js".to_string(),
            },
            Item::ReExport {
                from: "./gameExtension.js".to_string(),
            },
            Item::ReExport {
                from: "./scene.js".to_string(),
            },
            Item::ReExport {
                from: "./worldBundle.js".to_string(),
            },
            Item::ReExport {
                from: "./assets.js".to_string(),
            },
            Item::ReExport {
                from: "./diagnostics.js".to_string(),
            },
            Item::ReExport {
                from: "./policyView.js".to_string(),
            },
            Item::ReExport {
                from: "./telemetry.js".to_string(),
            },
            Item::ReExport {
                from: "./view.js".to_string(),
            },
            Item::ReExport {
                from: "./entityAuthoring.js".to_string(),
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
        voxel_conversion_module(),
        voxel_asset_module(),
        game_rules_module(),
        game_extension_module(),
        scene_module(),
        world_bundle_module(),
        assets_module(),
        diagnostics_module(),
        policy_view_module(),
        telemetry_module(),
        view_module(),
        entity_authoring_module(),
        index_module(),
    ]
}
