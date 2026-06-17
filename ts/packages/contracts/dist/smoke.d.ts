import { type EntityId, type CommandEnvelope, type ScriptView, type ReplayRecord, type DiagnosticReportSet, type SourceTrace, type RendererResourceReport, type FlatSceneDocument, type SceneValidationReport, type BootstrapRecord, type WorldBundleManifest, type LoadPlan, type RegenConflictReport, type CatalogValidationReport, type LockValidationReport, type RenderMaterial, type CameraBasis, type CameraCreateRequest, type CameraHandle, type CameraPose, type CameraProjectionRequest, type CameraProjectionSnapshot, type CameraSnapshot, type FirstPersonCameraInput, type FirstPersonCameraInputEnvelope, type PerspectiveProjection, type ViewportSize } from './index.js';
export declare const __contractSmoke: {
    readonly entity: EntityId;
    readonly addTag: {
        readonly domain: "entity";
        readonly command: import("./index.js").EntityCommand;
    };
    readonly envelope: CommandEnvelope;
    readonly view: ScriptView;
    readonly outcome: {
        readonly status: "accepted";
    };
    readonly createDiff: {
        readonly op: "create";
        readonly handle: import("./index.js").RenderHandle;
        readonly parent: import("./index.js").RenderHandle | null;
        readonly node: import("./index.js").RenderNode;
    };
    readonly diff: {
        readonly op: "destroy";
        readonly handle: import("./index.js").RenderHandle;
    };
    readonly record: ReplayRecord;
    readonly reportSet: DiagnosticReportSet;
    readonly trace: SourceTrace;
    readonly resources: RendererResourceReport;
    readonly sampleScene: FlatSceneDocument;
    readonly cycleReport: SceneValidationReport;
    readonly bootstrap: BootstrapRecord;
    readonly manifest: WorldBundleManifest;
    readonly loadPlan: LoadPlan;
    readonly regenReport: RegenConflictReport;
    readonly catalogReport: CatalogValidationReport;
    readonly lockReport: LockValidationReport;
    readonly renderMaterial: RenderMaterial;
    readonly fallback: {
        readonly outcome: "failClosed";
        readonly reason: string;
    };
    readonly camera: CameraHandle;
    readonly cameraPose: CameraPose;
    readonly cameraBasis: CameraBasis;
    readonly projection: PerspectiveProjection;
    readonly viewport: ViewportSize;
    readonly cameraCreate: CameraCreateRequest;
    readonly firstPersonInput: FirstPersonCameraInput;
    readonly firstPersonEnvelope: FirstPersonCameraInputEnvelope;
    readonly cameraSnapshot: CameraSnapshot;
    readonly cameraProjectionRequest: CameraProjectionRequest;
    readonly cameraProjection: CameraProjectionSnapshot;
};
//# sourceMappingURL=smoke.d.ts.map