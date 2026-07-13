// Import/typecheck smoke for @asha/contracts.
//
// This is the proof for the Phase 2 exit criterion "a TypeScript package can
// import generated branded IDs and command unions" (see
// governance/protocol-border-consumers.md). It is NOT part of the public API
// (index.ts does not re-export it). Its only job is to fail `tsc` if the
// generated contracts stop being importable or usable — proving that branded
// IDs and the command/view/diff/replay unions compile when consumed exactly as
// a downstream package would consume them, with no policy, renderer, UI,
// bridge, Electron, or browser globals in scope.
//
// It is value-level on purpose: constructing real union values exercises the
// discriminants and field shapes, not just the type names.

import {
  entityId,
  modeId,
  tagId,
  renderHandle,
  stepIndex,
  replayHash,
  REPLAY_FORMAT_VERSION,
  type EntityId,
  type Command,
  type CommandEnvelope,
  type ScriptView,
  type ScriptOutcome,
  type RenderDiff,
  type ReplayRecord,
  type DiagnosticReport,
  type DiagnosticReportSet,
  type SourceTrace,
  type RendererResourceReport,
  projectId,
  sceneId,
  runtimeSessionId,
  sceneNodeId,
  type FlatSceneDocument,
  type SceneValidationReport,
  type BootstrapRecord,
  type ProjectBundleManifest,
  type LoadPlan,
  type RegenConflictReport,
  type CatalogValidationReport,
  type LockValidationReport,
  type RenderMaterial,
  type FallbackDecision,
  cameraHandle,
  type CameraBasis,
  type CameraCreateRequest,
  type CameraHandle,
  type CameraPose,
  type CameraProjectionRequest,
  type CameraProjectionSnapshot,
  type CameraSnapshot,
  type FirstPersonCameraInput,
  type FirstPersonCameraInputEnvelope,
  type PerspectiveProjection,
  type ViewportSize,
} from './index.js';

// Branded IDs are nominally typed and built through their constructors.
const entity: EntityId = entityId(1);

// A command authored the way a policy would author it.
const addTag: Command = {
  domain: 'entity',
  command: { kind: 'addTag', id: entity, tag: tagId(2) },
};

const envelope: CommandEnvelope = { kind: 'policy', command: addTag };

// A read-only view value.
const view: ScriptView = {
  entities: [{ id: entity, tags: [tagId(2)] }],
  subjects: [],
  processes: [],
  modes: [modeId(3)],
  signals: [],
  tags: [tagId(2)],
};

const outcome: ScriptOutcome = { status: 'accepted' };

// A retained-mode render diff value: create an abstract cube node, then destroy.
const createDiff: RenderDiff = {
  op: 'create',
  handle: renderHandle(5),
  parent: null,
  node: {
    geometry: { shape: 'cube' },
    material: { color: [1, 1, 1, 1], wireframe: false },
    transform: {
      translation: [0, 0, 0],
      rotation: [0, 0, 0, 1],
      scale: [1, 1, 1],
    },
    visible: true,
    layer: 'scene',
    metadata: { source: entity, tags: [tagId(2)], label: 'cube' },
  },
};
const diff: RenderDiff = { op: 'destroy', handle: renderHandle(5) };

// A replay record value, with the format version sourced from the contract.
const record: ReplayRecord = {
  formatVersion: REPLAY_FORMAT_VERSION,
  initialHash: replayHash(0),
  steps: [
    {
      index: stepIndex(0),
      command: envelope,
      outcome: { status: 'accepted', events: [{ event: 'entityCreated', id: entity }] },
      postHash: replayHash(1),
    },
  ],
  snapshots: [],
};

// A diagnostic report value, authored the way a devtools panel would consume
// one: a broken source trace pointing at a missing sprite texture, plus a
// fatal corrupt-bundle report. Proves the generated diagnostic contracts are
// importable and usable (scene-capability-06, #2330).
const missingAsset: DiagnosticReport = {
  scope: 'scene',
  severity: 'error',
  code: 'sceneAssetMissing',
  reference: 'person-spawn-03',
  source: {
    sceneNodeId: 3,
    runtimeEntityId: 456,
    assetId: 'sprite/hard-hat',
    chunkCoord: null,
    renderHandle: 43,
    bundlePath: null,
  },
  message: 'scene node references a sprite the catalog does not contain',
  remedy: { action: 'provideAsset', detail: 'add sprite/hard-hat to the catalog' },
};

const corruptArtifact: DiagnosticReport = {
  scope: 'projectBundle',
  severity: 'fatal',
  code: 'corruptBundleArtifact',
  reference: 'chunks/0_0_0.snap',
  source: {
    sceneNodeId: null,
    runtimeEntityId: null,
    assetId: null,
    chunkCoord: [0, 0, 0],
    renderHandle: null,
    bundlePath: 'chunks/0_0_0.snap',
  },
  message: 'durable artifact failed its content hash',
  remedy: { action: 'restoreArtifact', detail: 'restore from a known-good bundle copy' },
};

// A world-composition round-trip mismatch (#2368): consolidated so load/save
// execution and save→reload equivalence have stable generated codes too.
const roundTripMismatch: DiagnosticReport = {
  scope: 'worldComposition',
  severity: 'error',
  code: 'roundTripMismatch',
  reference: 'round-trip',
  source: {
    sceneNodeId: null,
    runtimeEntityId: null,
    assetId: null,
    chunkCoord: null,
    renderHandle: null,
    bundlePath: null,
  },
  message: 'save/load round-trip lost state: pre-save hash != reloaded hash',
  remedy: { action: 'inspect', detail: 'the save/compaction path is not equivalence-preserving' },
};

// A load-stage failure (#2368): a stage of an executed load plan failed.
const loadStageFailed: DiagnosticReport = {
  scope: 'worldComposition',
  severity: 'fatal',
  code: 'loadStageFailed',
  reference: 'load:bootstrap',
  source: {
    sceneNodeId: null,
    runtimeEntityId: null,
    assetId: null,
    chunkCoord: null,
    renderHandle: null,
    bundlePath: 'scene/scene.json',
  },
  message: 'load stage `bootstrap` failed during composition',
  remedy: { action: 'inspect', detail: 'inspect the failing load stage input' },
};

// A consolidated report set spanning every diagnostic scope — what a single
// devtools diagnostics panel would render across scene/asset/bundle/render/
// resources/composition without any `any` or raw-JSON fallback.
const reportSet: DiagnosticReportSet = {
  reports: [missingAsset, corruptArtifact, roundTripMismatch, loadStageFailed],
};

const trace: SourceTrace = {
  renderHandle: 43,
  sceneNodeId: 3,
  runtimeEntityId: 456,
  assetId: 'sprite/hard-hat',
  assetResolved: false,
};

const resources: RendererResourceReport = {
  liveHandles: 2,
  geometries: 1,
  materials: 1,
  spriteInstances: 1,
  spritesUpdatedLastTick: 1,
  resourcesCreated: 4,
  resourcesDisposed: 4,
  fallbackMaterials: 0,
};

// A typed scene authored the way a TS authoring tool would — the same logical
// document as harness/fixtures/scenes/sample-flat.json, expressed against the
// generated contract. This is the "TS can author/load a typed scene fixture"
// proof for #2365: TS *expresses* the scene; Rust *validates* it. Note the
// discriminated SceneNodeKind — an empty group carries no asset key, an
// asset-backed node does, exactly matching the wire.
const sampleScene: FlatSceneDocument = {
  schemaVersion: 1,
  id: sceneId(100),
  metadata: { name: 'sample', authoringFormatVersion: 1 },
  dependencies: [
    { id: 'mesh/static-mesh-fixture-a', version: { req: 'any' }, hash: null },
  ],
  nodes: [
    {
      id: sceneNodeId(1),
      parent: null,
      childOrder: 0,
      label: null,
      tags: [],
      transform: { translation: [0, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
      kind: { kind: 'emptyGroup' },
    },
    {
      id: sceneNodeId(2),
      parent: sceneNodeId(1),
      childOrder: 0,
      label: 'mesh-a',
      tags: ['a-tag', 'b-tag'],
      transform: { translation: [1, 0, 0], rotation: [0, 0, 0, 1], scale: [1, 1, 1] },
      kind: {
        kind: 'staticMesh',
        asset: { id: 'mesh/static-mesh-fixture-a', version: { req: 'any' }, hash: null },
      },
    },
  ],
};

// TS can *express* a semantically invalid scene (a cycle) — it has no authority
// to reject it. Only Rust validation produces this classified report; here we
// merely prove the report shape is importable/typed for a devtools panel.
const cycleReport: SceneValidationReport = {
  errors: [
    {
      code: 'cycle',
      node: null,
      parent: null,
      expectedKind: null,
      actualKind: null,
      transformReason: null,
      cyclePath: [sceneNodeId(1), sceneNodeId(2), sceneNodeId(1)],
    },
  ],
};

// The atomic bootstrap record a scene→authority init produces, read-side typed.
const bootstrap: BootstrapRecord = {
  sceneId: sceneId(100),
  runtimeSessionId: runtimeSessionId(7),
  schemaVersion: 1,
  nodeCount: 2,
  entityCount: 2,
  spatialSessionHash: 0,
  sourceTrace: [
    { sceneNodeId: sceneNodeId(1), runtimeEntityId: entity },
    { sceneNodeId: sceneNodeId(2), runtimeEntityId: entityId(2) },
  ],
};

// A ProjectBundle manifest a devtools panel would *display* from a Rust-produced
// fixture.
// This is the "TS can display a manifest from a Rust-produced fixture" proof for
// #2366: read-only, no authority to load or mutate the bundle.
const manifest: ProjectBundleManifest = {
  bundleSchemaVersion: 1,
  protocolVersion: 1,
  project: { id: projectId(7), name: 'sample-project' },
  scene: { id: sceneId(100), schemaVersion: 1, artifact: 'scene/scene.json' },
  assetLock: { artifact: 'assets/lock.json', assetCount: 1 },
  generator: { seed: 42, version: 1, params: 'default' },
  artifacts: [
    { path: 'assets/lock.json', class: 'durable', role: 'assetLock', contentHash: '422f72d827e3137c' },
    { path: 'cache/mesh_0_0_0.bin', class: 'cache', role: 'cache', contentHash: null },
    { path: 'scene/scene.json', class: 'durable', role: 'sceneDocument', contentHash: '1723540f7db7a459' },
  ],
};

// An ordered load plan, displayed the way a devtools timeline would render it.
const loadPlan: LoadPlan = {
  steps: [
    { step: 'validateVersions', bundleSchemaVersion: 1, protocolVersion: 1 },
    { step: 'loadAssetLock', artifact: 'assets/lock.json', assetCount: 1 },
    { step: 'loadSceneDocument', artifact: 'scene/scene.json', scene: sceneId(100) },
    {
      step: 'applyVoxelEdits',
      editLogs: ['voxel/edits.log'],
      snapshots: ['voxel/chunk_0_0_0.snapshot'],
      histories: [],
    },
    { step: 'bootstrapScene', scene: sceneId(100), runtimeSession: runtimeSessionId(7) },
    { step: 'validateFinalState' },
  ],
};

// A regenerate-and-replay generator diagnostic, with the stable conflict fields
// (coord, event id, old/new generated material, edit value, suggested action).
const regenReport: RegenConflictReport = {
  savedVersion: 1,
  newVersion: 2,
  conflicts: [
    {
      eventId: 3,
      coord: { x: 4, y: 0, z: 2 },
      oldGenerated: { kind: 'empty' },
      newGenerated: { kind: 'solid', material: 7 },
      editValue: { kind: 'solid', material: 9 },
      suggested: 'reviewConflict',
    },
  ],
  replayedEdits: 5,
  stagingSessionHash: 0,
};

// Catalog validation + asset-lock drift, displayed read-only by a devtools
// inspector — the #2367 proof that TS can render diagnostics without `any`/raw
// JSON, and that no TS package becomes the catalog validator (these are produced
// by Rust). Covers missing, wrong-kind, stale, dependency drift, and cycle path.
const catalogReport: CatalogValidationReport = {
  errors: [
    {
      code: 'wrong-kind-reference',
      id: null,
      kind: null,
      from: 'material:env/brick',
      slot: 'texture',
      expected: 'texture',
      actual: 'mesh',
      reference: 'mesh/oops',
      dependency: null,
      cyclePath: [],
    },
    {
      code: 'dependency-cycle',
      id: null,
      kind: null,
      from: null,
      slot: null,
      expected: null,
      actual: null,
      reference: null,
      dependency: null,
      cyclePath: ['material:a', 'material:b', 'material:a'],
    },
  ],
};

const lockReport: LockValidationReport = {
  findings: [
    {
      id: 'mesh/static-mesh-fixture-a',
      code: 'stale-version',
      lockedKind: null,
      currentKind: null,
      lockedVersion: 1,
      currentVersion: 2,
      lockedHash: null,
      currentHash: null,
      addedDependencies: [],
      removedDependencies: [],
    },
    {
      id: 'texture/old',
      code: 'missing',
      lockedKind: 'texture',
      currentKind: null,
      lockedVersion: null,
      currentVersion: null,
      lockedHash: null,
      currentHash: null,
      addedDependencies: [],
      removedDependencies: [],
    },
  ],
};

// The renderer-facing material projection a renderer would consume: it cannot
// even *name* a collision field (that's a separate CollisionMaterial type).
const renderMaterial: RenderMaterial = {
  color: { r: 0.5, g: 0.5, b: 0.5, a: 1 },
  texture: { id: 'texture/brick', version: { req: 'atLeast', value: 2 }, hash: null },
  roughness: 1,
  textureTint: { r: 1, g: 1, b: 1, a: 1 },
  emissionColor: { r: 0.5, g: 0.5, b: 0.5, a: 1 },
  emissive: 0,
  uvStrategy: 'planar',
};

// A fallback decision for a missing collision-critical asset: fail closed.
const fallback: FallbackDecision = {
  outcome: 'failClosed',
  reason: 'collision-critical asset missing; refusing to load incomplete authority',
};

// A deterministic camera/view surface value set for first-person mover evidence.
// These contracts are view/projection infrastructure, not gameplay authority or
// renderer object handles.
const camera: CameraHandle = cameraHandle(11);
const cameraPose: CameraPose = {
  position: [0, 1.6, 0],
  yawDegrees: 0,
  pitchDegrees: 0,
};
const cameraBasis: CameraBasis = {
  forward: [0, 0, -1],
  right: [1, 0, 0],
  up: [0, 1, 0],
};
const projection: PerspectiveProjection = {
  fovYDegrees: 60,
  near: 0.1,
  far: 1000,
};
const viewport: ViewportSize = { width: 1280, height: 720 };
const cameraCreate: CameraCreateRequest = {
  initialPose: cameraPose,
  projection,
  viewport,
};
const firstPersonInput: FirstPersonCameraInput = {
  moveForward: 1,
  moveRight: 0,
  moveUp: 0,
  yawDeltaDegrees: 15,
  pitchDeltaDegrees: -5,
  dtSeconds: 1 / 60,
  moveSpeedUnitsPerSecond: 3,
};
const firstPersonEnvelope: FirstPersonCameraInputEnvelope = {
  camera,
  input: firstPersonInput,
  tick: 1,
};
const cameraSnapshot: CameraSnapshot = {
  camera,
  tick: 1,
  pose: cameraPose,
  basis: cameraBasis,
  projection,
  viewport,
};
const cameraProjectionRequest: CameraProjectionRequest = { camera, viewport: null };
const cameraProjection: CameraProjectionSnapshot = {
  ...cameraSnapshot,
  viewMatrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, -1.6, 0, 1],
  projectionMatrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, -1, -1, 0, 0, -0.2, 0],
  viewProjectionMatrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 1.6, -1, -1, 0, 0, -0.2, 0],
  projectionHash: 'sha256:camera-smoke',
};

// Exported so the values are "used" (lint-clean) and tree-shakeable. Consumers
// of @asha/contracts never see this — it is not re-exported by index.ts.
export const __contractSmoke = {
  entity,
  addTag,
  envelope,
  view,
  outcome,
  createDiff,
  diff,
  record,
  reportSet,
  trace,
  resources,
  sampleScene,
  cycleReport,
  bootstrap,
  manifest,
  loadPlan,
  regenReport,
  catalogReport,
  lockReport,
  renderMaterial,
  fallback,
  camera,
  cameraPose,
  cameraBasis,
  projection,
  viewport,
  cameraCreate,
  firstPersonInput,
  firstPersonEnvelope,
  cameraSnapshot,
  cameraProjectionRequest,
  cameraProjection,
} as const;
