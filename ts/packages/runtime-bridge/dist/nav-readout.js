export const GENERATED_TUNNEL_NAV_PROJECTION = {
    id: 'generated_tunnel_nav_projection',
    available: true,
    walkableCells: 66,
    projectionHash: 'd1f6ac3e051d6b6e',
    sourceFixture: 'harness/fixtures/nav/generated-tunnel-path.snapshot.txt',
};
const DEFAULT_NAV_QUERY = {
    start: { kind: 'spawn_marker', id: 'exit_hint' },
    goal: { kind: 'spawn_marker', id: 'player_start' },
    maxVisited: 128,
};
export const GENERATED_TUNNEL_REACHABLE_PATH = {
    scenario: 'generated_tunnel_reachable',
    projection: GENERATED_TUNNEL_NAV_PROJECTION,
    query: DEFAULT_NAV_QUERY,
    outcome: 'reached',
    rejectionReason: null,
    visited: 21,
    path: [
        [3, 1, 7],
        [2, 1, 7],
        [1, 1, 7],
        [1, 1, 6],
        [1, 1, 5],
        [1, 1, 4],
        [1, 1, 3],
        [1, 1, 2],
        [1, 1, 1],
    ],
    pathHash: 'e8e1ea7a09811ced',
};
export const GENERATED_TUNNEL_NO_PATH = {
    scenario: 'generated_tunnel_no_path',
    projection: GENERATED_TUNNEL_NAV_PROJECTION,
    query: DEFAULT_NAV_QUERY,
    outcome: 'no_path',
    rejectionReason: 'blocked',
    visited: 18,
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