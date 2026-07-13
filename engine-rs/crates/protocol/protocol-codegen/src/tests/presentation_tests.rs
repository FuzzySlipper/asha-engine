use super::*;

pub(super) fn extend_round_trip_coverage(coverage: &mut BTreeSet<String>) {
    coverage.extend([
        interface_coverage_key("presentation", "PresentationOriginRef"),
        interface_coverage_key("presentation", "PresentationOpMeta"),
        variant_coverage_key("presentation", "AudioEmitter", "global2d"),
        variant_coverage_key("presentation", "AudioEmitter", "world3d"),
        variant_coverage_key("presentation", "AudioEmitter", "entityAttached"),
        interface_coverage_key("presentation", "AudioClipRef"),
        interface_coverage_key("presentation", "AudioSourceDescriptor"),
        interface_coverage_key("presentation", "AudioSourcePatch"),
        variant_coverage_key("presentation", "AudioProjectionOp", "emit"),
        variant_coverage_key("presentation", "AudioProjectionOp", "create"),
        variant_coverage_key("presentation", "AudioProjectionOp", "update"),
        variant_coverage_key("presentation", "AudioProjectionOp", "destroy"),
        interface_coverage_key("presentation", "AudioProjectionDiagnostic"),
        interface_coverage_key("presentation", "AudioProjectionReadout"),
        variant_coverage_key("presentation", "PresentationOp", "audio"),
        variant_coverage_key("presentation", "BillboardAnchor", "world"),
        variant_coverage_key("presentation", "BillboardAnchor", "entityAttached"),
        interface_coverage_key("presentation", "BillboardTemplateArgument"),
        interface_coverage_key("presentation", "BillboardTextureRef"),
        variant_coverage_key("presentation", "BillboardContent", "text"),
        variant_coverage_key("presentation", "BillboardContent", "value"),
        variant_coverage_key("presentation", "BillboardContent", "icon"),
        variant_coverage_key("presentation", "BillboardFontRef", "system"),
        variant_coverage_key("presentation", "BillboardFontRef", "asset"),
        interface_coverage_key("presentation", "BillboardDescriptor"),
        interface_coverage_key("presentation", "BillboardPatch"),
        variant_coverage_key("presentation", "BillboardProjectionOp", "create"),
        variant_coverage_key("presentation", "BillboardProjectionOp", "update"),
        variant_coverage_key("presentation", "BillboardProjectionOp", "destroy"),
        interface_coverage_key("presentation", "BillboardProjectionDiagnostic"),
        interface_coverage_key("presentation", "BillboardProjectionReadout"),
        variant_coverage_key("presentation", "PresentationOp", "billboard"),
        variant_coverage_key("presentation", "ParticleAnchor", "world"),
        variant_coverage_key("presentation", "ParticleAnchor", "entityAttached"),
        interface_coverage_key("presentation", "ParticleSpriteRef"),
        interface_coverage_key("presentation", "ParticleScalarKey"),
        interface_coverage_key("presentation", "ParticleColorKey"),
        interface_coverage_key("presentation", "ParticleEmitterDescriptor"),
        interface_coverage_key("presentation", "ParticleEmitterPatch"),
        variant_coverage_key("presentation", "ParticleProjectionOp", "emit"),
        variant_coverage_key("presentation", "ParticleProjectionOp", "create"),
        variant_coverage_key("presentation", "ParticleProjectionOp", "update"),
        variant_coverage_key("presentation", "ParticleProjectionOp", "destroy"),
        interface_coverage_key("presentation", "ParticleProjectionDiagnostic"),
        interface_coverage_key("presentation", "ParticleProjectionReadout"),
        variant_coverage_key("presentation", "PresentationOp", "particle"),
        interface_coverage_key("presentation", "TelemetryOverlayDescriptor"),
        interface_coverage_key("presentation", "TelemetryOverlayPatch"),
        variant_coverage_key("presentation", "TelemetryOverlayProjectionOp", "create"),
        variant_coverage_key("presentation", "TelemetryOverlayProjectionOp", "update"),
        variant_coverage_key("presentation", "TelemetryOverlayProjectionOp", "destroy"),
        interface_coverage_key("presentation", "TelemetryOverlayDiagnostic"),
        interface_coverage_key("presentation", "TelemetryOverlayReadout"),
        variant_coverage_key("presentation", "PresentationOp", "telemetryOverlay"),
        interface_coverage_key("presentation", "AnimationResolvedMotion"),
        interface_coverage_key("presentation", "AnimationTransitionProjection"),
        variant_coverage_key("presentation", "AnimationTransitionFactMoment", "started"),
        variant_coverage_key("presentation", "AnimationTransitionFactMoment", "completed"),
        interface_coverage_key("presentation", "AnimationTransitionFactRef"),
        interface_coverage_key("presentation", "AnimationControllerProjectionState"),
        interface_coverage_key("presentation", "AnimationProjectionDescriptor"),
        variant_coverage_key("presentation", "AnimationProjectionOp", "create"),
        variant_coverage_key("presentation", "AnimationProjectionOp", "update"),
        variant_coverage_key("presentation", "AnimationProjectionOp", "destroy"),
        interface_coverage_key("presentation", "AnimationProjectionDiagnostic"),
        interface_coverage_key("presentation", "AnimationProjectionReadout"),
        variant_coverage_key("presentation", "PresentationOp", "animation"),
        interface_coverage_key("presentation", "PresentationFrameDiff"),
        interface_coverage_key("presentation", "RuntimeProjectionFrame"),
    ]);
}

#[test]
fn presentation_rust_serialization_matches_ir_shape() {
    use protocol_presentation::{
        AnimationControllerProjectionState, AnimationProjectionDescriptor,
        AnimationProjectionDiagnostic, AnimationProjectionDiagnosticCode,
        AnimationProjectionHandle, AnimationProjectionOp, AnimationProjectionReadout,
        AnimationResolvedMotion, AnimationTransitionFactMoment, AnimationTransitionFactRef,
        AnimationTransitionProjection, AudioBus, AudioClipRef, AudioEmitter, AudioHandle,
        AudioProjectionDiagnostic, AudioProjectionDiagnosticCode, AudioProjectionOp,
        AudioProjectionReadout, AudioSourceDescriptor, AudioSourcePatch, BillboardAnchor,
        BillboardContent, BillboardDescriptor, BillboardFontRef, BillboardHandle, BillboardLayer,
        BillboardPatch, BillboardProjectionDiagnostic, BillboardProjectionDiagnosticCode,
        BillboardProjectionOp, BillboardProjectionReadout, BillboardTemplateArgument,
        BillboardTextureRef, ParticleAnchor, ParticleColorKey, ParticleEmitterDescriptor,
        ParticleEmitterHandle, ParticleEmitterPatch, ParticleProjectionDiagnostic,
        ParticleProjectionDiagnosticCode, ParticleProjectionOp, ParticleProjectionReadout,
        ParticleScalarKey, ParticleSpriteRef, PresentationOp, PresentationOpMeta,
        PresentationOriginKind, PresentationOriginRef, ProjectionReplayScope,
        TelemetryOverlayDescriptor, TelemetryOverlayDiagnostic, TelemetryOverlayDiagnosticCode,
        TelemetryOverlayHandle, TelemetryOverlayPatch, TelemetryOverlayProjectionOp,
        TelemetryOverlayReadout, RUNTIME_PROJECTION_SCHEMA_VERSION,
    };

    let presentation = module("presentation");
    let origin = PresentationOriginRef {
        kind: PresentationOriginKind::OwnerFact,
        id: "combat.primary-fire.accepted:4".to_string(),
        authority_tick: 4,
        causation_id: Some("command:4".to_string()),
        correlation_id: Some("encounter:fixture".to_string()),
    };
    let meta = PresentationOpMeta {
        sequence: 0,
        origin: Some(origin.clone()),
    };
    let descriptor = AudioSourceDescriptor {
        clip: AudioClipRef {
            asset: "audio/fixture-pulse".to_string(),
            content_hash: "aabb".to_string(),
        },
        bus: AudioBus::Sfx,
        volume: 0.8,
        pitch: 1.0,
        looping: false,
        spatial_blend: 1.0,
        attenuation: 12.0,
        pan: 0.0,
        emitter: AudioEmitter::EntityAttached {
            entity: 9,
            offset: [0.0, 1.0, 0.0],
        },
    };
    let audio = AudioProjectionOp::Emit {
        signal_id: "shot:4".to_string(),
        descriptor: descriptor.clone(),
    };
    let op = PresentationOp::Audio {
        meta: meta.clone(),
        op: audio.clone(),
    };

    let serialized_origin = serde_json::to_value(&origin).unwrap();
    compare_object_to_interface(&presentation, "PresentationOriginRef", &serialized_origin)
        .unwrap();
    let serialized_meta = serde_json::to_value(&meta).unwrap();
    compare_object_to_interface(&presentation, "PresentationOpMeta", &serialized_meta).unwrap();
    let serialized_descriptor = serde_json::to_value(&descriptor).unwrap();
    compare_object_to_interface(
        &presentation,
        "AudioSourceDescriptor",
        &serialized_descriptor,
    )
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "AudioClipRef",
        &serialized_descriptor["clip"],
    )
    .unwrap();
    compare_object_to_variant(
        &presentation,
        "AudioEmitter",
        "entityAttached",
        &serialized_descriptor["emitter"],
    )
    .unwrap();

    for emitter in [
        AudioEmitter::Global2d,
        AudioEmitter::World3d {
            position: [1.0, 2.0, 3.0],
        },
    ] {
        let value = serde_json::to_value(&emitter).unwrap();
        let tag = value["kind"].as_str().unwrap();
        compare_object_to_variant(&presentation, "AudioEmitter", tag, &value).unwrap();
    }

    let serialized_audio = serde_json::to_value(&audio).unwrap();
    compare_object_to_variant(
        &presentation,
        "AudioProjectionOp",
        "emit",
        &serialized_audio,
    )
    .unwrap();
    let serialized_op = serde_json::to_value(&op).unwrap();
    compare_object_to_variant(&presentation, "PresentationOp", "audio", &serialized_op).unwrap();

    for variant in [
        AudioProjectionOp::Create {
            handle: AudioHandle::new(2),
            descriptor,
        },
        AudioProjectionOp::Update {
            handle: AudioHandle::new(2),
            patch: AudioSourcePatch {
                volume: Some(0.4),
                ..AudioSourcePatch::default()
            },
        },
        AudioProjectionOp::Destroy {
            handle: AudioHandle::new(2),
        },
    ] {
        let value = serde_json::to_value(&variant).unwrap();
        let tag = value["op"].as_str().unwrap();
        compare_object_to_variant(&presentation, "AudioProjectionOp", tag, &value).unwrap();
        if tag == "update" {
            compare_object_to_interface(&presentation, "AudioSourcePatch", &value["patch"])
                .unwrap();
        }
    }

    let diagnostic = AudioProjectionDiagnostic {
        code: AudioProjectionDiagnosticCode::UnavailableHost,
        sequence: 0,
        handle: None,
        message: "audio host unavailable".to_string(),
        origin: Some(origin.clone()),
    };
    let readout = AudioProjectionReadout {
        active_sources: 0,
        cached_clips: 1,
        emitted_signals: 1,
        diagnostics: vec![diagnostic],
    };
    let serialized_readout = serde_json::to_value(&readout).unwrap();
    compare_object_to_interface(&presentation, "AudioProjectionReadout", &serialized_readout)
        .unwrap();
    compare_object_to_interface(
        &presentation,
        "AudioProjectionDiagnostic",
        &serialized_readout["diagnostics"][0],
    )
    .unwrap();

    let billboard_descriptor = BillboardDescriptor {
        anchor: BillboardAnchor::EntityAttached {
            entity: 9,
            offset: [0.0, 1.8, 0.0],
        },
        content: BillboardContent::Text {
            localization_key: "actor.name".into(),
            fallback_text: "Target {number}".into(),
            arguments: vec![BillboardTemplateArgument {
                name: "number".into(),
                value: "9".into(),
            }],
        },
        font: BillboardFontRef::Asset {
            asset: "font/ui-sans".into(),
            content_hash: "aabb".into(),
            family: "Asha UI".into(),
        },
        height_pixels: 24.0,
        color: [1.0, 1.0, 1.0, 1.0],
        background: [0.0, 0.0, 0.0, 0.7],
        max_distance: 40.0,
        layer: BillboardLayer::Occluded,
        visible: true,
    };
    let serialized_billboard_descriptor = serde_json::to_value(&billboard_descriptor).unwrap();
    compare_object_to_interface(
        &presentation,
        "BillboardDescriptor",
        &serialized_billboard_descriptor,
    )
    .unwrap();
    compare_object_to_variant(
        &presentation,
        "BillboardAnchor",
        "entityAttached",
        &serialized_billboard_descriptor["anchor"],
    )
    .unwrap();
    compare_object_to_variant(
        &presentation,
        "BillboardContent",
        "text",
        &serialized_billboard_descriptor["content"],
    )
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "BillboardTemplateArgument",
        &serialized_billboard_descriptor["content"]["arguments"][0],
    )
    .unwrap();
    compare_object_to_variant(
        &presentation,
        "BillboardFontRef",
        "asset",
        &serialized_billboard_descriptor["font"],
    )
    .unwrap();

    for anchor in [BillboardAnchor::World {
        position: [1.0, 2.0, 3.0],
    }] {
        let value = serde_json::to_value(&anchor).unwrap();
        compare_object_to_variant(&presentation, "BillboardAnchor", "world", &value).unwrap();
    }
    for content in [
        BillboardContent::Value {
            label_key: "actor.health".into(),
            fallback_label: "Health".into(),
            value: "8/10".into(),
            unit_key: None,
            fallback_unit: None,
        },
        BillboardContent::Icon {
            texture: BillboardTextureRef {
                asset: "texture/alert".into(),
                content_hash: "ccdd".into(),
            },
            alt_key: "alert".into(),
            fallback_alt: "Alert".into(),
        },
    ] {
        let value = serde_json::to_value(&content).unwrap();
        let tag = value["kind"].as_str().unwrap();
        compare_object_to_variant(&presentation, "BillboardContent", tag, &value).unwrap();
        if tag == "icon" {
            compare_object_to_interface(&presentation, "BillboardTextureRef", &value["texture"])
                .unwrap();
        }
    }
    let system_font = serde_json::to_value(BillboardFontRef::System {
        family: "sans-serif".into(),
    })
    .unwrap();
    compare_object_to_variant(&presentation, "BillboardFontRef", "system", &system_font).unwrap();

    let billboard_create = BillboardProjectionOp::Create {
        handle: BillboardHandle::new(3),
        descriptor: billboard_descriptor,
    };
    let billboard_op = PresentationOp::Billboard {
        meta: PresentationOpMeta {
            sequence: 1,
            origin: None,
        },
        op: billboard_create.clone(),
    };
    let serialized_billboard_op = serde_json::to_value(&billboard_op).unwrap();
    compare_object_to_variant(
        &presentation,
        "PresentationOp",
        "billboard",
        &serialized_billboard_op,
    )
    .unwrap();
    for variant in [
        billboard_create,
        BillboardProjectionOp::Update {
            handle: BillboardHandle::new(3),
            patch: BillboardPatch {
                visible: Some(false),
                ..BillboardPatch::default()
            },
        },
        BillboardProjectionOp::Destroy {
            handle: BillboardHandle::new(3),
        },
    ] {
        let value = serde_json::to_value(&variant).unwrap();
        let tag = value["op"].as_str().unwrap();
        compare_object_to_variant(&presentation, "BillboardProjectionOp", tag, &value).unwrap();
        if tag == "update" {
            compare_object_to_interface(&presentation, "BillboardPatch", &value["patch"]).unwrap();
        }
    }
    let billboard_readout = BillboardProjectionReadout {
        active_billboards: 1,
        loaded_fonts: 1,
        loaded_icons: 0,
        culled_billboards: 0,
        diagnostics: vec![BillboardProjectionDiagnostic {
            code: BillboardProjectionDiagnosticCode::UnavailableHost,
            sequence: 1,
            handle: Some(BillboardHandle::new(3)),
            message: "billboard host unavailable".into(),
            origin: None,
        }],
    };
    let serialized_billboard_readout = serde_json::to_value(billboard_readout).unwrap();
    compare_object_to_interface(
        &presentation,
        "BillboardProjectionReadout",
        &serialized_billboard_readout,
    )
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "BillboardProjectionDiagnostic",
        &serialized_billboard_readout["diagnostics"][0],
    )
    .unwrap();

    let particle_descriptor = ParticleEmitterDescriptor {
        anchor: ParticleAnchor::EntityAttached {
            entity: 9,
            offset: [0.0, 1.0, 0.0],
        },
        sprite: ParticleSpriteRef {
            asset: "sprite-sheet/sparks".into(),
            content_hash: "eeff".into(),
            frame_count: 4,
        },
        rate_per_second: 12.0,
        burst_count: 8,
        lifetime_seconds: [0.2, 0.6],
        velocity_min: [-1.0, 1.0, -1.0],
        velocity_max: [1.0, 3.0, 1.0],
        acceleration: [0.0, -4.0, 0.0],
        size_curve: vec![
            ParticleScalarKey {
                age: 0.0,
                value: 0.25,
            },
            ParticleScalarKey {
                age: 1.0,
                value: 0.0,
            },
        ],
        color_curve: vec![
            ParticleColorKey {
                age: 0.0,
                color: [1.0, 0.8, 0.2, 1.0],
            },
            ParticleColorKey {
                age: 1.0,
                color: [1.0, 0.2, 0.0, 0.0],
            },
        ],
        flipbook_frames_per_second: 16.0,
        seed: 44,
        max_particles: 64,
        visible: true,
    };
    let serialized_particle_descriptor = serde_json::to_value(&particle_descriptor).unwrap();
    compare_object_to_interface(
        &presentation,
        "ParticleEmitterDescriptor",
        &serialized_particle_descriptor,
    )
    .unwrap();
    compare_object_to_variant(
        &presentation,
        "ParticleAnchor",
        "entityAttached",
        &serialized_particle_descriptor["anchor"],
    )
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "ParticleSpriteRef",
        &serialized_particle_descriptor["sprite"],
    )
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "ParticleScalarKey",
        &serialized_particle_descriptor["sizeCurve"][0],
    )
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "ParticleColorKey",
        &serialized_particle_descriptor["colorCurve"][0],
    )
    .unwrap();
    let world_anchor = serde_json::to_value(ParticleAnchor::World {
        position: [1.0, 2.0, 3.0],
    })
    .unwrap();
    compare_object_to_variant(&presentation, "ParticleAnchor", "world", &world_anchor).unwrap();

    let particle_emit = ParticleProjectionOp::Emit {
        signal_id: "impact:4".into(),
        descriptor: particle_descriptor.clone(),
    };
    let particle_op = PresentationOp::Particle {
        meta: PresentationOpMeta {
            sequence: 2,
            origin: None,
        },
        op: particle_emit.clone(),
    };
    let serialized_particle_op = serde_json::to_value(&particle_op).unwrap();
    compare_object_to_variant(
        &presentation,
        "PresentationOp",
        "particle",
        &serialized_particle_op,
    )
    .unwrap();
    for variant in [
        particle_emit,
        ParticleProjectionOp::Create {
            handle: ParticleEmitterHandle::new(4),
            descriptor: particle_descriptor,
        },
        ParticleProjectionOp::Update {
            handle: ParticleEmitterHandle::new(4),
            patch: ParticleEmitterPatch {
                visible: Some(false),
                ..ParticleEmitterPatch::default()
            },
        },
        ParticleProjectionOp::Destroy {
            handle: ParticleEmitterHandle::new(4),
        },
    ] {
        let value = serde_json::to_value(&variant).unwrap();
        let tag = value["op"].as_str().unwrap();
        compare_object_to_variant(&presentation, "ParticleProjectionOp", tag, &value).unwrap();
        if tag == "update" {
            compare_object_to_interface(&presentation, "ParticleEmitterPatch", &value["patch"])
                .unwrap();
        }
    }
    let particle_readout = ParticleProjectionReadout {
        active_emitters: 1,
        active_particles: 8,
        loaded_sprites: 1,
        emitted_bursts: 1,
        dropped_particles: 0,
        diagnostics: vec![ParticleProjectionDiagnostic {
            code: ParticleProjectionDiagnosticCode::UnavailableHost,
            sequence: 2,
            handle: Some(ParticleEmitterHandle::new(4)),
            message: "particle host unavailable".into(),
            origin: None,
        }],
    };
    let serialized_particle_readout = serde_json::to_value(particle_readout).unwrap();
    compare_object_to_interface(
        &presentation,
        "ParticleProjectionReadout",
        &serialized_particle_readout,
    )
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "ParticleProjectionDiagnostic",
        &serialized_particle_readout["diagnostics"][0],
    )
    .unwrap();

    let overlay_create = TelemetryOverlayProjectionOp::Create {
        handle: TelemetryOverlayHandle::new(5),
        descriptor: TelemetryOverlayDescriptor {
            title: "ASHA runtime".into(),
            corner: protocol_presentation::TelemetryOverlayCorner::TopRight,
            refresh_interval_ms: 250,
            max_frame_time_samples: 60,
            visible: true,
        },
    };
    let overlay_op = PresentationOp::TelemetryOverlay {
        meta: PresentationOpMeta {
            sequence: 3,
            origin: None,
        },
        op: overlay_create.clone(),
    };
    let serialized_overlay_op = serde_json::to_value(&overlay_op).unwrap();
    compare_object_to_variant(
        &presentation,
        "PresentationOp",
        "telemetryOverlay",
        &serialized_overlay_op,
    )
    .unwrap();
    for variant in [
        overlay_create,
        TelemetryOverlayProjectionOp::Update {
            handle: TelemetryOverlayHandle::new(5),
            patch: TelemetryOverlayPatch {
                visible: Some(false),
                ..TelemetryOverlayPatch::default()
            },
        },
        TelemetryOverlayProjectionOp::Destroy {
            handle: TelemetryOverlayHandle::new(5),
        },
    ] {
        let value = serde_json::to_value(&variant).unwrap();
        let tag = value["op"].as_str().unwrap();
        compare_object_to_variant(&presentation, "TelemetryOverlayProjectionOp", tag, &value)
            .unwrap();
        if tag == "create" {
            compare_object_to_interface(
                &presentation,
                "TelemetryOverlayDescriptor",
                &value["descriptor"],
            )
            .unwrap();
        } else if tag == "update" {
            compare_object_to_interface(&presentation, "TelemetryOverlayPatch", &value["patch"])
                .unwrap();
        }
    }
    let overlay_readout = serde_json::to_value(TelemetryOverlayReadout {
        active_overlays: 1,
        rendered_snapshots: 4,
        diagnostics: vec![TelemetryOverlayDiagnostic {
            code: TelemetryOverlayDiagnosticCode::UnavailableHost,
            sequence: 3,
            handle: Some(TelemetryOverlayHandle::new(5)),
            message: "overlay host unavailable".into(),
            origin: None,
        }],
    })
    .unwrap();
    compare_object_to_interface(&presentation, "TelemetryOverlayReadout", &overlay_readout)
        .unwrap();
    compare_object_to_interface(
        &presentation,
        "TelemetryOverlayDiagnostic",
        &overlay_readout["diagnostics"][0],
    )
    .unwrap();

    let motion = AnimationResolvedMotion {
        clip_a: "idle".into(),
        clip_b: Some("run".into()),
        blend_weight_milli: 400,
        speed_milli: 1_000,
    };
    let controller = AnimationControllerProjectionState {
        graph_id: "player".into(),
        graph_version: 1,
        graph_hash: "fnv1a64:graph".into(),
        state_id: "locomotion".into(),
        revision: 2,
        state_hash: "fnv1a64:state".into(),
        motion: motion.clone(),
        transition: Some(AnimationTransitionProjection {
            transition_id: "idle.move".into(),
            from_state_id: "idle".into(),
            to_state_id: "locomotion".into(),
            elapsed_ticks: 1,
            duration_ticks: 2,
            target_motion: motion,
        }),
        timing_fact: Some(Box::new(AnimationTransitionFactRef {
            fact_id: "combat.primary-fire.accepted:9:animation:7:ready.primary_fire:started".into(),
            source_fact_id: "combat.primary-fire.accepted:9".into(),
            authority_tick: 9,
            controller_input_sequence: 3,
            controller_tick: 1,
            causation_id: "combat.primary-fire:9".into(),
            correlation_id: "fps.session:1".into(),
            transition_id: "ready.primary_fire".into(),
            from_state_id: "ready".into(),
            to_state_id: "primary_fire".into(),
            moment: AnimationTransitionFactMoment::Started,
            duration_ticks: 4,
            fact_hash: "fnv1a64:fact".into(),
        })),
    };
    let animation_create = AnimationProjectionOp::Create {
        handle: AnimationProjectionHandle::new(6),
        descriptor: AnimationProjectionDescriptor {
            target: protocol_render::RenderHandle(9),
            asset: "mesh-animation/character".into(),
            tick_duration_millis: 50,
            controller: controller.clone(),
        },
    };
    let animation_op = PresentationOp::Animation {
        meta: PresentationOpMeta {
            sequence: 4,
            origin: Some(origin),
        },
        op: animation_create.clone(),
    };
    let serialized_animation_op = serde_json::to_value(&animation_op).unwrap();
    compare_object_to_variant(
        &presentation,
        "PresentationOp",
        "animation",
        &serialized_animation_op,
    )
    .unwrap();
    for variant in [
        animation_create,
        AnimationProjectionOp::Update {
            handle: AnimationProjectionHandle::new(6),
            controller: controller.clone(),
        },
        AnimationProjectionOp::Destroy {
            handle: AnimationProjectionHandle::new(6),
        },
    ] {
        let value = serde_json::to_value(&variant).unwrap();
        let tag = value["op"].as_str().unwrap();
        compare_object_to_variant(&presentation, "AnimationProjectionOp", tag, &value).unwrap();
        if tag == "create" {
            compare_object_to_interface(
                &presentation,
                "AnimationProjectionDescriptor",
                &value["descriptor"],
            )
            .unwrap();
        }
    }
    let serialized_controller = serde_json::to_value(&controller).unwrap();
    compare_object_to_interface(
        &presentation,
        "AnimationControllerProjectionState",
        &serialized_controller,
    )
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "AnimationResolvedMotion",
        &serialized_controller["motion"],
    )
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "AnimationTransitionProjection",
        &serialized_controller["transition"],
    )
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "AnimationTransitionFactRef",
        &serialized_controller["timingFact"],
    )
    .unwrap();
    let animation_readout = serde_json::to_value(AnimationProjectionReadout {
        active_controllers: 1,
        sampled_frames: 2,
        compatibility_fallbacks: 0,
        diagnostics: vec![AnimationProjectionDiagnostic {
            code: AnimationProjectionDiagnosticCode::UnavailableHost,
            sequence: 4,
            handle: Some(AnimationProjectionHandle::new(6)),
            target: Some(protocol_render::RenderHandle(9)),
            message: "animation host unavailable".into(),
            origin: None,
        }],
    })
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "AnimationProjectionReadout",
        &animation_readout,
    )
    .unwrap();
    compare_object_to_interface(
        &presentation,
        "AnimationProjectionDiagnostic",
        &animation_readout["diagnostics"][0],
    )
    .unwrap();

    let frame = json!({
        "schemaVersion": RUNTIME_PROJECTION_SCHEMA_VERSION,
        "authorityTick": 4,
        "scene": { "ops": [] },
        "presentation": {
            "replayScope": serde_json::to_value(
                ProjectionReplayScope::ExcludedFromReplayTruth
            ).unwrap(),
            "ops": [
                serialized_op,
                serialized_billboard_op,
                serialized_particle_op,
                serialized_overlay_op,
                serialized_animation_op
            ],
        },
    });
    compare_object_to_interface(&presentation, "RuntimeProjectionFrame", &frame).unwrap();
    compare_object_to_interface(
        &presentation,
        "PresentationFrameDiff",
        &frame["presentation"],
    )
    .unwrap();
}
