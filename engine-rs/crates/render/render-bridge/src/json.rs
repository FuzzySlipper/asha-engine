//! Std-only JSON encoder for render-diff fixtures.
//!
//! The workspace has zero external dependencies, so this hand-writes the exact
//! JSON shape of the generated `render.ts` contract. It exists so a Rust test
//! can emit a fixture that the TypeScript `@asha/runtime-bridge` render decoder
//! consumes — the shared, inspectable artifact at the render boundary.
//!
//! Each diff op is written on one line (compact) inside an indented frame array,
//! which keeps the committed fixture small and reviewable.

use protocol_render::{
    AnimatedMeshAsset, AnimatedMeshInstanceDescriptor, AnimatedMeshPlaybackCommand, BillboardMode,
    Geometry, Material, MeshAttributeName, MeshCollisionPolicy, MeshMaterialSlot,
    MeshPayloadDescriptor, MeshPayloadSource, RenderDiff, RenderFrameDiff, RenderMetadata,
    RenderNode, SpriteAttachment, SpriteDepthPolicy, SpriteInstanceDescriptor, SpriteShading,
    SpriteSizeMode, StaticMeshAsset, StaticMeshInstanceDescriptor, Transform,
};

/// Encode a single frame as a pretty `{ "ops": [ … ] }` object — the shape the
/// `renderer-three` golden harness applies directly (one frame per fixture).
pub fn encode_frame(frame: &RenderFrameDiff) -> String {
    let mut out = String::from("{\n  \"ops\": [\n");
    for (oi, op) in frame.ops.iter().enumerate() {
        out.push_str("    ");
        encode_diff(&mut out, op);
        if oi + 1 < frame.ops.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

/// Encode a sequence of frames as a pretty JSON array of frame objects.
pub fn encode_sequence(frames: &[RenderFrameDiff]) -> String {
    let mut out = String::from("[\n");
    for (fi, frame) in frames.iter().enumerate() {
        out.push_str("  { \"ops\": [\n");
        for (oi, op) in frame.ops.iter().enumerate() {
            out.push_str("    ");
            encode_diff(&mut out, op);
            if oi + 1 < frame.ops.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ] }");
        if fi + 1 < frames.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("]\n");
    out
}

fn encode_diff(out: &mut String, diff: &RenderDiff) {
    match diff {
        RenderDiff::Create {
            handle,
            parent,
            node,
        } => {
            out.push_str(&format!(
                "{{ \"op\": \"create\", \"handle\": {}, \"parent\": ",
                handle.raw()
            ));
            match parent {
                Some(p) => out.push_str(&p.raw().to_string()),
                None => out.push_str("null"),
            }
            out.push_str(", \"node\": ");
            encode_node(out, node);
            out.push_str(" }");
        }
        RenderDiff::Update {
            handle,
            transform,
            material,
            visible,
            metadata,
        } => {
            out.push_str(&format!(
                "{{ \"op\": \"update\", \"handle\": {}, \"transform\": ",
                handle.raw()
            ));
            encode_opt(out, transform.as_ref(), encode_transform);
            out.push_str(", \"material\": ");
            encode_opt(out, material.as_ref(), encode_material);
            out.push_str(", \"visible\": ");
            match visible {
                Some(v) => out.push_str(if *v { "true" } else { "false" }),
                None => out.push_str("null"),
            }
            out.push_str(", \"metadata\": ");
            encode_opt(out, metadata.as_ref(), encode_metadata);
            out.push_str(" }");
        }
        RenderDiff::Destroy { handle } => {
            out.push_str(&format!(
                "{{ \"op\": \"destroy\", \"handle\": {} }}",
                handle.raw()
            ));
        }
        RenderDiff::ReplaceMeshPayload { handle, payload } => {
            out.push_str(&format!(
                "{{ \"op\": \"replaceMeshPayload\", \"handle\": {}, \"payload\": ",
                handle.raw()
            ));
            encode_mesh_payload(out, payload);
            out.push_str(" }");
        }
        RenderDiff::DefineMaterial { material } => {
            out.push_str("{ \"op\": \"defineMaterial\", \"material\": ");
            encode_material_descriptor(out, material);
            out.push_str(" }");
        }
        RenderDiff::DefineTexture { texture } => {
            out.push_str("{ \"op\": \"defineTexture\", \"texture\": ");
            encode_texture_descriptor(out, texture);
            out.push_str(" }");
        }
        RenderDiff::DefineSpriteAtlas { atlas } => {
            out.push_str("{ \"op\": \"defineSpriteAtlas\", \"atlas\": ");
            encode_sprite_atlas(out, atlas);
            out.push_str(" }");
        }
        RenderDiff::DefineStaticMesh { asset } => {
            out.push_str("{ \"op\": \"defineStaticMesh\", \"asset\": ");
            encode_static_mesh_asset(out, asset);
            out.push_str(" }");
        }
        RenderDiff::DefineAnimatedMesh { asset } => {
            out.push_str("{ \"op\": \"defineAnimatedMesh\", \"asset\": ");
            encode_animated_mesh_asset(out, asset);
            out.push_str(" }");
        }
        RenderDiff::CreateStaticMeshInstance {
            handle,
            parent,
            instance,
        } => {
            out.push_str(&format!(
                "{{ \"op\": \"createStaticMeshInstance\", \"handle\": {}, \"parent\": ",
                handle.raw()
            ));
            match parent {
                Some(p) => out.push_str(&p.raw().to_string()),
                None => out.push_str("null"),
            }
            out.push_str(", \"instance\": ");
            encode_static_mesh_instance(out, instance);
            out.push_str(" }");
        }
        RenderDiff::CreateAnimatedMeshInstance {
            handle,
            parent,
            instance,
        } => {
            out.push_str(&format!(
                "{{ \"op\": \"createAnimatedMeshInstance\", \"handle\": {}, \"parent\": ",
                handle.raw()
            ));
            match parent {
                Some(p) => out.push_str(&p.raw().to_string()),
                None => out.push_str("null"),
            }
            out.push_str(", \"instance\": ");
            encode_animated_mesh_instance(out, instance);
            out.push_str(" }");
        }
        RenderDiff::SetAnimatedMeshPlayback { handle, playback } => {
            out.push_str(&format!(
                "{{ \"op\": \"setAnimatedMeshPlayback\", \"handle\": {}, \"playback\": ",
                handle.raw()
            ));
            encode_animated_mesh_playback(out, playback);
            out.push_str(" }");
        }
        RenderDiff::CreateSprite {
            handle,
            parent,
            sprite,
        } => {
            out.push_str(&format!(
                "{{ \"op\": \"createSprite\", \"handle\": {}, \"parent\": ",
                handle.raw()
            ));
            match parent {
                Some(p) => out.push_str(&p.raw().to_string()),
                None => out.push_str("null"),
            }
            out.push_str(", \"sprite\": ");
            encode_sprite(out, sprite);
            out.push_str(" }");
        }
        RenderDiff::UpdateSprite {
            handle,
            frame,
            tint,
            render_order,
            visible,
        } => {
            out.push_str(&format!(
                "{{ \"op\": \"updateSprite\", \"handle\": {}, \"frame\": ",
                handle.raw()
            ));
            match frame {
                Some(f) => out.push_str(&f.to_string()),
                None => out.push_str("null"),
            }
            out.push_str(", \"tint\": ");
            match tint {
                Some(t) => encode_f32_array(out, t),
                None => out.push_str("null"),
            }
            out.push_str(", \"renderOrder\": ");
            match render_order {
                Some(o) => out.push_str(&o.to_string()),
                None => out.push_str("null"),
            }
            out.push_str(", \"visible\": ");
            match visible {
                Some(v) => out.push_str(if *v { "true" } else { "false" }),
                None => out.push_str("null"),
            }
            out.push_str(" }");
        }
    }
}

fn encode_material_slots(out: &mut String, slots: &[MeshMaterialSlot]) {
    out.push('[');
    for (i, s) in slots.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "{{ \"slot\": {}, \"material\": {} }}",
            s.slot,
            encode_json_string(&s.material)
        ));
    }
    out.push(']');
}

fn encode_collision_policy(out: &mut String, policy: &MeshCollisionPolicy) {
    match policy {
        MeshCollisionPolicy::VisualOnly => out.push_str("{ \"kind\": \"visualOnly\" }"),
        MeshCollisionPolicy::Proxy { proxy_asset } => out.push_str(&format!(
            "{{ \"kind\": \"proxy\", \"proxyAsset\": {} }}",
            encode_json_string(proxy_asset)
        )),
        MeshCollisionPolicy::AabbFallback => out.push_str("{ \"kind\": \"aabbFallback\" }"),
    }
}

fn encode_texture_descriptor(out: &mut String, t: &protocol_render::TextureDescriptor) {
    out.push_str(&format!(
        "{{ \"id\": {}, \"width\": {}, \"height\": {}, \"filter\": \"{}\", \"wrap\": \"{}\", \"contentHash\": ",
        encode_json_string(&t.id),
        t.width,
        t.height,
        t.filter.label(),
        t.wrap.label()
    ));
    match &t.content_hash {
        Some(h) => out.push_str(&encode_json_string(h)),
        None => out.push_str("null"),
    }
    out.push_str(&format!(", \"version\": {} }}", t.version));
}

fn encode_sprite_atlas(out: &mut String, a: &protocol_render::SpriteAtlasDescriptor) {
    out.push_str(&format!(
        "{{ \"id\": {}, \"texture\": {}, \"frames\": [",
        encode_json_string(&a.id),
        encode_json_string(&a.texture)
    ));
    for (i, rect) in a.frames.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("{{ \"frame\": {}, \"uvMin\": ", rect.frame));
        encode_f32_array(out, &rect.uv_min);
        out.push_str(", \"uvMax\": ");
        encode_f32_array(out, &rect.uv_max);
        out.push_str(" }");
    }
    out.push_str("] }");
}

fn encode_material_descriptor(out: &mut String, m: &protocol_render::RenderMaterialDescriptor) {
    out.push_str(&format!(
        "{{ \"id\": {}, \"color\": ",
        encode_json_string(&m.id)
    ));
    encode_f32_array(out, &m.color);
    out.push_str(", \"texture\": ");
    match &m.texture {
        Some(t) => out.push_str(&encode_json_string(t)),
        None => out.push_str("null"),
    }
    out.push_str(&format!(
        ", \"roughness\": {}, \"emissive\": {}, \"uvStrategy\": \"{}\" }}",
        m.roughness,
        m.emissive,
        m.uv_strategy.label()
    ));
}

fn encode_static_mesh_asset(out: &mut String, asset: &StaticMeshAsset) {
    out.push_str(&format!(
        "{{ \"asset\": {}, \"payload\": ",
        encode_json_string(&asset.asset)
    ));
    encode_mesh_payload(out, &asset.payload);
    out.push_str(", \"materialSlots\": ");
    encode_material_slots(out, &asset.material_slots);
    out.push_str(", \"collision\": ");
    encode_collision_policy(out, &asset.collision);
    out.push_str(" }");
}

fn encode_static_mesh_instance(out: &mut String, instance: &StaticMeshInstanceDescriptor) {
    out.push_str(&format!(
        "{{ \"asset\": {}, \"transform\": ",
        encode_json_string(&instance.asset)
    ));
    encode_transform(out, &instance.transform);
    out.push_str(", \"materialOverrides\": ");
    encode_material_slots(out, &instance.material_overrides);
    out.push_str(", \"metadata\": ");
    encode_metadata(out, &instance.metadata);
    out.push_str(" }");
}

fn encode_animated_mesh_asset(out: &mut String, asset: &AnimatedMeshAsset) {
    out.push_str(&format!(
        "{{ \"asset\": {}, \"runtimeFormat\": \"{}\", \"contentHash\": ",
        encode_json_string(&asset.asset),
        asset.runtime_format.label()
    ));
    match &asset.content_hash {
        Some(hash) => out.push_str(&encode_json_string(hash)),
        None => out.push_str("null"),
    }
    out.push_str(", \"clips\": [");
    for (i, clip) in asset.clips.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "{{ \"id\": {}, \"name\": ",
            encode_json_string(&clip.id)
        ));
        match &clip.name {
            Some(name) => out.push_str(&encode_json_string(name)),
            None => out.push_str("null"),
        }
        out.push_str(", \"durationSeconds\": ");
        match clip.duration_seconds {
            Some(duration) => out.push_str(&format!("{duration}")),
            None => out.push_str("null"),
        }
        out.push_str(" }");
    }
    out.push_str("], \"defaultClip\": ");
    match &asset.default_clip {
        Some(clip) => out.push_str(&encode_json_string(clip)),
        None => out.push_str("null"),
    }
    out.push_str(", \"materialSlots\": ");
    encode_material_slots(out, &asset.material_slots);
    out.push_str(", \"bounds\": { \"min\": ");
    encode_f32_array(out, &asset.bounds.min);
    out.push_str(", \"max\": ");
    encode_f32_array(out, &asset.bounds.max);
    out.push_str(" } }");
}

fn encode_animated_mesh_instance(out: &mut String, instance: &AnimatedMeshInstanceDescriptor) {
    out.push_str(&format!(
        "{{ \"asset\": {}, \"transform\": ",
        encode_json_string(&instance.asset)
    ));
    encode_transform(out, &instance.transform);
    out.push_str(", \"materialOverrides\": ");
    encode_material_slots(out, &instance.material_overrides);
    out.push_str(", \"playback\": ");
    match &instance.playback {
        Some(playback) => encode_animated_mesh_playback(out, playback),
        None => out.push_str("null"),
    }
    out.push_str(", \"metadata\": ");
    encode_metadata(out, &instance.metadata);
    out.push_str(" }");
}

fn encode_animated_mesh_playback(out: &mut String, playback: &AnimatedMeshPlaybackCommand) {
    match playback {
        AnimatedMeshPlaybackCommand::Play {
            clip,
            r#loop,
            speed,
            weight,
            restart,
            fade_seconds,
        } => {
            out.push_str(&format!(
                "{{ \"action\": \"play\", \"clip\": {}, \"loop\": \"{}\", \"speed\": {}, \"weight\": {}, \"restart\": {}, \"fadeSeconds\": ",
                encode_json_string(clip),
                r#loop.label(),
                speed,
                weight,
                restart
            ));
            match fade_seconds {
                Some(fade) => out.push_str(&format!("{fade}")),
                None => out.push_str("null"),
            }
            out.push_str(" }");
        }
        AnimatedMeshPlaybackCommand::Stop { fade_seconds } => {
            out.push_str("{ \"action\": \"stop\", \"fadeSeconds\": ");
            match fade_seconds {
                Some(fade) => out.push_str(&format!("{fade}")),
                None => out.push_str("null"),
            }
            out.push_str(" }");
        }
        AnimatedMeshPlaybackCommand::Pause => out.push_str("{ \"action\": \"pause\" }"),
        AnimatedMeshPlaybackCommand::Resume => out.push_str("{ \"action\": \"resume\" }"),
    }
}

fn encode_sprite_attachment(out: &mut String, attachment: &SpriteAttachment) {
    out.push_str("{ \"sourceEntity\": ");
    match attachment.source_entity {
        Some(id) => out.push_str(&id.raw().to_string()),
        None => out.push_str("null"),
    }
    out.push_str(", \"sourceSceneNode\": ");
    match attachment.source_scene_node {
        Some(n) => out.push_str(&n.to_string()),
        None => out.push_str("null"),
    }
    out.push_str(", \"attachmentPoint\": ");
    match &attachment.attachment_point {
        Some(p) => out.push_str(&encode_json_string(p)),
        None => out.push_str("null"),
    }
    out.push_str(" }");
}

fn encode_sprite(out: &mut String, sprite: &SpriteInstanceDescriptor) {
    out.push_str(&format!(
        "{{ \"asset\": {}, \"frame\": {}, \"pivot\": ",
        encode_json_string(&sprite.asset),
        sprite.frame
    ));
    encode_f32_array(out, &sprite.pivot);
    out.push_str(", \"size\": ");
    encode_f32_array(out, &sprite.size);
    out.push_str(&format!(
        ", \"sizeMode\": \"{}\", \"billboard\": \"{}\", \"tint\": ",
        match sprite.size_mode {
            SpriteSizeMode::World => "world",
            SpriteSizeMode::Pixel => "pixel",
        },
        match sprite.billboard {
            BillboardMode::None => "none",
            BillboardMode::Spherical => "spherical",
            BillboardMode::Cylindrical => "cylindrical",
        }
    ));
    encode_f32_array(out, &sprite.tint);
    out.push_str(&format!(
        ", \"renderOrder\": {}, \"depth\": \"{}\", \"shading\": \"{}\", \"transform\": ",
        sprite.render_order,
        match sprite.depth {
            SpriteDepthPolicy::Default => "default",
            SpriteDepthPolicy::DepthTestOff => "depthTestOff",
            SpriteDepthPolicy::DepthWriteOff => "depthWriteOff",
        },
        match sprite.shading {
            SpriteShading::Unlit => "unlit",
            SpriteShading::Lit => "lit",
            SpriteShading::Shadowed => "shadowed",
            SpriteShading::Custom => "custom",
        }
    ));
    encode_transform(out, &sprite.transform);
    out.push_str(", \"attachment\": ");
    encode_sprite_attachment(out, &sprite.attachment);
    out.push_str(", \"metadata\": ");
    encode_metadata(out, &sprite.metadata);
    out.push_str(" }");
}

fn attr_name(name: MeshAttributeName) -> &'static str {
    match name {
        MeshAttributeName::Position => "position",
        MeshAttributeName::Normal => "normal",
        MeshAttributeName::Uv => "uv",
        MeshAttributeName::Color => "color",
    }
}

fn encode_mesh_payload(out: &mut String, payload: &MeshPayloadDescriptor) {
    let layout = &payload.layout;
    out.push_str(&format!(
        "{{ \"layout\": {{ \"vertexCount\": {}, \"indexCount\": {}, \"indexWidth\": \"u32\", \"attributes\": [",
        layout.vertex_count, layout.index_count
    ));
    for (i, a) in layout.attributes.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "{{ \"name\": \"{}\", \"components\": {}, \"kind\": \"f32\" }}",
            attr_name(a.name),
            a.components
        ));
    }
    out.push_str("] }, \"groups\": [");
    for (i, g) in payload.groups.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "{{ \"materialSlot\": {}, \"start\": {}, \"count\": {} }}",
            g.material_slot, g.start, g.count
        ));
    }
    out.push_str("], \"bounds\": { \"min\": ");
    encode_f32_array(out, &payload.bounds.min);
    out.push_str(", \"max\": ");
    encode_f32_array(out, &payload.bounds.max);
    out.push_str(" }, \"source\": ");
    match &payload.source {
        MeshPayloadSource::Inline {
            positions,
            normals,
            indices,
        } => {
            out.push_str("{ \"kind\": \"inline\", \"positions\": ");
            encode_f32_array(out, positions);
            out.push_str(", \"normals\": ");
            encode_f32_array(out, normals);
            out.push_str(", \"indices\": ");
            encode_u32_array(out, indices);
            out.push_str(" }");
        }
        MeshPayloadSource::Handle {
            buffer,
            positions_byte_offset,
            normals_byte_offset,
            indices_byte_offset,
        } => {
            out.push_str(&format!(
                "{{ \"kind\": \"handle\", \"buffer\": {buffer}, \"positionsByteOffset\": {positions_byte_offset}, \"normalsByteOffset\": {normals_byte_offset}, \"indicesByteOffset\": {indices_byte_offset} }}"
            ));
        }
    }
    out.push_str(&format!(
        ", \"provenance\": \"{}\" }}",
        payload.provenance.label()
    ));
}

fn encode_u32_array(out: &mut String, values: &[u32]) {
    out.push('[');
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&v.to_string());
    }
    out.push(']');
}

fn encode_node(out: &mut String, node: &RenderNode) {
    out.push_str("{ \"geometry\": ");
    encode_geometry(out, &node.geometry);
    out.push_str(", \"material\": ");
    encode_material(out, &node.material);
    out.push_str(", \"transform\": ");
    encode_transform(out, &node.transform);
    out.push_str(&format!(
        ", \"visible\": {}, \"layer\": \"{}\", \"metadata\": ",
        node.visible,
        match node.layer {
            protocol_render::RenderLayer::Scene => "scene",
            protocol_render::RenderLayer::Debug => "debug",
        }
    ));
    encode_metadata(out, &node.metadata);
    out.push_str(" }");
}

fn encode_geometry(out: &mut String, geometry: &Geometry) {
    match geometry {
        Geometry::Cube => out.push_str("{ \"shape\": \"cube\" }"),
        Geometry::Sphere => out.push_str("{ \"shape\": \"sphere\" }"),
        Geometry::Quad => out.push_str("{ \"shape\": \"quad\" }"),
        Geometry::Point => out.push_str("{ \"shape\": \"point\" }"),
        Geometry::Line { a, b } => {
            out.push_str("{ \"shape\": \"line\", \"a\": ");
            encode_f32_array(out, a);
            out.push_str(", \"b\": ");
            encode_f32_array(out, b);
            out.push_str(" }");
        }
    }
}

fn encode_material(out: &mut String, material: &Material) {
    out.push_str("{ \"color\": ");
    encode_f32_array(out, &material.color);
    out.push_str(&format!(", \"wireframe\": {} }}", material.wireframe));
}

fn encode_transform(out: &mut String, t: &Transform) {
    out.push_str("{ \"translation\": ");
    encode_f32_array(out, &t.translation);
    out.push_str(", \"rotation\": ");
    encode_f32_array(out, &t.rotation);
    out.push_str(", \"scale\": ");
    encode_f32_array(out, &t.scale);
    out.push_str(" }");
}

fn encode_metadata(out: &mut String, metadata: &RenderMetadata) {
    out.push_str("{ \"source\": ");
    match metadata.source {
        Some(id) => out.push_str(&id.raw().to_string()),
        None => out.push_str("null"),
    }
    out.push_str(", \"tags\": [");
    for (i, tag) in metadata.tags.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&tag.raw().to_string());
    }
    out.push_str("], \"label\": ");
    match &metadata.label {
        Some(label) => out.push_str(&encode_json_string(label)),
        None => out.push_str("null"),
    }
    out.push_str(" }");
}

fn encode_opt<T>(out: &mut String, value: Option<&T>, encode: fn(&mut String, &T)) {
    match value {
        Some(v) => encode(out, v),
        None => out.push_str("null"),
    }
}

fn encode_f32_array(out: &mut String, values: &[f32]) {
    out.push('[');
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("{v}"));
    }
    out.push(']');
}

fn encode_json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}
