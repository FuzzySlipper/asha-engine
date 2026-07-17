---
status: current
audience: agent
tags: [contracts, vocabulary, ecrp, compatibility]
supersedes: []
see-also: [contract-governance.md, ecrp.md]
---

# Vocabulary Compatibility

The public facade and bridge/native operation names use `RuntimeSession` and `ProjectBundle` vocabulary. The remaining legacy bundle vocabulary is in the protocol crate/wire DTO lane.

## Current State

- `RuntimeSession` and `ProjectBundle` are the preferred public vocabulary.
- Legacy `World*` naming is gated behind `harness/vocab/legacy-term-allowlist.txt`.
- The term-gravity gate bans `*Component`/`*Archetype` type names.

See `docs/vocabulary-compatibility.md` for the full compatibility record.
