import { GENERATED_TUNNEL_NAV_MARKER_CELLS } from './nav-readout.js';
const DEFAULT_ENEMY_POSITION = GENERATED_TUNNEL_NAV_MARKER_CELLS.exit_hint;
const DEFAULT_TARGET_POSITION = GENERATED_TUNNEL_NAV_MARKER_CELLS.player_start;
const FORBIDDEN_CAPABILITY_RULES = [
    { capability: 'clock', token: 'Date', pattern: /\bDate\s*\./ },
    { capability: 'random', token: 'Math.random', pattern: /\bMath\s*\.\s*random\s*\(/ },
    { capability: 'network', token: 'fetch', pattern: /\bfetch\s*\(/ },
    { capability: 'dom', token: 'window', pattern: /\bwindow\b/ },
    { capability: 'dom', token: 'document', pattern: /\bdocument\b/ },
    { capability: 'dom', token: 'localStorage', pattern: /\blocalStorage\b/ },
    { capability: 'filesystem', token: 'node:fs', pattern: /['"]node:fs['"]/ },
    { capability: 'filesystem', token: 'fs', pattern: /\bfrom\s+['"]fs['"]/ },
    { capability: 'process', token: 'process', pattern: /\bprocess\b/ },
    { capability: 'dynamic_code', token: 'eval', pattern: /\beval\s*\(/ },
    { capability: 'dynamic_code', token: 'Function', pattern: /\bFunction\s*\(/ },
    { capability: 'module_import', token: 'import(', pattern: /\bimport\s*\(/ },
    { capability: 'module_import', token: 'require', pattern: /\brequire\s*\(/ },
];
export function createGeneratedTunnelEnemyPolicyFixture(input) {
    const view = createEnemyPolicyView(input);
    return {
        kind: 'generated_tunnel_enemy_policy_fixture.v0',
        view,
        frame: proposeEnemyPolicyFrame(view),
        nonClaims: [
            'not_policy_runtime',
            'not_authority',
            'not_local_state_mutation',
            'not_dom_or_network_capable',
        ],
    };
}
export function createEnemyPolicyView(input) {
    const tick = input.tick ?? 0;
    const enemyPosition = input.enemy?.position ?? DEFAULT_ENEMY_POSITION;
    const targetPosition = input.target?.position ?? DEFAULT_TARGET_POSITION;
    return {
        kind: 'enemy_policy_view.v0',
        tick,
        enemy: {
            id: input.enemy?.id ?? 'generated-tunnel.enemy.1',
            position: enemyPosition,
        },
        target: {
            id: input.target?.id ?? 'generated-tunnel.player',
            position: targetPosition,
            camera: input.target.camera,
        },
        nav: input.nav,
        combat: {
            primaryFireRangeUnits: input.combat?.primaryFireRangeUnits ?? 8,
            lineOfSight: input.combat?.lineOfSight ?? 'clear',
        },
        readOnly: true,
        proposalOnly: true,
    };
}
export function proposeEnemyPolicyFrame(view) {
    const diagnostics = [];
    const proposals = [];
    if (view.kind !== 'enemy_policy_view.v0' || !view.readOnly || !view.proposalOnly) {
        diagnostics.push({
            code: 'invalid_policy_view',
            detail: 'enemy policy view must be the read-only proposal-only v0 shape',
        });
        return frameResult(view.tick, proposals, diagnostics);
    }
    const latestPath = view.nav.latestPath;
    const firstWaypoint = latestPath.path[0] ?? null;
    const secondWaypoint = latestPath.path[1] ?? null;
    const nextWaypoint = secondWaypoint ?? firstWaypoint;
    if (latestPath.outcome === 'reached' && nextWaypoint !== null) {
        proposals.push({
            kind: 'enemy_policy.move_toward_target.v0',
            actor: view.enemy.id,
            target: view.target.id,
            from: view.enemy.position,
            nextWaypoint,
            pathHash: latestPath.pathHash,
            authority: 'rust_runtime_must_validate',
        });
    }
    else {
        diagnostics.push({
            code: 'blocked_nav_path',
            detail: `nav path ${latestPath.pathHash} did not reach the target`,
        });
    }
    const distance = distanceUnits(view.enemy.position, view.target.position);
    if (distance > view.combat.primaryFireRangeUnits) {
        diagnostics.push({
            code: 'target_out_of_range',
            detail: `target is ${distance.toFixed(3)} units away; range is ${view.combat.primaryFireRangeUnits}`,
        });
    }
    else if (view.combat.lineOfSight !== 'clear') {
        diagnostics.push({
            code: 'line_of_sight_blocked',
            detail: 'primary fire proposal requires clear line of sight',
        });
    }
    else {
        proposals.push({
            kind: 'enemy_policy.primary_fire_intent.v0',
            actor: view.enemy.id,
            target: view.target.id,
            intent: {
                kind: 'runtime_action_intent.v0',
                action: 'primary_fire',
                phase: 'pressed',
                camera: view.target.camera,
                tick: view.tick,
                source: 'enemy_policy',
                pressed: true,
            },
            distanceUnits: roundDistance(distance),
            authority: 'rust_runtime_must_validate',
        });
    }
    return frameResult(view.tick, proposals, diagnostics);
}
export function validateEnemyPolicySource(source) {
    const diagnostics = [];
    for (const rule of FORBIDDEN_CAPABILITY_RULES) {
        if (rule.pattern.test(source)) {
            diagnostics.push({
                code: 'forbidden_capability_reference',
                capability: rule.capability,
                token: rule.token,
                detail: `${rule.token} is not available to constrained enemy policies`,
            });
        }
    }
    return diagnostics;
}
function frameResult(tick, proposals, diagnostics) {
    return {
        kind: 'enemy_policy_proposal_frame.v0',
        tick,
        proposals,
        diagnostics,
        proposalHash: hashProposalFrame(tick, proposals, diagnostics),
    };
}
function distanceUnits(a, b) {
    const dx = a[0] - b[0];
    const dy = a[1] - b[1];
    const dz = a[2] - b[2];
    return Math.sqrt(dx * dx + dy * dy + dz * dz);
}
function roundDistance(distance) {
    return Math.round(distance * 1000) / 1000;
}
function hashProposalFrame(tick, proposals, diagnostics) {
    const parts = [`tick:${tick}`];
    for (const proposal of proposals) {
        if (proposal.kind === 'enemy_policy.move_toward_target.v0') {
            parts.push([
                proposal.kind,
                proposal.actor,
                proposal.target,
                proposal.from.join(','),
                proposal.nextWaypoint?.join(',') ?? 'none',
                proposal.pathHash,
            ].join('|'));
        }
        else {
            parts.push([
                proposal.kind,
                proposal.actor,
                proposal.target,
                proposal.intent.action,
                proposal.intent.phase,
                proposal.intent.source,
                proposal.intent.tick,
                proposal.distanceUnits,
            ].join('|'));
        }
    }
    for (const diagnostic of diagnostics) {
        parts.push([diagnostic.code, diagnostic.detail].join('|'));
    }
    return fnv1a64(parts.join('\n'));
}
function fnv1a64(text) {
    let hash = 0xcbf29ce484222325n;
    const prime = 0x100000001b3n;
    const mask = 0xffffffffffffffffn;
    for (let index = 0; index < text.length; index += 1) {
        hash ^= BigInt(text.charCodeAt(index));
        hash = (hash * prime) & mask;
    }
    return hash.toString(16).padStart(16, '0');
}
//# sourceMappingURL=enemy-policy.js.map