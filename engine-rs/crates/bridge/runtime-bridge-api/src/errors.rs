use crate::*;

// ── Error taxonomy ────────────────────────────────────────────────────────────

/// Typed, classified error channel shared by every bridge operation. There is no
/// string/JSON error blob escape hatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBridgeError {
    pub kind: RuntimeBridgeErrorKind,
    pub message: String,
}

/// Stable classification an orchestrator/renderer can switch on without parsing prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeBridgeErrorKind {
    /// An operation was called before `initialize_engine`.
    NotInitialized,
    /// The input violated an invariant the bridge can check cheaply.
    InvalidInput,
    /// A handle (engine/buffer/replay) is unknown or already released.
    UnknownHandle,
    /// A borrowed buffer view was used after it was released/superseded.
    BufferExpired,
    /// The native transport could not be loaded (addon missing/ABI mismatch).
    NativeUnavailable,
    /// An unexpected internal failure (a bug, not an input problem).
    Internal,
}

impl RuntimeBridgeError {
    pub fn new(kind: RuntimeBridgeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Map to the shared foundation category so tools can treat bridge failures
    /// uniformly with the rest of the workspace.
    pub fn category(&self) -> ErrorCategory {
        match self.kind {
            RuntimeBridgeErrorKind::InvalidInput => ErrorCategory::Invalid,
            RuntimeBridgeErrorKind::UnknownHandle => ErrorCategory::NotFound,
            RuntimeBridgeErrorKind::BufferExpired => ErrorCategory::Conflict,
            RuntimeBridgeErrorKind::NotInitialized | RuntimeBridgeErrorKind::NativeUnavailable => {
                ErrorCategory::Unsupported
            }
            RuntimeBridgeErrorKind::Internal => ErrorCategory::Internal,
        }
    }
}

impl core::fmt::Display for RuntimeBridgeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "runtime bridge error [{:?}]: {}",
            self.kind, self.message
        )
    }
}

impl std::error::Error for RuntimeBridgeError {}

pub type BridgeResult<T> = Result<T, RuntimeBridgeError>;
