// @asha/script-host — invokes constrained policies and collects their proposed
// commands deterministically.
//
// The host is a *coordinator*, not an authority. It hands a policy a read-only
// view, gathers the commands the policy proposes (in order), and reports
// structured diagnostics if a policy misbehaves. It never validates commands,
// never owns or mutates state, and never reaches outside the policy lane — Rust
// remains the sole validator (proven over the bridge in a later task).

import type { Policy, PolicyCommand, PolicyView } from '@asha/script-sdk';

// Sandboxed world-policy invocation with classified violations (#2393).
export * from './sandbox.js';

// Deterministic policy tick stage (#2394).
export * from './tick.js';

// ── Policy registration ───────────────────────────────────────────────────────

/** A policy paired with a stable name, so diagnostics can identify it. */
export interface NamedPolicy {
  readonly name: string;
  readonly policy: Policy;
}

/** Convenience constructor for a {@link NamedPolicy}. */
export function definePolicy(name: string, policy: Policy): NamedPolicy {
  return { name, policy };
}

// ── Diagnostics ───────────────────────────────────────────────────────────────

/** A structured note about something that went wrong during invocation. */
export interface HostDiagnostic {
  readonly kind: 'policy-threw';
  /** The `name` of the policy that produced this diagnostic. */
  readonly policy: string;
  /** A human-readable message extracted from the thrown value. */
  readonly message: string;
}

/** The result of running one or more policies over a single view. */
export interface HostRunResult {
  /** Proposed commands, in the order policies emitted them. */
  readonly commands: readonly PolicyCommand[];
  /** Structured diagnostics; empty on a fully successful run. */
  readonly diagnostics: readonly HostDiagnostic[];
}

// ── Command collection ────────────────────────────────────────────────────────

/**
 * An explicit, ordered buffer of proposed commands.
 *
 * The buffer never validates or transforms commands — it only preserves order.
 * {@link CommandBuffer.collected} returns a fresh array so a caller cannot reach
 * back in and mutate the host's internal state.
 */
export class CommandBuffer {
  readonly #commands: PolicyCommand[] = [];

  /** Append one command. */
  push(command: PolicyCommand): void {
    this.#commands.push(command);
  }

  /** Append a sequence of commands, preserving their order. */
  extend(commands: readonly PolicyCommand[]): void {
    for (const command of commands) {
      this.#commands.push(command);
    }
  }

  /** The commands collected so far, as a defensive copy. */
  collected(): readonly PolicyCommand[] {
    return [...this.#commands];
  }

  /** How many commands have been collected. */
  get length(): number {
    return this.#commands.length;
  }
}

// ── Invocation ────────────────────────────────────────────────────────────────

/** Extract a readable message from an unknown thrown value. */
function messageOf(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

/**
 * Invoke a single named policy against a read-only view and collect its
 * proposed commands. If the policy throws, the throw is captured as a structured
 * diagnostic and the returned command list is empty for that policy.
 */
export function invokePolicy(named: NamedPolicy, view: PolicyView): HostRunResult {
  return invokePolicies([named], view);
}

/**
 * Invoke several named policies against the same view, collecting all proposed
 * commands in policy order. A policy that throws contributes a diagnostic but
 * does not abort the remaining policies, and contributes no commands.
 */
export function invokePolicies(
  policies: readonly NamedPolicy[],
  view: PolicyView,
): HostRunResult {
  const buffer = new CommandBuffer();
  const diagnostics: HostDiagnostic[] = [];

  for (const { name, policy } of policies) {
    try {
      buffer.extend(policy(view));
    } catch (error: unknown) {
      diagnostics.push({
        kind: 'policy-threw',
        policy: name,
        message: messageOf(error),
      });
    }
  }

  return { commands: buffer.collected(), diagnostics };
}
