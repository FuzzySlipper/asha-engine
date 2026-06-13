import type { SceneId, WorldId } from './scene.js';
import type { VoxelCoord, VoxelValue } from './voxel.js';
export type ArtifactClass = 'durable' | 'generated' | 'cache';
export type KnownArtifactRole = 'sceneDocument' | 'assetLock' | 'worldStateSnapshot' | 'voxelChunkSnapshot' | 'voxelEditLog' | 'replayRecord' | 'generatedMetadata' | 'cache';
export type LoadStage = 'versions' | 'assetLock' | 'sceneDocument' | 'terrainGeneration' | 'voxelEdits' | 'bootstrap' | 'finalValidation';
export type SuggestedAction = 'keepEdit' | 'reviewConflict';
export interface ArtifactEntry {
    readonly path: string;
    readonly class: ArtifactClass;
    readonly role: string;
    readonly contentHash: string | null;
}
export interface GeneratorMetadata {
    readonly seed: number;
    readonly version: number;
    readonly params: string;
}
export interface WorldSection {
    readonly id: WorldId;
    readonly name: string | null;
}
export interface SceneSection {
    readonly id: SceneId;
    readonly schemaVersion: number;
    readonly artifact: string;
}
export interface AssetLockSection {
    readonly artifact: string;
    readonly assetCount: number;
}
export interface WorldBundleManifest {
    readonly bundleSchemaVersion: number;
    readonly protocolVersion: number;
    readonly world: WorldSection;
    readonly scene: SceneSection;
    readonly assetLock: AssetLockSection;
    readonly generator: GeneratorMetadata;
    readonly artifacts: readonly ArtifactEntry[];
}
export type ManifestError = {
    readonly code: 'unsupportedSchema';
    readonly found: number;
    readonly supported: number;
} | {
    readonly code: 'unsupportedProtocol';
    readonly found: number;
    readonly supported: number;
} | {
    readonly code: 'duplicateArtifact';
    readonly path: string;
} | {
    readonly code: 'missingArtifact';
    readonly role: string;
    readonly path: string;
} | {
    readonly code: 'durableMissingHash';
    readonly path: string;
};
export interface ManifestValidationReport {
    readonly errors: readonly ManifestError[];
}
export type LoadStep = {
    readonly step: 'validateVersions';
    readonly bundleSchemaVersion: number;
    readonly protocolVersion: number;
} | {
    readonly step: 'loadAssetLock';
    readonly artifact: string;
    readonly assetCount: number;
} | {
    readonly step: 'loadSceneDocument';
    readonly artifact: string;
    readonly scene: SceneId;
} | {
    readonly step: 'generateTerrain';
    readonly seed: number;
    readonly version: number;
    readonly params: string;
} | {
    readonly step: 'applyVoxelEdits';
    readonly editLogs: readonly string[];
    readonly snapshots: readonly string[];
} | {
    readonly step: 'bootstrapScene';
    readonly scene: SceneId;
    readonly world: WorldId;
} | {
    readonly step: 'validateFinalState';
};
export interface LoadPlan {
    readonly steps: readonly LoadStep[];
}
export type LoadPlanError = {
    readonly code: 'manifest';
    readonly error: ManifestError;
} | {
    readonly code: 'missingPrerequisiteArtifact';
    readonly role: string;
} | {
    readonly code: 'outOfOrder';
    readonly step: LoadStage;
    readonly after: LoadStage;
} | {
    readonly code: 'missingStage';
    readonly stage: LoadStage;
};
export interface CompactionSummary {
    readonly compactedEdits: number;
    readonly retainedEdits: number;
    readonly snapshotChunks: readonly string[];
}
export interface SaveSummary {
    readonly writes: readonly ArtifactEntry[];
    readonly compaction: CompactionSummary;
}
export interface GeneratorMismatch {
    readonly savedVersion: number;
    readonly currentVersion: number;
}
export interface EditConflict {
    readonly eventId: number;
    readonly coord: VoxelCoord;
    readonly oldGenerated: VoxelValue;
    readonly newGenerated: VoxelValue;
    readonly editValue: VoxelValue;
    readonly suggested: SuggestedAction;
}
export interface RegenConflictReport {
    readonly savedVersion: number;
    readonly newVersion: number;
    readonly conflicts: readonly EditConflict[];
    readonly replayedEdits: number;
    readonly stagingWorldHash: number;
}
//# sourceMappingURL=worldBundle.d.ts.map