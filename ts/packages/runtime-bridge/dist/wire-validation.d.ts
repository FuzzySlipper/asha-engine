type WireCandidate = object | boolean | number | string | null;
export declare function serializeOperationInput(operation: string, value: WireCandidate): string;
export declare function validateOperationInput(operation: string, value: WireCandidate): void;
export declare function parseOperationOutput<T extends WireCandidate>(operation: string, payload: string): T;
export declare function validateOperationOutput<T extends WireCandidate>(operation: string, value: T): T;
export {};
//# sourceMappingURL=wire-validation.d.ts.map