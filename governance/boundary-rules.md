# Boundary rules

1. TypeScript may never mutate authoritative state.
2. Policy code receives generated read-only views; it returns proposed commands only.
3. Rust validates all commands. TypeScript does not validate.
4. Generated contract files in ts/packages/contracts/src/generated/ are not hand-edited.
5. No lower-level Rust crate may depend on a higher-level crate.
6. Policy/catalog packages may not import renderer, UI, WASM bridge, or Electron packages.
7. Renderer packages may not import policy packages.
8. Tool omniscience must not leak into runtime packages.
9. App/UI/renderer/devtools couple only to the `@asha/runtime-bridge` facade for runtime, not
   to the native addon (`@asha/native-bridge`) or the WASM replay path
   (`@asha/wasm-replay-bridge`). Only the facade imports the native addon. (ADR 0006)
10. `napi-rs` is the runtime transport; WASM is the replay/golden verification target. Neither
    is a public interface. Generated contracts remain the semantic/governance border.
11. Scene documents describe an *authored* initial arrangement; the live Rust `WorldState`
    (`core-scene`) owns runtime truth after bootstrap. An authored `SceneDocument` /
    `FlatSceneDocument` is never runtime authority and is never mutated by runtime movement.
    Scene bootstrap is one atomic authority initialization, not N ordinary create commands.
12. Render handles and the render scene graph are derived projection, never durable/save
    authority. Authority identity is `SceneNodeId` / `EntityId` (`core-ids`); a `RenderHandle`
    must not be treated as authority, save-file truth, or a stable durable id. Renderer/UI/
    devtools packages consume scene/world projections — they may not treat scene documents or
    render handles as authority.
13. Asset references that enter scene/save authority use the typed `AssetRef<T>` vocabulary
    (`core-assets`) with kind-prefixed scoped-kebab-case `AssetId`s — never free strings or
    source paths. Asset catalogs may be TS-authored, but Rust validates asset identity, kind,
    and references before authority accepts them; catalogs do not bypass Rust validation.
14. A world bundle is a **directory/manifest** of classified artifacts (`svc-serialization`):
    every artifact is `durable`, `generated`, or `cache`. Cache artifacts are disposable —
    deleting them must never change loaded authority. A future `.asha` archive is only a
    transport wrapper around the same files (directory is truth; the two must round-trip).
    Durable artifacts carry content hashes; the manifest fails closed on an unknown newer
    bundle/protocol version rather than guessing.
15. Bundle load order is an **authority constraint**, not an implementation detail: versions →
    asset lock → scene document → terrain generation → voxel edits/snapshots → atomic scene
    bootstrap → final validation. Final authority application must follow this order even if
    decoding is internally parallel; an out-of-order or missing-prerequisite plan is rejected
    with a classified diagnostic (`LoadPlan::verify_order`).
16. Save-time compaction is **explicit** and never runs during ordinary simulation ticks: a
    save may fold old edit history into chunk snapshots (`rule-world-bundle`), but replay and
    save stay separate concepts and a compacted snapshot plus the retained edit log must
    reconstruct identical chunk hashes. A terrain-generator version mismatch **fails closed**
    by default; development may opt into a regenerate-and-replay *diagnostic* that reports
    per-edit conflicts but never silently rewrites a save.
17. Asset **catalogs are Rust-validated** (`core-catalog`): TS may author catalog data, but
    only Rust validation decides whether references may enter authority/runtime. The asset
    dependency graph is a Rust-validated **DAG** — cycles are rejected with the full cycle path,
    never deferred (they corrupt load ordering and asset locks). Asset locks are validated, not
    silently updated, on load; drift is classified (missing / wrong-kind / stale version|hash /
    dependency drift).
18. A **material asset** is the single source, but consumers receive **separated projections**:
    collision/authority gets `CollisionMaterial` (solid/collidable/occludes/structural — no
    texture or colour), the renderer gets `RenderMaterial` (colour/texture/roughness/emissive/UV
    — no collision class). Render protocol must not carry collision class; the collision service
    must not carry texture/UV. Fallback is registry policy by **asset kind + context of use**
    (collision-critical fails closed; cosmetic/overlay gets a debug placeholder), never a
    per-reference override scattered through scenes. Single-asset revalidation reports dependent
    impact and distinguishes a live-safe visual-only change from an authority/structural change
    that needs revalidation or a full reload — it advises only and never mutates renderer or
    catalog state directly.
19. Voxel-generated meshes and authored **static mesh assets share one mesh payload descriptor**
    and one upload path (`protocol-render::MeshPayloadDescriptor`); they differ only by a
    `MeshProvenance` tag — never a duplicated upload protocol. A static mesh **asset** (shared
    geometry + material slots + collision policy) and its **instances** (transform + per-slot
    material overrides) are separate: the renderer shares one `BufferGeometry` across instances of
    an asset and disposes it only when the last instance is gone (reference-safe). A *visual-only*
    mesh skips collision; a *physical* mesh must carry an explicit collision proxy or opt into the
    payload-AABB fallback — a physical mesh with neither is a classified error, never a silent
    non-physical mesh. glTF is offline import tooling only; the renderer never loads glTF directly.
    Render material slots carry a material **asset id** (a `RenderMaterial` mapping), never collision
    authority (rule 18).
20. Non-UI **sprites/billboards** use a plane `BufferGeometry` (never `THREE.Sprite`) so they fit
    the retained-handle lifecycle and future batching. Sprite frame/tint/visibility/order updates are
    **deterministic and projection/authority-tick driven**, never renderer wall-clock animation. The
    descriptor reserves lit/shadow/custom-shader modes — validation accepts them and must not bake in
    an unlit-only assumption — while full shader systems stay deferred. Sprite **attachment** references
    source scene/entity IDs and named attachment points, never a durable `RenderHandle` (rule 12).
    A sprite **pick** is traced to authority identity (handle + source ids + asset ref); the renderer
    reports the trace and decides no gameplay action — authority revalidates and acts.
