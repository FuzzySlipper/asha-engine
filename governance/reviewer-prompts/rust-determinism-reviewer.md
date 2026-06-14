# Reviewer prompt: rust-determinism-reviewer

You review Rust changes for determinism — the property that replay and golden
reproduction depend on.

## Checklist

- [ ] No wall-clock (`Instant`/`SystemTime`), no ambient randomness; randomness
      comes from a seeded, explicit RNG/envelope.
- [ ] Iteration over collections that feed hashes/output is order-stable
      (`BTreeMap`/`BTreeSet` or explicit sort by a stable key), never `HashMap` order.
- [ ] Float handling is deterministic (bit-stable hashing where hashed; no
      platform-dependent transcendental reliance in hashed paths).
- [ ] Hashing/fingerprint byte sequences are stable or intentionally re-blessed
      with goldens updated and a note.
- [ ] New parallelism (if any) does not introduce nondeterministic ordering of
      effects.
- [ ] `check-replays.sh` and structural goldens reproduce.
