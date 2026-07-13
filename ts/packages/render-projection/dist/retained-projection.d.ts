import type { AnimatedMeshAsset, AnimatedMeshInstanceDescriptor, AnimatedMeshPlaybackCommand, Material, MaterialInstanceParameters, MeshPayloadDescriptor, MeshPickHit, RenderDiff, RenderFrameDiff, RenderHandle, RenderLayer, RenderMaterialDescriptor, RenderMetadata, RenderNode, SpriteAtlasDescriptor, SpriteInstanceDescriptor, SpritePickHit, StaticMeshAsset, StaticMeshInstanceDescriptor, TextureDescriptor, Transform } from '@asha/contracts';
/** Raised when a render diff cannot be applied to the retained projection. */
export declare class RenderProjectionError extends Error {
    constructor(message: string);
}
export type RenderProjectionNodeKind = 'primitive' | 'staticMesh' | 'animatedMesh' | 'sprite';
export interface RenderProjectionNodeBase {
    readonly handle: RenderHandle;
    readonly parent: RenderHandle | null;
    readonly children: readonly RenderHandle[];
    readonly kind: RenderProjectionNodeKind;
    readonly layer: RenderLayer;
    readonly transform: Transform;
    readonly visible: boolean;
    readonly metadata: RenderMetadata;
    readonly material: Material | null;
    readonly meshPayload: MeshPayloadDescriptor | null;
}
export interface PrimitiveProjectionNode extends RenderProjectionNodeBase {
    readonly kind: 'primitive';
    readonly node: RenderNode;
}
export interface StaticMeshProjectionNode extends RenderProjectionNodeBase {
    readonly kind: 'staticMesh';
    readonly asset: string;
    readonly instance: StaticMeshInstanceDescriptor;
    readonly materialParameters: readonly MaterialInstanceParameterBinding[];
}
export interface MaterialInstanceParameterBinding {
    readonly slot: number;
    readonly parameters: MaterialInstanceParameters;
}
export interface AnimatedMeshProjectionNode extends RenderProjectionNodeBase {
    readonly kind: 'animatedMesh';
    readonly asset: string;
    readonly instance: AnimatedMeshInstanceDescriptor;
    readonly playback: AnimatedMeshPlaybackCommand | null;
}
export interface SpriteProjectionNode extends RenderProjectionNodeBase {
    readonly kind: 'sprite';
    readonly sprite: SpriteInstanceDescriptor;
    readonly frameUv: readonly [number, number, number, number];
    readonly renderOrder: number;
}
export type RenderProjectionNode = PrimitiveProjectionNode | StaticMeshProjectionNode | AnimatedMeshProjectionNode | SpriteProjectionNode;
export type RenderProjectionInstruction = {
    readonly op: 'defineMaterial';
    readonly material: RenderMaterialDescriptor;
} | {
    readonly op: 'defineTexture';
    readonly texture: TextureDescriptor;
} | {
    readonly op: 'defineSpriteAtlas';
    readonly atlas: SpriteAtlasDescriptor;
} | {
    readonly op: 'defineStaticMesh';
    readonly asset: StaticMeshAsset;
} | {
    readonly op: 'defineAnimatedMesh';
    readonly asset: AnimatedMeshAsset;
} | {
    readonly op: 'upsertNode';
    readonly node: RenderProjectionNode;
} | {
    readonly op: 'removeNode';
    readonly handle: RenderHandle;
};
export interface RenderProjectionSnapshot {
    readonly nodes: readonly RenderProjectionNode[];
    readonly materials: readonly RenderMaterialDescriptor[];
    readonly textures: readonly TextureDescriptor[];
    readonly spriteAtlases: readonly SpriteAtlasDescriptor[];
    readonly staticMeshes: readonly StaticMeshAsset[];
    readonly animatedMeshes: readonly AnimatedMeshAsset[];
}
/** A retained renderer-neutral projection driven only by render diffs. */
export declare class RenderProjection {
    #private;
    /** Apply a frame in authored order and return renderer-neutral instructions. */
    applyFrame(frame: RenderFrameDiff): readonly RenderProjectionInstruction[];
    /** Apply one diff. Throws `RenderProjectionError` on stale handles or malformed payloads. */
    applyDiff(diff: RenderDiff): readonly RenderProjectionInstruction[];
    has(handle: RenderHandle): boolean;
    get handleCount(): number;
    node(handle: RenderHandle): RenderProjectionNode | undefined;
    materialDescriptor(id: string): RenderMaterialDescriptor | undefined;
    textureDescriptor(id: string): TextureDescriptor | undefined;
    spriteAtlas(id: string): SpriteAtlasDescriptor | undefined;
    staticMesh(asset: string): StaticMeshAsset | undefined;
    animatedMesh(asset: string): AnimatedMeshAsset | undefined;
    staticMeshRefCount(asset: string): number;
    animatedMeshRefCount(asset: string): number;
    snapshot(): RenderProjectionSnapshot;
    pickMesh(handle: RenderHandle): MeshPickHit | undefined;
    pickSprite(handle: RenderHandle): SpritePickHit | undefined;
}
//# sourceMappingURL=retained-projection.d.ts.map