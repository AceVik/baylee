//! Whether a newer compiled catalog still holds, for every rule a model was
//! trained on, exactly the tree the model learned.
//!
//! Node ids and tag ids are positions in one catalog's traversal, so two
//! catalogs are compared by structure: a model node matches the catalog node
//! with the same tag *name*, the same bytes and matching children in order.
//! Both forests are interned exactly, so a structurally equal subtree is one
//! node in each, and the match is a single bottom-up pass.
use super::{Forest, Node, STRUCTURE_VERSION};
use std::collections::BTreeMap;

/// How a compiled catalog relates to the catalog a model was exported with.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Superset {
    /// Root keys of the model's catalog.
    pub model_roots: usize,
    /// Root keys of the compiled catalog.
    pub catalog_roots: usize,
    /// Keys only the compiled catalog has: rules the model never saw.
    pub new_roots: Vec<String>,
    /// Keys of the model's catalog whose tree is not the compiled one.
    pub changed: Vec<String>,
    /// Keys of the model's catalog the compiled catalog lacks.
    pub missing: Vec<String>,
    /// Rule operators (tag names) only the compiled catalog uses. A
    /// refreshed export cannot embed them without new training.
    pub new_tags: Vec<String>,
}

impl Superset {
    /// Every rule the model knows has its learned tree in the compiled
    /// catalog, which may add rules but changes and removes none.
    #[must_use]
    pub fn holds(&self) -> bool {
        self.changed.is_empty() && self.missing.is_empty()
    }
}

/// Compares the catalog a model was exported with (`model`, as read from its
/// `card_structure.json`) with a compiled catalog (`catalog`).
///
/// A model key holds when the compiled catalog has the same key and its tree
/// equals the model's node for node: tag names, bytes and ordered children.
/// The DSL source digest is deliberately not compared: a source edit that
/// leaves every known tree as it was changes nothing the model reads.
#[must_use]
pub fn superset(model: &Forest, catalog: &Forest) -> Superset {
    let tags: BTreeMap<&str, u32> = catalog
        .tags
        .iter()
        .enumerate()
        .map(|(id, name)| (name.as_str(), id as u32))
        .collect();
    let nodes: BTreeMap<&Node, u32> = catalog
        .nodes
        .iter()
        .enumerate()
        .map(|(id, node)| (node, id as u32))
        .collect();
    // A model node's equal in the catalog, if the catalog has one. Children
    // precede their parents in an interned forest, so one pass suffices; a
    // malformed order finds no equal and reports the key changed.
    let mut equal: Vec<Option<u32>> = Vec::with_capacity(model.nodes.len());
    for node in &model.nodes {
        let tag = model
            .tags
            .get(node.tag as usize)
            .and_then(|name| tags.get(name.as_str()));
        let children = node
            .children
            .iter()
            .map(|&child| equal.get(child as usize).copied().flatten())
            .collect::<Option<Vec<u32>>>();
        let found = match (tag, children) {
            (Some(&tag), Some(children)) => nodes
                .get(&Node {
                    tag,
                    bytes: node.bytes.clone(),
                    children,
                })
                .copied(),
            _ => None,
        };
        equal.push(found);
    }
    let mut report = Superset {
        model_roots: model.roots.len(),
        catalog_roots: catalog.roots.len(),
        ..Superset::default()
    };
    for (key, &node) in &model.roots {
        match catalog.roots.get(key) {
            None => report.missing.push(key.clone()),
            Some(&compiled) if equal.get(node as usize).copied().flatten() != Some(compiled) => {
                report.changed.push(key.clone());
            }
            Some(_) => {}
        }
    }
    report.new_roots = catalog
        .roots
        .keys()
        .filter(|key| !model.roots.contains_key(*key))
        .cloned()
        .collect();
    let known: std::collections::BTreeSet<&str> = model.tags.iter().map(String::as_str).collect();
    report.new_tags = catalog
        .tags
        .iter()
        .filter(|tag| !known.contains(tag.as_str()))
        .cloned()
        .collect();
    report
}

/// The public rule a catalog key belongs to: one card (`c/`, `i/`, `a/`,
/// `alt/` keys of its index), one token (`t/`, `ta/`) or one reserved
/// ability (`q/`). A card is known, new or changed as a whole.
#[must_use]
pub fn owner(key: &str) -> String {
    let mut parts = key.split('/');
    let kind = parts.next().unwrap_or_default();
    let id = parts.next().unwrap_or_default();
    match kind {
        "c" | "i" | "a" | "alt" => format!("card/{id}"),
        "t" | "ta" => format!("token/{id}"),
        _ => format!("{kind}/{id}"),
    }
}

/// The owners ([`owner`]) a superset withdraws: every card, token or
/// reserved ability one of whose keys changed, went missing, or was added
/// beside keys the model knows. Each of their keys is left out, as the keys
/// of a rule the model never saw are.
#[must_use]
pub fn withdrawn(report: &Superset, model: &Forest) -> Vec<String> {
    let mut owners: std::collections::BTreeSet<String> = report
        .changed
        .iter()
        .chain(&report.missing)
        .map(|key| owner(key))
        .collect();
    let known: std::collections::BTreeSet<String> =
        model.roots.keys().map(|key| owner(key)).collect();
    owners.extend(
        report
            .new_roots
            .iter()
            .map(|key| owner(key))
            .filter(|owner| known.contains(owner)),
    );
    owners.into_iter().collect()
}

/// The structure contract both catalogs must share before any comparison.
#[must_use]
pub fn same_contract(model: &Forest, catalog: &Forest) -> bool {
    model.version == STRUCTURE_VERSION
        && catalog.version == STRUCTURE_VERSION
        && model.tags.first().map(String::as_str) == Some("Unknown")
        && catalog.tags.first().map(String::as_str) == Some("Unknown")
}
