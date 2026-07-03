use protocol_render::{MeshProvenance, RenderDiff};
use render_bridge::VoxelChunkProjector;
use svc_levelgen::{generate_tunnel, TunnelGeneratorConfig};

#[test]
fn generated_tunnel_projects_to_voxel_render_frame_golden() {
    let tunnel =
        generate_tunnel(TunnelGeneratorConfig::tiny_enclosed(17)).expect("generate tunnel");
    let coords: Vec<_> = tunnel
        .render_chunks
        .iter()
        .map(|chunk| chunk.chunk)
        .collect();

    let mut projector = VoxelChunkProjector::new();
    let frame = projector.project_coords(&tunnel.world, &coords);

    assert_eq!(
        describe_frame(&frame, &tunnel),
        include_str!("../../../../../harness/goldens/render-diffs/generated-tunnel.snapshot")
    );
}

fn describe_frame(
    frame: &protocol_render::RenderFrameDiff,
    tunnel: &svc_levelgen::GeneratedTunnel,
) -> String {
    let mut creates = 0usize;
    let mut replacements = 0usize;
    let mut vertex_count = 0u32;
    let mut index_count = 0u32;
    let mut material_slots: Vec<u16> = Vec::new();
    let mut labels: Vec<String> = Vec::new();

    for op in &frame.ops {
        match op {
            RenderDiff::Create { node, .. } => {
                creates += 1;
                if let Some(label) = &node.metadata.label {
                    labels.push(label.clone());
                }
            }
            RenderDiff::ReplaceMeshPayload { payload, .. } => {
                replacements += 1;
                assert_eq!(payload.provenance, MeshProvenance::VoxelChunk);
                vertex_count += payload.layout.vertex_count;
                index_count += payload.layout.index_count;
                for group in &payload.groups {
                    material_slots.push(group.material_slot);
                }
            }
            _ => {}
        }
    }
    material_slots.sort_unstable();
    material_slots.dedup();

    let mut out = String::new();
    out.push_str("generated-tunnel-render 1\n");
    out.push_str(&format!("output_hash={:016x}\n", tunnel.record.output_hash));
    out.push_str(&format!("ops={}\n", frame.ops.len()));
    out.push_str(&format!("creates={creates}\n"));
    out.push_str(&format!("replace_mesh_payloads={replacements}\n"));
    out.push_str(&format!("vertices={vertex_count}\n"));
    out.push_str(&format!("indices={index_count}\n"));
    out.push_str(&format!("material_slots={material_slots:?}\n"));
    out.push_str(&format!("labels={labels:?}\n"));
    out
}
