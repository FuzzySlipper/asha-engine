# Voxel persistence format fixtures

Small committed samples of the `rule-voxel-edit::persist` text formats — the chunk
snapshot and the edit log. They pin the **on-disk format** so an accidental encoder
change is caught.

| File | Format | Generated/checked by |
|---|---|---|
| `sample-chunk.snapshot.txt` | RLE chunk snapshot (`encode_chunk_snapshot`) | `rule-voxel-edit::persist` test `snapshot_and_log_match_committed_goldens` (`include_str!`) |
| `sample-edits.log.txt` | edit-event log (`encode_edit_log`) | same test |

These are verified inline by `cargo test -p rule-voxel-edit` (under
`harness/ci/check-rust.sh`). They are tiny by design — illustrative format anchors, not a
full world. The larger composed save/compaction/durability goldens live in
`harness/fixtures/world-bundle/`; the full canonical world is in
`harness/fixtures/voxel-world/`.

To regenerate after an intentional format change, update the expected strings from the
test mismatch output and review the diff. See `docs/replay-model.md` (voxel durability)
and `docs/launchable-voxel.md`.
