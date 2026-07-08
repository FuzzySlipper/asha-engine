import type { WorldBundleManifest as GeneratedProjectBundleManifest, // vocab-allow: generated contract keeps legacy name until #5049.
SaveSummary, CompactionSummary } from '@asha/contracts';
import type { ProjectBundleLoadRequest, ProjectBundleSaveSummary } from './index.js';
type IfEqual<A, B, Yes, No> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? Yes : No;
type AssertExact<A, B> = IfEqual<A, B, A, never>;
export type _SchemaVersionMatches = AssertExact<ProjectBundleLoadRequest['bundleSchemaVersion'], GeneratedProjectBundleManifest['bundleSchemaVersion']>;
export type _ProtocolVersionMatches = AssertExact<ProjectBundleLoadRequest['protocolVersion'], GeneratedProjectBundleManifest['protocolVersion']>;
export type _CompactedEditsMatches = AssertExact<ProjectBundleSaveSummary['compactedEdits'], CompactionSummary['compactedEdits']>;
export type _RetainedEditsMatches = AssertExact<ProjectBundleSaveSummary['retainedEdits'], CompactionSummary['retainedEdits']>;
export type _CompactionSectionPresent = AssertExact<SaveSummary['compaction'], CompactionSummary>;
export {};
//# sourceMappingURL=world-dto-conformance.test.d.ts.map