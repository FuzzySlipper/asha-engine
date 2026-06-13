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
  | 'internal';

/** Status of one capability the harness probes. */
export type CapabilityStatus = 'ok' | 'mock' | 'unavailable';

/** Which transport backs the runtime facade for this run. */
export type RuntimeMode = 'native' | 'mock';

/** Outcome of a single named stage of the smoke run. */
export interface SmokeStage {
  readonly name: string;
  readonly ok: boolean;
  /** Stable, human/agent-legible evidence line for the stage. */
  readonly detail: string;
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
  /** Whether the native addon was loadable (vs. the mock fallback). */
  readonly nativeAvailable: boolean;
  /** Per-capability probe results. */
  readonly capabilities: {
    readonly runtimeBridge: CapabilityStatus;
    readonly worldLoad: CapabilityStatus;
    readonly renderer: CapabilityStatus;
    readonly projection: CapabilityStatus;
  };
  /** The abstract fixture world that was loaded (id + deterministic content hash). */
  readonly fixture: {
    readonly id: number;
    readonly worldHash: string;
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
  readonly stages: readonly SmokeStage[];
  readonly failures: readonly SmokeFailure[];
}

/** Render a result as a stable, multi-line text report (for the CLI + artifacts). */
export function formatResult(result: SmokeResult): string {
  const lines: string[] = [];
  lines.push(`asha-smoke: ${result.ok ? 'PASS' : 'FAIL'}`);
  lines.push(`command: ${result.command}`);
  lines.push(`runtimeMode: ${result.runtimeMode} (nativeAvailable=${result.nativeAvailable})`);
  lines.push(
    `capabilities: runtimeBridge=${result.capabilities.runtimeBridge} ` +
      `worldLoad=${result.capabilities.worldLoad} renderer=${result.capabilities.renderer} ` +
      `projection=${result.capabilities.projection}`,
  );
  lines.push(`fixture: id=${result.fixture.id} worldHash=${result.fixture.worldHash}`);
  lines.push(
    `diagnostics: total=${result.diagnostics.total} fatal=${result.diagnostics.fatal} ` +
      `blocksLoad=${result.diagnostics.blocksLoad}`,
  );
  lines.push(`render: applied=${result.render.applied} sceneNodes=${result.render.sceneNodes}`);
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
