import { BridgeGameRuntimeSession, type GameRuntimeBackendProfile, type GameRuntimeConfig, type GameRuntimeLauncher, type GameRuntimeSession } from './launcher.js';
export declare function referenceBackendProfile(config: GameRuntimeConfig): GameRuntimeBackendProfile;
export declare class ReferenceGameRuntimeSession extends BridgeGameRuntimeSession {
}
export declare class ReferenceGameRuntimeLauncher implements GameRuntimeLauncher {
    readonly mode = "reference";
    launch(config: GameRuntimeConfig): Promise<GameRuntimeSession>;
}
export declare function createReferenceGameRuntimeLauncher(): GameRuntimeLauncher;
//# sourceMappingURL=reference-launcher.d.ts.map