//! The house decks under `data/decks/`, loaded for self-play.
//!
//! A file is a deck in the format `xtask deck-check` reads: a `[deck:Name]`
//! header, one `baylee_core::deckrow` row per line, then optional
//! `[sideboard]` rows and `[commander]` lines, where a commander is a bare card
//! name. `#` starts a comment.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use baylee_cards::decks::{LoadedDeck, from_lines};
use baylee_core::ids::CardIndex;

use crate::working::{Refusal, Working};

/// Where the house decks live in the checkout this crate was built in.
#[must_use]
pub fn dir() -> PathBuf {
    crate::working::repo_root().join("data/decks")
}

/// A house deck and the file it came from.
#[derive(Clone, Debug)]
pub struct HouseDeck {
    /// The file's stem: what a command line names the deck by.
    pub key: String,
    /// The deck as the engine is dealt it.
    pub deck: LoadedDeck,
}

impl HouseDeck {
    /// Loads `path`.
    ///
    /// # Errors
    /// When the file cannot be read, has no deck header, or names a card
    /// the pool does not have.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let key = path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("a deck file has a name")?
            .to_owned();
        Self::parse(&key, &text).with_context(|| format!("loading {}", path.display()))
    }

    /// Loads the house deck named `key` (`allytifact` for
    /// `data/decks/allytifact.txt`).
    ///
    /// # Errors
    /// As [`HouseDeck::load`].
    pub fn named(key: &str) -> anyhow::Result<Self> {
        Self::load(&dir().join(format!("{key}.txt")))
    }

    /// Reads a deck from the text of its file.
    ///
    /// # Errors
    /// As [`HouseDeck::load`].
    pub fn parse(key: &str, text: &str) -> anyhow::Result<Self> {
        let mut name = None;
        let mut section = "";
        let (mut main, mut side, mut commanders) = (Vec::new(), Vec::new(), Vec::new());
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix('[') {
                let rest = rest.trim_end_matches(']');
                section = match rest.split_once(':') {
                    Some(("deck", deck)) => {
                        name = Some(deck.to_owned());
                        "deck"
                    }
                    _ => match rest {
                        "sideboard" => "sideboard",
                        "commander" => "commander",
                        other => bail!("unknown section [{other}]"),
                    },
                };
                continue;
            }
            match section {
                "deck" => main.push(line.to_owned()),
                "sideboard" => side.push(line.to_owned()),
                "commander" => {
                    // `from_lines` drops a leader it cannot resolve, and a
                    // Commander deck without its commander is another deck.
                    if baylee_cards::decks::by_name(line).is_none() {
                        bail!("unknown commander: {line}");
                    }
                    commanders.push(line.to_owned());
                }
                _ => bail!("a card row before the [deck:Name] header: {line}"),
            }
        }
        let name = name.context("no [deck:Name] header")?;
        let deck = from_lines(&name, &main, &side, &commanders).map_err(anyhow::Error::msg)?;
        Ok(Self {
            key: key.to_owned(),
            deck,
        })
    }

    /// Every card the deck can put into a game: library, sideboard and
    /// command zone.
    pub fn cards(&self) -> impl Iterator<Item = CardIndex> + '_ {
        self.deck
            .main
            .iter()
            .chain(&self.deck.sideboard)
            .chain(&self.deck.commanders)
            .map(|c| c.index)
    }

    /// The deck's distinct cards.
    #[must_use]
    pub fn distinct(&self) -> BTreeSet<CardIndex> {
        self.cards().collect()
    }

    /// The deck's cards that do not work, and why, each once.
    #[must_use]
    pub fn failing(&self, working: &Working) -> Vec<(CardIndex, Refusal)> {
        self.distinct()
            .into_iter()
            .filter_map(|card| working.check(card).err().map(|why| (card, why)))
            .collect()
    }

    /// The deck with every card that does not work taken out, and what was
    /// taken (card, copies).
    ///
    /// A copy in the library becomes one of the deck's own basic lands, the
    /// basics taken in turn as often as the deck runs each, so the mana stays
    /// the deck's. A sideboard card is dropped. A commander leaves the command
    /// zone, and its place among the deck's cards goes to a basic as well, so
    /// the deck keeps its size. The key gains `+working`, because this is
    /// another deck.
    ///
    /// # Errors
    /// When the deck has a card to replace and runs no basic land that works.
    pub fn working_only(&self, working: &Working) -> anyhow::Result<(Self, Vec<(CardIndex, u32)>)> {
        let basics: BTreeSet<CardIndex> = baylee_cards::decks::basic_lands()
            .into_iter()
            .flatten()
            .filter(|b| working.check(*b).is_ok())
            .collect();
        let fill: Vec<_> = self
            .deck
            .main
            .iter()
            .filter(|c| basics.contains(&c.index))
            .cloned()
            .collect();
        let mut removed: std::collections::BTreeMap<CardIndex, u32> =
            std::collections::BTreeMap::new();
        let mut take = |card: CardIndex| *removed.entry(card).or_default() += 1;
        let mut deck = self.deck.clone();
        let mut next = fill.iter().cycle();
        let mut replace = |deck: &mut LoadedDeck, at: Option<usize>| -> anyhow::Result<()> {
            let basic = next
                .next()
                .context("the deck runs no working basic land to put in a card's place")?
                .clone();
            match at {
                Some(at) => deck.main[at] = basic,
                None => deck.main.push(basic),
            }
            Ok(())
        };
        for at in 0..deck.main.len() {
            let card = deck.main[at].index;
            if working.check(card).is_err() {
                take(card);
                replace(&mut deck, Some(at))?;
            }
        }
        deck.sideboard.retain(|c| {
            let keep = working.check(c.index).is_ok();
            if !keep {
                take(c.index);
            }
            keep
        });
        let leaders = std::mem::take(&mut deck.commanders);
        for leader in leaders {
            if working.check(leader.index).is_ok() {
                deck.commanders.push(leader);
            } else {
                take(leader.index);
                replace(&mut deck, None)?;
            }
        }
        Ok((
            Self {
                key: format!("{}+working", self.key),
                deck,
            },
            removed.into_iter().collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_house_deck_loads() {
        let mut loaded = 0;
        for entry in std::fs::read_dir(dir()).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "txt") {
                let deck = HouseDeck::load(&path).unwrap();
                assert!(deck.deck.main.len() >= 40, "{} is a deck", deck.key);
                loaded += 1;
            }
        }
        assert!(loaded >= 2, "only {loaded} house decks");
    }

    /// Every house deck cut to its working cards keeps its size, holds only
    /// cards that work, and loses exactly the cards `failing` names.
    #[test]
    fn a_working_only_deck_keeps_its_size_and_only_working_cards() {
        let working = Working::scan(&crate::working::repo_root()).unwrap();
        for key in ["allytifact", "victory"] {
            let deck = HouseDeck::named(key).unwrap();
            let (cut, removed) = deck.working_only(&working).unwrap();
            assert_eq!(cut.key, format!("{key}+working"));
            assert!(cut.failing(&working).is_empty(), "{key} still fails");
            let failing: BTreeSet<CardIndex> =
                deck.failing(&working).into_iter().map(|(c, _)| c).collect();
            let taken: BTreeSet<CardIndex> = removed.iter().map(|(c, _)| *c).collect();
            assert_eq!(failing, taken);
            let size = |d: &HouseDeck| d.deck.main.len() + d.deck.commanders.len();
            assert_eq!(size(&cut), size(&deck), "{key} changed size");
        }
    }

    #[test]
    fn a_commander_leaves_the_library() {
        let text = "[deck:Test]\n1 Island\n1 Sol Ring\n[commander]\nSol Ring\n";
        let deck = HouseDeck::parse("test", text).unwrap();
        assert_eq!(deck.deck.main.len(), 1);
        assert_eq!(deck.deck.commanders.len(), 1);
        assert_eq!(deck.cards().count(), 2);
    }

    #[test]
    fn a_deck_that_is_not_one_says_why() {
        for (text, why) in [
            ("1 Island\n", "before the [deck:Name] header"),
            (
                "[deck:A]\n1 Island\n[commander]\nNo Such Card\n",
                "unknown commander",
            ),
            ("[deck:A]\n1 No Such Card\n", "unknown card"),
            ("[deck:A]\n[maybeboard]\n", "unknown section"),
        ] {
            let err = format!("{:#}", HouseDeck::parse("t", text).unwrap_err());
            assert!(err.contains(why), "{text:?} said {err}");
        }
    }
}
