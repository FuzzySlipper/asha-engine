// @asha/smoke — structured smoke-result schema and failure categorization (#2398).
//
// A smoke run produces ONE deterministic, inspectable result record instead of
// ambiguous console noise. A failed run names the exact failing subsystem and an
// actionable next step; a passing run carries enough evidence for a reviewer to
// trust it. The schema is Den-agnostic — any external tool can link to the artifact.
/** Render a result as a stable, multi-line text report (for the CLI + artifacts). */
export function formatResult(result) {
    const lines = [];
    lines.push(`asha-smoke: ${result.ok ? 'PASS' : 'FAIL'}`);
    lines.push(`command: ${result.command}`);
    lines.push(`runtimeMode: ${result.runtimeMode} (nativeAvailable=${result.nativeAvailable})`);
    lines.push(`capabilities: runtimeBridge=${result.capabilities.runtimeBridge} ` +
        `worldLoad=${result.capabilities.worldLoad} renderer=${result.capabilities.renderer} ` +
        `projection=${result.capabilities.projection}`);
    lines.push(`fixture: id=${result.fixture.id} worldHash=${result.fixture.worldHash}`);
    lines.push(`diagnostics: total=${result.diagnostics.total} fatal=${result.diagnostics.fatal} ` +
        `blocksLoad=${result.diagnostics.blocksLoad}`);
    lines.push(`render: applied=${result.render.applied} sceneNodes=${result.render.sceneNodes}`);
    for (const stage of result.stages) {
        lines.push(`stage ${stage.name}: ${stage.ok ? 'ok' : 'FAIL'} — ${stage.detail}`);
    }
    for (const failure of result.failures) {
        lines.push(`failure [${failure.category}] ${failure.subsystem}: ${failure.message} → ${failure.nextStep}`);
    }
    return lines.join('\n') + '\n';
}
//# sourceMappingURL=result.js.map