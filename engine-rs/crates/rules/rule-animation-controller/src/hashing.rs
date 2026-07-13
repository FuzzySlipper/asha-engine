use super::{
    AnimationCatalog, AnimationControllerInputRecord, AnimationControllerState,
    AnimationGraphDefinition,
};

pub(super) fn controller_state_hash(state: &AnimationControllerState) -> String {
    let mut canonical = state.clone();
    canonical.entity = 0;
    canonical.state_hash.clear();
    let encoded = serde_json::to_vec(&canonical).expect("animation controller state serializes");
    stable_hash(&encoded)
}

pub(super) fn replay_hash(records: &[AnimationControllerInputRecord]) -> String {
    let encoded = serde_json::to_vec(records).expect("animation input records serialize");
    stable_hash(&encoded)
}

pub(super) fn canonical_catalog_hash(catalog: &AnimationCatalog) -> String {
    let mut canonical = catalog.clone();
    canonical
        .assets
        .sort_by(|left, right| left.asset_id.cmp(&right.asset_id));
    for asset in &mut canonical.assets {
        asset.clips.sort();
    }
    canonical
        .graphs
        .sort_by(|left, right| left.graph_id.cmp(&right.graph_id));
    for graph in &mut canonical.graphs {
        canonicalize_graph(graph);
    }
    let encoded = serde_json::to_vec(&canonical).expect("animation catalog serializes");
    stable_hash(&encoded)
}

pub(super) fn canonical_graph_hash(graph: &AnimationGraphDefinition) -> String {
    let mut canonical = graph.clone();
    canonicalize_graph(&mut canonical);
    let encoded = serde_json::to_vec(&canonical).expect("animation graph serializes");
    stable_hash(&encoded)
}

fn canonicalize_graph(graph: &mut AnimationGraphDefinition) {
    graph
        .parameters
        .sort_by(|left, right| left.parameter_id.cmp(&right.parameter_id));
    graph
        .states
        .sort_by(|left, right| left.state_id.cmp(&right.state_id));
    graph
        .transitions
        .sort_by(|left, right| left.transition_id.cmp(&right.transition_id));
    for transition in &mut graph.transitions {
        transition.conditions.sort_by_key(|condition| {
            serde_json::to_string(condition).expect("animation condition serializes")
        });
    }
}

pub(super) fn stable_id(namespace: &str, value: &str) -> u64 {
    let mut bytes = Vec::with_capacity(namespace.len() + value.len() + 1);
    bytes.extend_from_slice(namespace.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(value.as_bytes());
    fnv1a64(&bytes)
}

pub(super) fn stable_hash(bytes: &[u8]) -> String {
    format!("fnv1a64:{:016x}", fnv1a64(bytes))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
