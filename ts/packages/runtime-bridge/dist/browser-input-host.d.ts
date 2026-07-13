import type { InputBindingCatalog, InputContextChangeReceipt, InputContextCommand, InputContextStackState, InputResolutionReceipt, RawInputSample, ResolvedInputAction } from '@asha/contracts';
import type { RuntimeInputPort } from './bridge.js';
export type BrowserInputSessionPort = Pick<RuntimeInputPort, 'configureInputSession' | 'applyInputContextCommand' | 'submitRawInput' | 'readInputContextState'>;
export interface BrowserKeyboardInput {
    readonly code: string;
    readonly repeat?: boolean;
    preventDefault?(): void;
}
export interface BrowserMouseMoveInput {
    readonly movementX: number;
    readonly movementY: number;
}
export interface BrowserPointerButtonInput {
    readonly button: number;
    preventDefault?(): void;
}
export interface BrowserWheelInput {
    readonly deltaY: number;
    preventDefault?(): void;
}
export type BrowserPointerLockIntent = {
    readonly kind: 'requestPointerLock';
    readonly reason: 'primaryButton' | 'programmatic';
} | {
    readonly kind: 'releasePointerLock';
    readonly reason: 'escapeKey' | 'programmatic';
};
export interface BrowserInputDelivery {
    readonly sample: RawInputSample;
    readonly receipt: InputResolutionReceipt;
    readonly activeContexts: readonly string[];
    readonly consumer: string | null;
    readonly reason: string;
}
export interface BrowserInputHostReadout {
    readonly activeContexts: readonly string[];
    readonly pointerLocked: boolean;
    readonly lastDelivery: BrowserInputDelivery | null;
    readonly recentDeliveries: readonly BrowserInputDelivery[];
}
export interface BrowserInputHostOptions {
    readonly session: BrowserInputSessionPort;
    readonly catalog?: InputBindingCatalog;
    readonly initialContexts?: readonly string[];
    readonly consumers?: Readonly<Record<string, string>>;
    readonly onResolvedAction?: (action: ResolvedInputAction, consumer: string | null) => void;
    readonly onContextChanged?: (state: InputContextStackState) => void;
}
export interface BrowserInputDomAttachment {
    readonly pointerTarget: HTMLElement;
    readonly keyboardTarget: Document;
    readonly mouseTarget?: Document;
    readonly acceptsKeyboard?: () => boolean;
    readonly onPointerLockIntent?: (intent: BrowserPointerLockIntent, event: Event) => void;
}
export declare class BrowserInputHost {
    #private;
    constructor(options: BrowserInputHostOptions);
    applyContextCommand(command: InputContextCommand): InputContextChangeReceipt;
    attachDom(attachment: BrowserInputDomAttachment): () => void;
    setPointerLockActive(active: boolean): BrowserInputHostReadout;
    requestPointerLock(): readonly BrowserPointerLockIntent[];
    releasePointerLock(): readonly BrowserPointerLockIntent[];
    handleKeyDown(event: BrowserKeyboardInput): BrowserInputDelivery;
    handleKeyUp(event: BrowserKeyboardInput): BrowserInputDelivery;
    handleMouseMove(event: BrowserMouseMoveInput): BrowserInputDelivery | null;
    handlePointerDown(event: BrowserPointerButtonInput): BrowserInputDelivery;
    handlePointerUp(event: BrowserPointerButtonInput): BrowserInputDelivery;
    handleWheel(event: BrowserWheelInput): BrowserInputDelivery | null;
    pointerLockIntentsForKey(event: BrowserKeyboardInput): readonly BrowserPointerLockIntent[];
    pointerLockIntentsForButton(event: BrowserPointerButtonInput): readonly BrowserPointerLockIntent[];
    readout(): BrowserInputHostReadout;
}
export declare function createDefaultBrowserInputCatalog(): InputBindingCatalog;
//# sourceMappingURL=browser-input-host.d.ts.map