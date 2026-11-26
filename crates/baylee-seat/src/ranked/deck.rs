//! The deck the account queues with: one of its own private decks at
//! Central, frozen once as the native client freezes it (the shelf's notes
//! dropped, the rows the commitment binds), so the deck the host deals is
//! the one Central was told about and the one the mind is told it holds.

use super::Player;
use anyhow::{Context as _, anyhow, bail};
use baylee_account::RankedDeck;
use baylee_account::ranked::Format;
use uuid::Uuid;

/// A private deck frozen for ranked play.
#[derive(Clone)]
pub struct Frozen {
    /// What the host is sent (`POST /ranked/{match}/play`).
    pub deck: RankedDeck,
    /// The commitment Central matches it under.
    pub digest: [u8; 32],
    /// The same deck as the bridge tells its mind ([`crate::deck::Deck`]).
    pub bridge: crate::deck::Deck,
}

impl std::fmt::Debug for Frozen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frozen")
            .field("format", &self.deck.format)
            .field("name", &self.bridge.name)
            .finish_non_exhaustive()
    }
}

impl Frozen {
    /// The account's deck `id`, as it stands now, in `format`.
    ///
    /// # Errors
    /// A deck the account does not have, of another format, or one that
    /// cannot be registered (a card this build does not know).
    pub async fn load(player: &Player, id: Uuid, format: Format) -> anyhow::Result<Self> {
        let snapshot = player
            .call(move |account| account.load_deck(id))
            .await
            .map_err(|e| anyhow!("reading the account's deck {id}: {e}"))?;
        Self::from_rows(
            &snapshot.draft.name,
            &snapshot.draft.format,
            format,
            &snapshot.draft.cards,
            &snapshot.draft.sideboard,
            &snapshot.draft.commanders,
        )
    }

    /// The deck from its shelf rows.
    ///
    /// # Errors
    /// As [`Frozen::load`].
    pub fn from_rows(
        name: &str,
        shelf_format: &str,
        format: Format,
        cards: &[String],
        sideboard: &[String],
        commanders: &[String],
    ) -> anyhow::Result<Self> {
        if shelf_format != format.as_str() {
            bail!(
                "the deck «{name}» is a {shelf_format} deck, not {}",
                format.as_str()
            );
        }
        let plain = |rows: &[String]| -> anyhow::Result<Vec<String>> {
            rows.iter()
                .map(|line| {
                    let mut row = baylee_core::deckrow::parse(line)
                        .map_err(|_| anyhow!("the deck «{name}» has a row it cannot read"))?;
                    row.note = None;
                    Ok(row.to_string())
                })
                .collect()
        };
        let registration = baylee_cards::ranked::registration(
            format.as_str(),
            &plain(cards)?,
            &plain(sideboard)?,
            commanders,
        )
        .map_err(|e| anyhow!("the deck «{name}» cannot be registered: {e}"))?;
        let [cards, sideboard, commanders] =
            baylee_cards::ranked::playable_rows(&registration.deck)
                .map_err(|e| anyhow!("the deck «{name}»: {e}"))?;
        let mut bridge = crate::deck::Deck {
            name: name.to_owned(),
            cards: without_leaders(&cards, &commanders)?,
            sideboard: sideboard.clone(),
            commanders: commanders.clone(),
            list: crate::mind::DeckList::default(),
        };
        bridge
            .resolve()
            .with_context(|| format!("the deck «{name}» as the mind is told it"))?;
        Ok(Self {
            deck: RankedDeck {
                format,
                cards,
                sideboard,
                commanders,
            },
            digest: registration.digest,
            bridge,
        })
    }
}

/// The main rows without one copy of each commander: the registered rows
/// list a commander among its cards, and the bridge's deck takes it by
/// name, once (as [`crate::deck::Deck::from_text`] does).
fn without_leaders(cards: &[String], commanders: &[String]) -> anyhow::Result<Vec<String>> {
    let mut leaders = commanders.to_vec();
    let mut out = Vec::with_capacity(cards.len());
    for line in cards {
        let mut row = baylee_core::deckrow::parse(line)
            .map_err(|_| anyhow!("a registered row the bridge cannot read"))?;
        if let Some(at) = leaders.iter().position(|name| *name == row.name) {
            leaders.remove(at);
            row.count = row.count.saturating_sub(1);
        }
        if row.count > 0 {
            out.push(row.to_string());
        }
    }
    Ok(out)
}
