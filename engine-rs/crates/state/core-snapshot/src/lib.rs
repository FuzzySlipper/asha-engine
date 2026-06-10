//! Snapshot and deterministic state hashing for the ASHA authority core.
//!
//! # Lane
//!
//! `rust-state` — may depend on `core-ids`, `core-state`, `core-events`,
//! `core-error`. Must not reference protocol, render, UI, or any TypeScript
//! package.
//!
//! # Design
//!
//! A [`StateSnapshot`] is a fully-inspectable, ordered capture of a
//! [`StateStore`] at a single point in time. It carries a [`SNAPSHOT_VERSION`]
//! marker so future migrations have an obvious attachment point.
//!
//! [`StateHash`] is a compact `u64` fingerprint of the full store state,
//! computed with FNV-1a over a deterministic byte encoding. Because the
//! underlying [`BTreeMap`] and [`BTreeSet`] structures in `core-state` iterate
//! in sorted order, the encoding is stable across runs without extra sorting.
//!
//! # Why FNV-1a instead of `DefaultHasher`
//!
//! `DefaultHasher` is seeded randomly per-process and is not stable across
//! runs or Rust versions. FNV-1a has a fixed seed and a well-known output, so
//! two independent processes hashing the same state always agree.

#![forbid(unsafe_code)]

use core_ids::{EntityId, ModeId, ProcessId, SignalId, SubjectId, TagId};
use core_state::StateStore;

// ── Snapshot version ──────────────────────────────────────────────────────────

/// Compatibility marker. Increment when the snapshot encoding changes.
pub const SNAPSHOT_VERSION: u32 = 1;

// ── Snapshot types ────────────────────────────────────────────────────────────

/// Captured state of one entity at snapshot time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitySnapshot {
    pub id: EntityId,
    /// Tags in ascending order.
    pub tags: Vec<TagId>,
}

/// Captured state of one process at snapshot time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSnapshot {
    pub id: ProcessId,
    pub mode: Option<ModeId>,
}

/// A fully-inspectable, versioned capture of a [`StateStore`].
///
/// All collections are sorted by ID so the snapshot is comparable with `==`
/// and printable in a stable way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSnapshot {
    pub version: u32,
    pub entities: Vec<EntitySnapshot>,
    pub subjects: Vec<SubjectId>,
    pub processes: Vec<ProcessSnapshot>,
    pub modes: Vec<ModeId>,
    pub signals: Vec<SignalId>,
    pub tags: Vec<TagId>,
    /// Deterministic FNV-1a hash of the full snapshot content.
    pub hash: StateHash,
}

/// Compact deterministic fingerprint of a [`StateStore`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateHash(pub u64);

// ── Public API ────────────────────────────────────────────────────────────────

/// Capture `store` as an inspectable [`StateSnapshot`] with an embedded hash.
pub fn snapshot(store: &StateStore) -> StateSnapshot {
    // Build ordered collections from BTreeMap/BTreeSet iterators.
    let entities: Vec<EntitySnapshot> = store
        .entities()
        .map(|r| EntitySnapshot {
            id: r.id,
            tags: r.tags.iter().copied().collect(),
        })
        .collect();

    let subjects: Vec<SubjectId> = store.subjects().map(|r| r.id).collect();

    let processes: Vec<ProcessSnapshot> = store
        .processes()
        .map(|r| ProcessSnapshot {
            id: r.id,
            mode: r.mode,
        })
        .collect();

    let modes: Vec<ModeId> = store.modes().map(|r| r.id).collect();
    let signals: Vec<SignalId> = store.signals().map(|r| r.id).collect();
    let tags: Vec<TagId> = store.tags().map(|r| r.id).collect();

    let hash = hash_store(store);

    StateSnapshot {
        version: SNAPSHOT_VERSION,
        entities,
        subjects,
        processes,
        modes,
        signals,
        tags,
        hash,
    }
}

/// Compute a deterministic FNV-1a hash of `store` without capturing a full
/// snapshot. Use this when only the fingerprint is needed.
pub fn hash_store(store: &StateStore) -> StateHash {
    let mut h = Fnv1a::new();

    // Domain separator bytes prevent accidental hash collisions between a
    // state with one entity vs. a state with one subject having the same raw ID.
    h.write_u8(0x01); // entities section
    for rec in store.entities() {
        h.write_u64(rec.id.raw());
        h.write_u64(rec.tags.len() as u64);
        for tag in &rec.tags {
            h.write_u64(tag.raw());
        }
    }

    h.write_u8(0x02); // subjects section
    for rec in store.subjects() {
        h.write_u64(rec.id.raw());
    }

    h.write_u8(0x03); // processes section
    for rec in store.processes() {
        h.write_u64(rec.id.raw());
        match rec.mode {
            Some(m) => {
                h.write_u8(1);
                h.write_u64(m.raw());
            }
            None => h.write_u8(0),
        }
    }

    h.write_u8(0x04); // modes section
    for rec in store.modes() {
        h.write_u64(rec.id.raw());
    }

    h.write_u8(0x05); // signals section
    for rec in store.signals() {
        h.write_u64(rec.id.raw());
    }

    h.write_u8(0x06); // tags section
    for rec in store.tags() {
        h.write_u64(rec.id.raw());
    }

    StateHash(h.finish())
}

// ── FNV-1a hasher ─────────────────────────────────────────────────────────────

const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
const FNV_PRIME: u64 = 1_099_511_628_211;

struct Fnv1a(u64);

impl Fnv1a {
    fn new() -> Self {
        Self(FNV_OFFSET)
    }

    fn write_u8(&mut self, byte: u8) {
        self.0 ^= byte as u64;
        self.0 = self.0.wrapping_mul(FNV_PRIME);
    }

    fn write_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.write_u8(byte);
        }
    }

    fn finish(self) -> u64 {
        self.0
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core_ids::{EntityId, TagId};

    fn make_store_with_entity(eid: u64) -> StateStore {
        let mut s = StateStore::new();
        s.insert_entity(EntityId::new(eid));
        s
    }

    // ── State hash fixture ────────────────────────────────────────────────

    #[test]
    fn state_hash_fixture_empty_store_is_stable() {
        let store = StateStore::new();
        let h1 = hash_store(&store);
        let h2 = hash_store(&store);
        assert_eq!(h1, h2, "empty store hash must be deterministic");
    }

    #[test]
    fn state_hash_fixture_same_sequence_same_hash() {
        let mut s1 = StateStore::new();
        let mut s2 = StateStore::new();
        let eid = EntityId::new(1);
        let tid = TagId::new(5);

        for s in [&mut s1, &mut s2] {
            s.insert_tag(tid);
            s.insert_entity(eid);
            s.entity_mut(eid).unwrap().tags.insert(tid);
        }

        assert_eq!(
            hash_store(&s1),
            hash_store(&s2),
            "identical state must produce identical hash"
        );
    }

    /// The hash must depend only on *what* is in the store, not the order it
    /// was inserted — i.e. it is independent of any nondeterministic iteration.
    #[test]
    fn state_hash_independent_of_insertion_order() {
        let mut a = StateStore::new();
        let mut b = StateStore::new();

        // Insert the same entities and tags in opposite orders.
        for raw in [3u64, 1, 2] {
            a.insert_tag(TagId::new(raw));
            a.insert_entity(EntityId::new(raw));
        }
        for raw in [2u64, 3, 1] {
            b.insert_tag(TagId::new(raw));
            b.insert_entity(EntityId::new(raw));
        }
        // Tag the entities in different orders too.
        a.entity_mut(EntityId::new(1))
            .unwrap()
            .tags
            .insert(TagId::new(3));
        a.entity_mut(EntityId::new(1))
            .unwrap()
            .tags
            .insert(TagId::new(1));
        b.entity_mut(EntityId::new(1))
            .unwrap()
            .tags
            .insert(TagId::new(1));
        b.entity_mut(EntityId::new(1))
            .unwrap()
            .tags
            .insert(TagId::new(3));

        assert_eq!(
            hash_store(&a),
            hash_store(&b),
            "hash must be insertion-order independent"
        );
    }

    #[test]
    fn state_hash_fixture_meaningful_change_produces_different_hash() {
        let s_before = make_store_with_entity(1);
        let mut s_after = make_store_with_entity(1);
        s_after.insert_entity(EntityId::new(2)); // add a second entity

        assert_ne!(
            hash_store(&s_before),
            hash_store(&s_after),
            "state change must alter the hash"
        );
    }

    #[test]
    fn state_hash_tag_added_changes_hash() {
        let mut store = StateStore::new();
        let eid = EntityId::new(1);
        let tid = TagId::new(7);
        store.insert_entity(eid);
        store.insert_tag(tid);

        let h_before = hash_store(&store);
        store.entity_mut(eid).unwrap().tags.insert(tid);
        let h_after = hash_store(&store);

        assert_ne!(h_before, h_after, "adding a tag must change the hash");
    }

    // ── Snapshot scaffold ─────────────────────────────────────────────────

    #[test]
    fn snapshot_version_marker() {
        let store = StateStore::new();
        let snap = snapshot(&store);
        assert_eq!(snap.version, SNAPSHOT_VERSION);
    }

    #[test]
    fn snapshot_hash_matches_standalone_hash() {
        let mut store = StateStore::new();
        store.insert_entity(EntityId::new(42));
        let snap = snapshot(&store);
        assert_eq!(snap.hash, hash_store(&store));
    }

    #[test]
    fn snapshot_entity_tags_are_sorted() {
        let mut store = StateStore::new();
        let eid = EntityId::new(1);
        store.insert_entity(eid);
        // Insert tags in reverse order.
        for raw in [5u64, 3, 1, 4, 2] {
            store.insert_tag(TagId::new(raw));
            store.entity_mut(eid).unwrap().tags.insert(TagId::new(raw));
        }
        let snap = snapshot(&store);
        let tag_raws: Vec<u64> = snap.entities[0].tags.iter().map(|t| t.raw()).collect();
        let mut sorted = tag_raws.clone();
        sorted.sort();
        assert_eq!(tag_raws, sorted, "snapshot tags must be in ascending order");
    }

    #[test]
    fn snapshot_collections_reflect_store_contents() {
        let mut store = StateStore::new();
        store.insert_entity(EntityId::new(1));
        store.insert_entity(EntityId::new(2));
        store.insert_subject(core_ids::SubjectId::new(10));
        store.insert_mode(core_ids::ModeId::new(3));

        let snap = snapshot(&store);
        assert_eq!(snap.entities.len(), 2);
        assert_eq!(snap.subjects.len(), 1);
        assert_eq!(snap.modes.len(), 1);
        assert_eq!(snap.processes.len(), 0);
        assert_eq!(snap.signals.len(), 0);
        assert_eq!(snap.tags.len(), 0);
    }
}
