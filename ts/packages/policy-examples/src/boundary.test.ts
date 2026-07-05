// Boundary smoke (TypeScript side): proves the commands a policy proposes are
// exactly the shared fixtures under harness/fixtures/commands/ that the Rust
// authority core consumes and validates (see
// engine-rs/crates/sim/sim-validator/tests/policy_command_boundary.rs).
//
// The boundary is: TypeScript proposed command -> shared fixture -> Rust
// validation. TypeScript only proposes/serializes; it never validates.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { commands, makeView, entityId, tagId, type PolicyCommand } from '@asha/script-sdk';
import { definePolicy, invokePolicy } from '@asha/script-host';

import { tagCountThreshold } from './index.js';

const commandsRoot = resolve(import.meta.dirname, '../../../../harness/fixtures/commands');

function loadCommand(name: string): PolicyCommand {
  return JSON.parse(
    readFileSync(resolve(commandsRoot, `${name}.json`), 'utf8'),
  ) as PolicyCommand;
}

void test('threshold policy emits the accepted boundary command fixture', () => {
  // Three entities tagged 1 -> threshold met -> propose defining signal 1.
  const view = makeView({
    entities: [
      { id: entityId(1), tags: [tagId(1)] },
      { id: entityId(2), tags: [tagId(1)] },
      { id: entityId(3), tags: [tagId(1)] },
    ],
    tags: [tagId(1)],
  });

  const result = invokePolicy(definePolicy('threshold', tagCountThreshold), view);

  assert.equal(result.commands.length, 1);
  assert.deepEqual(result.commands[0], loadCommand('threshold-accepted'));
});

void test('SDK can author the command Rust will reject (stale entity delete)', () => {
  // TypeScript can propose a structurally-valid command that the authority core
  // will reject against an empty state. Authoring it here proves the shared
  // fixture is exactly what the SDK produces; Rust owns the accept/reject call.
  assert.deepEqual(commands.deleteEntity(entityId(99)), loadCommand('stale-rejected'));
});
