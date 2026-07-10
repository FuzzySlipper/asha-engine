use crate::{MeshProvenance, RenderHandle};
use core_ids::EntityId;

/// A renderer-side sprite pick hit traced to authority identity. The renderer
/// reports this trace; authority revalidates it before acting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpritePickHit {
    pub handle: RenderHandle,
    pub source_entity: Option<EntityId>,
    pub source_scene_node: Option<u64>,
    /// The sprite asset id that was hit.
    pub asset: String,
    pub attachment_point: Option<String>,
}

/// A renderer-side mesh pick hint mapping a render handle to the authority
/// source that produced its mesh. Authority revalidates the hint before acting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshPickHit {
    pub handle: RenderHandle,
    pub provenance: MeshProvenance,
}
