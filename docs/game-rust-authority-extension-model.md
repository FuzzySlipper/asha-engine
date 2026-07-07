# Game Rust Authority Extension Model

Status: architecture note for #4488. This is the intended direction for serious
ASHA downstream game repos; it is not a claim that all extension points already
exist.

ASHA keeps the central rule:

> Rust owns authority. TypeScript owns expression and projection. Generated
> contracts define the border.

That rule still leaves room for a real game repo to own compiled Rust behavior.
The important distinction is between **generic ASHA authority** and
**authored game authority**.

## Ownership Split

ASHA Rust always owns generic engine truth:

- RuntimeSession lifecycle and canonical state application.
- command validation, accepted domain events, replay records, deterministic
  tick ordering, deterministic RNG, and session hashes.
- transform, collision, spatial queries, pathfinding primitives, generic
  health/lifecycle primitives, capability mutation ownership, and renderer
  projection formats.
- protocol/codegen and generated TypeScript contract packages.
- native/wasm/runtime bridge provider contracts and fail-closed backend
  selection.

Game-owned Rust may own authored rule decisions that are specific to that game:

- weapon effects, ability rules, damage formulas, and hit modifiers.
- quest, interaction, faction, aggro, spawn-condition, wave, and encounter
  rules.
- game-mode scoring/win-condition logic when built from ASHA-provided state
  views and emitted as typed proposals.
- content/package preflight tools and build metadata checks that do not claim
  runtime authority.

The game-owned Rust crate is not a replacement RuntimeSession. It is a compiled
rule contributor that ASHA invokes through a public extension boundary.

## Extension Shape

The durable model should be a boring compiled boundary, not a dynamic plugin
system:

1. ASHA exposes a small public `GameRuleModule` style Rust trait/API from an
   extension crate.
2. A game repo builds a Rust crate that implements that trait against generated
   ASHA view/request/receipt types.
3. The game repo declares the compiled rule module in an ASHA game manifest with
   a rule id, semantic version, contract hash, and deterministic capability
   requirements.
4. RuntimeSession loads the manifest, verifies compatibility, and calls the
   compiled module only at declared rule hooks.
5. Rule output is a typed proposal or receipt fragment. ASHA Rust validates it,
   applies accepted events through existing owner matrices, records replay, and
   projects readouts.

The boundary should feel closer to a stable Rust library API plus generated
schemas than to a scripting bridge. A future native host may link the game rule
crate statically or load a compiled artifact, but the invocation contract should
look the same either way.

## Determinism And Replay

Game Rust receives only deterministic inputs:

- generated read-only RuntimeSession/ECRP views,
- explicit tick/session/epoch identifiers,
- deterministic RNG handles or precomputed random draws supplied by ASHA,
- authored content refs whose hashes are part of the loaded ProjectBundle.

Game Rust must not read wall-clock time, ambient randomness, local files, network
state, DOM/browser state, or TypeScript globals during authority hooks.

Replay records must include:

- game rule module id/version/contract hash,
- hook id and deterministic input hash,
- proposal hash,
- ASHA validation/acceptance result,
- resulting domain event/rejection hashes.

Replaying a session must either load the same compatible game rule module or
fail closed with a missing-rule diagnostic. It must not silently substitute
TypeScript behavior or a reference fixture.

## TypeScript Role

Game TypeScript may describe and project:

- authored catalog values and content references,
- UI/control descriptors,
- policy/config choices that become typed proposals,
- HUD/menu/readout projections,
- browser input collection and standalone host shell behavior.

Game TypeScript must not own:

- damage application,
- health/lifecycle mutation,
- collision or pathfinding resolution,
- RuntimeSession restart/session authority,
- rule execution shortcuts,
- arbitrary JSON command hatches,
- generated contract truth.

When a game needs a new authoritative behavior, TypeScript may name the rule and
submit typed intent data. The compiled Rust rule and ASHA RuntimeSession decide
what happened.

## Forbidden Paths

The following paths are hard failures:

- downstream imports of ASHA Rust private crates or TypeScript `src/*` files;
- game TS mutating authoritative state or shadowing RuntimeSession health,
  combat, collision, lifecycle, replay, pathfinding, or generated level truth;
- arbitrary JSON command/action hatches that Rust does not type and validate;
- demo-local replacements for generic collision, combat, lifecycle,
  pathfinding, RuntimeSession, renderer backend, or protocol/codegen authority;
- dynamic JavaScript callbacks in the authority path;
- reference/mock RuntimeSession helpers used as live/product authority.

## Required Upstream Extension Points

ASHA does not yet expose the full game-owned authority boundary. The missing
upstream surfaces are:

- a Rust public extension crate defining the minimal rule-module trait/API and
  deterministic hook contexts;
- generated protocol schemas for game rule module manifests, hook requests,
  proposals, receipts, and replay evidence;
- RuntimeSession loading/compatibility checks for rule-module declarations in
  ProjectBundle or an adjacent ASHA game manifest;
- RuntimeSession invocation hooks for at least one narrow behavior, initially a
  weapon-effect or interaction rule;
- replay/golden tests proving module id/version/hash, proposal, validation, and
  accepted event hashes are captured;
- TypeScript package-root types that let game TS reference rule ids and submit
  typed intents without importing private generated files.

## Minimal `asha-demo` Candidate Slice

The smallest useful proving slice is a game-owned Rust weapon effect:

- `asha-demo` owns a small Rust crate that defines a `demo.primary_fire_effect`
  module.
- The rule reads a generated, read-only hit/effect context supplied by ASHA and
  returns a typed damage modifier proposal, such as `base_damage + close_range_bonus`.
- ASHA RuntimeSession validates that proposal against generic weapon/combat
  rules, applies health/lifecycle changes through existing Rust authority, and
  records replay evidence with the demo rule module id/version/hash.
- Demo TS only submits `primary_fire` and projects the resulting RuntimeSession
  receipt/HUD readout.

Until those upstream extension points exist, `asha-demo` Rust should stay in the
content/tooling lane: manifest preflight, package metadata checks, or build
validation. It should not become an alternate combat/collision/lifecycle stack.
