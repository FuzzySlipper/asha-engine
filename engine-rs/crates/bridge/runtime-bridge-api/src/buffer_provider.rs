//! Runtime-owned large-buffer handle provider and lifetime model (#2381).
//!
//! # Why this exists
//!
//! Large render payloads (mesh positions/normals/indices, future texture/atlas
//! bytes) must cross the runtime bridge **by handle**, never inlined through JSON
//! (ADR 0006). The bytes are owned by the Rust runtime; TypeScript borrows a
//! read-only view for the lifetime the provider permits, then must copy out.
//!
//! This module owns that ownership model: it allocates [`RuntimeBufferHandle`]s,
//! tracks each buffer's [`BufferKind`], byte length, monotonic [`version`] and
//! [`BufferLifetime`], and fails **closed** on every misuse an agent could make:
//!
//! - reading an **unknown** handle ([`RuntimeBridgeErrorKind::UnknownHandle`]);
//! - reading a **disposed/invalidated/stale** handle
//!   ([`RuntimeBridgeErrorKind::BufferExpired`]);
//! - reading a handle as the **wrong kind**
//!   ([`RuntimeBridgeErrorKind::InvalidInput`]);
//! - **double-disposing** a handle ([`RuntimeBridgeErrorKind::UnknownHandle`]).
//!
//! Raw memory never escapes: callers only ever obtain a borrowed `&[u8]` view or
//! the small [`BufferMetadata`] record. There is no public `Vec<u8>`, no raw
//! pointer, and no `serde_json::Value` escape hatch.
//!
//! [`version`]: BufferMetadata::version

use crate::{RuntimeBridgeError, RuntimeBridgeErrorKind, RuntimeBufferHandle, RuntimeBufferView};
use std::collections::BTreeMap;

/// What a runtime buffer's bytes represent. Lets the provider reject a wrong-kind
/// access instead of letting a consumer reinterpret bytes blindly (e.g. decoding
/// an opaque blob as a mesh geometry stream).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BufferKind {
    /// Concatenated mesh geometry bytes (vertex attribute streams + index stream),
    /// addressed by byte offsets carried in the render protocol's mesh handle
    /// source. The single source of large geometry payloads today.
    MeshGeometry,
    /// Bytes with no provider-level interpretation (smoke/seed buffers, future
    /// payload families). The provider tracks length/version/lifetime but ascribes
    /// no structure.
    Opaque,
}

impl BufferKind {
    /// Stable border label for diagnostics (never parsed back).
    pub fn label(self) -> &'static str {
        match self {
            BufferKind::MeshGeometry => "meshGeometry",
            BufferKind::Opaque => "opaque",
        }
    }
}

/// Who governs when a buffer's bytes may stop being readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BufferLifetime {
    /// Lives until an explicit [`RuntimeBufferProvider::dispose`]. Used for
    /// authored/static payloads whose lifetime the producer controls directly.
    Manual,
    /// Bound to the frame it was created in. Advancing the frame
    /// ([`RuntimeBufferProvider::advance_frame`]) invalidates it, so an
    /// outstanding view from a prior frame fails closed instead of reading bytes
    /// the runtime may have recycled.
    Frame,
}

impl BufferLifetime {
    /// Stable border label for diagnostics (never parsed back).
    pub fn label(self) -> &'static str {
        match self {
            BufferLifetime::Manual => "manual",
            BufferLifetime::Frame => "frame",
        }
    }
}

/// Small, copyable description of a buffer. Carries no bytes — reads go through
/// [`RuntimeBufferProvider::view`] so the borrow lifetime is explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferMetadata {
    pub handle: RuntimeBufferHandle,
    pub kind: BufferKind,
    /// Current byte length of the buffer's bytes.
    pub byte_len: u32,
    /// Monotonic generation, starting at 1 and incremented on every
    /// [`RuntimeBufferProvider::replace`]. A consumer that cached a version can
    /// detect that the bytes behind a handle were replaced underneath it.
    pub version: u32,
    /// Element stride in bytes for typed access, when the kind has a uniform
    /// element size; `None` for variable/heterogeneous layouts (e.g. a mesh
    /// geometry blob whose sub-streams are addressed by explicit byte offsets).
    pub stride: Option<u32>,
    pub lifetime: BufferLifetime,
    /// The frame index the buffer was created in (for `Frame` lifetime auditing).
    pub created_frame: u64,
}

/// Internal liveness of a buffer entry. Distinguishing `Invalidated` from removed
/// lets us return [`RuntimeBridgeErrorKind::BufferExpired`] (a stale-read, the
/// caller should re-fetch) rather than `UnknownHandle` (the handle never existed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BufferState {
    Live,
    Invalidated,
}

#[derive(Debug, Clone)]
struct BufferEntry {
    kind: BufferKind,
    bytes: Vec<u8>,
    version: u32,
    stride: Option<u32>,
    lifetime: BufferLifetime,
    created_frame: u64,
    state: BufferState,
}

impl BufferEntry {
    fn metadata(&self, handle: RuntimeBufferHandle) -> BufferMetadata {
        BufferMetadata {
            handle,
            kind: self.kind,
            byte_len: self.bytes.len() as u32,
            version: self.version,
            stride: self.stride,
            lifetime: self.lifetime,
            created_frame: self.created_frame,
        }
    }
}

/// Owns large buffer bytes behind opaque [`RuntimeBufferHandle`]s with explicit
/// lifetimes, versions, and invalidation. See the module docs for the failure model.
///
/// Allocation is deterministic: handles are assigned from a monotonic counter that
/// starts at 0, so a freshly-`reset` provider replays identical handle ids — which
/// keeps handle-backed fixtures hash-stable for conformance/replay tests.
#[derive(Debug)]
pub struct RuntimeBufferProvider {
    next_raw: u64,
    entries: BTreeMap<u64, BufferEntry>,
    current_frame: u64,
}

impl Default for RuntimeBufferProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeBufferProvider {
    pub fn new() -> Self {
        Self {
            next_raw: 0,
            entries: BTreeMap::new(),
            current_frame: 0,
        }
    }

    /// Drop all buffers and reset deterministic allocation. Used when an engine is
    /// (re)initialized so a replay starts from the same handle ids.
    pub fn reset(&mut self) {
        self.next_raw = 0;
        self.entries.clear();
        self.current_frame = 0;
    }

    /// Allocate a new buffer owning `bytes`, returning its opaque handle. `version`
    /// starts at 1. The bytes are moved in; the caller keeps no reference.
    pub fn create(
        &mut self,
        kind: BufferKind,
        lifetime: BufferLifetime,
        stride: Option<u32>,
        bytes: Vec<u8>,
    ) -> RuntimeBufferHandle {
        let raw = self.next_raw;
        self.next_raw += 1;
        self.entries.insert(
            raw,
            BufferEntry {
                kind,
                bytes,
                version: 1,
                stride,
                lifetime,
                created_frame: self.current_frame,
                state: BufferState::Live,
            },
        );
        RuntimeBufferHandle::new(raw)
    }

    /// Replace a live buffer's bytes in place, bumping its [`version`] and returning
    /// the new metadata. Kind/lifetime/stride are preserved. Fails closed if the
    /// handle is unknown or no longer live (a stale/disposed handle is not a valid
    /// replace target).
    ///
    /// [`version`]: BufferMetadata::version
    pub fn replace(
        &mut self,
        handle: RuntimeBufferHandle,
        bytes: Vec<u8>,
    ) -> Result<BufferMetadata, RuntimeBridgeError> {
        let current_frame = self.current_frame;
        let entry = self.live_entry_mut(handle)?;
        entry.bytes = bytes;
        entry.version += 1;
        entry.created_frame = current_frame;
        Ok(entry.metadata(handle))
    }

    /// Read a buffer's metadata without borrowing its bytes. Fails closed for
    /// unknown/expired handles.
    pub fn metadata(
        &self,
        handle: RuntimeBufferHandle,
    ) -> Result<BufferMetadata, RuntimeBridgeError> {
        let entry = self.live_entry(handle)?;
        Ok(entry.metadata(handle))
    }

    /// Borrow a read-only view over a live buffer's bytes. The view is valid until
    /// the buffer is replaced, invalidated, disposed, or (for `Frame` lifetime) the
    /// frame advances — the borrow checker enforces this statically because the
    /// returned view borrows `self`.
    pub fn view(
        &self,
        handle: RuntimeBufferHandle,
    ) -> Result<RuntimeBufferView<'_>, RuntimeBridgeError> {
        let entry = self.live_entry(handle)?;
        Ok(RuntimeBufferView {
            handle,
            bytes: &entry.bytes,
        })
    }

    /// Borrow a view, asserting the buffer is of `expected` kind. A wrong-kind
    /// access fails closed with [`RuntimeBridgeErrorKind::InvalidInput`] rather than
    /// handing back bytes the caller would misinterpret.
    pub fn view_of_kind(
        &self,
        handle: RuntimeBufferHandle,
        expected: BufferKind,
    ) -> Result<RuntimeBufferView<'_>, RuntimeBridgeError> {
        let entry = self.live_entry(handle)?;
        if entry.kind != expected {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!(
                    "buffer {} is kind {} but was accessed as {}",
                    handle.raw(),
                    entry.kind.label(),
                    expected.label()
                ),
            ));
        }
        Ok(RuntimeBufferView {
            handle,
            bytes: &entry.bytes,
        })
    }

    /// Borrow a view, asserting the caller's cached `expected_version` still matches.
    /// If the bytes were replaced (version advanced) the read fails closed with
    /// [`RuntimeBridgeErrorKind::BufferExpired`] — the caller is looking at a stale
    /// generation and must re-read metadata.
    pub fn view_versioned(
        &self,
        handle: RuntimeBufferHandle,
        expected_version: u32,
    ) -> Result<RuntimeBufferView<'_>, RuntimeBridgeError> {
        let entry = self.live_entry(handle)?;
        if entry.version != expected_version {
            return Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::BufferExpired,
                format!(
                    "buffer {} version is {} but caller expected {}",
                    handle.raw(),
                    entry.version,
                    expected_version
                ),
            ));
        }
        Ok(RuntimeBufferView {
            handle,
            bytes: &entry.bytes,
        })
    }

    /// Mark a buffer stale without forgetting it existed. Subsequent reads fail with
    /// [`RuntimeBridgeErrorKind::BufferExpired`]. Idempotent; an unknown handle still
    /// fails closed.
    pub fn invalidate(&mut self, handle: RuntimeBufferHandle) -> Result<(), RuntimeBridgeError> {
        let entry = self.known_entry_mut(handle)?;
        entry.state = BufferState::Invalidated;
        Ok(())
    }

    /// Drop a buffer entirely, freeing its bytes. A second dispose of the same handle
    /// fails closed with [`RuntimeBridgeErrorKind::UnknownHandle`] (it no longer
    /// exists), making double-dispose detectable rather than silent.
    pub fn dispose(&mut self, handle: RuntimeBufferHandle) -> Result<(), RuntimeBridgeError> {
        match self.entries.remove(&handle.raw()) {
            Some(_) => Ok(()),
            None => Err(self.unknown(handle)),
        }
    }

    /// Advance the frame counter, invalidating every `Frame`-lifetime buffer created
    /// in an earlier frame. `Manual`-lifetime buffers are untouched. Returns the
    /// number of buffers invalidated (useful for leak/lifecycle assertions).
    pub fn advance_frame(&mut self) -> usize {
        self.current_frame += 1;
        let frame = self.current_frame;
        let mut invalidated = 0;
        for entry in self.entries.values_mut() {
            if entry.lifetime == BufferLifetime::Frame
                && entry.state == BufferState::Live
                && entry.created_frame < frame
            {
                entry.state = BufferState::Invalidated;
                invalidated += 1;
            }
        }
        invalidated
    }

    /// The current frame index.
    pub fn current_frame(&self) -> u64 {
        self.current_frame
    }

    /// Number of known entries (live + invalidated-but-not-disposed). Backs
    /// resource-leak assertions.
    pub fn known_count(&self) -> usize {
        self.entries.len()
    }

    /// Number of entries still readable.
    pub fn live_count(&self) -> usize {
        self.entries
            .values()
            .filter(|e| e.state == BufferState::Live)
            .count()
    }

    // ── internal lookups ──────────────────────────────────────────────────────

    fn unknown(&self, handle: RuntimeBufferHandle) -> RuntimeBridgeError {
        RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::UnknownHandle,
            format!("no buffer for handle {}", handle.raw()),
        )
    }

    fn known_entry_mut(
        &mut self,
        handle: RuntimeBufferHandle,
    ) -> Result<&mut BufferEntry, RuntimeBridgeError> {
        match self.entries.get_mut(&handle.raw()) {
            Some(entry) => Ok(entry),
            None => Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::UnknownHandle,
                format!("no buffer for handle {}", handle.raw()),
            )),
        }
    }

    fn live_entry(&self, handle: RuntimeBufferHandle) -> Result<&BufferEntry, RuntimeBridgeError> {
        match self.entries.get(&handle.raw()) {
            None => Err(self.unknown(handle)),
            Some(entry) if entry.state != BufferState::Live => Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::BufferExpired,
                format!("buffer {} has been invalidated", handle.raw()),
            )),
            Some(entry) => Ok(entry),
        }
    }

    fn live_entry_mut(
        &mut self,
        handle: RuntimeBufferHandle,
    ) -> Result<&mut BufferEntry, RuntimeBridgeError> {
        // Resolve the classified error before taking the mutable borrow.
        match self.entries.get(&handle.raw()).map(|e| e.state) {
            None => Err(self.unknown(handle)),
            Some(state) if state != BufferState::Live => Err(RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::BufferExpired,
                format!("buffer {} has been invalidated", handle.raw()),
            )),
            Some(_) => Ok(self
                .entries
                .get_mut(&handle.raw())
                .expect("checked present")),
        }
    }
}

// ── Deterministic conformance fixtures ──────────────────────────────────────────
//
// Byte builders for tests that need a handle-backed payload without depending on
// `protocol-render` (which would invert the layer order). `fixtures` produce raw
// bytes + the byte offsets a mesh handle source carries; higher layers map these
// onto `MeshPayloadSource::Handle`.

/// Deterministic fixture builders. No randomness, no clock — identical bytes every
/// run so handle-backed goldens stay stable.
pub mod fixtures {
    /// A handle-backed mesh geometry blob: positions, then normals, then indices,
    /// concatenated little-endian, with the byte offsets a mesh handle source needs.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct MeshGeometryBlob {
        pub bytes: Vec<u8>,
        pub positions_byte_offset: u32,
        pub normals_byte_offset: u32,
        pub indices_byte_offset: u32,
        pub vertex_count: u32,
        pub index_count: u32,
    }

    /// Build a mesh geometry blob from f32 positions/normals and u32 indices, laid
    /// out as `[positions | normals | indices]`. Deterministic and allocation-bounded.
    pub fn mesh_geometry_blob(
        positions: &[f32],
        normals: &[f32],
        indices: &[u32],
    ) -> MeshGeometryBlob {
        let mut bytes =
            Vec::with_capacity(positions.len() * 4 + normals.len() * 4 + indices.len() * 4);
        let positions_byte_offset = bytes.len() as u32;
        for v in positions {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let normals_byte_offset = bytes.len() as u32;
        for v in normals {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let indices_byte_offset = bytes.len() as u32;
        for i in indices {
            bytes.extend_from_slice(&i.to_le_bytes());
        }
        MeshGeometryBlob {
            bytes,
            positions_byte_offset,
            normals_byte_offset,
            indices_byte_offset,
            vertex_count: (positions.len() / 3) as u32,
            index_count: indices.len() as u32,
        }
    }

    /// A single deterministic triangle (3 verts, 3 indices) in the XY plane with
    /// +Z normals. The smallest non-trivial handle-backed mesh.
    pub fn unit_triangle() -> MeshGeometryBlob {
        let positions = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let normals = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let indices = [0u32, 1, 2];
        mesh_geometry_blob(&positions, &normals, &indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opaque(provider: &mut RuntimeBufferProvider, bytes: &[u8]) -> RuntimeBufferHandle {
        provider.create(
            BufferKind::Opaque,
            BufferLifetime::Manual,
            None,
            bytes.to_vec(),
        )
    }

    #[test]
    fn create_read_metadata_round_trips() {
        let mut p = RuntimeBufferProvider::new();
        let h = p.create(
            BufferKind::MeshGeometry,
            BufferLifetime::Manual,
            Some(12),
            vec![1, 2, 3, 4],
        );
        let meta = p.metadata(h).unwrap();
        assert_eq!(meta.handle, h);
        assert_eq!(meta.kind, BufferKind::MeshGeometry);
        assert_eq!(meta.byte_len, 4);
        assert_eq!(meta.version, 1);
        assert_eq!(meta.stride, Some(12));
        assert_eq!(meta.lifetime, BufferLifetime::Manual);
        assert_eq!(p.view(h).unwrap().bytes, &[1, 2, 3, 4]);
    }

    #[test]
    fn deterministic_handle_allocation() {
        let mut p = RuntimeBufferProvider::new();
        let a = opaque(&mut p, &[0]);
        let b = opaque(&mut p, &[1]);
        assert_eq!(a.raw(), 0);
        assert_eq!(b.raw(), 1);
        p.reset();
        let a2 = opaque(&mut p, &[0]);
        assert_eq!(a2.raw(), 0, "reset replays identical handle ids");
    }

    #[test]
    fn unknown_handle_fails_closed() {
        let p = RuntimeBufferProvider::new();
        let err = p.view(RuntimeBufferHandle::new(7)).unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::UnknownHandle);
        assert_eq!(
            p.metadata(RuntimeBufferHandle::new(7)).unwrap_err().kind,
            RuntimeBridgeErrorKind::UnknownHandle
        );
    }

    #[test]
    fn wrong_kind_access_fails_closed() {
        let mut p = RuntimeBufferProvider::new();
        let h = opaque(&mut p, &[9, 9]);
        let err = p.view_of_kind(h, BufferKind::MeshGeometry).unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::InvalidInput);
        // Same kind succeeds.
        assert_eq!(
            p.view_of_kind(h, BufferKind::Opaque).unwrap().bytes,
            &[9, 9]
        );
    }

    #[test]
    fn replace_bumps_version_and_stales_old_generation() {
        let mut p = RuntimeBufferProvider::new();
        let h = opaque(&mut p, &[1]);
        assert_eq!(p.metadata(h).unwrap().version, 1);
        let meta = p.replace(h, vec![2, 2]).unwrap();
        assert_eq!(meta.version, 2);
        assert_eq!(meta.byte_len, 2);
        assert_eq!(p.view(h).unwrap().bytes, &[2, 2]);
        // A consumer holding version 1 fails closed.
        let err = p.view_versioned(h, 1).unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::BufferExpired);
        assert!(p.view_versioned(h, 2).is_ok());
    }

    #[test]
    fn invalidate_then_read_is_buffer_expired() {
        let mut p = RuntimeBufferProvider::new();
        let h = opaque(&mut p, &[1, 2]);
        p.invalidate(h).unwrap();
        let err = p.view(h).unwrap_err();
        assert_eq!(err.kind, RuntimeBridgeErrorKind::BufferExpired);
        // Still known (not disposed), so it counts toward known but not live.
        assert_eq!(p.known_count(), 1);
        assert_eq!(p.live_count(), 0);
        // Invalidate is idempotent.
        assert!(p.invalidate(h).is_ok());
        // Replacing an invalidated buffer fails closed.
        assert_eq!(
            p.replace(h, vec![0]).unwrap_err().kind,
            RuntimeBridgeErrorKind::BufferExpired
        );
    }

    #[test]
    fn dispose_frees_and_double_dispose_fails_closed() {
        let mut p = RuntimeBufferProvider::new();
        let h = opaque(&mut p, &[1]);
        assert_eq!(p.known_count(), 1);
        p.dispose(h).unwrap();
        assert_eq!(p.known_count(), 0);
        // Reads after dispose are unknown (the handle is gone, never existed-state).
        assert_eq!(
            p.view(h).unwrap_err().kind,
            RuntimeBridgeErrorKind::UnknownHandle
        );
        // Second dispose fails closed.
        assert_eq!(
            p.dispose(h).unwrap_err().kind,
            RuntimeBridgeErrorKind::UnknownHandle
        );
    }

    #[test]
    fn frame_lifetime_invalidated_on_advance() {
        let mut p = RuntimeBufferProvider::new();
        let frame_buf = p.create(BufferKind::Opaque, BufferLifetime::Frame, None, vec![1]);
        let manual_buf = p.create(BufferKind::Opaque, BufferLifetime::Manual, None, vec![2]);
        // Same frame: still readable.
        assert!(p.view(frame_buf).is_ok());
        let invalidated = p.advance_frame();
        assert_eq!(invalidated, 1);
        // Frame buffer is now stale; manual buffer survives.
        assert_eq!(
            p.view(frame_buf).unwrap_err().kind,
            RuntimeBridgeErrorKind::BufferExpired
        );
        assert!(p.view(manual_buf).is_ok());
        // A buffer created in the new frame survives the same advance boundary.
        let fresh = p.create(BufferKind::Opaque, BufferLifetime::Frame, None, vec![3]);
        assert!(p.view(fresh).is_ok());
    }

    #[test]
    fn no_leak_after_create_replace_dispose_cycles() {
        let mut p = RuntimeBufferProvider::new();
        for _ in 0..32 {
            let h = opaque(&mut p, &[0; 16]);
            p.replace(h, vec![1; 32]).unwrap();
            p.dispose(h).unwrap();
        }
        assert_eq!(p.known_count(), 0);
        assert_eq!(p.live_count(), 0);
    }

    #[test]
    fn lifecycle_pressure_tracks_state_and_leaks_nothing() {
        // Conformance under realistic lifecycle pressure (#2383): create several
        // buffer-backed payloads (incl. one non-trivially large), replace some,
        // invalidate some, dispose some, and assert the provider's accounting.
        let mut p = RuntimeBufferProvider::new();

        // A non-trivially large synthetic mesh geometry buffer (~4k verts) that
        // exercises real byte volume without making the test slow.
        let large = {
            let vertex_count = 4096usize;
            let mut positions = Vec::with_capacity(vertex_count * 3);
            let mut normals = Vec::with_capacity(vertex_count * 3);
            for i in 0..vertex_count {
                positions.extend_from_slice(&[i as f32, (i as f32) * 0.5, 0.0]);
                normals.extend_from_slice(&[0.0, 0.0, 1.0]);
            }
            let indices: Vec<u32> = (0..(vertex_count as u32 - 2))
                .flat_map(|i| [i, i + 1, i + 2])
                .collect();
            fixtures::mesh_geometry_blob(&positions, &normals, &indices)
        };
        assert!(
            large.bytes.len() > 100_000,
            "large buffer should be sizeable"
        );

        let big = p.create(
            BufferKind::MeshGeometry,
            BufferLifetime::Manual,
            None,
            large.bytes,
        );
        let a = p.create(
            BufferKind::MeshGeometry,
            BufferLifetime::Frame,
            None,
            vec![1, 2, 3, 4],
        );
        let b = p.create(
            BufferKind::MeshGeometry,
            BufferLifetime::Manual,
            None,
            vec![9; 12],
        );
        assert_eq!(p.known_count(), 3);
        assert_eq!(p.live_count(), 3);

        // Replace one: version bumps, bytes change, still one entry.
        let meta = p.replace(b, vec![7; 24]).unwrap();
        assert_eq!(meta.version, 2);
        assert_eq!(p.metadata(b).unwrap().byte_len, 24);

        // Advance a frame: the Frame-lifetime buffer goes stale; manual survive.
        assert_eq!(p.advance_frame(), 1);
        assert_eq!(
            p.view(a).unwrap_err().kind,
            RuntimeBridgeErrorKind::BufferExpired
        );
        assert!(p.view(big).is_ok());
        assert!(p.view(b).is_ok());
        assert_eq!(p.live_count(), 2);
        assert_eq!(p.known_count(), 3); // stale-but-not-disposed still tracked

        // Dispose all: nothing leaks; double dispose fails closed.
        for h in [big, a, b] {
            p.dispose(h).unwrap();
        }
        assert_eq!(p.known_count(), 0);
        assert_eq!(p.live_count(), 0);
        assert_eq!(
            p.dispose(big).unwrap_err().kind,
            RuntimeBridgeErrorKind::UnknownHandle
        );
    }

    #[test]
    fn diagnostics_are_stable_and_classified() {
        // Stale, double-dispose, and wrong-kind produce stable kinds + messages so
        // a renderer/agent can route deterministically (#2383 diagnostics).
        let mut p = RuntimeBufferProvider::new();
        let h = p.create(BufferKind::Opaque, BufferLifetime::Manual, None, vec![0; 4]);

        p.invalidate(h).unwrap();
        let stale = p.view(h).unwrap_err();
        assert_eq!(stale.kind, RuntimeBridgeErrorKind::BufferExpired);
        assert_eq!(stale.message, "buffer 0 has been invalidated");

        let wrong = p.create(BufferKind::Opaque, BufferLifetime::Manual, None, vec![0; 4]);
        let wrong_kind = p.view_of_kind(wrong, BufferKind::MeshGeometry).unwrap_err();
        assert_eq!(wrong_kind.kind, RuntimeBridgeErrorKind::InvalidInput);
        assert_eq!(
            wrong_kind.message,
            "buffer 1 is kind opaque but was accessed as meshGeometry"
        );

        p.dispose(wrong).unwrap();
        let double = p.dispose(wrong).unwrap_err();
        assert_eq!(double.kind, RuntimeBridgeErrorKind::UnknownHandle);
        assert_eq!(double.message, "no buffer for handle 1");
    }

    #[test]
    fn mesh_geometry_fixture_is_deterministic_and_offset_correct() {
        let blob = fixtures::unit_triangle();
        assert_eq!(blob.vertex_count, 3);
        assert_eq!(blob.index_count, 3);
        assert_eq!(blob.positions_byte_offset, 0);
        assert_eq!(blob.normals_byte_offset, 9 * 4);
        assert_eq!(blob.indices_byte_offset, 18 * 4);
        assert_eq!(blob.bytes.len(), (9 + 9 + 3) * 4);
        // Deterministic across calls.
        assert_eq!(blob, fixtures::unit_triangle());

        // Round-trips through the provider as a MeshGeometry buffer.
        let mut p = RuntimeBufferProvider::new();
        let h = p.create(
            BufferKind::MeshGeometry,
            BufferLifetime::Manual,
            None,
            blob.bytes.clone(),
        );
        let view = p.view_of_kind(h, BufferKind::MeshGeometry).unwrap();
        assert_eq!(view.bytes.len(), blob.bytes.len());
    }
}
