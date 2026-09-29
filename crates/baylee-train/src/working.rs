//! Which cards work: the owner's rule for what the trained AI may learn from.
//!
//! A card works when two things hold:
//!
//! - its definition says `Coverage::Implemented`, codegen's (or an author's)
//!   claim that every clause of its text was read, and
//! - a test in the engine's `card_tests/` or `combo_tests/` plays it,
//!   naming it by oracle id as `card_index("<oracle id>")`.
//!
//! The first alone is a claim; the second is somebody having played the card
//! and checked what it did. A net trained on a card that only claims to work
//! learns the engine's bug as a rule of the game, so a game that dealt any
//! other card is not training data.
//!
//! The tests are read as text, from the checkout the binary was built in: the
//! rule is about which tests exist, not about which of them ran.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use baylee_core::ids::CardIndex;
use sha2::{Digest, Sha256};

/// The directories whose tests count, relative to the repository root.
pub const TEST_DIRS: [&str; 2] = [
    "crates/baylee-engine/src/engine/card_tests",
    "crates/baylee-engine/src/engine/combo_tests",
];

/// The checkout this crate was built in.
#[must_use]
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Why a card is not in the working set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The pool has no definition for it.
    NotInPool,
    /// Its definition is not `Coverage::Implemented`.
    NotImplemented,
    /// No test in [`TEST_DIRS`] plays it.
    Untested,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotInPool => "not in the pool",
            Self::NotImplemented => "not implemented",
            Self::Untested => "implemented, no card or combo test plays it",
        })
    }
}

/// The working set, and the counts it was cut from.
#[derive(Clone, Debug)]
pub struct Working {
    /// Card definitions in the pool.
    pub pool: usize,
    /// Of those, `Coverage::Implemented`.
    pub implemented: usize,
    /// Pool cards some test plays, implemented or not.
    pub tested: BTreeSet<CardIndex>,
    /// Oracle ids a test names that the pool has no card for. A test of a
    /// card that is not in this build cannot vouch for anything.
    pub unknown: Vec<String>,
    /// The cards that work.
    pub cards: BTreeSet<CardIndex>,
}

impl Working {
    /// Reads the tests under `root` and cuts the pool.
    ///
    /// # Errors
    /// When a test directory cannot be read.
    pub fn scan(root: &Path) -> io::Result<Self> {
        let mut ids = BTreeSet::new();
        for dir in TEST_DIRS {
            tested_oracle_ids(&root.join(dir), &mut ids)?;
        }
        let mut tested = BTreeSet::new();
        let mut unknown = Vec::new();
        for id in ids {
            match baylee_cards::by_oracle_id(&id) {
                Some(def) => {
                    tested.insert(def.index);
                }
                None => unknown.push(id),
            }
        }
        let mut pool = 0;
        let mut implemented = 0;
        let mut cards = BTreeSet::new();
        for def in baylee_cards::all() {
            pool += 1;
            if def.is_implemented() {
                implemented += 1;
                if tested.contains(&def.index) {
                    cards.insert(def.index);
                }
            }
        }
        Ok(Self {
            pool,
            implemented,
            tested,
            unknown,
            cards,
        })
    }

    /// Whether `card` works, and why not.
    ///
    /// # Errors
    /// The first half of the rule it fails.
    pub fn check(&self, card: CardIndex) -> Result<(), Refusal> {
        if self.cards.contains(&card) {
            return Ok(());
        }
        match baylee_cards::by_index(card) {
            None => Err(Refusal::NotInPool),
            Some(def) if !def.is_implemented() => Err(Refusal::NotImplemented),
            Some(_) => Err(Refusal::Untested),
        }
    }

    /// The working set's fingerprint: SHA-256 over the sorted indices, each
    /// as four little-endian bytes, in hex.
    ///
    /// Every dataset carries it, so a game dealt from a set that has since
    /// lost a card can be found and dropped.
    #[must_use]
    pub fn hash(&self) -> String {
        let mut sha = Sha256::new();
        for card in &self.cards {
            sha.update(card.get().to_le_bytes());
        }
        sha.finalize()
            .iter()
            .fold(String::with_capacity(64), |mut hex, b| {
                use std::fmt::Write as _;
                let _ = write!(hex, "{b:02x}");
                hex
            })
    }
}

/// Every oracle id named as `card_index("…")` in a `.rs` file under `dir`,
/// however deep.
///
/// # Errors
/// When `dir` or a file under it cannot be read.
pub fn tested_oracle_ids(dir: &Path, out: &mut BTreeSet<String>) -> io::Result<()> {
    const CALL: &str = "card_index(\"";
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            tested_oracle_ids(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path)?;
            let mut rest = text.as_str();
            while let Some(at) = rest.find(CALL) {
                rest = &rest[at + CALL.len()..];
                if let Some(end) = rest.find('"') {
                    out.insert(rest[..end].to_owned());
                    rest = &rest[end..];
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn working() -> Working {
        Working::scan(&repo_root()).expect("the engine's test directories read")
    }

    /// A textual reader that misreads its marker inflates or empties the set,
    /// so both bounds are asserted: 2094 cards worked on 2026-09-29 of a pool
    /// of 2749, and a set can never be larger than the implemented cards.
    #[test]
    fn the_working_set_is_the_size_the_pool_says() {
        let w = working();
        assert!(w.cards.len() >= 1500, "only {} cards work", w.cards.len());
        assert!(w.cards.len() <= w.implemented);
        assert!(w.implemented <= w.pool);
        assert!(w.tested.len() >= w.cards.len());
    }

    /// Each half of the rule refuses on its own: a card that is implemented
    /// and untested, and one that is tested and not implemented, are both
    /// out, whenever the pool holds one.
    #[test]
    fn a_card_needs_both_halves_of_the_rule() {
        let w = working();
        let untested = baylee_cards::all()
            .find(|d| d.is_implemented() && !w.tested.contains(&d.index))
            .map(|d| d.index);
        let unimplemented = baylee_cards::all()
            .find(|d| !d.is_implemented() && w.tested.contains(&d.index))
            .map(|d| d.index);
        assert!(
            untested.is_some() || unimplemented.is_some(),
            "no card fails either half, so neither refusal is exercised"
        );
        if let Some(card) = untested {
            assert_eq!(w.check(card), Err(Refusal::Untested));
        }
        if let Some(card) = unimplemented {
            assert_eq!(w.check(card), Err(Refusal::NotImplemented));
        }
        let good = *w.cards.first().expect("some card works");
        assert_eq!(w.check(good), Ok(()));
        assert_eq!(w.check(CardIndex::new(u32::MAX)), Err(Refusal::NotInPool));
    }

    #[test]
    fn the_scan_reads_nested_files_and_only_rust() {
        let dir = std::env::temp_dir().join(format!("baylee-train-scan-{}", std::process::id()));
        let nested = dir.join("deeper");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            dir.join("a.rs"),
            "let x = card_index(\"one\"); let y = card_index(\"two\");",
        )
        .unwrap();
        std::fs::write(nested.join("b.rs"), "card_index(\"three\")").unwrap();
        std::fs::write(dir.join("c.txt"), "card_index(\"not rust\")").unwrap();
        let mut ids = BTreeSet::new();
        tested_oracle_ids(&dir, &mut ids).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
        assert_eq!(ids, ["one", "three", "two"]);
    }

    #[test]
    fn the_hash_follows_the_set() {
        let mut w = working();
        let before = w.hash();
        assert_eq!(before.len(), 64);
        let first = *w.cards.first().unwrap();
        w.cards.remove(&first);
        assert_ne!(w.hash(), before);
        w.cards.insert(first);
        assert_eq!(w.hash(), before);
    }
}
