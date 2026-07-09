# Bulk Voxel Edit Transactions

ASHA supports large authored voxel edits through a Rust-owned transaction surface
in `rule-voxel-edit`. The transaction surface wraps the canonical generated
`VoxelCommand` union (`SetVoxel`, `FillRegion`, `GenerateChunk`) instead of
creating a second command language.

## Authority Shape

`rule_voxel_edit::execute_transaction` accepts a `VoxelEditTransaction` with:

- `mode`: `PreviewOnly` or `Apply`
- `commands`: a bounded list of generated `VoxelCommand` values
- `limits`: command, event, and touched-voxel quotas

Validation runs sequentially on a scratch `VoxelWorld`. This allows a transaction
to generate a chunk and then edit that same chunk while keeping the real world
unchanged until the full transaction is accepted. `Apply` mode replaces the real
world only after all commands validate and all quotas pass. `PreviewOnly` returns
the same deterministic event log and projected hash without mutation.

## Receipt

`VoxelEditTransactionReceipt` records:

- accepted and rejected counts
- touched-voxel and event counts
- `before_hash`, `projected_hash`, `after_hash`, and `transaction_hash`
- the accepted `VoxelEditEvent` log
- classified transaction rejections

Quota rejections are fail-closed. A rejected apply transaction leaves
`after_hash == before_hash` and does not mutate authority.

## Persistence

The receipt's accepted event log is the durable persistence path. Existing
`rule_voxel_edit::persist` edit-log encode/decode/replay APIs round-trip the
events and reconstruct the same voxel content hash after reload.

## Studio And Demo Follow-Up

Studio and demo consumers should use this transaction surface for model-building
operations that exceed compact smoke-test limits. UI layers may compose brush,
stamp, import, or fill tools into generated `VoxelCommand` lists, but Rust owns
validation, quotas, receipts, hashes, and persistence. TypeScript must display
the receipt and request preview/apply; it must not invent a separate bulk edit
authority model.
