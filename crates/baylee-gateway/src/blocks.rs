//! Blocked cards (owner, 12.10.2026; `docs/protocol.md` §"Blocked cards"):
//! cards the operator took out of play on this gateway, for good or until
//! a moment, through the admin console (`admin.rs`).
//!
//! A deck holding a blocked card, in its main deck, sideboard or command
//! zone, is not played here: it is refused where a deck is chosen for a
//! room ([`crate::routes::tables::own_deck`]), where its player says ready,
//! and where a game is built ([`crate::routes::tables::room_preset`], and
//! the one-tap game against the house), which also covers the house deck
//! an AI chair plays when the host gave it none. A game already running is
//! never touched: the gateway runs no rules, and a block is about which
//! decks sit down, not about cards on a table.
//!
//! The blocks in force live in memory ([`Blocks`]), because the places that
//! ask hold the lobby's lock and may not wait on the database. They are read
//! from `card_block` before the gateway serves, after every change the
//! console makes, and every [`REFRESH`] in case another process wrote one.
//! A block with an end stops counting at that moment by itself: every
//! question compares its `until` with the clock.
//!
//! Players learn about blocks from `GET /cards/blocked` and from a
//! `blocked` list on every row of `GET /decks` and `GET /decks/shared`, so a
//! client can mark a deck before the gateway refuses it. A local game
//! (`LocalHost`) asks no gateway and is not affected.

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use axum::response::Json;
use parking_lot::RwLock;
use time::OffsetDateTime;

use crate::{ErrorBody, err_saying};

/// How often the blocks are read again from the database.
const REFRESH: Duration = Duration::from_secs(60);

/// One block in force.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Block {
    /// The card's `CardIndex`.
    pub(crate) index: u32,
    /// Its English name: this build's, else the one stored with the block.
    pub(crate) name: String,
    /// Why, in the admin's words.
    pub(crate) reason: String,
    /// When it ends by itself, if ever.
    pub(crate) until: Option<OffsetDateTime>,
}

impl Block {
    fn in_force(&self, now: OffsetDateTime) -> bool {
        self.until.is_none_or(|until| until > now)
    }

    /// The block as a player is told it: `{"index", "name", "reason",
    /// "until"}`, `until` an RFC 3339 moment in UTC or `null`.
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "index": self.index,
            "name": self.name,
            "reason": self.reason,
            "until": self.until.map(iso),
        })
    }

    /// What a player is told when a deck is refused for this card.
    pub(crate) fn sentence(&self, deck: &str) -> String {
        let until = self
            .until
            .map_or_else(String::new, |at| format!(" until {}", readable(at)));
        format!(
            "the deck \u{201c}{deck}\u{201d} holds {}, which is blocked on this gateway{until}: {}",
            self.name, self.reason
        )
    }
}

/// `2026-10-08T12:00:00Z`.
pub(crate) fn iso(at: OffsetDateTime) -> String {
    let at = at.to_offset(time::UtcOffset::UTC);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    )
}

/// `2026-10-08 12:00 UTC`, for a sentence.
fn readable(at: OffsetDateTime) -> String {
    let at = at.to_offset(time::UtcOffset::UTC);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02} UTC",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute()
    )
}

/// This build's English name of the card `index`, if the pool has it.
pub(crate) fn pool_name(index: u32) -> Option<&'static str> {
    baylee_cards::pool::rows()
        .iter()
        .find(|card| card.index == index)
        .map(|card| card.english_name.as_str())
}

/// The blocks in force, as the gateway last read them.
#[derive(Default)]
pub(crate) struct Blocks(RwLock<Arc<Vec<Block>>>);

impl Blocks {
    /// The blocks in force at `now`.
    pub(crate) fn now_in_force(&self, now: OffsetDateTime) -> Vec<Block> {
        let all = Arc::clone(&self.0.read());
        all.iter().filter(|b| b.in_force(now)).cloned().collect()
    }

    /// The block in force at `now` on the card `index`, if any.
    pub(crate) fn of(&self, index: u32, now: OffsetDateTime) -> Option<Block> {
        let all = Arc::clone(&self.0.read());
        all.iter()
            .find(|b| b.index == index && b.in_force(now))
            .cloned()
    }

    /// Replaces what is held with `blocks`.
    pub(crate) fn set(&self, blocks: Vec<Block>) {
        *self.0.write() = Arc::new(blocks);
    }

    /// Reads the blocks in force from the database.
    ///
    /// # Errors
    ///
    /// When the database fails; what was held stays.
    pub(crate) async fn reload(
        &self,
        db: &sea_orm::DatabaseConnection,
    ) -> Result<(), sea_orm::DbErr> {
        let rows = baylee_db::card_blocks::in_force(db, OffsetDateTime::now_utc()).await?;
        self.set(
            rows.into_iter()
                .map(|row| Block {
                    index: row.card_index,
                    name: pool_name(row.card_index).map_or(row.card_name, str::to_owned),
                    reason: row.reason,
                    until: row.until,
                })
                .collect(),
        );
        Ok(())
    }

    /// The blocked cards among `indices`, once each, in the order met.
    pub(crate) fn among(
        &self,
        indices: impl IntoIterator<Item = u32>,
        now: OffsetDateTime,
    ) -> Vec<Block> {
        let all = Arc::clone(&self.0.read());
        if all.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<Block> = Vec::new();
        for index in indices {
            if found.iter().any(|b| b.index == index) {
                continue;
            }
            if let Some(block) = all.iter().find(|b| b.index == index && b.in_force(now)) {
                found.push(block.clone());
            }
        }
        found
    }

    /// The blocked cards of a stored deck's lines, main deck, sideboard and
    /// commanders, once each. A line that does not read, or names no card of
    /// this pool, is passed over: the list routes answer decks whatever they
    /// hold, and a deck with such a line is refused for that elsewhere.
    pub(crate) fn in_lines(
        &self,
        cards: &[String],
        sideboard: &[String],
        commanders: &[String],
        now: OffsetDateTime,
    ) -> Vec<Block> {
        if self.0.read().is_empty() {
            return Vec::new();
        }
        let rows = cards.iter().chain(sideboard).filter_map(|line| {
            baylee_core::deckrow::parse(line)
                .ok()
                .and_then(|row| baylee_cards::decks::by_name(&row.name))
        });
        let leaders = commanders
            .iter()
            .filter_map(|name| baylee_cards::decks::by_name(name));
        self.among(
            rows.chain(leaders).map(baylee_core::ids::CardIndex::get),
            now,
        )
    }

    /// The first blocked card of a deck about to be played, as the refusal
    /// a player is answered with.
    pub(crate) fn check_loaded(
        &self,
        deck: &baylee_cards::decks::LoadedDeck,
        now: OffsetDateTime,
    ) -> Result<(), (StatusCode, Json<ErrorBody>)> {
        let cards = deck
            .main
            .iter()
            .chain(&deck.sideboard)
            .chain(&deck.commanders)
            .map(|card| card.index.get());
        match self.among(cards, now).first() {
            Some(block) => Err(refusal(block, &deck.name)),
            None => Ok(()),
        }
    }

    /// As [`Self::check_loaded`], for a stored deck.
    pub(crate) fn check_stored(
        &self,
        deck: &crate::store::Deck,
        now: OffsetDateTime,
    ) -> Result<(), (StatusCode, Json<ErrorBody>)> {
        match self
            .in_lines(&deck.cards, &deck.sideboard, &deck.commanders, now)
            .first()
        {
            Some(block) => Err(refusal(block, &deck.name)),
            None => Ok(()),
        }
    }
}

/// `409`, naming the deck, the card, the reason and the end.
pub(crate) fn refusal(block: &Block, deck: &str) -> (StatusCode, Json<ErrorBody>) {
    err_saying(StatusCode::CONFLICT, block.sentence(deck))
}

/// Reads the blocks again every [`REFRESH`].
pub(crate) fn spawn_refresh(state: crate::Shared) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(REFRESH);
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(e) = state.blocks.reload(&state.db).await {
                tracing::warn!("card blocks: could not read them again: {e}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(index: u32, until: Option<OffsetDateTime>) -> Block {
        Block {
            index,
            name: format!("Card {index}"),
            reason: "testing".into(),
            until,
        }
    }

    #[test]
    fn a_block_with_an_end_stops_counting_by_itself() {
        let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).expect("a moment");
        let blocks = Blocks::default();
        blocks.set(vec![
            block(1, None),
            block(2, Some(now + time::Duration::hours(1))),
            block(3, Some(now)),
        ]);
        let found: Vec<u32> = blocks
            .among([3, 2, 1, 2, 4], now)
            .iter()
            .map(|b| b.index)
            .collect();
        assert_eq!(found, vec![2, 1]);
        assert!(blocks.of(2, now + time::Duration::hours(2)).is_none());
    }

    #[test]
    fn the_sentence_names_the_deck_the_card_the_end_and_the_reason() {
        let at = OffsetDateTime::from_unix_timestamp(1_800_000_000).expect("a moment");
        let said = block(7, Some(at)).sentence("Mono Red");
        assert_eq!(
            said,
            "the deck \u{201c}Mono Red\u{201d} holds Card 7, which is blocked on this \
             gateway until 2027-01-15 08:00 UTC: testing"
        );
        assert!(!block(7, None).sentence("X").contains("until"));
    }
}
