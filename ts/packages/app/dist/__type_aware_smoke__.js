const indexOnly = {};
export const badIndexSignatureAccess = indexOnly.missing;
export function missingBoundaryReturnType(bridge) {
    return bridge.getProjectBundleCompositionStatus();
}
export function acceptsAny(value) {
    void value;
}
function acceptsVoidCallback(callback) {
    callback();
}
export function misusesPromiseCallback() {
    acceptsVoidCallback(async () => {
        return Promise.resolve();
    });
}
export function floatsPromise() {
    Promise.resolve('floating');
}
export function unsafeJsonAccess(payload) {
    const decoded = JSON.parse(payload);
    return decoded.value.trim();
}
//# sourceMappingURL=__type_aware_smoke__.js.map