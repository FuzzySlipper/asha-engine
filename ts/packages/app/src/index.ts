// @asha/app — composition + the authority-safe command submission path (ADR 0008).
//
// `app` is the ONLY package that submits commands: it turns an `@asha/editor-tools`
// proposal into a submission through the approved bridge path. UI/editor packages
// produce proposals and preview targets but never mutate authoritative state.

import type {
  EntityAuthoringCommand,
  EntityAuthoringOutcome,
  Face,
  PickRay,
  PickResult,
  VoxelCommand,
  VoxelCoord,
} from '@asha/contracts';
import { EditorStore, proposeCommand, previewTargets } from '@asha/editor-tools';
import type { CommandBatch, CommandResult, RuntimeBridge } from '@asha/runtime-bridge';

export { EditorStore } from '@asha/editor-tools';
export type { EditorContext, EditorAction, VoxelSelection } from '@asha/editor-tools';

// The transport-agnostic composition root (task #2439) lives in shell.ts.
export * from './shell.js';

/**
 * Where committed commands go. The real wiring is {@link bridgeCommandSink}, which
 * sends the batch through `@asha/runtime-bridge` (`submitCommands`) to Rust for
 * validation + application. Injected so the editor controller stays decoupled from
 * the transport and is trivially testable.
 */
export type CommandSink = (commands: readonly VoxelCommand[]) => void;

/** Observes the classified accept/reject summary authority returns for a batch. */
export type CommandResultHandler = (result: CommandResult) => void;

/**
 * The real command sink: submit the generated `VoxelCommand`s through the runtime
 * facade's `submitCommands` verb (carrying the `protocol_voxel::CommandBatch`
 * border straight to Rust authority) and forward the classified {@link
 * CommandResult} to `onResult` for UI/diagnostics. The app is the ONLY package
 * permitted to take this transport dependency.
 */
export function bridgeCommandSink(
  bridge: RuntimeBridge,
  onResult?: CommandResultHandler,
): CommandSink {
  return (commands) => {
    const batch: CommandBatch = { commands: [...commands] };
    const result = bridge.submitCommands(batch);
    onResult?.(result);
  };
}

/**
 * The single authority-safe edit path. Holds the persistent {@link EditorStore},
 * computes a non-authoritative preview, and — only on {@link commit} — submits the
 * proposed command through the injected {@link CommandSink}. It never mutates voxel
 * state itself.
 */
export class VoxelEditController {
  readonly store: EditorStore;
  readonly #sink: CommandSink;

  constructor(sink: CommandSink, store: EditorStore = new EditorStore()) {
    this.#sink = sink;
    this.store = store;
  }

  /** The cells the current brush would affect — non-authoritative preview data. */
  preview(): VoxelCoord[] {
    return previewTargets(this.store.getState());
  }

  /** The command the current context would commit, without submitting it. */
  proposal(): VoxelCommand | null {
    return proposeCommand(this.store.getState());
  }

  /**
   * Submit the current proposal through the bridge path (the only mutation route).
   * Returns the submitted command, or `null` if there was nothing to commit (no
   * selection / non-editing tool) — in which case the sink is not called.
   */
  commit(): VoxelCommand | null {
    const command = this.proposal();
    if (command === null) {
      return null;
    }
    this.#sink([command]);
    return command;
  }

  /**
   * Cancel the current draft: clear the selection (and therefore the preview)
   * without submitting anything. Symmetric with {@link commit} — the edit lifecycle
   * ends either by committing the proposal or cancelling it. Never calls the sink.
   */
  cancel(): void {
    this.store.dispatch({ type: 'clearSelection' });
  }
}

// ── Generic entity authoring submission (#2485) ────────────────────────────────

/**
 * Where a proposed generic entity authoring command goes for validation. The real
 * wiring routes the command through the runtime facade to `svc-entity-authoring`
 * (Rust) and returns the classified {@link EntityAuthoringOutcome}; injected so the
 * authoring controller stays decoupled from the transport and trivially testable.
 *
 * Note: a dedicated bridge *authoring* verb is the remaining native integration
 * (the same class as the `submit_commands` native gap); until it is wired, a host
 * supplies a sink backed by whichever authority transport is available (wasm
 * authority for tests/smoke, native for the desktop host). The app never applies a
 * command itself — it only forwards proposals and reflects the authority outcome.
 */
export type EntityAuthoringSink = (command: EntityAuthoringCommand) => EntityAuthoringOutcome;

/**
 * The single authority-safe generic-entity authoring path. Forwards a proposal
 * (built by `@asha/ui-dom` / `@asha/editor-tools`) through the injected
 * {@link EntityAuthoringSink} and records the classified outcome for the devtools
 * inspector to display. It never mutates entity authority itself.
 */
export class EntityAuthoringController {
  readonly #sink: EntityAuthoringSink;
  #last: EntityAuthoringOutcome | null = null;

  constructor(sink: EntityAuthoringSink) {
    this.#sink = sink;
  }

  /**
   * Submit a proposed authoring command for validation. Returns the classified
   * outcome (also retained as {@link lastOutcome}); on rejection authority is
   * unchanged — the controller mutates nothing locally either way.
   */
  submit(command: EntityAuthoringCommand): EntityAuthoringOutcome {
    const outcome = this.#sink(command);
    this.#last = outcome;
    return outcome;
  }

  /** The last authoring outcome, for the inspector's "last command result" readout. */
  lastOutcome(): EntityAuthoringOutcome | null {
    return this.#last;
  }
}

// ── Picking → selection (authority pick through the launch path) ───────────────

/**
 * Casts a world-space {@link PickRay} against authority and returns the classified
 * {@link PickResult}. The real implementation is {@link bridgePicker} (backed by
 * `@asha/runtime-bridge` `pickVoxel`); injected so the selection flow stays
 * decoupled from the transport and trivially testable.
 */
export type VoxelPicker = (ray: PickRay) => PickResult;

/** The real picker: route the ray through the runtime facade's `pickVoxel` verb. */
export function bridgePicker(bridge: RuntimeBridge): VoxelPicker {
  return (ray) => bridge.pickVoxel(ray);
}

/**
 * The single pointer→selection path: cast `ray` against authority (Rust owns the
 * voxel-grid raycast — the renderer only built the ray) and update editor selection
 * through pure actions. A hit selects the struck voxel + face; a classified miss
 * clears the selection. Selection is keyed on authority voxel coordinates, never a
 * render handle, so it survives reprojection. Returns the authority result for UI
 * diagnostics. Never mutates voxel state.
 */
export function pickAndSelect(store: EditorStore, pick: VoxelPicker, ray: PickRay): PickResult {
  const result = pick(ray);
  if (result.outcome === 'hit') {
    store.dispatch({
      type: 'setSelection',
      selection: { voxel: result.hit.voxel, face: result.hit.face },
    });
  } else {
    store.dispatch({ type: 'clearSelection' });
  }
  return result;
}

/** A renderer pick hint: the voxel + face a renderer-side mesh pick claims was hit. */
export interface RendererPickClaim {
  readonly voxel: VoxelCoord;
  readonly face: Face;
}

/**
 * Revalidate a renderer pick hint against the authoritative pick. Authority is the
 * sole source of voxel coordinates — the renderer's claim is never trusted for
 * selection. If authority hit a voxel/face that disagrees with the claim, the hint
 * was stale (a desynced renderer mesh): returns a classified `hitMismatch` rejection
 * so the caller fails closed instead of acting on the wrong cell. A confirmed hit or
 * a plain miss passes the authority result through unchanged.
 */
export function revalidatePickHint(authority: PickResult, claim: RendererPickClaim): PickResult {
  if (authority.outcome !== 'hit') {
    return authority;
  }
  const { voxel, face } = authority.hit;
  const matches =
    voxel.x === claim.voxel.x &&
    voxel.y === claim.voxel.y &&
    voxel.z === claim.voxel.z &&
    face === claim.face;
  if (matches) {
    return authority;
  }
  return {
    outcome: 'miss',
    rejection: {
      reason: 'hitMismatch',
      authoritativeVoxel: voxel,
      authoritativeFace: face,
      claimedVoxel: claim.voxel,
      claimedFace: claim.face,
    },
  };
}
