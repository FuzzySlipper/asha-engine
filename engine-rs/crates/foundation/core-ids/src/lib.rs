//! Typed abstract ID primitives for the ASHA authority core.
//!
//! # Lane
//!
//! `rust-foundation` — no knowledge of state, protocol, render, or TS packages.
//!
//! # Allowed dependencies
//!
//! None. This crate is `std`-only with zero external dependencies so that every
//! other crate in the workspace can depend on it without pulling in transitive
//! baggage.
//!
//! # Design
//!
//! Each ID is a `Copy` newtype over `u64`. The newtype wrapper makes it a
//! compile-time error to pass an `EntityId` where a `ProcessId` is expected.
//! All types implement `Eq`, `Ord`, `Hash`, `Debug`, and `Display` so they can
//! be used as map keys, sorted, and printed in fixtures without ceremony.

#![forbid(unsafe_code)]

macro_rules! id_type {
    (
        $(#[$attr:meta])*
        $name:ident
    ) => {
        $(#[$attr])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            /// Construct an ID from a raw `u64`.
            #[inline]
            pub const fn new(raw: u64) -> Self {
                Self(raw)
            }

            /// Return the underlying `u64`.
            #[inline]
            pub const fn raw(self) -> u64 {
                self.0
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

id_type!(
    /// Identifies a discrete simulated entity (e.g. an actor, object, or zone fixture).
    EntityId
);

id_type!(
    /// Identifies an agent or authority subject acting within the simulation.
    SubjectId
);

id_type!(
    /// Identifies an ongoing process or coroutine-like activity.
    ProcessId
);

id_type!(
    /// Identifies a discrete mode or state-machine variant.
    ModeId
);

id_type!(
    /// Identifies an event signal type (used in abstract fixture vocabulary).
    SignalId
);

id_type!(
    /// Identifies a tag label applied to entities or processes.
    TagId
);

// ── Scene / world identifiers ─────────────────────────────────────────────────
//
// Added for the scene/world foundation (scene-capability-01). These are *durable*
// authored/authority identities. They are deliberately distinct newtypes from
// `EntityId` (runtime authority) and from `protocol-render`'s `RenderHandle`
// (a derived projection handle, not save-file truth): a `SceneNodeId` is a stable
// authored identity, never a render handle. The source trace
// `SceneNodeId → EntityId → RenderHandle` reuses these existing newtypes; the
// trace *record* type itself lands with bootstrap work (subtask #2316).

id_type!(
    /// Identifies an authored, loadable scene document (`SceneDocument`).
    ///
    /// Stable across project moves and independent of array position; never a
    /// render handle.
    SceneId
);

id_type!(
    /// Identifies a live runtime world (`SpatialSessionState`) bootstrapped from a scene.
    ///
    /// A scene document is loaded *into* a world; the two identities are kept
    /// separate so a world save is authority-owned rather than tied to the
    /// originating scene document.
    WorldId
);

id_type!(
    /// Identifies one node within a scene document, stable across tree⇄flat
    /// transforms and serialization.
    ///
    /// This is the durable authored identity used for duplicate/cycle checks,
    /// parent lookup, and source tracing. It is **not** a render handle and must
    /// not depend on array position.
    SceneNodeId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_and_raw_roundtrip() {
        assert_eq!(EntityId::new(42).raw(), 42);
        assert_eq!(SubjectId::new(0).raw(), 0);
        assert_eq!(ProcessId::new(u64::MAX).raw(), u64::MAX);
        assert_eq!(ModeId::new(1).raw(), 1);
        assert_eq!(SignalId::new(99).raw(), 99);
        assert_eq!(TagId::new(7).raw(), 7);
        assert_eq!(SceneId::new(3).raw(), 3);
        assert_eq!(WorldId::new(4).raw(), 4);
        assert_eq!(SceneNodeId::new(5).raw(), 5);
    }

    /// Scene/world IDs are independent newtypes: a `SceneNodeId` cannot be passed
    /// where an `EntityId` (or any other ID) is expected, which is what keeps a
    /// stable authored node identity from being confused with a runtime entity or
    /// a derived render handle.
    #[test]
    fn scene_ids_are_distinct_types() {
        let scene = SceneId::new(1);
        let world = WorldId::new(1);
        let node = SceneNodeId::new(1);
        assert_eq!(scene.raw(), world.raw());
        assert_eq!(scene.raw(), node.raw());
        // `assert_eq!(scene, world)` / `assert_eq!(node, EntityId::new(1))` would
        // be compile errors — the types do not unify.
        assert_eq!(format!("{scene:?}"), "SceneId(1)");
        assert_eq!(format!("{world:?}"), "WorldId(1)");
        assert_eq!(format!("{node}"), "SceneNodeId(1)");
    }

    #[test]
    fn equality_and_hash() {
        use std::collections::HashSet;

        let a = EntityId::new(1);
        let b = EntityId::new(1);
        let c = EntityId::new(2);
        assert_eq!(a, b);
        assert_ne!(a, c);

        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b); // duplicate
        set.insert(c);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn ordering() {
        let ids: Vec<EntityId> = vec![EntityId::new(3), EntityId::new(1), EntityId::new(2)];
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(sorted[0].raw(), 1);
        assert_eq!(sorted[1].raw(), 2);
        assert_eq!(sorted[2].raw(), 3);
    }

    #[test]
    fn debug_and_display_shape() {
        let e = EntityId::new(5);
        assert_eq!(format!("{e:?}"), "EntityId(5)");
        assert_eq!(format!("{e}"), "EntityId(5)");

        let s = SignalId::new(0);
        assert_eq!(format!("{s:?}"), "SignalId(0)");
    }

    #[test]
    fn copy_semantics() {
        let original = ProcessId::new(10);
        let copied = original;
        // Both are usable after copy.
        assert_eq!(original.raw(), copied.raw());
    }

    /// Proves that distinct ID types cannot be confused at the type level.
    /// This is a compile-time guarantee; the runtime assertion just confirms
    /// the values are independently tracked.
    #[test]
    fn distinct_id_types_do_not_collapse() {
        let entity = EntityId::new(1);
        let process = ProcessId::new(1);
        // Same raw value, but different types — cannot be compared with ==.
        // The assertion below verifies independent storage.
        assert_eq!(entity.raw(), process.raw());
        // The following would be a compile error (uncomment to verify):
        // assert_eq!(entity, process);
    }
}
