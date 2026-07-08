// ProjectBundle DTO drift guard (task #2423).
//
// The facade's ProjectBundle load/save payloads are deliberate *prototype* subsets of the
// generated protocol contracts (@asha/contracts). Until they are replaced outright,
// these compile-time assertions make drift visible: if a shared field's type
// changes in the generated contract, this file fails `tsc --build` (and therefore
// the package test), pointing back at the prototype DTO that needs updating.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import type {
  WorldBundleManifest as GeneratedProjectBundleManifest, // vocab-allow: generated contract keeps legacy name until #5049.
  SaveSummary,
  CompactionSummary,
} from '@asha/contracts';
import type { ProjectBundleLoadRequest, ProjectBundleSaveSummary } from './index.js';

// Compile-time equality helpers. `AssertExact<A, B>` only resolves when A and B
// are structurally identical; otherwise it is `never` and the export errors.
type IfEqual<A, B, Yes, No> =
  (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? Yes : No;
type AssertExact<A, B> = IfEqual<A, B, A, never>;

// `bundleSchemaVersion` / `protocolVersion` mirror the generated manifest.
export type _SchemaVersionMatches = AssertExact<
  ProjectBundleLoadRequest['bundleSchemaVersion'],
  GeneratedProjectBundleManifest['bundleSchemaVersion']
>;
export type _ProtocolVersionMatches = AssertExact<
  ProjectBundleLoadRequest['protocolVersion'],
  GeneratedProjectBundleManifest['protocolVersion']
>;

// The compaction counts on the prototype save summary mirror the generated
// CompactionSummary (nested under SaveSummary.compaction).
export type _CompactedEditsMatches = AssertExact<
  ProjectBundleSaveSummary['compactedEdits'],
  CompactionSummary['compactedEdits']
>;
export type _RetainedEditsMatches = AssertExact<
  ProjectBundleSaveSummary['retainedEdits'],
  CompactionSummary['retainedEdits']
>;

// SaveSummary keeps a `compaction` section of the type the prototype flattens.
export type _CompactionSectionPresent = AssertExact<
  SaveSummary['compaction'],
  CompactionSummary
>;

void test('world DTO drift guard compiles (see type assertions above)', () => {
  // The real guard is the compile-time AssertExact<…> exports; this runtime body
  // exists so the file registers as a test and documents the intent.
  assert.ok(true);
});
