// @asha/policy-core — baseline constrained-policy fixtures.
//
// This package holds the smallest, most fundamental policies. A policy is a
// pure function (see `@asha/script-sdk`'s `Policy` type) from a generated
// read-only view to proposed commands. Policies here propose only; the Rust
// authority core validates. No wall-clock, ambient randomness, DOM, renderer,
// bridge, Electron, filesystem, or network access is permitted (enforced by the
// ts-policy lane's lint and dependency rules).

import type { Policy } from '@asha/script-sdk';

/**
 * The no-op policy: the smallest possible constrained policy.
 *
 * It accepts the read-only view (which it does not read or mutate) and
 * deterministically proposes no commands. It is the baseline fixture for
 * script-host invocation and for sandbox/lint checks — if anything about the
 * policy lane breaks, the no-op policy is the first place it shows.
 */
export const noopPolicy: Policy = () => [];
