// World DTO drift guard (task #2423).
//
// The facade's world load/save payloads are deliberate *prototype* subsets of the
// generated protocol contracts (@asha/contracts). Until they are replaced outright,
// these compile-time assertions make drift visible: if a shared field's type
// changes in the generated contract, this file fails `tsc --build` (and therefore
// the package test), pointing back at the prototype DTO that needs updating.
import { test } from 'node:test';
import assert from 'node:assert/strict';
void test('world DTO drift guard compiles (see type assertions above)', () => {
    // The real guard is the compile-time AssertExact<…> exports; this runtime body
    // exists so the file registers as a test and documents the intent.
    assert.ok(true);
});
//# sourceMappingURL=world-dto-conformance.test.js.map