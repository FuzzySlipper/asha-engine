export interface AshaGameManifest {
    readonly asha: {
        readonly engineVersion: string;
        readonly contractsVersion: string;
        readonly runtimeBridgeVersion: string;
        readonly devtoolsProtocolVersion: string;
        readonly publishArtifactFormatVersion: string;
        readonly engineSource: string;
    };
    readonly workspace: {
        readonly sceneRoots: readonly string[];
        readonly assetRoots: readonly string[];
        readonly replayRoots: readonly string[];
        readonly catalogPackages: readonly string[];
        readonly policyPackages: readonly string[];
    };
    readonly runtime: {
        readonly devCommand: string;
        readonly devtoolsEndpoint: string;
        readonly wasmOrNativeEntry: string;
    };
    readonly studio: {
        readonly workspaceMode: boolean;
        readonly attachEnabled: boolean;
        readonly allowedSourceWrites: readonly string[];
    };
    readonly publish: {
        readonly command: string;
        readonly artifactDir: string;
        readonly verifyCommand: string;
    };
}
export type AshaGameManifestDiagnosticCode = 'toml_parse_error' | 'missing_required_field' | 'missing_root' | 'bad_version' | 'unsupported_endpoint' | 'invalid_write_scope' | 'invalid_path';
export interface AshaGameManifestDiagnostic {
    readonly code: AshaGameManifestDiagnosticCode;
    readonly path: string;
    readonly message: string;
}
export type AshaConsumerCompatibilityDiagnosticCode = 'missing_metadata' | 'incompatible_version';
export interface AshaConsumerCompatibilityDiagnostic {
    readonly code: AshaConsumerCompatibilityDiagnosticCode;
    readonly path: string;
    readonly message: string;
}
export interface AshaCompatibilitySurfaceMetadata {
    readonly compatibilityVersion: string;
    readonly packageVersion: string;
}
export interface AshaProtocolCompatibilityMetadata {
    readonly compatibilityVersion: string;
}
export interface AshaConsumerCompatibilityMetadata {
    readonly contracts: AshaCompatibilitySurfaceMetadata;
    readonly runtimeBridge: AshaCompatibilitySurfaceMetadata;
    readonly devtoolsProtocol: AshaProtocolCompatibilityMetadata;
    readonly publishArtifact: AshaProtocolCompatibilityMetadata;
}
export type AshaConsumerCompatibilityValidation = {
    readonly ok: true;
    readonly metadata: AshaConsumerCompatibilityMetadata;
    readonly diagnostics: readonly [];
} | {
    readonly ok: false;
    readonly diagnostics: readonly AshaConsumerCompatibilityDiagnostic[];
};
export declare const ASHA_GAME_WORKSPACE_COMPATIBILITY: AshaConsumerCompatibilityMetadata;
export type AshaGameManifestValidation = {
    readonly ok: true;
    readonly manifest: AshaGameManifest;
    readonly diagnostics: readonly [];
} | {
    readonly ok: false;
    readonly diagnostics: readonly AshaGameManifestDiagnostic[];
};
export type AshaGameAssetKind = 'static_mesh' | 'material' | 'texture' | 'scene';
export interface AshaGameAssetCatalogEntry {
    readonly id: string;
    readonly kind: AshaGameAssetKind;
    readonly source: string;
    readonly importProfile: string | null;
    readonly publish: {
        readonly include: boolean;
        readonly outputKey: string;
    };
    readonly diagnostics: {
        readonly owner: string;
        readonly notes: readonly string[];
    };
}
export interface AshaGameAssetCatalog {
    readonly schemaVersion: 1;
    readonly entries: readonly AshaGameAssetCatalogEntry[];
}
export type AshaGameAssetCatalogDiagnosticCode = 'duplicate_asset_id' | 'missing_asset_file' | 'forbidden_asset_path' | 'unsupported_asset_kind' | 'invalid_asset_entry';
export interface AshaGameAssetCatalogDiagnostic {
    readonly code: AshaGameAssetCatalogDiagnosticCode;
    readonly path: string;
    readonly message: string;
}
export type AshaGameAssetCatalogValidation = {
    readonly ok: true;
    readonly catalog: AshaGameAssetCatalog;
    readonly diagnostics: readonly [];
} | {
    readonly ok: false;
    readonly diagnostics: readonly AshaGameAssetCatalogDiagnostic[];
};
export interface AshaGameAssetDevResolution {
    readonly assetId: string;
    readonly sourcePath: string;
    readonly devCacheKey: string;
    readonly publishOutputKey: string;
}
export interface AshaGamePublishAssetManifest {
    readonly schemaVersion: 1;
    readonly entries: readonly {
        readonly assetId: string;
        readonly kind: AshaGameAssetKind;
        readonly sourcePath: string;
        readonly outputKey: string;
    }[];
}
export declare function parseAshaGameManifestToml(toml: string): AshaGameManifestValidation;
export declare function validateAshaConsumerCompatibility(manifest: AshaGameManifest, metadata: Partial<AshaConsumerCompatibilityMetadata>): AshaConsumerCompatibilityValidation;
export declare function validateAshaGameAssetCatalog(catalog: AshaGameAssetCatalog, manifest: AshaGameManifest, fileExists: (path: string) => boolean): AshaGameAssetCatalogValidation;
export declare function resolveAshaGameAssetForDev(catalog: AshaGameAssetCatalog, assetId: string): AshaGameAssetDevResolution | null;
export declare function buildAshaGamePublishAssetManifest(catalog: AshaGameAssetCatalog): AshaGamePublishAssetManifest;
//# sourceMappingURL=index.d.ts.map