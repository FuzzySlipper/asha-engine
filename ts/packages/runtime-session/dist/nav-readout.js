import { projectGeneratedTunnelMarkerToNavCell, readTinyGeneratedTunnelSpawnMarker, } from './generated-tunnel.js';
export const GENERATED_TUNNEL_NAV_PROJECTION = {
    id: 'generated_tunnel_nav_projection',
    available: true,
    walkableCells: 45,
    projectionHash: '59b4093625b10e49',
    sourceFixture: 'harness/fixtures/nav/generated-tunnel-path.snapshot.txt',
};
const DEFAULT_NAV_QUERY = {
    start: { kind: 'spawn_marker', id: 'exit_hint' },
    goal: { kind: 'spawn_marker', id: 'player_start' },
    maxVisited: 128,
};
export const GENERATED_TUNNEL_NAV_MARKER_CELLS = {
    exit_hint: projectGeneratedTunnelMarkerToNavCell(readTinyGeneratedTunnelSpawnMarker('exit_hint')),
    player_start: projectGeneratedTunnelMarkerToNavCell(readTinyGeneratedTunnelSpawnMarker('player_start')),
};
export const GENERATED_TUNNEL_REACHABLE_PATH = {
    scenario: 'generated_tunnel_reachable',
    projection: GENERATED_TUNNEL_NAV_PROJECTION,
    query: DEFAULT_NAV_QUERY,
    outcome: 'reached',
    rejectionReason: null,
    visited: 41,
    path: [
        GENERATED_TUNNEL_NAV_MARKER_CELLS.exit_hint,
        [3, 1, 8],
        [2, 1, 8],
        [2, 1, 7],
        [2, 1, 6],
        [2, 1, 5],
        [2, 1, 4],
        [2, 1, 3],
        GENERATED_TUNNEL_NAV_MARKER_CELLS.player_start,
    ],
    pathHash: '09ed0284f7c175e1',
};
export const GENERATED_TUNNEL_NO_PATH = {
    scenario: 'generated_tunnel_no_path',
    projection: GENERATED_TUNNEL_NAV_PROJECTION,
    query: DEFAULT_NAV_QUERY,
    outcome: 'no_path',
    rejectionReason: 'blocked',
    visited: 25,
    path: [],
    pathHash: 'a8c7f832281a39c5',
};
export const GENERATED_TUNNEL_NAV_POLICY_VIEW = {
    kind: 'nav_policy_view.v0',
    projection: GENERATED_TUNNEL_NAV_PROJECTION,
    defaultQuery: DEFAULT_NAV_QUERY,
    latestPath: GENERATED_TUNNEL_REACHABLE_PATH,
    readOnly: true,
    proposalOnly: true,
};
//# sourceMappingURL=nav-readout.js.map