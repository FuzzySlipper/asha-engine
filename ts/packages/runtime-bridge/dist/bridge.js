export const frameCursor = (frame) => frame;
/** Typed, classified error for every facade operation. No JSON error blobs. */
export class RuntimeBridgeError extends Error {
    kind;
    operation;
    path;
    retryable;
    details;
    provenance;
    constructor(kind, message, context = {}) {
        super(`runtime bridge error [${kind}]: ${message}`);
        this.kind = kind;
        this.name = 'RuntimeBridgeError';
        this.operation = context.operation ?? null;
        this.path = context.path ?? null;
        this.retryable = context.retryable ?? false;
        this.details = context.details ?? [];
        this.provenance = context.provenance ?? 'runtime_facade';
    }
}
export function nonNegativeSafeInteger(value, field) {
    if (!Number.isSafeInteger(value) || value < 0) {
        throw new RuntimeBridgeError('invalid_input', `${field} must be a non-negative safe integer`);
    }
    return value;
}
export function u32(value, field) {
    nonNegativeSafeInteger(value, field);
    if (value > 0xffffffff) {
        throw new RuntimeBridgeError('invalid_input', `${field} must fit in u32`);
    }
    return value;
}
export { RUNTIME_BRIDGE_PORT_CONTRACTS, runtimeBridgePorts, } from './generated/surfaces.js';
//# sourceMappingURL=bridge.js.map