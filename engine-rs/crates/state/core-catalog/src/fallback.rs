//! Fallback policy by asset **kind + context of use** (scene-capability-03,
//! subtask #2324).
//!
//! Fallback behaviour is global registry policy, not a per-reference override
//! scattered through scenes: the *same* missing material falls back to a debug
//! material in a cosmetic context but **fails closed** in a collision-critical
//! one. Keeping this here lets the policy evolve without editing every scene node.

use core_assets::AssetKind;

/// The context an asset reference is used in. Drives fallback, independent of the
/// asset kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetContext {
    /// A debug/HUD overlay (e.g. an editor gizmo sprite).
    DebugOverlay,
    /// A purely cosmetic surface (decorative material/texture/sprite).
    CosmeticSurface,
    /// Collision/structural-critical geometry or material — authority depends on it.
    CollisionCritical,
    /// Non-critical background decoration that may simply be omitted.
    BackgroundDecoration,
}

/// The concrete debug placeholder a fallback resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackVisual {
    /// A magenta square (missing overlay/sprite).
    MagentaSquare,
    /// A neutral grey debug material (missing cosmetic material/texture).
    GreyMaterial,
}

/// What to do when a referenced asset is missing in a given context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackOutcome {
    /// Substitute a debug placeholder and continue.
    UseFallback {
        reason: &'static str,
        visual: FallbackVisual,
    },
    /// Refuse to load — the asset is authority-critical.
    FailClosed { reason: &'static str },
    /// Silently omit the entity/visual (non-critical decoration).
    Skip { reason: &'static str },
}

/// Resolve the fallback for a missing asset of `kind` used in `context`.
///
/// Context dominates: anything collision-critical fails closed regardless of kind,
/// while the same kind in a cosmetic/overlay context gets a debug placeholder.
pub fn fallback_for(kind: AssetKind, context: AssetContext) -> FallbackOutcome {
    match context {
        AssetContext::CollisionCritical => FallbackOutcome::FailClosed {
            reason: "collision-critical asset missing; refusing to load incomplete authority",
        },
        AssetContext::BackgroundDecoration => FallbackOutcome::Skip {
            reason: "non-critical background decoration omitted",
        },
        AssetContext::DebugOverlay => FallbackOutcome::UseFallback {
            reason: "debug overlay missing; using magenta placeholder",
            visual: FallbackVisual::MagentaSquare,
        },
        AssetContext::CosmeticSurface => match kind {
            // Sprites read better as a magenta square; surfaces as grey material.
            AssetKind::Sprite | AssetKind::SpriteSheet => FallbackOutcome::UseFallback {
                reason: "cosmetic sprite missing; using magenta placeholder",
                visual: FallbackVisual::MagentaSquare,
            },
            _ => FallbackOutcome::UseFallback {
                reason: "cosmetic surface missing; using grey debug material",
                visual: FallbackVisual::GreyMaterial,
            },
        },
    }
}
