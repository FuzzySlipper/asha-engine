import type {
  ProjectResourceBeginRequest,
  ProjectResourceTransactionReceipt,
  ProjectSourceBatchValidationReceipt,
  RuntimeProjectCloseReceipt,
  RuntimeProjectCloseRequest,
  RuntimeProjectLoadReceipt,
  RuntimeProjectLoadRequest,
  RuntimeProjectSourceBatch,
  StagedProjectResourceRef,
} from '@asha/contracts';

import {
  RuntimeBridgeError,
  nonNegativeSafeInteger,
  type ProjectResourceStageInput,
} from './bridge.js';

/** Project-source and lifecycle behavior for the transport mock. This mirrors
 * contract semantics only and never claims Rust admission authority. */
export class MockRuntimeProjectLifecycle {
  #resourceGeneration = 0;
  #nextResourceHandle = 0;
  #resources = new Map<
    number,
    { readonly generation: number; readonly path: string; readonly bytes: Uint8Array }
  >();
  #pendingManifestHash: string | null = null;
  #lifecycle = { generation: 0, revision: 0 };
  #activeProject: RuntimeProjectLoadReceipt['activeProject'] = null;

  reset(): void {
    this.#resourceGeneration = 0;
    this.#nextResourceHandle = 0;
    this.#resources.clear();
    this.#pendingManifestHash = null;
    this.#lifecycle = { generation: 0, revision: 0 };
    this.#activeProject = null;
  }

  begin(request: ProjectResourceBeginRequest): ProjectResourceTransactionReceipt {
    if (request.manifestJson.length === 0) {
      throw new RuntimeBridgeError('invalid_input', 'project source manifest is empty');
    }
    this.#resourceGeneration += 1;
    return {
      generation: this.#resourceGeneration,
      manifestHash: `mock-project-source:${request.manifestJson.length}`,
    };
  }

  stage(request: ProjectResourceStageInput): StagedProjectResourceRef {
    const generation = nonNegativeSafeInteger(request.generation, 'project resource generation');
    if (generation !== this.#resourceGeneration) {
      throw new RuntimeBridgeError('invalid_input', 'project resource generation is not active');
    }
    const handle = this.#nextResourceHandle;
    this.#nextResourceHandle += 1;
    const bytes = request.bytes.slice();
    this.#resources.set(handle, { generation, path: request.path, bytes });
    return { handle, generation, version: 1, byteLen: bytes.byteLength };
  }

  admit(request: RuntimeProjectSourceBatch): ProjectSourceBatchValidationReceipt {
    const paths = request.bodies.map((body) => body.path);
    const resourceBodies = request.bodies.filter((body) => body.kind === 'resource');
    for (const body of resourceBodies) {
      const staged = this.#resources.get(body.resource.handle);
      if (
        staged === undefined
        || staged.generation !== body.resource.generation
        || staged.path !== body.path
      ) {
        return {
          accepted: false,
          manifestHash: null,
          paths: [],
          diagnostics: [{
            code: 'unknownResourceHandle',
            path: body.path,
            message: 'mock transport has no matching staged project resource',
          }],
        };
      }
    }
    for (const body of resourceBodies) this.#resources.delete(body.resource.handle);
    this.#pendingManifestHash = `mock-project-source:${request.manifestJson.length}`;
    return {
      accepted: true,
      manifestHash: this.#pendingManifestHash,
      paths,
      diagnostics: [],
    };
  }

  load(request: RuntimeProjectLoadRequest): RuntimeProjectLoadReceipt {
    if (
      request.expectedLifecycle.generation !== this.#lifecycle.generation
      || request.expectedLifecycle.revision !== this.#lifecycle.revision
    ) {
      return this.#rejectedLoad(request, 'staleLifecycle', 'mock runtime project lifecycle is stale');
    }
    if (this.#activeProject !== null || this.#pendingManifestHash === null) {
      return this.#rejectedLoad(
        request,
        this.#activeProject === null ? 'missingAdmittedSource' : 'alreadyActive',
        'mock runtime project cannot activate the pending source',
      );
    }
    this.#lifecycle = {
      generation: this.#lifecycle.generation + 1,
      revision: this.#lifecycle.revision + 1,
    };
    this.#activeProject = {
      projectId: 1,
      manifestHash: this.#pendingManifestHash,
      admissionHash: `mock-admission:${request.source.materializationHash}`,
      contentSetHash: 'mock-content-set',
      compositionHash: 'mock-composition',
      entrySceneId: 1,
      sceneCount: 1,
      entityCount: 1,
      voxelAssetCount: 0,
      lifecycle: this.#lifecycle,
    };
    this.#pendingManifestHash = null;
    return {
      accepted: true,
      source: request.source,
      activeProject: this.#activeProject,
      lifecycle: this.#lifecycle,
      diagnostics: [],
    };
  }

  close(request: RuntimeProjectCloseRequest): RuntimeProjectCloseReceipt {
    const active = this.#activeProject;
    if (
      active === null
      || request.expectedLifecycle.generation !== this.#lifecycle.generation
      || request.expectedLifecycle.revision !== this.#lifecycle.revision
    ) {
      return {
        accepted: false,
        closedProjectId: null,
        closedManifestHash: null,
        lifecycle: this.#lifecycle,
        diagnostics: [{
          phase: 'lifecycle',
          code: active === null ? 'noActiveProject' : 'staleLifecycle',
          documentId: null,
          path: null,
          message: 'mock runtime project close rejected',
        }],
      };
    }
    this.#lifecycle = {
      generation: this.#lifecycle.generation,
      revision: this.#lifecycle.revision + 1,
    };
    this.#activeProject = null;
    return {
      accepted: true,
      closedProjectId: active.projectId,
      closedManifestHash: active.manifestHash,
      lifecycle: this.#lifecycle,
      diagnostics: [],
    };
  }

  clearResources(): void {
    this.#resources.clear();
  }

  #rejectedLoad(
    request: RuntimeProjectLoadRequest,
    code: string,
    message: string,
  ): RuntimeProjectLoadReceipt {
    return {
      accepted: false,
      source: request.source,
      activeProject: null,
      lifecycle: this.#lifecycle,
      diagnostics: [{
        phase: 'lifecycle',
        code,
        documentId: null,
        path: null,
        message,
      }],
    };
  }
}
