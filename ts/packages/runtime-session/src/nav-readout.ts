export type NavPathScenario = 'generated_tunnel_reachable' | 'generated_tunnel_no_path';

export type NavPathEndpoint =
  | { readonly kind: 'spawn_marker'; readonly id: 'exit_hint' | 'player_start' }
  | { readonly kind: 'voxel'; readonly coord: readonly [number, number, number] };

export interface NavPathQueryRequest {
  readonly scenario?: NavPathScenario;
  readonly start?: NavPathEndpoint;
  readonly goal?: NavPathEndpoint;
  readonly maxVisited?: number;
}

export interface NavProjectionReadout {
  readonly id: 'generated_tunnel_nav_projection';
  readonly available: true;
  readonly walkableCells: 45;
  readonly projectionHash: '59b4093625b10e49';
  readonly sourceFixture: 'harness/fixtures/nav/generated-tunnel-path.snapshot.txt';
}

export interface NavPathReadout {
  readonly scenario: NavPathScenario;
  readonly projection: NavProjectionReadout;
  readonly query: {
    readonly start: NavPathEndpoint;
    readonly goal: NavPathEndpoint;
    readonly maxVisited: number;
  };
  readonly outcome: 'reached' | 'no_path';
  readonly rejectionReason: null | 'blocked';
  readonly visited: number;
  readonly path: readonly (readonly [number, number, number])[];
  readonly pathHash: string;
}

export interface NavPolicyViewReadout {
  readonly kind: 'nav_policy_view.v0';
  readonly projection: NavProjectionReadout;
  readonly defaultQuery: NavPathReadout['query'];
  readonly latestPath: NavPathReadout;
  readonly readOnly: true;
  readonly proposalOnly: true;
}

export const GENERATED_TUNNEL_NAV_PROJECTION: NavProjectionReadout = {
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
} as const;

export const GENERATED_TUNNEL_REACHABLE_PATH: NavPathReadout = {
  scenario: 'generated_tunnel_reachable',
  projection: GENERATED_TUNNEL_NAV_PROJECTION,
  query: DEFAULT_NAV_QUERY,
  outcome: 'reached',
  rejectionReason: null,
  visited: 45,
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

export const GENERATED_TUNNEL_NO_PATH: NavPathReadout = {
  scenario: 'generated_tunnel_no_path',
  projection: GENERATED_TUNNEL_NAV_PROJECTION,
  query: DEFAULT_NAV_QUERY,
  outcome: 'no_path',
  rejectionReason: 'blocked',
  visited: 25,
  path: [],
  pathHash: 'a8c7f832281a39c5',
};

export const GENERATED_TUNNEL_NAV_POLICY_VIEW: NavPolicyViewReadout = {
  kind: 'nav_policy_view.v0',
  projection: GENERATED_TUNNEL_NAV_PROJECTION,
  defaultQuery: DEFAULT_NAV_QUERY,
  latestPath: GENERATED_TUNNEL_REACHABLE_PATH,
  readOnly: true,
  proposalOnly: true,
};
