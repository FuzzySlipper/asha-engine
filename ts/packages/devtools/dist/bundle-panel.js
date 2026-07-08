// @asha/devtools — project-bundle save/load and diagnostics panel read models
// (#2379).
//
// Observational read models for the project-bundle manifest, the ordered authority
// load plan, the save/compaction plan, generator-mismatch + round-trip diagnostics,
// and a navigable diagnostics panel. The load/save *actions* submit typed requests
// through the runtime-bridge facade only — this module never touches the filesystem
// and never mutates authority. Fail-closed outcomes are surfaced, never papered over.
import { RuntimeBridgeError, } from '@asha/runtime-bridge';
/** Build the manifest inspector model from a generated ProjectBundle manifest. */
export function buildManifestModel(manifest) {
    const classCounts = { durable: 0, generated: 0, cache: 0 };
    const artifacts = manifest.artifacts.map((artifact) => {
        classCounts[artifact.class] += 1;
        return {
            path: artifact.path,
            class: artifact.class,
            role: artifact.role,
            contentHash: artifact.contentHash,
            durableMissingHash: artifact.class === 'durable' && artifact.contentHash === null,
        };
    });
    return {
        bundleSchemaVersion: manifest.bundleSchemaVersion,
        protocolVersion: manifest.protocolVersion,
        projectBundleId: manifest.world.id,
        sceneId: manifest.scene.id,
        assetCount: manifest.assetLock.assetCount,
        artifacts,
        classCounts,
    };
}
function describeLoadStep(step) {
    switch (step.step) {
        case 'validateVersions':
            return `validate versions (bundle ${step.bundleSchemaVersion}, protocol ${step.protocolVersion})`;
        case 'loadAssetLock':
            return `load asset lock ${step.artifact} (${step.assetCount} assets)`;
        case 'loadSceneDocument':
            return `load scene document ${step.artifact} (scene ${step.scene})`;
        case 'generateTerrain':
            return `generate terrain (seed ${step.seed}, generator v${step.version})`;
        case 'applyVoxelEdits':
            return `apply voxel edits (${step.editLogs.length} logs, ${step.snapshots.length} snapshots)`;
        case 'bootstrapScene':
            return `bootstrap scene ${step.scene} → world ${step.world}`;
        case 'restoreWorldState': // vocab-allow: generated load-step tag keeps legacy name until #5049.
            return `restore runtime session state ${step.artifact}`;
        case 'validateFinalState':
            return `validate final state`;
    }
}
/** Build the ordered load-plan read model from a generated load plan. */
export function buildLoadPlanModel(plan) {
    return {
        steps: plan.steps.map((step, index) => ({ index, step: step.step, summary: describeLoadStep(step) })),
    };
}
/** Build the save/compaction read model from a generated save summary. */
export function buildSavePlanModel(summary) {
    const writes = summary.writes.map((artifact) => ({
        path: artifact.path,
        class: artifact.class,
        role: artifact.role,
        contentHash: artifact.contentHash,
        durableMissingHash: artifact.class === 'durable' && artifact.contentHash === null,
    }));
    const compaction = summary.compaction;
    return {
        writes,
        compactedEdits: compaction.compactedEdits,
        retainedEdits: compaction.retainedEdits,
        snapshotChunks: compaction.snapshotChunks.length,
    };
}
/** Build the durability read model from projected evidence (pure, no authority read). */
export function buildVoxelDurabilityModel(evidence) {
    return {
        fixture: evidence.fixture,
        postLoad: evidence.postLoad,
        postEdit: evidence.postEdit,
        postReload: evidence.postReload,
        editedSession: evidence.postLoad !== evidence.postEdit,
        durable: evidence.postEdit === evidence.postReload,
        compactedEdits: evidence.compactedEdits,
        retainedEdits: evidence.retainedEdits,
    };
}
/** Deterministic display lines summarizing save/reload/replay durability. */
export function summarizeVoxelDurability(view) {
    return [
        `fixture ${view.fixture}: durable=${view.durable} edited=${view.editedSession}`,
        `postLoad=${view.postLoad} postEdit=${view.postEdit} postReload=${view.postReload}`,
        `compaction folded=${view.compactedEdits} retained=${view.retainedEdits}`,
    ];
}
/** Describe a fail-closed generator version mismatch (never rewrites a save). */
export function describeGeneratorMismatch(mismatch) {
    return {
        savedVersion: mismatch.savedVersion,
        currentVersion: mismatch.currentVersion,
        detail: `save used generator v${mismatch.savedVersion}; current build is v${mismatch.currentVersion} — regenerate-and-replay to inspect conflicts`,
    };
}
/** Build the round-trip / regenerate-and-replay read model (a diagnostic, never a rewrite). */
export function buildRegenReport(report) {
    return {
        savedVersion: report.savedVersion,
        newVersion: report.newVersion,
        replayedEdits: report.replayedEdits,
        conflictCount: report.conflicts.length,
        stagingWorldHash: report.stagingWorldHash,
        equivalent: report.conflicts.length === 0,
    };
}
/**
 * Resolve a diagnostic's source ref to the most specific available target, so a
 * panel can navigate to the failing render handle / scene node / entity / asset /
 * chunk / artifact path. Returns `none` when no locus is present (never silent).
 */
export function navigateSource(source) {
    if (source.renderHandle !== null) {
        return { kind: 'renderHandle', handle: source.renderHandle };
    }
    if (source.sceneNodeId !== null) {
        return { kind: 'sceneNode', sceneNodeId: source.sceneNodeId };
    }
    if (source.runtimeEntityId !== null) {
        return { kind: 'entity', entityId: source.runtimeEntityId };
    }
    if (source.assetId !== null) {
        return { kind: 'asset', assetId: source.assetId };
    }
    if (source.chunkCoord !== null) {
        return { kind: 'chunk', coord: source.chunkCoord };
    }
    if (source.bundlePath !== null) {
        return { kind: 'bundlePath', path: source.bundlePath };
    }
    return { kind: 'none' };
}
/** Build the diagnostics panel model: severity, remedy, and navigable source per report. */
export function buildDiagnosticsPanel(set) {
    const diagnostics = set.reports.map((report) => ({
        scope: report.scope,
        severity: report.severity,
        code: report.code,
        message: report.message,
        remedy: report.remedy === null ? null : { action: report.remedy.action, detail: report.remedy.detail },
        target: navigateSource(report.source),
    }));
    const fatalCount = diagnostics.filter((d) => d.severity === 'fatal').length;
    return { diagnostics, fatalCount, blocksLoad: fatalCount > 0 };
}
// ── Load / save action requests (through the facade only) ─────────────────────────
/** Derive the typed facade load request from a manifest (no local mutation). */
export function buildProjectBundleLoadRequest(manifest) {
    return {
        bundleSchemaVersion: manifest.bundleSchemaVersion,
        protocolVersion: manifest.protocolVersion,
        sceneId: manifest.scene.id,
    };
}
function recoveryHint(error) {
    switch (error.kind) {
        case 'invalid_input':
            return 'bundle is incompatible with this build — inspect the manifest version/protocol diagnostics';
        case 'not_initialized':
            return 'load a ProjectBundle before saving';
        case 'native_unavailable':
            return 'the native runtime is unavailable — retry on the mock facade or rebuild the addon';
        default:
            return 'inspect composition diagnostics for the failing artifact';
    }
}
/**
 * Submit a project-bundle load through the facade. The prior world is left untouched
 * on failure (the facade stages the swap); this returns a classified result rather
 * than throwing, so a panel can render the fail-closed outcome.
 */
export function submitProjectBundleLoad(bridge, request) {
    try {
        return { ok: true, value: bridge.loadProjectBundle(request) };
    }
    catch (error) {
        if (error instanceof RuntimeBridgeError) {
            return { ok: false, kind: error.kind, message: error.message, recovery: recoveryHint(error) };
        }
        throw error;
    }
}
/** Submit a save through the facade, returning a classified result. */
export function submitProjectBundleSave(bridge) {
    try {
        return { ok: true, value: bridge.saveProjectBundle() };
    }
    catch (error) {
        if (error instanceof RuntimeBridgeError) {
            return { ok: false, kind: error.kind, message: error.message, recovery: recoveryHint(error) };
        }
        throw error;
    }
}
//# sourceMappingURL=bundle-panel.js.map