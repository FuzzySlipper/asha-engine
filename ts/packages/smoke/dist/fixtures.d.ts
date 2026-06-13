import type { RenderFrameDiff } from '@asha/contracts';
import type { WorldLoadRequest } from '@asha/runtime-bridge';
/** The abstract fixture world the smoke harness loads through the facade. */
export declare const FIXTURE_WORLD: WorldLoadRequest;
/** A deterministic FNV-1a hash over the fixture world definition (stable evidence). */
export declare function fixtureWorldHash(request: WorldLoadRequest): string;
/**
 * A deterministic fixture render frame: create one mesh node, then upload the quad
 * payload. Drives the renderer through its real create→replaceMeshPayload path.
 */
export declare function fixtureRenderFrame(): RenderFrameDiff;
//# sourceMappingURL=fixtures.d.ts.map