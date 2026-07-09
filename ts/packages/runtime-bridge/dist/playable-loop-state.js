export function readRuntimeSessionPlayableLoopState(session, request = {}) {
    const shell = {
        paused: request.shell?.paused ?? false,
        menuMode: request.shell?.menuMode ?? 'closed',
    };
    if (session === null) {
        return missingRuntimeSessionPlayableLoopState(shell, request.unavailableReason);
    }
    const lifecycle = session.readLifecycleStatus();
    const telemetry = session.readTelemetry();
    const ecrp = session.readEcrpRuntimeReadout();
    const currentEpochRecords = recordsSinceLastRestart(telemetry.replayRecords);
    const player = playableHealth(lifecycle.player.health);
    const enemy = playableHealth(lifecycle.enemy.health);
    const target = readEnemyRenderTarget(ecrp.entities.flatMap((entity) => entity.capabilities));
    const blockedReasons = commandBlockReasons({ shellPaused: shell.paused, playerDead: player.dead, enemyDead: enemy.dead });
    const shotsFired = currentEpochRecords.filter(isPlayerFireRecord).length;
    const hits = enemy.max > enemy.current ? Math.min(shotsFired, 1) : 0;
    return {
        kind: 'runtime_session.playable_loop_state.v0',
        status: 'runtime_authority',
        sequenceId: telemetry.sequenceId,
        tick: telemetry.tick,
        sessionHash: telemetry.sessionHash,
        currentEpoch: {
            restartCount: telemetry.restartCount,
            replayRecordStartIndex: telemetry.replayRecords.length - currentEpochRecords.length,
            replayRecordCount: currentEpochRecords.length,
            source: 'after_last_restart_record',
        },
        counters: {
            actionTick: shotsFired,
            shotsFired,
            hits,
            remainingTargets: enemy.dead ? 0 : 1,
            totalTargets: 1,
        },
        health: { player, enemy },
        commands: {
            canFire: blockedReasons.length === 0,
            canRestart: lifecycle.restart.eligible || lifecycle.outcome.terminal || telemetry.restartCount >= 0,
            blockedReasons,
        },
        shell,
        target,
        diagnostics: [],
        nonClaims: ['not_ui_authority', 'not_replay_history_counter', 'not_demo_local_authority'],
    };
}
function isPlayerFireRecord(record) {
    return record.kind === 'submitRuntimeActionIntent' && record.actionSource !== 'enemy_policy';
}
function missingRuntimeSessionPlayableLoopState(shell, unavailableReason) {
    const player = playableHealth({ current: 0, max: 1, dead: true });
    const enemy = playableHealth({ current: 0, max: 1, dead: true });
    return {
        kind: 'runtime_session.playable_loop_state.v0',
        status: 'missing_backend',
        sequenceId: 0,
        tick: 0,
        sessionHash: 'missing-rust-backend',
        currentEpoch: {
            restartCount: 0,
            replayRecordStartIndex: 0,
            replayRecordCount: 0,
            source: 'after_last_restart_record',
        },
        counters: {
            actionTick: 0,
            shotsFired: 0,
            hits: 0,
            remainingTargets: 0,
            totalTargets: 0,
        },
        health: { player, enemy },
        commands: {
            canFire: false,
            canRestart: false,
            blockedReasons: ['missing_backend'],
        },
        shell,
        target: null,
        diagnostics: [{
                code: 'missing_runtime_session',
                message: unavailableReason ?? 'RuntimeSession is unavailable; playable loop state is fail-closed.',
            }],
        nonClaims: ['not_ui_authority', 'not_replay_history_counter', 'not_demo_local_authority'],
    };
}
function playableHealth(health) {
    return {
        current: health.current,
        max: health.max,
        dead: health.dead,
        percent: Math.max(0, Math.min(100, (health.current / health.max) * 100)),
    };
}
function recordsSinceLastRestart(records) {
    for (let index = records.length - 1; index >= 0; index -= 1) {
        const record = records[index];
        if (record?.kind === 'requestSessionRestart' || record?.kind === 'restart') {
            return records.slice(index + 1);
        }
    }
    return records;
}
function commandBlockReasons(input) {
    const reasons = [];
    if (input.shellPaused) {
        reasons.push('paused');
    }
    if (input.playerDead) {
        reasons.push('player_dead');
    }
    if (input.enemyDead) {
        reasons.push('target_defeated');
    }
    return reasons;
}
function readEnemyRenderTarget(capabilities) {
    const renderTargets = capabilities.filter((capability) => capability.kind === 'renderProjection' && capability.target.role === 'enemy');
    const [target] = renderTargets;
    return renderTargets.length === 1 && target !== undefined ? target.target : null;
}
//# sourceMappingURL=playable-loop-state.js.map