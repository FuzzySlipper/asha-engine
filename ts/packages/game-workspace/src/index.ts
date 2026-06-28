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

export type AshaGameManifestDiagnosticCode =
  | 'toml_parse_error'
  | 'missing_required_field'
  | 'missing_root'
  | 'bad_version'
  | 'unsupported_endpoint'
  | 'invalid_write_scope'
  | 'invalid_path';

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

type TomlScalar = string | boolean | readonly string[];
type TomlSection = Record<string, TomlScalar>;
type TomlDocument = Record<string, TomlSection>;

const REQUIRED_SECTIONS = ['asha', 'workspace', 'runtime', 'studio', 'publish'] as const;
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
    }
    if (!manifest.workspace.assetRoots.some((root) => isSameOrChildPath(entry.source, root))) {
      diagnostics.push(assetDiag('forbidden_asset_path', `${path}.source`, `asset source "${entry.source}" is outside manifest asset roots`));
    } else if (!fileExists(entry.source)) {
      diagnostics.push(assetDiag('missing_asset_file', `${path}.source`, `asset source does not exist: ${entry.source}`));
    }
  }

  return diagnostics.length === 0 ? { ok: true, catalog, diagnostics: [] } : { ok: false, diagnostics };
}

export function resolveAshaGameAssetForDev(
  catalog: AshaGameAssetCatalog,
  assetId: string,
): AshaGameAssetDevResolution | null {
  const entry = catalog.entries.find((candidate) => candidate.id === assetId);
  if (entry === undefined) {
    return null;
  }
  return {
    assetId: entry.id,
    sourcePath: entry.source,
    devCacheKey: `dev-cache/${entry.kind}/${entry.id}`,
    publishOutputKey: entry.publish.outputKey,
  };
}

export function buildAshaGamePublishAssetManifest(catalog: AshaGameAssetCatalog): AshaGamePublishAssetManifest {
  return {
    schemaVersion: 1,
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
  validatePath(manifest.publish.artifactDir, 'publish.artifact_dir', diagnostics);

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

function isSameOrChildPath(candidate: string, root: string): boolean {
  return candidate === root || candidate.startsWith(`${root}/`);
}

function isSupportedAssetKind(kind: string): kind is AshaGameAssetKind {
  return kind === 'static_mesh' || kind === 'material' || kind === 'texture' || kind === 'scene';
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

function diag(code: AshaGameManifestDiagnosticCode, path: string, message: string): AshaGameManifestDiagnostic {
  return { code, path, message };
}

function compatDiag(code: AshaConsumerCompatibilityDiagnosticCode, path: string, message: string): AshaConsumerCompatibilityDiagnostic {
  return { code, path, message };
}

function assetDiag(code: AshaGameAssetCatalogDiagnosticCode, path: string, message: string): AshaGameAssetCatalogDiagnostic {
  return { code, path, message };
}
