//! Complete ordered typed card-rule trees, interned as a DAG without hashing
//! collisions or ability caps. Public card definitions only; no game state.
#![allow(missing_docs, clippy::cast_possible_truncation)]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const STRUCTURE_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Node {
    pub tag: u32,
    pub bytes: Vec<u8>,
    pub children: Vec<u32>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Forest {
    pub version: u32,
    pub dsl_source_sha256: String,
    pub tags: Vec<String>,
    pub nodes: Vec<Node>,
    pub roots: BTreeMap<String, u32>,
    #[serde(skip)]
    intern: BTreeMap<Node, u32>,
    #[serde(skip)]
    tag_ids: BTreeMap<String, u32>,
    #[serde(skip)]
    depth: usize,
}

impl Forest {
    pub fn new() -> Self {
        let mut f = Self {
            version: STRUCTURE_VERSION,
            dsl_source_sha256: DSL_SOURCE_SHA256.into(),
            ..Self::default()
        };
        f.node("Unknown", &[], vec![]);
        f
    }
    fn enter(&mut self) {
        self.depth += 1;
        assert!(
            self.depth <= 256,
            "cyclic or excessive card DSL depth; no truncation permitted"
        );
    }
    fn leave(&mut self) {
        self.depth -= 1;
    }
    /// Intern one ordered node by exact equality.
    ///
    /// # Panics
    /// If the catalog exceeds two million distinct nodes. Nothing is truncated.
    pub fn node(&mut self, tag: &str, bytes: &[u8], children: Vec<u32>) -> u32 {
        let id = *self.tag_ids.entry(tag.into()).or_insert_with(|| {
            let id = self.tags.len() as u32;
            self.tags.push(tag.into());
            id
        });
        let node = Node {
            tag: id,
            bytes: bytes.to_vec(),
            children,
        };
        if let Some(&id) = self.intern.get(&node) {
            return id;
        }
        let id = self.nodes.len() as u32;
        assert!(
            id < 2_000_000,
            "card structure DAG overflow; no truncation permitted"
        );
        self.nodes.push(node.clone());
        self.intern.insert(node, id);
        id
    }
    fn field<T: TreeEncode + ?Sized>(&mut self, name: &str, value: &T) -> u32 {
        let child = value.tree(self);
        self.node(name, &[], vec![child])
    }
}

pub trait TreeEncode {
    fn tree(&self, b: &mut Forest) -> u32;
}
impl<T: TreeEncode + ?Sized> TreeEncode for &T {
    fn tree(&self, b: &mut Forest) -> u32 {
        (**self).tree(b)
    }
}
impl<T: TreeEncode> TreeEncode for [T] {
    fn tree(&self, b: &mut Forest) -> u32 {
        let children = self.iter().map(|x| x.tree(b)).collect();
        b.node("Sequence", &[], children)
    }
}
impl<T: TreeEncode, const N: usize> TreeEncode for [T; N] {
    fn tree(&self, b: &mut Forest) -> u32 {
        self.as_slice().tree(b)
    }
}
impl<T: TreeEncode> TreeEncode for Option<T> {
    fn tree(&self, b: &mut Forest) -> u32 {
        match self {
            None => b.node("None", &[], vec![]),
            Some(v) => {
                let child = v.tree(b);
                b.node("Some", &[], vec![child])
            }
        }
    }
}
impl TreeEncode for str {
    fn tree(&self, b: &mut Forest) -> u32 {
        b.node("UTF8", self.as_bytes(), vec![])
    }
}
impl TreeEncode for bool {
    fn tree(&self, b: &mut Forest) -> u32 {
        b.node("bool", &[u8::from(*self)], vec![])
    }
}
impl TreeEncode for std::num::NonZeroU32 {
    fn tree(&self, b: &mut Forest) -> u32 {
        b.node("NonZeroU32", &self.get().to_le_bytes(), vec![])
    }
}

macro_rules! integers { ($($t:ty),*) => { $(impl TreeEncode for $t { fn tree(&self, b: &mut Forest) -> u32 { b.node(stringify!($t), &self.to_le_bytes(), vec![]) } })* }; }
integers!(u8, u16, u32, u64, u128, i8, i16, i32, i64);
macro_rules! bits { ($($t:ty),*) => { $(impl TreeEncode for $t { fn tree(&self, b: &mut Forest) -> u32 { b.node(stringify!($t), &self.bits().to_le_bytes(), vec![]) } })* }; }
bits!(
    baylee_cards_dsl::KeywordSet,
    baylee_core::color::ColorSet,
    baylee_core::color::ColorPair,
    baylee_core::types::TypeSet,
    baylee_core::types::SupertypeSet
);
macro_rules! ids { ($($t:ty),*) => { $(impl TreeEncode for $t { fn tree(&self, b: &mut Forest) -> u32 { b.node(stringify!($t), &self.get().to_le_bytes(), vec![]) } })* }; }
ids!(
    baylee_core::ids::PlayerId,
    baylee_core::ids::CardIndex,
    baylee_core::ids::SubtypeId
);
impl TreeEncode for baylee_core::color::Color {
    fn tree(&self, b: &mut Forest) -> u32 {
        b.node("Color", &[*self as u8], vec![])
    }
}
impl TreeEncode for baylee_core::mana::ManaColor {
    fn tree(&self, b: &mut Forest) -> u32 {
        b.node("ManaColor", &[*self as u8], vec![])
    }
}
impl TreeEncode for baylee_core::mana::ManaCost {
    fn tree(&self, b: &mut Forest) -> u32 {
        self.symbols().collect::<Vec<_>>().as_slice().tree(b)
    }
}
impl TreeEncode for baylee_core::mana::Variable {
    fn tree(&self, b: &mut Forest) -> u32 {
        b.node("Variable", &[*self as u8], vec![])
    }
}
impl TreeEncode for baylee_core::mana::ManaSymbol {
    fn tree(&self, b: &mut Forest) -> u32 {
        use baylee_core::mana::ManaSymbol as S;
        let (tag, child) = match self {
            S::Generic(x) => ("Generic", Some(x.tree(b))),
            S::Colorless => ("Colorless", None),
            S::White => ("White", None),
            S::Blue => ("Blue", None),
            S::Black => ("Black", None),
            S::Red => ("Red", None),
            S::Green => ("Green", None),
            S::Hybrid(x) => ("Hybrid", Some(x.tree(b))),
            S::TwoOrColor(x) => ("TwoOrColor", Some(x.tree(b))),
            S::Phyrexian(x) => ("Phyrexian", Some(x.tree(b))),
            S::HybridPhyrexian(x) => ("HybridPhyrexian", Some(x.tree(b))),
            S::Snow => ("Snow", None),
            S::Variable(x) => ("Variable", Some(x.tree(b))),
            S::HalfGeneric => ("HalfGeneric", None),
            S::Infinite => ("Infinite", None),
        };
        b.node(
            &format!("ManaSymbol::{tag}"),
            &[],
            child.into_iter().collect(),
        )
    }
}

include!(concat!(env!("OUT_DIR"), "/card_tree.rs"));
mod catalog;
pub use catalog::catalog;
pub mod refs;
#[cfg(test)]
mod tests;

/// Canonical JSON digest, shared with `structure_data4.py`.
///
/// # Errors
/// If the catalog cannot be serialized.
///
/// # Panics
/// If formatting into an owned string unexpectedly fails.
pub fn digest(forest: &Forest) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    let mut out = String::new();
    for byte in Sha256::digest(serde_json::to_vec(&serde_json::to_value(forest)?)?) {
        write!(out, "{byte:02x}").expect("write to String");
    }
    Ok(out)
}

impl<A: TreeEncode, B: TreeEncode> TreeEncode for (A, B) {
    fn tree(&self, b: &mut Forest) -> u32 {
        let children = vec![self.0.tree(b), self.1.tree(b)];
        b.node("Tuple2", &[], children)
    }
}
impl TreeEncode for baylee_core::ids::ObjectId {
    fn tree(&self, b: &mut Forest) -> u32 {
        let children = vec![self.slot().tree(b), self.generation().tree(b)];
        b.node("ObjectId", &[], children)
    }
}
