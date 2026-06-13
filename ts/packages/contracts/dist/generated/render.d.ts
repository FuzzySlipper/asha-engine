import type { EntityId, TagId } from './ids.js';
export type RenderHandle = number & {
    readonly __brand: 'RenderHandle';
};
export declare const renderHandle: (raw: number) => RenderHandle;
export interface Transform {
    readonly translation: readonly [number, number, number];
    readonly rotation: readonly [number, number, number, number];
    readonly scale: readonly [number, number, number];
}
export type Geometry = {
    readonly shape: 'cube';
} | {
    readonly shape: 'sphere';
} | {
    readonly shape: 'quad';
} | {
    readonly shape: 'point';
} | {
    readonly shape: 'line';
    readonly a: readonly [number, number, number];
    readonly b: readonly [number, number, number];
};
export interface Material {
    readonly color: readonly [number, number, number, number];
    readonly wireframe: boolean;
}
export type RenderLayer = 'scene' | 'debug';
export interface RenderMetadata {
    readonly source: EntityId | null;
    readonly tags: readonly TagId[];
    readonly label: string | null;
}
export interface RenderNode {
    readonly geometry: Geometry;
    readonly material: Material;
    readonly transform: Transform;
    readonly visible: boolean;
    readonly layer: RenderLayer;
    readonly metadata: RenderMetadata;
}
export type MeshAttributeKind = 'f32';
export type MeshAttributeName = 'position' | 'normal' | 'uv' | 'color';
export interface MeshAttribute {
    readonly name: MeshAttributeName;
    readonly components: number;
    readonly kind: MeshAttributeKind;
}
export type MeshIndexWidth = 'u32';
export interface MeshBufferLayout {
    readonly vertexCount: number;
    readonly indexCount: number;
    readonly indexWidth: MeshIndexWidth;
    readonly attributes: readonly MeshAttribute[];
}
export interface MeshGroupDescriptor {
    readonly materialSlot: number;
    readonly start: number;
    readonly count: number;
}
export interface MeshBoundsDescriptor {
    readonly min: readonly [number, number, number];
    readonly max: readonly [number, number, number];
}
export type MeshProvenance = 'voxelChunk' | 'staticAsset' | 'generated' | 'debug';
export type MeshPayloadSource = {
    readonly kind: 'inline';
    readonly positions: readonly number[];
    readonly normals: readonly number[];
    readonly indices: readonly number[];
} | {
    readonly kind: 'handle';
    readonly buffer: number;
    readonly positionsByteOffset: number;
    readonly normalsByteOffset: number;
    readonly indicesByteOffset: number;
};
export interface MeshPayloadDescriptor {
    readonly layout: MeshBufferLayout;
    readonly groups: readonly MeshGroupDescriptor[];
    readonly bounds: MeshBoundsDescriptor;
    readonly source: MeshPayloadSource;
    readonly provenance: MeshProvenance;
}
export interface MeshMaterialSlot {
    readonly slot: number;
    readonly material: string;
}
export type MeshCollisionPolicy = {
    readonly kind: 'visualOnly';
} | {
    readonly kind: 'proxy';
    readonly proxyAsset: string;
} | {
    readonly kind: 'aabbFallback';
};
export interface StaticMeshAsset {
    readonly asset: string;
    readonly payload: MeshPayloadDescriptor;
    readonly materialSlots: readonly MeshMaterialSlot[];
    readonly collision: MeshCollisionPolicy;
}
export interface StaticMeshInstanceDescriptor {
    readonly asset: string;
    readonly transform: Transform;
    readonly materialOverrides: readonly MeshMaterialSlot[];
    readonly metadata: RenderMetadata;
}
export type SpriteSizeMode = 'world' | 'pixel';
export type BillboardMode = 'none' | 'spherical' | 'cylindrical';
export type SpriteDepthPolicy = 'default' | 'depthTestOff' | 'depthWriteOff';
export type SpriteShading = 'unlit' | 'lit' | 'shadowed' | 'custom';
export interface SpriteAttachment {
    readonly sourceEntity: EntityId | null;
    readonly sourceSceneNode: number | null;
    readonly attachmentPoint: string | null;
}
export interface SpriteInstanceDescriptor {
    readonly asset: string;
    readonly frame: number;
    readonly pivot: readonly [number, number];
    readonly size: readonly [number, number];
    readonly sizeMode: SpriteSizeMode;
    readonly billboard: BillboardMode;
    readonly tint: readonly [number, number, number, number];
    readonly renderOrder: number;
    readonly depth: SpriteDepthPolicy;
    readonly shading: SpriteShading;
    readonly transform: Transform;
    readonly attachment: SpriteAttachment;
    readonly metadata: RenderMetadata;
}
export interface SpritePickHit {
    readonly handle: RenderHandle;
    readonly sourceEntity: EntityId | null;
    readonly sourceSceneNode: number | null;
    readonly asset: string;
    readonly attachmentPoint: string | null;
}
export type TextureFilter = 'nearest' | 'linear';
export type TextureWrap = 'clamp' | 'repeat';
export interface TextureDescriptor {
    readonly id: string;
    readonly width: number;
    readonly height: number;
    readonly filter: TextureFilter;
    readonly wrap: TextureWrap;
    readonly contentHash: string | null;
    readonly version: number;
}
export interface SpriteFrameRect {
    readonly frame: number;
    readonly uvMin: readonly [number, number];
    readonly uvMax: readonly [number, number];
}
export interface SpriteAtlasDescriptor {
    readonly id: string;
    readonly texture: string;
    readonly frames: readonly SpriteFrameRect[];
}
export type MaterialUvStrategy = 'flat' | 'planar' | 'atlas';
export interface RenderMaterialDescriptor {
    readonly id: string;
    readonly color: readonly [number, number, number, number];
    readonly texture: string | null;
    readonly roughness: number;
    readonly emissive: number;
    readonly uvStrategy: MaterialUvStrategy;
}
export type RenderDiff = {
    readonly op: 'create';
    readonly handle: RenderHandle;
    readonly parent: RenderHandle | null;
    readonly node: RenderNode;
} | {
    readonly op: 'update';
    readonly handle: RenderHandle;
    readonly transform: Transform | null;
    readonly material: Material | null;
    readonly visible: boolean | null;
    readonly metadata: RenderMetadata | null;
} | {
    readonly op: 'destroy';
    readonly handle: RenderHandle;
} | {
    readonly op: 'replaceMeshPayload';
    readonly handle: RenderHandle;
    readonly payload: MeshPayloadDescriptor;
} | {
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
    readonly op: 'createStaticMeshInstance';
    readonly handle: RenderHandle;
    readonly parent: RenderHandle | null;
    readonly instance: StaticMeshInstanceDescriptor;
} | {
    readonly op: 'createSprite';
    readonly handle: RenderHandle;
    readonly parent: RenderHandle | null;
    readonly sprite: SpriteInstanceDescriptor;
} | {
    readonly op: 'updateSprite';
    readonly handle: RenderHandle;
    readonly frame: number | null;
    readonly tint: readonly [number, number, number, number] | null;
    readonly renderOrder: number | null;
    readonly visible: boolean | null;
};
export interface RenderFrameDiff {
    readonly ops: readonly RenderDiff[];
}
//# sourceMappingURL=render.d.ts.map