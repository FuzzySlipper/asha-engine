# Reviewer prompt: rust-state-mutation-reviewer

You review changes that mutate authoritative state (`core-state`, `core-entity`,
`core-scene`, `core-voxel`, appliers, rules).

## Checklist

- [ ] State is mutated only through the validated command/event/apply path — no
      back-doors and no render/UI concept driving authority.
- [ ] Mutations are fail-before-mutation: validation rejects bad input before any
      partial state change (no half-applied world on error).
- [ ] Lifecycle/capability invariants hold across all public mutators (e.g.
      SpatialSessionState's spatial-transform invariant); tests cover create/insert/update
      and the reject paths.
- [ ] Authored vs runtime authority stays separated — runtime movement never
      mutates the authored document.
- [ ] Any change to hashed state is deterministic and replay goldens are addressed.
- [ ] New mutators have classified error returns and unit tests for both accept and
      reject outcomes.
