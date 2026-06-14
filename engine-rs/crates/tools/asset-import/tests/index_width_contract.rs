//! Contract-drift guard for the static-mesh artifact (#2429).
//!
//! The emitted `indexWidth` must be *derived* from the `protocol_render`
//! `MeshIndexWidth` enum, not a hardcoded literal that could drift from the
//! contract. This fails if the importer re-hardcodes the field or the enum's wire
//! label changes without the artifact following.

use asset_import::{artifacts, fixtures, import_text};
use protocol_render::MeshIndexWidth;

#[test]
fn static_mesh_index_width_is_derived_from_the_protocol_enum() {
    let outcome = import_text(fixtures::VALID_QUAD, "fixtures/quad.mesh.json");
    let assets = outcome.assets.expect("fixture imports cleanly");
    let json = artifacts::render_static_mesh(&assets.static_mesh);

    // The artifact's value must equal the enum's own label for the payload's width.
    let expected = assets.static_mesh.payload.layout.index_width.label();
    assert_eq!(expected, MeshIndexWidth::U32.label());
    assert!(
        json.contains(&format!("\"indexWidth\": \"{expected}\"")),
        "emitted indexWidth must equal the protocol enum label; got:\n{json}"
    );

    // Exactly one indexWidth field — guards against a re-introduced stale literal
    // emitted alongside the derived one.
    assert_eq!(
        json.matches("\"indexWidth\"").count(),
        1,
        "static-mesh artifact must emit indexWidth exactly once"
    );
}
