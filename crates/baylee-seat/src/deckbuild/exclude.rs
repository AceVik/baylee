//! Cards a deck may not hold for its pilot, whatever the format allows: a
//! file of names, one a line (`deck lend --exclude`, `deck build
//! --exclude`). The trained net plays an export made on an older card
//! catalog and reads some of this build's cards only by summary, so its
//! lent decks keep to the cards it reads exactly; its agent hands that
//! list over as such a file.
//!
//! A name is exact, as the registry spells it (a card's own name, or its
//! whole name where it has two faces); `#` starts a comment to the end of
//! its line (no card name holds one); blank lines are nothing. A name that
//! is no card of this build refuses the whole file, naming it, so a list
//! made for another build is never half applied. What the file was is kept
//! with every version made under it ([`Exclusion::sha256`]).

use baylee_core::ids::CardIndex;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

/// An exclusion file, read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Exclusion {
    /// The cards no deck may hold.
    pub cards: BTreeSet<CardIndex>,
    /// SHA-256 of the file's bytes, in hex: which list a version kept to.
    pub sha256: String,
}

/// `text` as the cards it names.
///
/// # Errors
/// The first name that is no card of this build, with its line.
pub fn parse(text: &str) -> Result<BTreeSet<CardIndex>, String> {
    let mut cards = BTreeSet::new();
    for (at, line) in text.lines().enumerate() {
        let name = line.split('#').next().unwrap_or_default().trim();
        if name.is_empty() {
            continue;
        }
        let card = baylee_cards::decks::by_name(name).ok_or_else(|| {
            format!(
                "line {} names «{name}», which is no card of this build (names are exact, as \
                 the registry spells them)",
                at + 1
            )
        })?;
        cards.insert(card);
    }
    Ok(cards)
}

/// The exclusion file at `path`.
///
/// # Errors
/// A file that cannot be read, or one naming a card this build lacks.
pub fn load(path: &Path) -> Result<Exclusion, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| format!("{} is not UTF-8 text", path.display()))?;
    let cards = parse(text).map_err(|why| format!("{}: {why}", path.display()))?;
    let mut sha256 = String::with_capacity(64);
    for byte in Sha256::digest(&bytes) {
        let _ = write!(sha256, "{byte:02x}");
    }
    Ok(Exclusion { cards, sha256 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_read_exactly_and_comments_are_nothing() {
        let cards =
            parse("# the net's withdrawn cards\nIsland\n\n  Sol Ring   # by summary only\n")
                .unwrap();
        let expected: BTreeSet<CardIndex> = ["Island", "Sol Ring"]
            .iter()
            .map(|n| baylee_cards::decks::by_name(n).unwrap())
            .collect();
        assert_eq!(cards, expected);
        let why = parse("Island\nsol ring\n").unwrap_err();
        assert!(why.contains("line 2 names «sol ring»"), "{why}");
        assert!(
            parse("Not A Card At All")
                .unwrap_err()
                .contains("no card of this build")
        );
    }

    #[test]
    fn a_file_is_kept_by_its_digest() {
        let dir = std::env::temp_dir().join(format!("baylee-exclude-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("exclude.txt");
        std::fs::write(&path, "Island\n").unwrap();
        let read = load(&path).unwrap();
        assert_eq!(read.cards.len(), 1);
        assert_eq!(
            read.sha256, "91d5d88c1eb4afa3ddc0bf716b8863a29fcab462fe8c6dc1c8e180c6d8c66296",
            "SHA-256 of the file's bytes"
        );
        std::fs::write(&path, "Island\n# and nothing else\n").unwrap();
        assert_ne!(
            load(&path).unwrap().sha256,
            read.sha256,
            "another file, another digest"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
