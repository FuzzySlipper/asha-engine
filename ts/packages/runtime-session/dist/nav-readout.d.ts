export type NavPathScenario = 'generated_tunnel_reachable' | 'generated_tunnel_no_path';
export type NavPathEndpoint = {
    readonly kind: 'spawn_marker';
    readonly id: 'exit_hint' | 'player_start';
} | {
    readonly kind: 'voxel';
    readonly coord: readonly [number, number, number];
};
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
export declare const GENERATED_TUNNEL_NAV_PROJECTION: NavProjectionReadout;
export declare const GENERATED_TUNNEL_NAV_MARKER_CELLS: {
    readonly exit_hint: readonly [number, number, number];
    readonly player_start: readonly [number, number, number];
};
export declare const GENERATED_TUNNEL_REACHABLE_PATH: NavPathReadout;
export declare const GENERATED_TUNNEL_NO_PATH: NavPathReadout;
export declare const GENERATED_TUNNEL_NAV_POLICY_VIEW: NavPolicyViewReadout;
//# sourceMappingURL=nav-readout.d.ts.map