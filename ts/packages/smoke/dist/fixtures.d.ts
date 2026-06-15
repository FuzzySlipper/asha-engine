import type { CommandEnvelope, PickRay, RenderFrameDiff, VoxelCommand } from '@asha/contracts';
import type { CommandBatch, WorldLoadRequest } from '@asha/runtime-bridge';
/** The abstract fixture world the smoke harness loads through the facade. */
export declare const FIXTURE_WORLD: WorldLoadRequest;
/** A deterministic FNV-1a hash over the fixture world definition (stable evidence). */
export declare function fixtureWorldHash(request: WorldLoadRequest): string;
/**
 * Deterministic, contract-shaped command envelopes (generated `@asha/contracts`
 * types). The smoke edit stage submits these instead of an ad-hoc `{ kind:
 * 'smoke-edit' }` literal, so the edit it proposes is a real authority command.
 */
export declare function fixtureCommandEnvelopes(): readonly CommandEnvelope[];
/**
 * A deterministic generated `VoxelCommand` the smoke edit stage submits: set the
 * origin voxel solid with launch material 1. It lands in the reference bridge's
 * resident origin chunk (0,0,0) with an in-catalog material, so Rust authority
 * accepts it on the native path.
 */
export declare function fixtureVoxelCommand(): VoxelCommand;
/**
 * The fixture edit batch for the facade. `submitCommands` now carries the generated
 * `VoxelCommand` union (manifest `protocol_voxel::CommandBatch`) straight to Rust
 * authority — no `{ kind: 'smoke-edit' }` placeholder command tunnel.
 */
export declare function fixtureCommandBatch(): CommandBatch;
/**
 * A deterministic pick ray for the picking stage: cast from outside the launch grid
 * toward the origin voxel along +X. Against the reference facade (no authority
 * geometry) it classifies as a miss; against a wired native facade it can hit the
 * origin voxel. Either way the picking PATH is exercised and a classified result is
 * returned — never a swallowed error.
 */
export declare function fixturePickRay(): PickRay;
/**
 * A deterministic render-update frame for the post-edit render stage: nudge the
 * fixture node's transform. This proves the renderer applies a retained-mode UPDATE
 * (leak-safe, no destroy+create) reflecting a committed edit, without adding a node.
 */
export declare function fixtureEditUpdateFrame(): RenderFrameDiff;
/**
 * A deterministic fixture render frame: create one mesh node, then upload the quad
 * payload. Drives the renderer through its real create→replaceMeshPayload path.
 */
export declare function fixtureRenderFrame(): RenderFrameDiff;
//# sourceMappingURL=fixtures.d.ts.map