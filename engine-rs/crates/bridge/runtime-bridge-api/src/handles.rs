// ── Opaque handle types ─────────────────────────────────────────────────────--

macro_rules! handle {
    ($(#[$a:meta])* $name:ident) => {
        $(#[$a])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);
        impl $name {
            pub const fn new(raw: u64) -> Self { Self(raw) }
            pub const fn raw(self) -> u64 { self.0 }
        }
    };
}

handle!(
    /// Opaque engine/session handle. Never a `StateStore`.
    EngineHandle
);
handle!(
    /// Opaque handle to bridge-owned buffer bytes (e.g. mesh geometry).
    RuntimeBufferHandle
);
handle!(
    /// Monotonic cursor into the render-diff stream.
    FrameCursor
);
handle!(
    /// Opaque replay-session handle (quarantined surface).
    ReplaySessionHandle
);

/// A borrowed, read-only view over bridge-owned bytes. Valid until the owning
/// buffer is released or the next frame supersedes it (see manifest `get_buffer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBufferView<'a> {
    pub handle: RuntimeBufferHandle,
    pub bytes: &'a [u8],
}
