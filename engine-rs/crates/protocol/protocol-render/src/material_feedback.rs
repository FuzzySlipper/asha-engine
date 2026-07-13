//! Typed visual material descriptors and retained per-instance feedback values.

/// How a material samples colour across geometry. Mirrors
/// `core_catalog::material::UvStrategy` — the *visual* projection only; no
/// collision/authority field ever appears here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialUvStrategy {
    /// A single flat colour, no texture sampling.
    #[default]
    Flat,
    /// Sample the bound texture with planar UVs.
    Planar,
    /// Sample an atlas sub-rectangle (sprite sheets).
    Atlas,
}

impl MaterialUvStrategy {
    /// Stable border label.
    pub fn label(self) -> &'static str {
        match self {
            MaterialUvStrategy::Flat => "flat",
            MaterialUvStrategy::Planar => "planar",
            MaterialUvStrategy::Atlas => "atlas",
        }
    }
}

/// The renderer-facing projection of a catalog material, keyed by its asset id so
/// the renderer can resolve a static-mesh slot or sprite ref without a placeholder.
/// This is visual projection only; collision/authority fields cannot be named.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderMaterialDescriptor {
    /// Descriptor schema. Version 2 adds explicit texture-tint and emission
    /// colour/intensity fields. Decoders migrate unversioned v1 payloads.
    pub schema_version: u32,
    /// Catalog material asset id, e.g. `material/concrete-wet`.
    pub id: String,
    /// Linear RGBA, each component in `0.0..=1.0`.
    pub color: [f32; 4],
    /// Optional bound texture asset id (a `texture/...` ref).
    pub texture: Option<String>,
    pub roughness: f32,
    /// Linear RGBA multiplier applied to the base colour/texture. White is neutral.
    pub texture_tint: [f32; 4],
    /// Linear RGB emission colour. Black with zero intensity is neutral.
    pub emission_color: [f32; 3],
    pub emission_intensity: f32,
    pub uv_strategy: MaterialUvStrategy,
}

/// A complete visual-parameter block for one material slot on one retained
/// instance. The operation replaces the block; `None` resets descriptor defaults.
/// These values are presentation only and never become gameplay authority.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialInstanceParameters {
    /// Linear RGBA multiplier applied on top of the descriptor's texture tint.
    pub texture_tint: [f32; 4],
    pub emission_color: [f32; 3],
    pub emission_intensity: f32,
}
