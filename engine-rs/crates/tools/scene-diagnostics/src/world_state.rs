//! Runtime world-state snapshot **round-trip equivalence** (post-launchable-02,
//! Den task #2484).
//!
//! The voxel round-trip ([`crate::roundtrip`]) and the bundle round-trip
//! ([`crate::equivalence`]) prove voxel authority and the scene/entity bootstrap
//! identity survive save → reload. This module proves the third facet: a
//! **runtime-diverged entity store** — runtime-created entities, diverged
//! transforms, capability presence/absence, relations (transform attachment,
//! containment, source ancestry), source traces, and asset references — survives
//! the world-state-snapshot codec round-trip with authority-equivalent state.
//!
//! A lost or drifted facet is reported as a **classified**
//! [`protocol_diagnostics`] report whose `reference` names the mismatch category
//! (so a failure routes to its owning lane: state/rules for entity authority,
//! persistence for the codec), not just an opaque assertion. A snapshot that fails
//! to decode at all is a `Fatal` [`DiagnosticCode::CorruptBundleArtifact`]; a clean
//! decode that drifts is an `Error` [`DiagnosticCode::RoundTripMismatch`]; a
//! missing snapshot where one was expected is a [`DiagnosticCode::MissingSourceTrace`].

use std::collections::BTreeMap;

use core_entity::store::{EntityRecord, EntitySnapshot};
use core_entity::{decode_snapshot, encode_snapshot, EntityHash, EntityStore};
use protocol_diagnostics::{
    DiagnosticCode, DiagnosticReport, DiagnosticReportSet, DiagnosticSourceRef, RemedyAction,
    SuggestedRemedy,
};

/// A deterministic comparison of a runtime entity store before save (B) and after
/// reload (C).
#[derive(Debug, Clone)]
pub struct WorldStateEquivalenceReport {
    pub entities_b: usize,
    pub entities_c: usize,
    pub entity_hash_b: EntityHash,
    pub entity_hash_c: EntityHash,
    /// Classified mismatch diagnostics (empty == authority-equivalent).
    pub diagnostics: DiagnosticReportSet,
}

impl WorldStateEquivalenceReport {
    /// `true` if every compared facet matched.
    pub fn is_equivalent(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// A deterministic, greppable summary (golden-friendly).
    pub fn to_report_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "worldStateEquivalence equivalent={} entitiesB={} entitiesC={}\n",
            self.is_equivalent(),
            self.entities_b,
            self.entities_c,
        ));
        out.push_str(&format!(
            "entityHashB={:016x} entityHashC={:016x}\n",
            self.entity_hash_b.0, self.entity_hash_c.0
        ));
        out.push_str(&crate::text::report_set_to_text(&self.diagnostics));
        out
    }
}

/// Run the real world-state-snapshot codec round-trip for `store`: encode it to
/// the canonical artifact form, decode it back, restore a store, and compare the
/// pre-save and post-reload snapshots facet by facet. Nothing here hard-codes the
/// reloaded result — it drives `core_entity`'s real `encode`/`decode`/`from_snapshot`.
pub fn world_state_round_trip(store: &EntityStore) -> WorldStateEquivalenceReport {
    let snapshot_b = store.snapshot();
    let text = encode_snapshot(&snapshot_b);

    let mut diagnostics = DiagnosticReportSet::new();
    let (snapshot_c, entity_hash_c) = match decode_snapshot(&text) {
        Ok(decoded) => {
            let restored = EntityStore::from_snapshot(decoded);
            (Some(restored.snapshot()), restored.hash())
        }
        Err(e) => {
            diagnostics.push(decode_failure(&format!("{e}")));
            (None, EntityHash(0))
        }
    };

    let entities_c = snapshot_c.as_ref().map(|s| s.records.len()).unwrap_or(0);
    if let Some(snapshot_c) = &snapshot_c {
        compare_into(&snapshot_b, snapshot_c, &mut diagnostics);
    }

    WorldStateEquivalenceReport {
        entities_b: snapshot_b.records.len(),
        entities_c,
        entity_hash_b: store.hash(),
        entity_hash_c,
        diagnostics,
    }
}

/// Compare two runtime entity snapshots facet by facet, returning classified
/// mismatch diagnostics (empty == authority-equivalent). `b` is the pre-save
/// authority, `c` the post-reload authority.
pub fn compare_entity_snapshots(b: &EntitySnapshot, c: &EntitySnapshot) -> DiagnosticReportSet {
    let mut set = DiagnosticReportSet::new();
    compare_into(b, c, &mut set);
    set
}

fn compare_into(b: &EntitySnapshot, c: &EntitySnapshot, set: &mut DiagnosticReportSet) {
    let by_id_b: BTreeMap<u64, &EntityRecord> =
        b.records.iter().map(|r| (r.core.id.raw(), r)).collect();
    let by_id_c: BTreeMap<u64, &EntityRecord> =
        c.records.iter().map(|r| (r.core.id.raw(), r)).collect();

    for (id, rec_b) in &by_id_b {
        match by_id_c.get(id) {
            None => set.push(mismatch(
                &format!("entity-presence:{id}"),
                format!("entity {id} present before save but missing after reload"),
            )),
            Some(rec_c) => compare_records(*id, rec_b, rec_c, set),
        }
    }
    for id in by_id_c.keys() {
        if !by_id_b.contains_key(id) {
            set.push(mismatch(
                &format!("entity-presence:{id}"),
                format!("entity {id} appeared after reload but was not present before save"),
            ));
        }
    }
}

fn compare_records(id: u64, b: &EntityRecord, c: &EntityRecord, set: &mut DiagnosticReportSet) {
    // Lifecycle + source provenance (the source trace).
    if b.core.lifecycle != c.core.lifecycle || b.core.source != c.core.source {
        set.push(mismatch(
            &format!("entity-source-trace:{id}"),
            format!(
                "entity {id} lifecycle/source changed across reload: {}/{} -> {}/{}",
                b.core.lifecycle.label(),
                b.core.source.label(),
                c.core.lifecycle.label(),
                c.core.source.label(),
            ),
        ));
    }
    if b.core.labels != c.core.labels {
        set.push(mismatch(
            &format!("entity-labels:{id}"),
            format!("entity {id} label set changed across reload"),
        ));
    }
    // Transform: presence and value (runtime divergence must survive exactly).
    if b.transform != c.transform {
        set.push(mismatch(
            &format!("entity-transform:{id}"),
            format!("entity {id} transform capability presence/value changed across reload"),
        ));
    }
    // Other capabilities: presence/absence must be preserved without forcing any
    // of render/collision/bounds/controller/asset to be mandatory.
    if b.bounds != c.bounds
        || b.render != c.render
        || b.collision != c.collision
        || b.controller != c.controller
    {
        set.push(mismatch(
            &format!("entity-capability:{id}"),
            format!("entity {id} capability presence/value changed across reload"),
        ));
    }
    if b.asset_binding != c.asset_binding {
        set.push(mismatch(
            &format!("entity-asset:{id}"),
            format!("entity {id} asset binding changed across reload"),
        ));
    }
    // Relations: containment, transform attachment, and source ancestry each keep
    // their distinct taxonomy across reload.
    if b.containment != c.containment
        || b.transform_parent != c.transform_parent
        || b.derived_from != c.derived_from
    {
        set.push(mismatch(
            &format!("entity-relation:{id}"),
            format!(
                "entity {id} relation taxonomy drifted across reload \
                 (containment/transformParent/derivedFrom)"
            ),
        ));
    }
}

/// A missing world-state snapshot where one was expected (e.g. a manifest claims
/// runtime divergence but no artifact is present). Classified so it routes to the
/// persistence lane.
pub fn missing_world_state_snapshot(path: &str) -> DiagnosticReport {
    DiagnosticReport::new(
        DiagnosticCode::MissingSourceTrace,
        "world-state-snapshot",
        DiagnosticSourceRef::empty(),
        format!("expected world-state snapshot artifact `{path}` is missing"),
    )
    .with_remedy(SuggestedRemedy::new(
        RemedyAction::RestoreArtifact,
        "restore the world-state snapshot artifact or re-save the diverged runtime authority",
    ))
}

fn mismatch(reference: &str, message: String) -> DiagnosticReport {
    DiagnosticReport::new(
        DiagnosticCode::RoundTripMismatch,
        reference,
        DiagnosticSourceRef::empty(),
        message,
    )
    .with_remedy(SuggestedRemedy::new(
        RemedyAction::Inspect,
        "world-state save/reload did not preserve authority-equivalent runtime state; \
         inspect the named facet",
    ))
}

fn decode_failure(detail: &str) -> DiagnosticReport {
    DiagnosticReport::new(
        DiagnosticCode::CorruptBundleArtifact,
        "world-state-snapshot",
        DiagnosticSourceRef::empty(),
        format!("world-state snapshot failed to decode on round-trip: {detail}"),
    )
    .with_remedy(SuggestedRemedy::new(
        RemedyAction::RestoreArtifact,
        "the world-state snapshot artifact does not decode; restore from a known-good save",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_assets::{AssetId, AssetReference, AssetVersionReq};
    use core_entity::{
        ControllerCapability, EntityLifecycleCommand, EntitySource, EntityTransform,
        RelationCommand,
    };
    use core_ids::{EntityId, ProcessId, SceneNodeId, SubjectId, TagId};
    use core_math::Vec3;

    /// A mixed world exercising every fixture vocabulary class #2484 requires.
    fn mixed_world() -> EntityStore {
        let mut store = EntityStore::new();
        let mk = |store: &mut EntityStore, id, source, labels: Vec<u32>| {
            store
                .apply(EntityLifecycleCommand::Create {
                    id: EntityId::new(id),
                    source,
                    labels: labels.into_iter().map(|t| TagId::new(t as u64)).collect(),
                })
                .unwrap();
        };

        // 1. scene-sourced spatial rendered entity (transform diverged).
        mk(
            &mut store,
            1,
            EntitySource::SceneBootstrap {
                node: SceneNodeId::new(10),
            },
            vec![3],
        );
        store.attach_transform(
            EntityId::new(1),
            EntityTransform::at(Vec3::new(5.0, 0.0, 1.0)),
        );
        store.attach_render_projection(EntityId::new(1), true);

        // 2. runtime-created spatial non-rendered collider.
        mk(
            &mut store,
            2,
            EntitySource::RuntimeCreated {
                by: Some(ProcessId::new(7)),
            },
            vec![],
        );
        store.attach_transform(EntityId::new(2), EntityTransform::IDENTITY);
        store.attach_collision(EntityId::new(2), true);

        // 3. non-spatial logical entity (no transform), controller association.
        mk(
            &mut store,
            3,
            EntitySource::PolicyProposed {
                by: SubjectId::new(4),
            },
            vec![1],
        );
        store.attach_controller(
            EntityId::new(3),
            ControllerCapability::Subject(SubjectId::new(4)),
        );

        // 4. contained member (containment relation into the collider).
        mk(
            &mut store,
            4,
            EntitySource::RuntimeCreated { by: None },
            vec![],
        );
        store
            .apply_relation(RelationCommand::SetContainment {
                member: EntityId::new(4),
                container: EntityId::new(2),
            })
            .unwrap();

        // 5. transform attachment + asset binding + source-ancestry trace.
        mk(
            &mut store,
            5,
            EntitySource::Imported {
                asset: AssetReference::new(
                    AssetId::parse("mesh/crate").unwrap(),
                    AssetVersionReq::Exact(2),
                    None,
                ),
            },
            vec![],
        );
        store.attach_transform(
            EntityId::new(5),
            EntityTransform::at(Vec3::new(0.0, 2.0, 0.0)),
        );
        store.attach_asset_binding(
            EntityId::new(5),
            AssetReference::new(
                AssetId::parse("mesh/crate").unwrap(),
                AssetVersionReq::Any,
                None,
            ),
        );
        store
            .apply_relation(RelationCommand::AttachTransformParent {
                child: EntityId::new(5),
                parent: EntityId::new(1),
            })
            .unwrap();
        store
            .apply_relation(RelationCommand::SetDerivedFrom {
                derived: EntityId::new(5),
                origin: EntityId::new(4),
            })
            .unwrap();

        store
    }

    #[test]
    fn mixed_world_round_trips_equivalently() {
        let report = world_state_round_trip(&mixed_world());
        assert!(report.is_equivalent(), "{}", report.to_report_text());
        assert_eq!(report.entity_hash_b, report.entity_hash_c);
        assert_eq!(report.entities_b, 5);
    }

    #[test]
    fn a_dropped_transform_is_classified() {
        let b = mixed_world().snapshot();
        let mut c_store = mixed_world();
        // Simulate a lossy reload: strip a runtime-diverged transform.
        c_store.attach_transform(EntityId::new(1), EntityTransform::IDENTITY);
        let c = c_store.snapshot();
        let set = compare_entity_snapshots(&b, &c);
        assert!(!set.is_empty());
        let text = crate::text::report_set_to_text(&set);
        assert!(text.contains("ref=entity-transform:1"), "{text}");
        assert!(text.contains("roundTripMismatch"), "{text}");
    }

    #[test]
    fn a_missing_entity_is_classified() {
        let b = mixed_world().snapshot();
        let c = EntitySnapshot {
            records: b.records.iter().take(4).cloned().collect(),
        };
        let set = compare_entity_snapshots(&b, &c);
        let text = crate::text::report_set_to_text(&set);
        assert!(text.contains("ref=entity-presence:5"), "{text}");
    }

    #[test]
    fn corrupt_snapshot_decode_is_fatal() {
        // Build a report from a snapshot that cannot decode by hand-routing through
        // the same classifier the round-trip uses.
        let report = decode_failure("schema version 99 is newer than supported 1");
        assert_eq!(report.code, DiagnosticCode::CorruptBundleArtifact);
    }
}
