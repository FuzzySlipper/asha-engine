export type BridgeSurface = 'stable' | 'quarantined';
export interface BridgeOperation {
    /** snake_case name, identical to bridge-manifest.toml `[[operation]].name`. */
    readonly manifestName: string;
    /** camelCase method on the `RuntimeBridge` facade. */
    readonly facadeMethod: string;
    readonly surface: BridgeSurface;
}
export declare const MANIFEST_OPERATIONS: readonly BridgeOperation[];
//# sourceMappingURL=operations.d.ts.map