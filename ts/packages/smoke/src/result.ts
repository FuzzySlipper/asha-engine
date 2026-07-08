// @asha/smoke — structured smoke-result schema and failure categorization (#2398).
//
// A smoke run produces ONE deterministic, inspectable result record instead of
// ambiguous console noise. A failed run names the exact failing subsystem and an
// actionable next step; a passing run carries enough evidence for a reviewer to
// trust it. The schema is Den-agnostic — any external tool can link to the artifact.

/** Which subsystem a smoke run failed in (stable codes for agent routing). */
export type SmokeFailureCategory =
  | 'missing_native_bridge'
  | 'missing_wasm_target'
  | 'contract_drift'
  | 'missing_fixture'
  | 'load_failure'
  | 'projection_failure'
  | 'render_init_failure'
  | 'ui_command_rejected'
  // Distinct picking/preview/replay/leak classifications so a failure in those
  // stages routes to the right subsystem instead of collapsing into `internal`.
  | 'pick_failure'
  | 'preview_failure'
  | 'replay_failure'
  | 'resource_leak'
  | 'internal';

/** Status of one capability the harness probes. */
export type CapabilityStatus = 'ok' | 'mock' | 'unavailable';

/** Which transport backs the runtime facade for this run. */
export type RuntimeMode = 'native' | 'mock';

/**
 * What the run is trying to *prove*, independent of transport:
 * - `reference`: the deterministic mock/dev smoke (renderer upload path, etc.).
 * - `authority`: the real load→authority→projection→render→edit→save loop,
 *   reading render diffs through the facade and submitting contract-shaped commands.
 */
export type SmokeMode = 'reference' | 'authority';

/**
 * The single, headline outcome category. A passing run says *which* proof it
 * earned; a failed run is never silently downgraded to a mock success.
 */
export type SmokeOutcome = 'mock_reference_passed' | 'native_authority_passed' | 'failed';

/** Outcome of a single named stage of the smoke run. */
export interface SmokeStage {
  readonly name: string;
  readonly ok: boolean;
  /** Stable, human/agent-legible evidence line for the stage. */
  readonly detail: string;
}

/**
 * Deterministic renderer/resource counters captured across the run. These prove the
 * lifecycle is bounded: handles created are destroyed, buffers released, and fallbacks
 * are visible rather than hidden. `leakedHandles`/`outstandingBuffers` must be 0 after
 * the cleanup stage.
 */
export interface SmokeCounters {
  /** Live render handles after cleanup (must be 0 — no leak). */
  readonly leakedHandles: number;
  /** Peak live render handles during the run (scene + overlay). */
  readonly peakHandles: number;
  /** Scene-layer nodes after the render-update stage. */
  readonly sceneNodes: number;
  /** Debug-layer (preview overlay) nodes at preview time. */
  readonly debugNodes: number;
  /** Placeholder-fallback material resolutions (diagnostic, not a failure). */
  readonly fallbackMaterials: number;
  /** Sprite-frame fallbacks (diagnostic). */
  readonly spriteFallbacks: number;
  /** Bridge buffers still held after cleanup (must be 0). */
  readonly outstandingBuffers: number;
}

/** One failure, with the subsystem and an actionable next step. */
export interface SmokeFailure {
  readonly category: SmokeFailureCategory;
  readonly subsystem: string;
  readonly message: string;
  readonly nextStep: string;
}

/** The full, deterministic result of a smoke run. */
export interface SmokeResult {
  readonly ok: boolean;
  readonly command: string;
  readonly runtimeMode: RuntimeMode;
  /** What the run set out to prove (reference vs. real authority path). */
  readonly smokeMode: SmokeMode;
  /** Headline outcome category, distinguishing reference from authority proofs. */
  readonly outcome: SmokeOutcome;
  /** Whether the native addon was loadable (vs. the mock fallback). */
  readonly nativeAvailable: boolean;
  /** Per-capability probe results. */
  readonly capabilities: {
    readonly runtimeBridge: CapabilityStatus;
    readonly projectBundleLoad: CapabilityStatus;
    readonly renderer: CapabilityStatus;
    readonly projection: CapabilityStatus;
  };
  /** The abstract fixture ProjectBundle that was loaded (id + deterministic content hash). */
  readonly fixture: {
    readonly id: number;
    readonly projectBundleHash: string;
  };
  /** Diagnostics summary from the load/composition path. */
  readonly diagnostics: {
    readonly total: number;
    readonly fatal: number;
    readonly blocksLoad: boolean;
  };
  /** Render/projection evidence (node count after applying the fixture frame). */
  readonly render: {
    readonly applied: boolean;
    readonly sceneNodes: number;
  };
  /** Deterministic renderer/resource counters (leak + fallback evidence). */
  readonly counters: SmokeCounters;
  readonly stages: readonly SmokeStage[];
  readonly failures: readonly SmokeFailure[];
}

/** Render a result as a stable, multi-line text report (for the CLI + artifacts). */
export function formatResult(result: SmokeResult): string {
  const lines: string[] = [];
  lines.push(`asha-smoke: ${result.ok ? 'PASS' : 'FAIL'} [${result.outcome}]`);
  lines.push(`command: ${result.command}`);
  lines.push(`smokeMode: ${result.smokeMode}`);
  lines.push(`runtimeMode: ${result.runtimeMode} (nativeAvailable=${result.nativeAvailable})`);
  lines.push(
    `capabilities: runtimeBridge=${result.capabilities.runtimeBridge} ` +
      `projectBundleLoad=${result.capabilities.projectBundleLoad} renderer=${result.capabilities.renderer} ` +
      `projection=${result.capabilities.projection}`,
  );
  lines.push(`fixture: id=${result.fixture.id} projectBundleHash=${result.fixture.projectBundleHash}`);
  lines.push(
    `diagnostics: total=${result.diagnostics.total} fatal=${result.diagnostics.fatal} ` +
      `blocksLoad=${result.diagnostics.blocksLoad}`,
  );
  lines.push(`render: applied=${result.render.applied} sceneNodes=${result.render.sceneNodes}`);
  lines.push(
    `counters: leakedHandles=${result.counters.leakedHandles} peakHandles=${result.counters.peakHandles} ` +
      `sceneNodes=${result.counters.sceneNodes} debugNodes=${result.counters.debugNodes} ` +
      `fallbackMaterials=${result.counters.fallbackMaterials} spriteFallbacks=${result.counters.spriteFallbacks} ` +
      `outstandingBuffers=${result.counters.outstandingBuffers}`,
  );
  for (const stage of result.stages) {
    lines.push(`stage ${stage.name}: ${stage.ok ? 'ok' : 'FAIL'} — ${stage.detail}`);
  }
  for (const failure of result.failures) {
    lines.push(
      `failure [${failure.category}] ${failure.subsystem}: ${failure.message} → ${failure.nextStep}`,
    );
  }
  return lines.join('\n') + '\n';
}
