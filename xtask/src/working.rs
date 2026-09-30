//! Which cards work: the owner's rule for what the trained AI may learn from.
//!
//! A card works when two things hold:
//!
//! - its definition says `Coverage::Implemented`, codegen's (or an author's)
//!   claim that every clause of its text was read, and
//! - the engine's test code names it: its oracle id as a string literal
//!   (`card_index("<oracle id>")`, or a row of an oracle-id table) or its
//!   ledger constant (`index::MOX_OPAL`).
//!
//! The first alone is a claim; the second is somebody having played the card
//! and checked what it did. A net trained on a card that only claims to work
//! learns the engine's bug as a rule of the game, so a game that dealt any
//! other card is not training data.
//!
//! Test code is `crates/baylee-engine/src/**`: every file named `*_tests.rs`,
//! everything under `card_tests/` and `combo_tests/`, every file a
//! `#[cfg(test)] mod name;` brings in (and the files under it), and the body
//! of every inline `#[cfg(test)] mod name { … }`. Card files and the generated
//! tables live in other crates and are never read. Each card keeps the
//! strongest [`Evidence`] found for it, so a mere mention can be told apart
//! from a card a test played.
//!
//! The tests are read as text, from the checkout the binary was built in: the
//! rule is about which tests exist, not about which of them ran.
//!
//! # Where this came from
//!
//! This is `baylee_train::working` from `c42/trained-ai` (9ac1a56f), copied
//! whole so that the precon status (`crate::precons`) and `deck-check` hold a
//! deck to exactly the rule the trained AI is dealt from. Only `repo_root`
//! moved (this crate sits one level up). When that branch lands, one of the
//! two copies goes and the other is the rule; they must not drift apart
//! before then.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use baylee_core::ids::CardIndex;
use sha2::{Digest, Sha256};

/// The engine's sources, relative to the repository root.
pub const ENGINE_SRC: &str = "crates/baylee-engine/src";

/// The ledger's constants, relative to the repository root.
pub const INDEX_SRC: &str = "crates/baylee-core/src/generated/index";

/// The checkout this crate was built in. The binary is handed its root by
/// `main`; the tests ask this.
#[cfg(test)]
#[must_use]
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// How test code names a card, strongest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum How {
    /// `card_index("<oracle id>")`: the shared testkit's way of dealing it.
    CardIndex,
    /// Its oracle id as a string literal, e.g. a row of an oracle-id table.
    OracleLiteral,
    /// Its ledger constant, `index::NAME`.
    IndexConst,
}

impl std::fmt::Display for How {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CardIndex => "card_index",
            Self::OracleLiteral => "oracle literal",
            Self::IndexConst => "index constant",
        })
    }
}

/// Where test code names a card: the strongest way, and the first file
/// (relative to the engine's sources) that names it so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    /// How.
    pub how: How,
    /// Where.
    pub file: String,
}

/// Why a card is not in the working set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The pool has no definition for it.
    NotInPool,
    /// Its definition is not `Coverage::Implemented`.
    NotImplemented,
    /// No engine test code names it.
    Untested,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotInPool => "not in the pool",
            Self::NotImplemented => "not implemented",
            Self::Untested => "implemented, no engine test names it",
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
    /// Pool cards engine test code names, implemented or not, with how.
    pub tested: BTreeMap<CardIndex, Evidence>,
    /// Oracle ids `card_index("…")` names that the pool has no card for. A
    /// test of a card that is not in this build cannot vouch for anything.
    pub unknown: Vec<String>,
    /// The cards that work.
    pub cards: BTreeSet<CardIndex>,
}

impl Working {
    /// Reads the engine's test code under `root` and cuts the pool.
    ///
    /// # Errors
    /// When a source directory cannot be read.
    pub fn scan(root: &Path) -> io::Result<Self> {
        let consts = index_constants(&root.join(INDEX_SRC))?;
        let src = root.join(ENGINE_SRC);
        let mut tested: BTreeMap<CardIndex, Evidence> = BTreeMap::new();
        let mut unknown = BTreeSet::new();
        for (file, text) in test_code(&src)? {
            for (how, card) in named_cards(&text, &consts, &mut unknown) {
                let better = tested.get(&card).is_none_or(|e| how < e.how);
                if better {
                    tested.insert(
                        card,
                        Evidence {
                            how,
                            file: file.clone(),
                        },
                    );
                }
            }
        }
        let mut pool = 0;
        let mut implemented = 0;
        let mut cards = BTreeSet::new();
        for def in baylee_cards::all() {
            pool += 1;
            if def.is_implemented() {
                implemented += 1;
                if tested.contains_key(&def.index) {
                    cards.insert(def.index);
                }
            }
        }
        Ok(Self {
            pool,
            implemented,
            tested,
            unknown: unknown.into_iter().collect(),
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

/// Every ledger constant, `NAME` → index, from the generated per-set files.
///
/// # Errors
/// When the directory or a file in it cannot be read.
pub fn index_constants(dir: &Path) -> io::Result<BTreeMap<String, CardIndex>> {
    const HEAD: &str = "pub const ";
    const NEW: &str = "CardIndex::new(";
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        for line in std::fs::read_to_string(&path)?.lines() {
            let Some(rest) = line.trim_start().strip_prefix(HEAD) else {
                continue;
            };
            let Some((name, value)) = rest.split_once(':') else {
                continue;
            };
            let Some(number) = value
                .split_once(NEW)
                .and_then(|(_, n)| n.split_once(')'))
                .and_then(|(n, _)| n.trim().parse().ok())
            else {
                continue;
            };
            out.insert(name.trim().to_owned(), CardIndex::new(number));
        }
    }
    Ok(out)
}

/// The engine's test code: `(file relative to src, text)` for every whole
/// test file, and for every inline `#[cfg(test)]` module of any other file.
///
/// # Errors
/// When a file under `src` cannot be read.
pub fn test_code(src: &Path) -> io::Result<Vec<(String, String)>> {
    let mut files = Vec::new();
    rust_files(src, &mut files)?;
    files.sort();
    let mut texts = BTreeMap::new();
    for file in &files {
        texts.insert(file.clone(), std::fs::read_to_string(file)?);
    }
    // Files a `#[cfg(test)] mod name;` brings in, as the directories whose
    // every file is test code.
    let mut test_roots: Vec<PathBuf> = Vec::new();
    for (file, text) in &texts {
        for name in cfg_test_modules(text).declared {
            let dir = module_dir(file);
            test_roots.push(dir.join(format!("{name}.rs")));
            test_roots.push(dir.join(&name));
        }
    }
    let mut out = Vec::new();
    for (file, text) in texts {
        let relative = file
            .strip_prefix(src)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        let whole = relative.ends_with("_tests.rs")
            || relative
                .split('/')
                .any(|d| d == "card_tests" || d == "combo_tests")
            || test_roots.iter().any(|root| file.starts_with(root));
        if whole {
            out.push((relative, text));
        } else {
            for body in cfg_test_modules(&text).inline {
                out.push((relative.clone(), body));
            }
        }
    }
    Ok(out)
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// The directory a file's child modules live in: its own for `mod.rs`,
/// `lib.rs` and `main.rs`, else one named after it.
fn module_dir(file: &Path) -> PathBuf {
    let parent = file.parent().unwrap_or(Path::new(""));
    match file.file_stem().and_then(|s| s.to_str()) {
        Some("mod" | "lib" | "main") | None => parent.to_path_buf(),
        Some(stem) => parent.join(stem),
    }
}

/// The `#[cfg(test)]` modules of one file.
#[derive(Debug, Default, PartialEq, Eq)]
struct CfgTest {
    /// `#[cfg(test)] mod name;`: the names.
    declared: Vec<String>,
    /// `#[cfg(test)] mod name { … }`: the bodies.
    inline: Vec<String>,
}

fn cfg_test_modules(text: &str) -> CfgTest {
    const ATTR: &str = "#[cfg(test)]";
    let mut found = CfgTest::default();
    let mut rest = text;
    while let Some(at) = rest.find(ATTR) {
        rest = &rest[at + ATTR.len()..];
        // Other attributes may stand between it and the item.
        let mut item = rest.trim_start();
        while item.starts_with("#[") {
            match item.find(']') {
                Some(end) => item = item[end + 1..].trim_start(),
                None => break,
            }
        }
        let item = item.strip_prefix("pub(crate) ").unwrap_or(item);
        let item = item.strip_prefix("pub ").unwrap_or(item);
        let Some(module) = item.strip_prefix("mod ") else {
            continue;
        };
        let name: String = module
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let after = module[name.len()..].trim_start();
        if after.starts_with(';') {
            found.declared.push(name);
        } else if after.starts_with('{') {
            let mut depth = 0_usize;
            let mut end = after.len();
            for (i, c) in after.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            found.inline.push(after[..end].to_owned());
        }
    }
    found
}

/// Whether `s` is shaped like an oracle id: 8-4-4-4-12 lowercase hex.
fn uuid_shaped(s: &str) -> bool {
    s.len() == 36
        && s.char_indices().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_digit() || ('a'..='f').contains(&c),
        })
}

/// The cards `text` names, each with how. `card_index("…")` ids the pool
/// does not have go to `unknown`.
fn named_cards(
    text: &str,
    consts: &BTreeMap<String, CardIndex>,
    unknown: &mut BTreeSet<String>,
) -> Vec<(How, CardIndex)> {
    const CALL: &str = "card_index(\"";
    const INDEX: &str = "index::";
    let mut out = Vec::new();
    // String literals: every stretch between two quotes that is exactly an
    // oracle id. Not every other stretch: an escaped quote earlier in the
    // file would shift that pairing and hide every literal after it, and a
    // 36-character id-shaped stretch outside quotes is not something Rust
    // source holds.
    let mut offset = 0;
    for segment in text.split('"') {
        if uuid_shaped(segment) && text[offset + segment.len()..].starts_with('"') {
            let called = text[..offset].ends_with(CALL);
            match baylee_cards::by_oracle_id(segment) {
                Some(def) => out.push((
                    if called {
                        How::CardIndex
                    } else {
                        How::OracleLiteral
                    },
                    def.index,
                )),
                None if called => {
                    unknown.insert(segment.to_owned());
                }
                None => {}
            }
        }
        offset += segment.len() + 1;
    }
    let mut rest = text;
    while let Some(at) = rest.find(INDEX) {
        rest = &rest[at + INDEX.len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        if let Some(card) = consts.get(&name) {
            out.push((How::IndexConst, *card));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn working() -> Working {
        Working::scan(&repo_root()).expect("the engine's sources read")
    }

    /// A textual reader that misreads its markers inflates or empties the
    /// set, so both bounds are asserted: on 2026-09-29 main held 2242
    /// implemented cards, of which engine test code named 2241. Most are
    /// dealt through `card_index`; a literal is the other common shape (a
    /// fetch-land table, a call rustfmt wrapped onto two lines). A constant
    /// is never a card's best evidence today, so it is not asserted here but
    /// in `literals_and_constants_are_read_with_how`.
    #[test]
    fn the_working_set_is_the_size_the_pool_says() {
        let w = working();
        assert!(w.cards.len() >= 2000, "only {} cards work", w.cards.len());
        assert!(w.cards.len() <= w.implemented);
        assert!(w.implemented <= w.pool);
        assert!(w.tested.len() >= w.cards.len());
        for how in [How::CardIndex, How::OracleLiteral] {
            assert!(
                w.tested.values().any(|e| e.how == how),
                "no card is named by {how}, so that reader finds nothing"
            );
        }
    }

    /// Each half of the rule refuses on its own: a card that is implemented
    /// and untested, and one that is tested and not implemented, are both
    /// out, whenever the pool holds one.
    #[test]
    fn a_card_needs_both_halves_of_the_rule() {
        let w = working();
        let untested = baylee_cards::all()
            .find(|d| d.is_implemented() && !w.tested.contains_key(&d.index))
            .map(|d| d.index);
        let unimplemented = baylee_cards::all()
            .find(|d| !d.is_implemented() && w.tested.contains_key(&d.index))
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

    /// The rule reaches the three shapes the PM found the house decks' cards
    /// tested in on 2026-09-29: a fetch-land table row (`card_tests/lands.rs`),
    /// an older topical test file (`mana_tests.rs`), and a ledger constant.
    #[test]
    fn a_card_named_in_any_test_shape_is_tested() {
        let w = working();
        let evidence = |name: &str| {
            let card = baylee_cards::decks::by_name(name).expect("in the pool");
            w.tested.get(&card).cloned()
        };
        let strand = evidence("Flooded Strand").expect("the fetch table names it");
        assert!(strand.how <= How::OracleLiteral, "{strand:?}");
        assert!(evidence("Cavern of Souls").is_some());
        assert!(evidence("Force of Will").is_some());
    }

    #[test]
    fn cfg_test_modules_are_found_inline_and_declared() {
        let text = "fn a() {}\n#[cfg(test)]\nmod helpers;\n#[cfg(test)]\n#[allow(x)]\nmod tests {\n    fn b() { let s = \"x\"; }\n}\nfn c() {}\n";
        let found = cfg_test_modules(text);
        assert_eq!(found.declared, ["helpers"]);
        assert_eq!(found.inline.len(), 1);
        assert!(found.inline[0].contains("fn b()"));
        assert!(!found.inline[0].contains("fn c()"));
    }

    #[test]
    fn literals_and_constants_are_read_with_how() {
        let (id, def) = baylee_cards::generated::ALL[0];
        let card = def.index;
        let consts = BTreeMap::from([("SOME_CARD".to_owned(), card)]);
        let mut unknown = BTreeSet::new();
        let text = format!(
            "card_index(\"{id}\"); (\"{id}\", 1); index::SOME_CARD; card_index(\"00000000-0000-0000-0000-000000000000\")"
        );
        let found = named_cards(&text, &consts, &mut unknown);
        assert_eq!(
            found,
            [
                (How::CardIndex, card),
                (How::OracleLiteral, card),
                (How::IndexConst, card)
            ]
        );
        assert_eq!(
            unknown.into_iter().collect::<Vec<_>>(),
            ["00000000-0000-0000-0000-000000000000"]
        );
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
