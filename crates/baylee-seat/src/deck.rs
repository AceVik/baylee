//! The deck a bridge brings: read from a file a player exported, or one of
//! the acceptance decks, and told to the mind as the cards the engine plays.
//!
//! One deck, two shapes. The lobby stores a deck as rows of text ("4 Lightning
//! Bolt"), which is what [`Deck::upload`] sends; the mind is told it as card
//! indices ([`DeckList`]), resolved by the same name table the gateway uses
//! when it deals the deck ([`baylee_cards::decks::from_lines`]), so a deck the
//! bridge can describe is a deck the table will deal.

use crate::mind::{DeckCard, DeckList};
use anyhow::{Context as _, anyhow, bail};
use baylee_cards::decks::LoadedDeck;
use baylee_core::acceptance::{Zone, parse_decks};
use baylee_core::ids::CardIndex;

/// The acceptance decks, as this build was made with them.
const ACCEPTANCE: &str = include_str!("../../../data/acceptance-decks.txt");

/// A deck, in both shapes.
#[derive(Clone, Debug)]
pub struct Deck {
    /// The deck's name in the lobby.
    pub name: String,
    /// The main deck as rows ("4 Lightning Bolt"), commanders not included.
    pub cards: Vec<String>,
    /// The sideboard as rows.
    pub sideboard: Vec<String>,
    /// The commanders, by name.
    pub commanders: Vec<String>,
    /// The deck as the engine plays it.
    pub list: DeckList,
}

impl Deck {
    /// One of the acceptance decks, by name (`Allytifact`, `Victory`).
    ///
    /// # Errors
    /// When there is no deck of that name, or a card in it is not in the
    /// pool.
    pub fn acceptance(name: &str) -> anyhow::Result<Self> {
        let rows = parse_decks(ACCEPTANCE).map_err(|e| anyhow!("the acceptance decks: {e}"))?;
        let mut deck = Self {
            name: name.to_string(),
            cards: Vec::new(),
            sideboard: Vec::new(),
            commanders: Vec::new(),
            list: DeckList::default(),
        };
        for row in rows.iter().filter(|row| row.deck == name) {
            let line = format!("{} {}", row.count, row.name);
            match row.zone {
                Zone::Main => deck.cards.push(line),
                Zone::Sideboard => deck.sideboard.push(line),
                Zone::Commander => deck.commanders.push(row.name.clone()),
            }
        }
        if deck.cards.is_empty() {
            let known = baylee_cards::decks::acceptance_names(ACCEPTANCE).join(", ");
            bail!("no acceptance deck is called {name} (there are: {known})");
        }
        deck.resolve()?;
        Ok(deck)
    }

    /// A deck file in any format a player can export (Baylee text, JSON,
    /// YAML, a Moxfield text export).
    ///
    /// # Errors
    /// When the file cannot be read, is a link rather than a deck, or names
    /// a card that is not in the pool.
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let fallback = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Deck")
            .to_string();
        Self::from_text(&text, &fallback)
    }

    /// A deck document's text, named `fallback` when it names itself
    /// nothing.
    ///
    /// # Errors
    /// As [`Deck::from_file`].
    pub fn from_text(text: &str, fallback: &str) -> anyhow::Result<Self> {
        let read = match baylee_deckio::import(text).map_err(|e| anyhow!("the deck: {e:?}"))? {
            baylee_deckio::Import::Read { read, .. } => read,
            baylee_deckio::Import::Source(_) | baylee_deckio::Import::UnknownLink(_) => {
                bail!("that is a link to a deck; export the deck as text and pass the file")
            }
        };
        if !read.skipped.is_empty() {
            let lines: Vec<String> = read
                .skipped
                .iter()
                .map(|skipped| format!("line {}: {}", skipped.line, skipped.text))
                .collect();
            bail!(
                "lines the deck reader could not read: {}",
                lines.join(" | ")
            );
        }
        let stored = read.document.stored();
        let mut deck = Self {
            name: stored.name.clone().unwrap_or_else(|| fallback.to_string()),
            cards: Vec::new(),
            sideboard: stored.sideboard.iter().map(ToString::to_string).collect(),
            commanders: stored.commanders.clone(),
            list: DeckList::default(),
        };
        // A stored document lists its commanders among its cards; the lobby
        // takes them by name, once.
        let mut leaders = stored.commanders.clone();
        for row in &stored.cards {
            let mut row = row.clone();
            if let Some(at) = leaders.iter().position(|name| *name == row.name) {
                leaders.remove(at);
                row.count = row.count.saturating_sub(1);
            }
            if row.count > 0 {
                deck.cards.push(row.to_string());
            }
        }
        deck.resolve()?;
        Ok(deck)
    }

    /// What `POST /decks` takes.
    #[must_use]
    pub fn upload(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "cards": self.cards,
            "sideboard": self.sideboard,
            "commanders": self.commanders,
        })
    }

    /// Resolves the rows to the cards the engine plays, as the gateway will.
    fn resolve(&mut self) -> anyhow::Result<()> {
        let loaded = baylee_cards::decks::from_lines(
            &self.name,
            &self.cards,
            &self.sideboard,
            &self.commanders,
        )
        .map_err(|e| anyhow!("the deck {}: {e}", self.name))?;
        if loaded.commanders.len() != self.commanders.len() {
            bail!("the deck {}: a commander is not in the pool", self.name);
        }
        self.list = list_of(&loaded);
        Ok(())
    }
}

/// A dealt deck, told as so many copies of each card, in the order the
/// cards first appear.
#[must_use]
pub fn list_of(deck: &LoadedDeck) -> DeckList {
    fn counted(cards: impl Iterator<Item = CardIndex>) -> Vec<DeckCard> {
        let mut out: Vec<DeckCard> = Vec::new();
        for card in cards {
            match out.iter_mut().find(|entry| entry.card == card) {
                Some(entry) => entry.count += 1,
                None => out.push(DeckCard { card, count: 1 }),
            }
        }
        out
    }
    DeckList {
        name: deck.name.clone(),
        main: counted(deck.main.iter().map(|c| c.index)),
        sideboard: counted(deck.sideboard.iter().map(|c| c.index)),
        commanders: counted(deck.commanders.iter().map(|c| c.index)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both acceptance decks resolve, commanders apart from the library.
    #[test]
    fn the_acceptance_decks_resolve() {
        for name in ["Allytifact", "Victory"] {
            let deck = Deck::acceptance(name).unwrap();
            let main: u32 = deck.list.main.iter().map(|c| c.count).sum();
            assert!(main >= 60, "{name}: {main} cards");
            assert_eq!(deck.list.commanders.len(), deck.commanders.len(), "{name}");
        }
        assert!(Deck::acceptance("Nope").is_err());
    }

    /// A stored document counts its commander among its cards; the upload
    /// names it once, as a commander, and the library is dealt without it.
    #[test]
    fn a_commander_is_uploaded_once() {
        let deck = Deck::from_text("Commander\n1 Llanowar Elves\n\nDeck\n30 Forest\n", "Elves");
        let deck = deck.unwrap();
        assert!(
            deck.cards.iter().all(|row| !row.contains("Llanowar")),
            "{:?}",
            deck.cards
        );
        assert_eq!(deck.commanders, vec!["Llanowar Elves".to_string()]);
        assert_eq!(deck.list.main.iter().map(|c| c.count).sum::<u32>(), 30);
    }
}
