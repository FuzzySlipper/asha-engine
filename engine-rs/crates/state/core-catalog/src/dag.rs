//! The asset dependency graph: a Rust-validated DAG with cycle-path diagnostics
//! (scene-capability-03, subtask #2322).
//!
//! Edges run asset → dependency (mesh → material, sprite-sheet → texture, scene →
//! assets, material → texture). Only edges whose target is present in the catalog
//! are graph edges; a dependency on a *missing* asset is a separate validation
//! error, not a cycle. Cycle detection reports the full cycle path so a diagnostic
//! reads like `mesh/a -> material/b -> texture/c -> mesh/a`.

use std::collections::BTreeMap;

use core_assets::AssetId;

use crate::entry::Catalog;

/// The dependency graph derived from a catalog, keyed by canonical id string.
pub struct DependencyGraph<'a> {
    /// Node id → its present dependency ids (sorted, deterministic).
    edges: BTreeMap<&'a str, Vec<&'a str>>,
    /// id string → the owning [`AssetId`], for path reconstruction.
    ids: BTreeMap<&'a str, &'a AssetId>,
}

impl<'a> DependencyGraph<'a> {
    /// Build the graph from a catalog. Edges to ids absent from the catalog are
    /// skipped (missing deps are reported by validation, not here).
    pub fn build(catalog: &'a Catalog) -> DependencyGraph<'a> {
        let mut ids: BTreeMap<&str, &AssetId> = BTreeMap::new();
        for e in &catalog.entries {
            ids.insert(e.id.as_str(), &e.id);
        }
        let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for e in &catalog.entries {
            let mut deps: Vec<&str> = e
                .dependencies
                .iter()
                .map(|d| d.id().as_str())
                .filter(|d| ids.contains_key(d))
                .collect();
            deps.sort_unstable();
            deps.dedup();
            edges.entry(e.id.as_str()).or_default().extend(deps);
        }
        DependencyGraph { edges, ids }
    }

    /// The first dependency cycle in deterministic order, as an id path that ends
    /// where it begins (`a -> b -> a`), or `None` if the graph is a DAG.
    pub fn detect_cycle(&self) -> Option<Vec<AssetId>> {
        #[derive(Clone, Copy, PartialEq)]
        enum Color {
            White,
            Grey,
            Black,
        }
        let mut color: BTreeMap<&str, Color> =
            self.ids.keys().map(|&k| (k, Color::White)).collect();

        // Iterative DFS so deep catalogs cannot overflow the stack. `stack` holds
        // (node, next-child-index); `path` mirrors the grey frontier.
        for &root in self.ids.keys() {
            if color[root] != Color::White {
                continue;
            }
            let mut stack: Vec<(&str, usize)> = vec![(root, 0)];
            let mut path: Vec<&str> = vec![root];
            *color.get_mut(root).unwrap() = Color::Grey;

            while let Some(&(node, idx)) = stack.last() {
                let children = self.edges.get(node).map(Vec::as_slice).unwrap_or(&[]);
                if idx < children.len() {
                    stack.last_mut().unwrap().1 += 1;
                    let child = children[idx];
                    match color[child] {
                        Color::White => {
                            *color.get_mut(child).unwrap() = Color::Grey;
                            stack.push((child, 0));
                            path.push(child);
                        }
                        Color::Grey => {
                            // Back edge → cycle. Slice path from the first time we
                            // saw `child`, then close the loop.
                            let pos = path.iter().position(|&p| p == child).unwrap();
                            let mut cycle: Vec<AssetId> =
                                path[pos..].iter().map(|s| (*self.ids[s]).clone()).collect();
                            cycle.push((*self.ids[child]).clone());
                            return Some(cycle);
                        }
                        Color::Black => {}
                    }
                } else {
                    *color.get_mut(node).unwrap() = Color::Black;
                    stack.pop();
                    path.pop();
                }
            }
        }
        None
    }

    /// All assets that transitively depend on `target` (reverse reachability),
    /// sorted by id. Excludes `target` itself.
    pub fn dependents_of(&self, target: &AssetId) -> Vec<AssetId> {
        // Reverse adjacency.
        let mut rev: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (&from, tos) in &self.edges {
            for &to in tos {
                rev.entry(to).or_default().push(from);
            }
        }
        let mut out: Vec<&str> = Vec::new();
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let mut queue: Vec<&str> = vec![target.as_str()];
        while let Some(node) = queue.pop() {
            if let Some(parents) = rev.get(node) {
                for &p in parents {
                    if seen.insert(p) {
                        out.push(p);
                        queue.push(p);
                    }
                }
            }
        }
        out.sort_unstable();
        out.into_iter()
            .filter_map(|s| self.ids.get(s).map(|id| (*id).clone()))
            .collect()
    }
}
