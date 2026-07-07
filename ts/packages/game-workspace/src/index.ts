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
    readonly backendMode: AshaGameRuntimeBackendMode;
    readonly backendProfile: string;
    readonly backendProofRefs: readonly string[];
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
  readonly devResourceProfile: {
    readonly localRoots: readonly string[];
    readonly cacheDir: string;
    readonly resolutionPolicy: string;
  };
  readonly publishResourceProfile: {
    readonly outputDir: string;
    readonly archiveDir: string;
    readonly resolutionPolicy: string;
  };
}

export type AshaGameManifestDiagnosticCode =
  | 'toml_parse_error'
  | 'missing_required_field'
  | 'missing_root'
  | 'bad_version'
  | 'unsupported_endpoint'
  | 'unsupported_backend_mode'
  | 'missing_backend_ref'
  | 'private_transport_hint'
  | 'invalid_write_scope'
  | 'invalid_resource_profile'
  | 'invalid_path';

export type AshaGameRuntimeBackendMode = 'reference' | 'native' | 'wasm';

export interface AshaGameManifestDiagnostic {
  readonly code: AshaGameManifestDiagnosticCode;
  readonly path: string;
  readonly message: string;
}

export type AshaConsumerCompatibilityDiagnosticCode =
  | 'missing_metadata'
  | 'incompatible_version';

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

export type AshaConsumerCompatibilityValidation =
  | {
      readonly ok: true;
      readonly metadata: AshaConsumerCompatibilityMetadata;
      readonly diagnostics: readonly [];
    }
  | {
      readonly ok: false;
      readonly diagnostics: readonly AshaConsumerCompatibilityDiagnostic[];
    };

export const ASHA_GAME_WORKSPACE_COMPATIBILITY: AshaConsumerCompatibilityMetadata = {
  contracts: { compatibilityVersion: 'contracts.v0', packageVersion: '0.1.0' },
  runtimeBridge: { compatibilityVersion: 'runtime-bridge.v0', packageVersion: '0.1.0' },
  devtoolsProtocol: { compatibilityVersion: 'devtools-protocol.v0' },
  publishArtifact: { compatibilityVersion: 'publish-artifact.v0' },
};

export type AshaGameManifestValidation =
  | {
      readonly ok: true;
      readonly manifest: AshaGameManifest;
      readonly diagnostics: readonly [];
    }
  | {
      readonly ok: false;
      readonly diagnostics: readonly AshaGameManifestDiagnostic[];
    };

export type AshaGameAssetKind = 'static_mesh' | 'material' | 'texture' | 'scene';

export interface AshaGameAssetCatalogEntry {
  readonly id: string;
  readonly kind: AshaGameAssetKind;
  readonly source: string;
  readonly importProfile: string | null;
  readonly importMetadata?: {
    readonly sourceHash: string;
    readonly cacheKey: string;
    readonly generatedArtifactVersion: string;
  };
  readonly dependencies?: readonly string[];
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

export type AshaGameAssetCatalogDiagnosticCode =
  | 'duplicate_asset_id'
  | 'missing_asset_file'
  | 'forbidden_asset_path'
  | 'unsupported_asset_kind'
  | 'missing_asset_dependency'
  | 'duplicate_asset_dependency'
  | 'asset_dependency_cycle'
  | 'stale_import_metadata'
  | 'invalid_asset_entry';

export interface AshaGameAssetCatalogDiagnostic {
  readonly code: AshaGameAssetCatalogDiagnosticCode;
  readonly path: string;
  readonly message: string;
}

export type AshaGameAssetCatalogValidation =
  | {
      readonly ok: true;
      readonly catalog: AshaGameAssetCatalog;
      readonly diagnostics: readonly [];
    }
  | {
      readonly ok: false;
      readonly diagnostics: readonly AshaGameAssetCatalogDiagnostic[];
    };

export interface AshaGameAssetDevResolution {
  readonly assetId: string;
  readonly sourcePath: string;
  readonly sourceHash: string | null;
  readonly devCacheKey: string;
  readonly generatedArtifactVersion: string | null;
  readonly importStatus: 'clean' | 'stale' | 'missing_metadata' | 'unknown';
  readonly publishOutputKey: string;
}

export interface AshaGameAssetCatalogValidationOptions {
  readonly sourceHash?: (path: string) => string | null;
}

export interface AshaGamePublishAssetManifest {
  readonly schemaVersion: 1;
  readonly dependencyOrder: readonly string[];
  readonly entries: readonly {
    readonly assetId: string;
    readonly kind: AshaGameAssetKind;
    readonly sourcePath: string;
    readonly outputKey: string;
  }[];
}

export type AshaAuthoringOperationKind =
  | 'authoring.scene.save_source'
  | 'authoring.catalog.save_source'
  | 'authoring.asset.save_source'
  | 'authoring.policy.save_source';

export type AshaAuthoringSourceFormat =
  | 'proof-scene-json.v1'
  | 'asset-catalog-json.v1'
  | 'inline-asset-json.v1'
  | 'policy-json.deferred';

export type AshaAuthoringDiagnosticCode =
  | 'unsupported_operation'
  | 'disallowed_path'
  | 'invalid_extension'
  | 'forbidden_generated_path'
  | 'private_transport_hint'
  | 'stale_file_hash'
  | 'invalid_schema';

export interface AshaAuthoringDiagnostic {
  readonly code: AshaAuthoringDiagnosticCode;
  readonly path: string;
  readonly message: string;
}

export interface AshaAuthoringWriteScope {
  readonly operationKind: AshaAuthoringOperationKind;
  readonly allowedRoots: readonly string[];
  readonly format: AshaAuthoringSourceFormat;
  readonly requiredValidator: string;
}

export interface AshaAuthoringPersistenceContract {
  readonly contractVersion: 'authoring-persistence.v0';
  readonly writeScopes: readonly AshaAuthoringWriteScope[];
  readonly forbiddenRoots: readonly string[];
  readonly diagnostics: readonly AshaAuthoringDiagnostic[];
  readonly nonClaims: readonly string[];
}

export interface AshaAuthoringSaveRequest {
  readonly operationKind: AshaAuthoringOperationKind;
  readonly relativePath: string;
  readonly expectedPreviousHash: string | null;
  readonly payloadText: string;
}

export interface AshaAuthoringSaveReadback {
  readonly operationKind: AshaAuthoringOperationKind;
  readonly normalizedPath: string;
  readonly allowedRoot: string;
  readonly previousFileHash: string | null;
  readonly nextFileHash: string;
  readonly semanticDiffHash: string;
  readonly validationDiagnosticsHash: string;
  readonly dependentReadbackHashes: readonly string[];
}

export type AshaAuthoringSaveResult =
  | {
      readonly ok: true;
      readonly readback: AshaAuthoringSaveReadback;
      readonly diagnostics: readonly [];
    }
  | {
      readonly ok: false;
      readonly diagnostics: readonly AshaAuthoringDiagnostic[];
    };

export type AshaAuthoringWriteTargetResolution =
  | {
      readonly ok: true;
      readonly operationKind: AshaAuthoringOperationKind;
      readonly normalizedPath: string;
      readonly allowedRoot: string;
      readonly format: AshaAuthoringSourceFormat;
      readonly requiredValidator: string;
      readonly diagnostics: readonly [];
    }
  | {
      readonly ok: false;
      readonly diagnostics: readonly AshaAuthoringDiagnostic[];
    };

type TomlScalar = string | boolean | readonly string[];
type TomlSection = Record<string, TomlScalar>;
type TomlDocument = Record<string, TomlSection>;

const REQUIRED_SECTIONS = ['asha', 'workspace', 'runtime', 'studio', 'publish', 'dev_resource_profile', 'publish_resource_profile'] as const;
const VERSION_PATTERN = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;
const LOCAL_WEBSOCKET_ENDPOINT_PATTERN = /^wss?:\/\/(?:127\.0\.0\.1|localhost):\d+(?:\/[A-Za-z0-9._~:/?#[\]@!$&'()*+,;=-]*)?$/;

export function parseAshaGameManifestToml(toml: string): AshaGameManifestValidation {
  const parsed = parseTomlSubset(toml);
  if (!parsed.ok) {
    return { ok: false, diagnostics: parsed.diagnostics };
  }

  return decodeAndValidateManifest(parsed.document);
}

export function validateAshaConsumerCompatibility(
  manifest: AshaGameManifest,
  metadata: Partial<AshaConsumerCompatibilityMetadata>,
): AshaConsumerCompatibilityValidation {
  const diagnostics: AshaConsumerCompatibilityDiagnostic[] = [];
  const contracts = requireSurface(metadata.contracts, 'contracts', diagnostics);
  const runtimeBridge = requireSurface(metadata.runtimeBridge, 'runtimeBridge', diagnostics);
  const devtoolsProtocol = requireProtocol(metadata.devtoolsProtocol, 'devtoolsProtocol', diagnostics);
  const publishArtifact = requireProtocol(metadata.publishArtifact, 'publishArtifact', diagnostics);

  if (contracts !== null) {
    compareVersion(manifest.asha.contractsVersion, contracts.packageVersion, 'asha.contracts_version', diagnostics);
  }
  if (runtimeBridge !== null) {
    compareVersion(manifest.asha.runtimeBridgeVersion, runtimeBridge.packageVersion, 'asha.runtime_bridge_version', diagnostics);
  }
  if (devtoolsProtocol !== null) {
    compareVersion(manifest.asha.devtoolsProtocolVersion, devtoolsProtocol.compatibilityVersion, 'asha.devtools_protocol_version', diagnostics);
  }
  if (publishArtifact !== null) {
    compareVersion(manifest.asha.publishArtifactFormatVersion, publishArtifact.compatibilityVersion, 'asha.publish_artifact_format_version', diagnostics);
  }

  if (diagnostics.length > 0 || contracts === null || runtimeBridge === null || devtoolsProtocol === null || publishArtifact === null) {
    return { ok: false, diagnostics };
  }

  return {
    ok: true,
    metadata: { contracts, runtimeBridge, devtoolsProtocol, publishArtifact },
    diagnostics: [],
  };
}

export function validateAshaGameAssetCatalog(
  catalog: AshaGameAssetCatalog,
  manifest: AshaGameManifest,
  fileExists: (path: string) => boolean,
  options: AshaGameAssetCatalogValidationOptions = {},
): AshaGameAssetCatalogValidation {
  const diagnostics: AshaGameAssetCatalogDiagnostic[] = [];
  const seen = new Set<string>();
  for (const [index, entry] of catalog.entries.entries()) {
    const path = `entries[${index}]`;
    if (entry.id.length === 0 || entry.source.length === 0 || entry.publish.outputKey.length === 0) {
      diagnostics.push(assetDiag('invalid_asset_entry', path, 'asset id, source, and publish output key are required'));
    }
    if (seen.has(entry.id)) {
      diagnostics.push(assetDiag('duplicate_asset_id', `${path}.id`, `duplicate asset id "${entry.id}"`));
    }
    seen.add(entry.id);
    if (!isSupportedAssetKind(entry.kind)) {
      diagnostics.push(assetDiag('unsupported_asset_kind', `${path}.kind`, `unsupported asset kind "${entry.kind}"`));
    } else {
      validateKindSpecificAssetEntry(entry, path, diagnostics);
    }
    if (!manifest.workspace.assetRoots.some((root) => isSameOrChildPath(entry.source, root))) {
      diagnostics.push(assetDiag('forbidden_asset_path', `${path}.source`, `asset source "${entry.source}" is outside manifest asset roots`));
    } else if (!fileExists(entry.source)) {
      diagnostics.push(assetDiag('missing_asset_file', `${path}.source`, `asset source does not exist: ${entry.source}`));
    }
    validateImportMetadata(entry, path, options, diagnostics);
  }
  validateAssetDependencyGraph(catalog, diagnostics);

  return diagnostics.length === 0 ? { ok: true, catalog, diagnostics: [] } : { ok: false, diagnostics };
}

export function resolveAshaGameAssetForDev(
  catalog: AshaGameAssetCatalog,
  assetId: string,
  sourceHash?: string | null,
): AshaGameAssetDevResolution | null {
  const entry = catalog.entries.find((candidate) => candidate.id === assetId);
  if (entry === undefined) {
    return null;
  }
  const observedSourceHash = sourceHash ?? entry.importMetadata?.sourceHash ?? null;
  const metadata = entry.importMetadata;
  const importStatus =
    metadata === undefined
      ? 'missing_metadata'
      : sourceHash === undefined || sourceHash === null
        ? 'unknown'
        : sourceHash === metadata.sourceHash
          ? 'clean'
          : 'stale';
  return {
    assetId: entry.id,
    sourcePath: entry.source,
    sourceHash: observedSourceHash,
    devCacheKey: metadata?.cacheKey ?? `dev-cache/${entry.kind}/${entry.id}`,
    generatedArtifactVersion: metadata?.generatedArtifactVersion ?? null,
    importStatus,
    publishOutputKey: entry.publish.outputKey,
  };
}

export function buildAshaGamePublishAssetManifest(catalog: AshaGameAssetCatalog): AshaGamePublishAssetManifest {
  const dependencyOrder = orderAssetDependencies(catalog).filter((assetId) => {
    const entry = catalog.entries.find((candidate) => candidate.id === assetId);
    return entry?.publish.include === true;
  });
  return {
    schemaVersion: 1,
    dependencyOrder,
    entries: catalog.entries
      .filter((entry) => entry.publish.include)
      .map((entry) => ({
        assetId: entry.id,
        kind: entry.kind,
        sourcePath: entry.source,
        outputKey: entry.publish.outputKey,
      })),
  };
}

export function buildAshaAuthoringPersistenceContract(manifest: AshaGameManifest): AshaAuthoringPersistenceContract {
  const writeScopes = authoringWriteScopes(manifest);
  return {
    contractVersion: 'authoring-persistence.v0',
    writeScopes,
    forbiddenRoots: ['harness/out', 'node_modules', '.git', '../asha-engine', '../asha-studio'],
    diagnostics: writeScopes.flatMap((scope) =>
      scope.operationKind === 'authoring.policy.save_source'
        ? [authoringDiag('unsupported_operation', scope.operationKind, 'policy authoring is reserved until a policy schema contract exists')]
        : [],
    ),
    nonClaims: [
      'not_repo_crawler',
      'not_private_asset_database',
      'not_runtime_authority',
      'not_generated_artifact_source',
    ],
  };
}

export function resolveAshaAuthoringWriteTarget(
  manifest: AshaGameManifest,
  request: Pick<AshaAuthoringSaveRequest, 'operationKind' | 'relativePath'>,
): AshaAuthoringWriteTargetResolution {
  const diagnostics: AshaAuthoringDiagnostic[] = [];
  const normalizedPath = normalizeAuthoringPath(request.relativePath, diagnostics);
  const scope = authoringWriteScopes(manifest).find((candidate) => candidate.operationKind === request.operationKind);
  if (scope === undefined) {
    diagnostics.push(authoringDiag('unsupported_operation', 'operationKind', `unsupported authoring operation "${request.operationKind}"`));
    return { ok: false, diagnostics };
  }
  if (scope.operationKind === 'authoring.policy.save_source') {
    diagnostics.push(authoringDiag('unsupported_operation', request.operationKind, 'policy authoring is reserved until a policy schema contract exists'));
  }
  if (normalizedPath !== null) {
    if (isGeneratedOrPrivateAuthoringPath(normalizedPath)) {
      diagnostics.push(authoringDiag('forbidden_generated_path', 'relativePath', `authoring save cannot target generated/private path "${normalizedPath}"`));
    }
    if (containsPrivateTransportHint(normalizedPath)) {
      diagnostics.push(authoringDiag('private_transport_hint', 'relativePath', `authoring save cannot target private transport path "${normalizedPath}"`));
    }
    const allowedRoot = scope.allowedRoots.find((root) => isSameOrChildPath(normalizedPath, root));
    if (allowedRoot === undefined) {
      diagnostics.push(authoringDiag('disallowed_path', 'relativePath', `path "${normalizedPath}" is outside allowed roots for ${request.operationKind}`));
    }
    validateAuthoringExtension(scope, normalizedPath, diagnostics);
    if (diagnostics.length === 0 && allowedRoot !== undefined) {
      return {
        ok: true,
        operationKind: request.operationKind,
        normalizedPath,
        allowedRoot,
        format: scope.format,
        requiredValidator: scope.requiredValidator,
        diagnostics: [],
      };
    }
  }
  return { ok: false, diagnostics };
}

function validateAssetDependencyGraph(
  catalog: AshaGameAssetCatalog,
  diagnostics: AshaGameAssetCatalogDiagnostic[],
): void {
  const ids = new Set(catalog.entries.map((entry) => entry.id));
  for (const [index, entry] of catalog.entries.entries()) {
    const seen = new Set<string>();
    for (const dependency of entry.dependencies ?? []) {
      if (seen.has(dependency)) {
        diagnostics.push(assetDiag('duplicate_asset_dependency', `entries[${index}].dependencies`, `asset "${entry.id}" repeats dependency "${dependency}"`));
      }
      seen.add(dependency);
      if (!ids.has(dependency)) {
        diagnostics.push(assetDiag('missing_asset_dependency', `entries[${index}].dependencies`, `asset "${entry.id}" depends on missing asset "${dependency}"`));
      }
    }
  }

  const visiting = new Set<string>();
  const visited = new Set<string>();
  const byId = new Map(catalog.entries.map((entry) => [entry.id, entry]));
  function visit(assetId: string, trail: readonly string[]): void {
    if (visited.has(assetId)) return;
    if (visiting.has(assetId)) {
      diagnostics.push(assetDiag('asset_dependency_cycle', 'entries.dependencies', `asset dependency cycle: ${[...trail, assetId].join(' -> ')}`));
      return;
    }
    visiting.add(assetId);
    const entry = byId.get(assetId);
    for (const dependency of entry?.dependencies ?? []) {
      if (byId.has(dependency)) visit(dependency, [...trail, assetId]);
    }
    visiting.delete(assetId);
    visited.add(assetId);
  }
  for (const entry of catalog.entries) visit(entry.id, []);
}

function orderAssetDependencies(catalog: AshaGameAssetCatalog): readonly string[] {
  const byId = new Map(catalog.entries.map((entry) => [entry.id, entry]));
  const visited = new Set<string>();
  const ordered: string[] = [];
  function visit(assetId: string): void {
    if (visited.has(assetId)) return;
    visited.add(assetId);
    const entry = byId.get(assetId);
    for (const dependency of entry?.dependencies ?? []) {
      if (byId.has(dependency)) visit(dependency);
    }
    ordered.push(assetId);
  }
  for (const entry of catalog.entries) visit(entry.id);
  return ordered;
}

function parseTomlSubset(toml: string): { readonly ok: true; readonly document: TomlDocument } | { readonly ok: false; readonly diagnostics: readonly AshaGameManifestDiagnostic[] } {
  const document: TomlDocument = {};
  let currentSection: string | null = null;
  const diagnostics: AshaGameManifestDiagnostic[] = [];

  toml.split(/\r?\n/).forEach((rawLine, index) => {
    const lineNumber = index + 1;
    const line = stripComment(rawLine).trim();
    if (line.length === 0) {
      return;
    }

    const sectionMatch = /^\[([A-Za-z0-9_-]+)\]$/.exec(line);
    if (sectionMatch) {
      currentSection = sectionMatch[1]!;
      document[currentSection] ??= {};
      return;
    }

    if (currentSection === null) {
      diagnostics.push(diag('toml_parse_error', `line ${lineNumber}`, 'manifest keys must be inside a section'));
      return;
    }

    const assignmentMatch = /^([A-Za-z0-9_]+)\s*=\s*(.+)$/.exec(line);
    if (!assignmentMatch) {
      diagnostics.push(diag('toml_parse_error', `line ${lineNumber}`, 'expected key = value'));
      return;
    }

    const key = assignmentMatch[1]!;
    const rawValue = assignmentMatch[2]!.trim();
    const value = parseTomlValue(rawValue, `line ${lineNumber}`);
    if (value.ok) {
      document[currentSection]![key] = value.value;
    } else {
      diagnostics.push(value.diagnostic);
    }
  });

  return diagnostics.length === 0 ? { ok: true, document } : { ok: false, diagnostics };
}

function stripComment(line: string): string {
  let inString = false;
  for (let i = 0; i < line.length; i += 1) {
    const char = line[i];
    if (char === '"' && line[i - 1] !== '\\') {
      inString = !inString;
    }
    if (char === '#' && !inString) {
      return line.slice(0, i);
    }
  }
  return line;
}

function parseTomlValue(rawValue: string, path: string): { readonly ok: true; readonly value: TomlScalar } | { readonly ok: false; readonly diagnostic: AshaGameManifestDiagnostic } {
  if (rawValue === 'true') {
    return { ok: true, value: true };
  }
  if (rawValue === 'false') {
    return { ok: true, value: false };
  }
  if (rawValue.startsWith('"') && rawValue.endsWith('"')) {
    return { ok: true, value: rawValue.slice(1, -1) };
  }
  if (rawValue.startsWith('[') && rawValue.endsWith(']')) {
    const inner = rawValue.slice(1, -1).trim();
    if (inner.length === 0) {
      return { ok: true, value: [] };
    }
    const values = inner.split(',').map((part) => part.trim());
    if (!values.every((part) => part.startsWith('"') && part.endsWith('"'))) {
      return { ok: false, diagnostic: diag('toml_parse_error', path, 'only string arrays are supported in asha.game.toml') };
    }
    return { ok: true, value: values.map((part) => part.slice(1, -1)) };
  }
  return { ok: false, diagnostic: diag('toml_parse_error', path, 'expected a string, boolean, or string array') };
}

function decodeAndValidateManifest(document: TomlDocument): AshaGameManifestValidation {
  const diagnostics: AshaGameManifestDiagnostic[] = [];
  for (const section of REQUIRED_SECTIONS) {
    if (document[section] === undefined) {
      diagnostics.push(diag('missing_required_field', section, `missing [${section}] section`));
    }
  }

  const manifest: AshaGameManifest = {
    asha: {
      engineVersion: getString(document, 'asha', 'engine_version', diagnostics),
      contractsVersion: getString(document, 'asha', 'contracts_version', diagnostics),
      runtimeBridgeVersion: getString(document, 'asha', 'runtime_bridge_version', diagnostics),
      devtoolsProtocolVersion: getString(document, 'asha', 'devtools_protocol_version', diagnostics),
      publishArtifactFormatVersion: getString(document, 'asha', 'publish_artifact_format_version', diagnostics),
      engineSource: getString(document, 'asha', 'engine_source', diagnostics),
    },
    workspace: {
      sceneRoots: getStringArray(document, 'workspace', 'scene_roots', diagnostics),
      assetRoots: getStringArray(document, 'workspace', 'asset_roots', diagnostics),
      replayRoots: getStringArray(document, 'workspace', 'replay_roots', diagnostics),
      catalogPackages: getStringArray(document, 'workspace', 'catalog_packages', diagnostics),
      policyPackages: getStringArray(document, 'workspace', 'policy_packages', diagnostics),
    },
    runtime: {
      devCommand: getString(document, 'runtime', 'dev_command', diagnostics),
      devtoolsEndpoint: getString(document, 'runtime', 'devtools_endpoint', diagnostics),
      wasmOrNativeEntry: getString(document, 'runtime', 'wasm_or_native_entry', diagnostics),
      backendMode: getBackendMode(document, diagnostics),
      backendProfile: getString(document, 'runtime', 'backend_profile', diagnostics),
      backendProofRefs: getStringArray(document, 'runtime', 'backend_proof_refs', diagnostics),
    },
    studio: {
      workspaceMode: getBoolean(document, 'studio', 'workspace_mode', diagnostics),
      attachEnabled: getBoolean(document, 'studio', 'attach_enabled', diagnostics),
      allowedSourceWrites: getStringArray(document, 'studio', 'allowed_source_writes', diagnostics),
    },
    publish: {
      command: getString(document, 'publish', 'command', diagnostics),
      artifactDir: getString(document, 'publish', 'artifact_dir', diagnostics),
      verifyCommand: getString(document, 'publish', 'verify_command', diagnostics),
    },
    devResourceProfile: {
      localRoots: getStringArray(document, 'dev_resource_profile', 'local_roots', diagnostics),
      cacheDir: getString(document, 'dev_resource_profile', 'cache_dir', diagnostics),
      resolutionPolicy: getString(document, 'dev_resource_profile', 'resolution_policy', diagnostics),
    },
    publishResourceProfile: {
      outputDir: getString(document, 'publish_resource_profile', 'output_dir', diagnostics),
      archiveDir: getString(document, 'publish_resource_profile', 'archive_dir', diagnostics),
      resolutionPolicy: getString(document, 'publish_resource_profile', 'resolution_policy', diagnostics),
    },
  };

  validateManifest(manifest, diagnostics);
  return diagnostics.length === 0 ? { ok: true, manifest, diagnostics: [] } : { ok: false, diagnostics };
}

function validateManifest(manifest: AshaGameManifest, diagnostics: AshaGameManifestDiagnostic[]): void {
  validateVersion(manifest.asha.engineVersion, 'asha.engine_version', diagnostics);
  validateVersion(manifest.asha.contractsVersion, 'asha.contracts_version', diagnostics);
  validateVersion(manifest.asha.runtimeBridgeVersion, 'asha.runtime_bridge_version', diagnostics);

  validateNonEmptyRoots(manifest.workspace.sceneRoots, 'workspace.scene_roots', diagnostics);
  validateNonEmptyRoots(manifest.workspace.assetRoots, 'workspace.asset_roots', diagnostics);
  validateNonEmptyRoots(manifest.workspace.replayRoots, 'workspace.replay_roots', diagnostics);
  validateEngineSource(manifest.asha.engineSource, 'asha.engine_source', diagnostics);
  validatePath(manifest.runtime.wasmOrNativeEntry, 'runtime.wasm_or_native_entry', diagnostics);
  validateBackendMode(manifest, diagnostics);
  validatePath(manifest.publish.artifactDir, 'publish.artifact_dir', diagnostics);
  validateResourceProfiles(manifest, diagnostics);

  if (!LOCAL_WEBSOCKET_ENDPOINT_PATTERN.test(manifest.runtime.devtoolsEndpoint)) {
    diagnostics.push(diag('unsupported_endpoint', 'runtime.devtools_endpoint', 'devtools endpoint must be a local ws:// or wss:// URL with an explicit port'));
  }

  const writeRoots = [
    ...manifest.workspace.sceneRoots,
    ...manifest.workspace.assetRoots,
    ...manifest.workspace.catalogPackages,
    ...manifest.workspace.policyPackages,
  ];
  for (const writeScope of manifest.studio.allowedSourceWrites) {
    validatePath(writeScope, 'studio.allowed_source_writes', diagnostics);
    if (!writeRoots.some((root) => isSameOrChildPath(writeScope, root))) {
      diagnostics.push(diag('invalid_write_scope', 'studio.allowed_source_writes', `write scope "${writeScope}" is not within a declared workspace root`));
    }
  }
}

function validateResourceProfiles(manifest: AshaGameManifest, diagnostics: AshaGameManifestDiagnostic[]): void {
  validateNonEmptyRoots(manifest.devResourceProfile.localRoots, 'dev_resource_profile.local_roots', diagnostics);
  validatePath(manifest.devResourceProfile.cacheDir, 'dev_resource_profile.cache_dir', diagnostics);
  validatePath(manifest.publishResourceProfile.outputDir, 'publish_resource_profile.output_dir', diagnostics);
  validatePath(manifest.publishResourceProfile.archiveDir, 'publish_resource_profile.archive_dir', diagnostics);

  const workspaceRoots = [
    ...manifest.workspace.sceneRoots,
    ...manifest.workspace.assetRoots,
    ...manifest.workspace.replayRoots,
    ...manifest.workspace.catalogPackages,
    ...manifest.workspace.policyPackages,
  ];
  for (const root of manifest.devResourceProfile.localRoots) {
    if (!workspaceRoots.some((workspaceRoot) => isSameOrChildPath(root, workspaceRoot))) {
      diagnostics.push(diag('invalid_resource_profile', 'dev_resource_profile.local_roots', `dev resource root "${root}" is not within a declared workspace root`));
    }
  }

  for (const [path, value] of [
    ['publish_resource_profile.output_dir', manifest.publishResourceProfile.outputDir],
    ['publish_resource_profile.archive_dir', manifest.publishResourceProfile.archiveDir],
  ] as const) {
    if (workspaceRoots.some((root) => isSameOrChildPath(value, root))) {
      diagnostics.push(diag('invalid_resource_profile', path, `publish resource path "${value}" must not be inside a dev-local workspace root`));
    }
  }

  if (manifest.devResourceProfile.resolutionPolicy !== 'prefer-source') {
    diagnostics.push(diag('invalid_resource_profile', 'dev_resource_profile.resolution_policy', 'dev resolution_policy must be "prefer-source"'));
  }
  if (manifest.publishResourceProfile.resolutionPolicy !== 'locked') {
    diagnostics.push(diag('invalid_resource_profile', 'publish_resource_profile.resolution_policy', 'publish resolution_policy must be "locked"'));
  }
}

function requireSurface(
  surface: AshaCompatibilitySurfaceMetadata | undefined,
  path: string,
  diagnostics: AshaConsumerCompatibilityDiagnostic[],
): AshaCompatibilitySurfaceMetadata | null {
  if (surface === undefined || surface.compatibilityVersion.length === 0 || surface.packageVersion.length === 0) {
    diagnostics.push(compatDiag('missing_metadata', path, `missing ${path} compatibility metadata`));
    return null;
  }
  return surface;
}

function requireProtocol(
  protocol: AshaProtocolCompatibilityMetadata | undefined,
  path: string,
  diagnostics: AshaConsumerCompatibilityDiagnostic[],
): AshaProtocolCompatibilityMetadata | null {
  if (protocol === undefined || protocol.compatibilityVersion.length === 0) {
    diagnostics.push(compatDiag('missing_metadata', path, `missing ${path} compatibility metadata`));
    return null;
  }
  return protocol;
}

function compareVersion(
  manifestVersion: string,
  metadataVersion: string,
  path: string,
  diagnostics: AshaConsumerCompatibilityDiagnostic[],
): void {
  if (manifestVersion !== metadataVersion) {
    diagnostics.push(compatDiag('incompatible_version', path, `manifest declares "${manifestVersion}" but ASHA metadata provides "${metadataVersion}"`));
  }
}

function validateVersion(version: string, path: string, diagnostics: AshaGameManifestDiagnostic[]): void {
  if (!VERSION_PATTERN.test(version)) {
    diagnostics.push(diag('bad_version', path, `version "${version}" must be semver-like x.y.z`));
  }
}

function validateNonEmptyRoots(roots: readonly string[], path: string, diagnostics: AshaGameManifestDiagnostic[]): void {
  if (roots.length === 0) {
    diagnostics.push(diag('missing_root', path, 'at least one root is required'));
  }
  for (const root of roots) {
    validatePath(root, path, diagnostics);
  }
}

function validatePath(pathValue: string, path: string, diagnostics: AshaGameManifestDiagnostic[]): void {
  if (pathValue.length === 0 || pathValue.startsWith('/') || pathValue.split('/').includes('..')) {
    diagnostics.push(diag('invalid_path', path, `path "${pathValue}" must be non-empty, relative, and remain inside the game workspace`));
  }
}

function validateEngineSource(engineSource: string, path: string, diagnostics: AshaGameManifestDiagnostic[]): void {
  if (engineSource.length === 0 || engineSource.includes('engine-rs/crates') || engineSource.includes('/src/')) {
    diagnostics.push(diag('invalid_path', path, 'engine source must be a package/version or repo root path, not an ASHA internal source path'));
  }
}

function containsPrivateTransportHint(value: string): boolean {
  return [
    '@asha/native-bridge',
    '@asha/wasm-bridge',
    '@asha/wasm-replay-bridge',
    'native-bridge.node',
    'wasm.memory',
    'engine-rs/',
    '/src/',
  ].some((hint) => value.includes(hint));
}

function authoringWriteScopes(manifest: AshaGameManifest): readonly AshaAuthoringWriteScope[] {
  const allowed = new Set(manifest.studio.allowedSourceWrites);
  const allowedRoots = (roots: readonly string[]) => roots.filter((root) =>
    [...allowed].some((writeRoot) => isSameOrChildPath(writeRoot, root) || isSameOrChildPath(root, writeRoot)),
  );
  return [
    {
      operationKind: 'authoring.scene.save_source',
      allowedRoots: allowedRoots(manifest.workspace.sceneRoots),
      format: 'proof-scene-json.v1',
      requiredValidator: 'validateAshaProofSceneSourceDocument',
    },
    {
      operationKind: 'authoring.catalog.save_source',
      allowedRoots: allowedRoots(manifest.workspace.catalogPackages),
      format: 'asset-catalog-json.v1',
      requiredValidator: 'validateAshaGameAssetCatalog',
    },
    {
      operationKind: 'authoring.asset.save_source',
      allowedRoots: allowedRoots(manifest.workspace.assetRoots),
      format: 'inline-asset-json.v1',
      requiredValidator: 'validateAshaCatalogAssetPayload',
    },
    {
      operationKind: 'authoring.policy.save_source',
      allowedRoots: allowedRoots(manifest.workspace.policyPackages),
      format: 'policy-json.deferred',
      requiredValidator: 'deferred-policy-schema-contract',
    },
  ];
}

function normalizeAuthoringPath(value: string, diagnostics: AshaAuthoringDiagnostic[]): string | null {
  const replaced = value.replace(/\\/g, '/');
  const parts = replaced.split('/');
  if (value.length === 0 || replaced.startsWith('/') || parts.includes('..')) {
    diagnostics.push(authoringDiag('disallowed_path', 'relativePath', `path "${value}" must be non-empty, relative, and remain inside the game workspace`));
    return null;
  }
  const normalized: string[] = [];
  for (const part of parts) {
    if (part.length === 0 || part === '.') continue;
    normalized.push(part);
  }
  if (normalized.length === 0) {
    diagnostics.push(authoringDiag('disallowed_path', 'relativePath', `path "${value}" must name a file`));
    return null;
  }
  return normalized.join('/');
}

function isGeneratedOrPrivateAuthoringPath(value: string): boolean {
  return value === '.git'
    || value.startsWith('.git/')
    || value === 'harness/out'
    || value.startsWith('harness/out/')
    || value === 'node_modules'
    || value.startsWith('node_modules/')
    || value.startsWith('../asha-engine')
    || value.startsWith('../asha-studio');
}

function validateAuthoringExtension(
  scope: AshaAuthoringWriteScope,
  normalizedPath: string,
  diagnostics: AshaAuthoringDiagnostic[],
): void {
  if (scope.operationKind === 'authoring.scene.save_source' && !normalizedPath.endsWith('.scene.json')) {
    diagnostics.push(authoringDiag('invalid_extension', 'relativePath', 'scene authoring saves must target *.scene.json'));
  }
  if (scope.operationKind === 'authoring.catalog.save_source') {
    const catalogPathAllowed = scope.allowedRoots.some((root) => normalizedPath === `${root}/catalog.json`);
    if (!catalogPathAllowed) {
      diagnostics.push(authoringDiag('invalid_extension', 'relativePath', 'catalog authoring saves must target catalog.json in a catalog package root'));
    }
  }
  if (
    scope.operationKind === 'authoring.asset.save_source'
    && !(
      normalizedPath.endsWith('.mesh.json')
      || normalizedPath.endsWith('.material.json')
      || normalizedPath.endsWith('.texture.json')
    )
  ) {
    diagnostics.push(authoringDiag('invalid_extension', 'relativePath', 'asset authoring saves must target *.mesh.json, *.material.json, or *.texture.json'));
  }
}

function validateBackendMode(manifest: AshaGameManifest, diagnostics: AshaGameManifestDiagnostic[]): void {
  const { backendMode, backendProfile, backendProofRefs, wasmOrNativeEntry } = manifest.runtime;
  if (containsPrivateTransportHint(wasmOrNativeEntry)) {
    diagnostics.push(diag('private_transport_hint', 'runtime.wasm_or_native_entry', 'runtime entry must point at a public launcher/facade entry, not a raw private transport'));
  }
  if (containsPrivateTransportHint(backendProfile)) {
    diagnostics.push(diag('private_transport_hint', 'runtime.backend_profile', 'backend profile must not name private transports or ASHA internals'));
  }
  if (backendMode === 'reference') {
    if (backendProfile !== 'reference') {
      diagnostics.push(diag('unsupported_backend_mode', 'runtime.backend_profile', 'reference backend mode must use backend_profile = "reference"'));
    }
    return;
  }
  if (backendMode === 'native') {
    if (backendProfile.length === 0 || backendProfile === 'reference') {
      diagnostics.push(diag('missing_backend_ref', 'runtime.backend_profile', 'native backend mode requires a selected backend profile id'));
    }
    if (backendProofRefs.length === 0) {
      diagnostics.push(diag('missing_backend_ref', 'runtime.backend_proof_refs', 'native backend mode requires at least one public proof/evidence ref'));
    }
    return;
  }
  diagnostics.push(diag('unsupported_backend_mode', 'runtime.backend_mode', 'wasm backend mode is declared but deferred until a public WASM runtime facade is approved'));
}

function isSameOrChildPath(candidate: string, root: string): boolean {
  return candidate === root || candidate.startsWith(`${root}/`);
}

function isSupportedAssetKind(kind: string): kind is AshaGameAssetKind {
  return kind === 'static_mesh' || kind === 'material' || kind === 'texture' || kind === 'scene';
}

function validateKindSpecificAssetEntry(
  entry: AshaGameAssetCatalogEntry,
  path: string,
  diagnostics: AshaGameAssetCatalogDiagnostic[],
): void {
  const expected = {
    static_mesh: { importProfile: 'inline-static-mesh.v0', outputPrefix: 'meshes/', outputSuffix: '.mesh.json' },
    material: { importProfile: 'inline-material.v0', outputPrefix: 'materials/', outputSuffix: '.material.json' },
    texture: { importProfile: 'inline-texture.v0', outputPrefix: 'textures/', outputSuffix: '.texture.json' },
    scene: { importProfile: 'flat-scene.v0', outputPrefix: 'scenes/', outputSuffix: '.scene.json' },
  }[entry.kind];

  if (entry.importProfile !== expected.importProfile) {
    diagnostics.push(assetDiag('invalid_asset_entry', `${path}.importProfile`, `${entry.kind} assets require importProfile "${expected.importProfile}"`));
  }
  if (!entry.publish.outputKey.startsWith(expected.outputPrefix) || !entry.publish.outputKey.endsWith(expected.outputSuffix)) {
    diagnostics.push(assetDiag('invalid_asset_entry', `${path}.publish.outputKey`, `${entry.kind} publish output must match ${expected.outputPrefix}*${expected.outputSuffix}`));
  }
}

function validateImportMetadata(
  entry: AshaGameAssetCatalogEntry,
  path: string,
  options: AshaGameAssetCatalogValidationOptions,
  diagnostics: AshaGameAssetCatalogDiagnostic[],
): void {
  const metadata = entry.importMetadata;
  if (metadata === undefined) return;
  if (metadata.sourceHash.length === 0 || metadata.cacheKey.length === 0 || metadata.generatedArtifactVersion.length === 0) {
    diagnostics.push(assetDiag('invalid_asset_entry', `${path}.importMetadata`, 'sourceHash, cacheKey, and generatedArtifactVersion are required when import metadata is present'));
    return;
  }
  if (options.sourceHash !== undefined) {
    const observed = options.sourceHash(entry.source);
    if (observed !== null && observed !== metadata.sourceHash) {
      diagnostics.push(assetDiag('stale_import_metadata', `${path}.importMetadata.sourceHash`, `asset "${entry.id}" import metadata hash is stale`));
    }
  }
}

function getString(document: TomlDocument, section: string, key: string, diagnostics: AshaGameManifestDiagnostic[]): string {
  const value = document[section]?.[key];
  if (typeof value !== 'string') {
    diagnostics.push(diag('missing_required_field', `${section}.${key}`, 'expected a string field'));
    return '';
  }
  return value;
}

function getBoolean(document: TomlDocument, section: string, key: string, diagnostics: AshaGameManifestDiagnostic[]): boolean {
  const value = document[section]?.[key];
  if (typeof value !== 'boolean') {
    diagnostics.push(diag('missing_required_field', `${section}.${key}`, 'expected a boolean field'));
    return false;
  }
  return value;
}

function getStringArray(document: TomlDocument, section: string, key: string, diagnostics: AshaGameManifestDiagnostic[]): readonly string[] {
  const value = document[section]?.[key];
  if (!Array.isArray(value) || !value.every((entry) => typeof entry === 'string')) {
    diagnostics.push(diag('missing_required_field', `${section}.${key}`, 'expected a string array field'));
    return [];
  }
  return value;
}

function getBackendMode(document: TomlDocument, diagnostics: AshaGameManifestDiagnostic[]): AshaGameRuntimeBackendMode {
  const value = document['runtime']?.['backend_mode'];
  if (value === 'reference' || value === 'native' || value === 'wasm') {
    return value;
  }
  diagnostics.push(diag('unsupported_backend_mode', 'runtime.backend_mode', 'backend_mode must be one of reference, native, or wasm'));
  return 'reference';
}

function diag(code: AshaGameManifestDiagnosticCode, path: string, message: string): AshaGameManifestDiagnostic {
  return { code, path, message };
}

function compatDiag(code: AshaConsumerCompatibilityDiagnosticCode, path: string, message: string): AshaConsumerCompatibilityDiagnostic {
  return { code, path, message };
}

function authoringDiag(code: AshaAuthoringDiagnosticCode, path: string, message: string): AshaAuthoringDiagnostic {
  return { code, path, message };
}

function assetDiag(code: AshaGameAssetCatalogDiagnosticCode, path: string, message: string): AshaGameAssetCatalogDiagnostic {
  return { code, path, message };
}
