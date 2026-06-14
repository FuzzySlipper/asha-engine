// @asha/script-host — sandboxed world-policy invocation with classified violations
// (#2393).
//
// A constrained world policy may misbehave: it may throw (a crash or a touched
// forbidden global surfaces as a thrown ReferenceError), return something that is
// not an array, or emit a value that is not a well-formed `PolicyWorldCommand`.
// None of these may silently corrupt the run. This module invokes a policy behind a
// guard that classifies every such violation into a stable, agent-legible
// diagnostic and keeps only the well-formed commands.
//
// The host never validates command *semantics* (that is Rust authority via
// `svc-policy-view`); it only guards *shape* and *isolation*.

import type { PolicyEnv, PolicyWorldCommand, WorldPolicyWithEnv } from '@asha/script-sdk';

/** A world policy paired with a stable name, so diagnostics can identify it. */
export interface NamedWorldPolicy {
  readonly name: string;
  readonly policy: WorldPolicyWithEnv;
}

/** Convenience constructor for a {@link NamedWorldPolicy}. */
export function defineWorldPolicy(name: string, policy: WorldPolicyWithEnv): NamedWorldPolicy {
  return { name, policy };
}

/** The classified kinds of sandbox violation a guarded run can surface. */
export type SandboxViolationCode =
  // The policy threw — a crash, or it touched a forbidden ambient capability.
  | 'policyThrew'
  // The policy returned a value that is not an array of commands.
  | 'nonArrayResult'
  // A returned element is not a well-formed PolicyWorldCommand.
  | 'malformedCommand';

/** A structured, stable diagnostic about a sandbox violation. */
export interface SandboxViolation {
  readonly code: SandboxViolationCode;
  /** The `name` of the offending policy. */
  readonly policy: string;
  /** A human-readable, agent-legible detail. */
  readonly detail: string;
}

/** The result of a guarded world-policy run. */
export interface SandboxRunResult {
  /** Only the well-formed proposed commands, in order. */
  readonly commands: readonly PolicyWorldCommand[];
  /** Classified violations; empty on a clean run. */
  readonly violations: readonly SandboxViolation[];
}

const KNOWN_COMMAND_KINDS = new Set([
  'requestSetTransform',
  'requestAddLabel',
  'requestDisable',
  'noopMarker',
]);

/** Whether `value` is structurally a `PolicyWorldCommand` (shape only, not semantics). */
export function isWellFormedCommand(value: unknown): value is PolicyWorldCommand {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const kind = (value as { kind?: unknown }).kind;
  return typeof kind === 'string' && KNOWN_COMMAND_KINDS.has(kind);
}

function messageOf(error: unknown): string {
  return error instanceof Error ? `${error.name}: ${error.message}` : String(error);
}

/**
 * Invoke one named world policy behind the sandbox guard. A throw, a non-array
 * result, or any malformed element is classified into a {@link SandboxViolation};
 * the run still returns whatever well-formed commands were produced (a throw yields
 * none). Never throws.
 */
export function runWorldPolicySandboxed(
  named: NamedWorldPolicy,
  view: Parameters<WorldPolicyWithEnv>[0],
  env: PolicyEnv,
): SandboxRunResult {
  let produced: readonly PolicyWorldCommand[] | unknown;
  try {
    produced = named.policy(view, env);
  } catch (error: unknown) {
    return {
      commands: [],
      violations: [{ code: 'policyThrew', policy: named.name, detail: messageOf(error) }],
    };
  }

  if (!Array.isArray(produced)) {
    return {
      commands: [],
      violations: [
        {
          code: 'nonArrayResult',
          policy: named.name,
          detail: `policy returned ${typeof produced}, expected an array of commands`,
        },
      ],
    };
  }

  const commands: PolicyWorldCommand[] = [];
  const violations: SandboxViolation[] = [];
  for (const [index, candidate] of produced.entries()) {
    if (isWellFormedCommand(candidate)) {
      commands.push(candidate);
    } else {
      violations.push({
        code: 'malformedCommand',
        policy: named.name,
        detail: `result[${index}] is not a well-formed PolicyWorldCommand`,
      });
    }
  }

  return { commands, violations };
}
