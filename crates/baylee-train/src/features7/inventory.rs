//! Exact, unordered knowledge of the viewing seat's submitted cards.
//!
//! Input is one seat's submission, never a game state or opponents' decks.
//! Section membership survives: outside-game cards are not library cards.
//! This is a registration inventory, not a claim about the current library.
//! Copies, face-down objects and hidden transfers require a separate,
//! uncertainty-aware movement ledger before deriving remaining-card counts.
//! This multiset also does not encode setup procedure: no fixed hand and an
//! explicitly empty fixed hand both register zero additional hand cards, but
//! only the former requests the ordinary opening draw. The first observation
//! and any future setup encoder must retain that distinction.

use std::collections::BTreeMap;

use anyhow::{Context as _, ensure};
use baylee_core::card_metadata::{DeckChoices, RulesMetadata};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{DeckEntry, SeatSpec};
use serde::{Deserialize, Serialize};

/// Distinct sources of a seat's registered cards, never combined implicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeckSection7 {
    /// Cards submitted to be shuffled into the library.
    Library,
    /// Outside-game cards, including wish targets.
    Sideboard,
    /// Designated commanders, initially outside the library.
    Commander,
    /// Explicit additional starting hand in configured games/puzzles.
    StartingHand,
    /// Explicit starting permanents in configured games/puzzles.
    StartingBattlefield,
}

/// Resolved rules facts, without preset-local printing/watermark indices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrintingFacts7 {
    /// Exact watermark tags, one optional value per printed face.
    pub watermarks: Vec<Option<String>>,
    /// Silver-bordered or acorn physical printing.
    pub is_un: bool,
}

/// One semantically distinct kind of physical card in the submission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredCard7 {
    /// Stable catalog identity, including valid card index zero.
    pub card_id: u32,
    /// Library, sideboard, commander or explicit starting position.
    pub section: DeckSection7,
    /// Per-copy pregame choices; differently circled Spires stay separate.
    pub choices: DeckChoices,
    /// Independently submitted physical rules facts. `None` means absent
    /// metadata (catalog fallback), not a verified absence of watermarks.
    pub printing: Option<PrintingFacts7>,
    /// Exact multiplicity, without the encoder-6 saturation at 255.
    pub copies: u32,
}

/// A canonical registration multiset, bound to the seat it belongs to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredDeck7 {
    /// Whose private submission this is; callers must match their actor seat.
    pub viewer: PlayerId,
    /// Canonical unordered rows; no submitted or shuffled library order.
    pub cards: Vec<RegisteredCard7>,
}

impl RegisteredDeck7 {
    /// Builds only the supplied seat's multiset. The shared metadata table is
    /// dereferenced only by that seat's entries; unrelated rows are not read
    /// or exported. Artwork references have no rules meaning and are omitted.
    ///
    /// # Errors
    /// Rejects invalid identities, choices, referenced printing facts and
    /// counts too large to represent, instead of silently merging or clipping.
    pub fn from_submission(
        viewer: PlayerId,
        own: &SeatSpec,
        rules: &RulesMetadata,
    ) -> anyhow::Result<Self> {
        let sections = [
            (DeckSection7::Library, own.deck.as_slice()),
            (DeckSection7::Sideboard, own.sideboard.as_slice()),
            (DeckSection7::Commander, own.commanders.as_slice()),
            (
                DeckSection7::StartingHand,
                own.starting_hand.as_deref().unwrap_or_default(),
            ),
            (
                DeckSection7::StartingBattlefield,
                own.starting_battlefield.as_slice(),
            ),
        ];
        let mut rows = BTreeMap::<Vec<u8>, RegisteredCard7>::new();
        for (section, entries) in sections {
            for entry in entries {
                ensure!(
                    baylee_cards::by_index(entry.card).is_some(),
                    "registered card has no catalog identity"
                );
                ensure!(entry.choices.is_valid(), "invalid registered card choices");
                let card = RegisteredCard7 {
                    card_id: entry.card.get(),
                    section,
                    choices: entry.choices,
                    printing: printing_facts(entry, rules)?,
                    copies: 1,
                };
                // A typed row serialized with a fixed count is a complete,
                // deterministic key. New semantic fields cannot accidentally
                // be omitted from a hand-maintained equivalence comparator.
                let key = serde_json::to_vec(&card)?;
                match rows.entry(key) {
                    std::collections::btree_map::Entry::Vacant(slot) => {
                        slot.insert(card);
                    }
                    std::collections::btree_map::Entry::Occupied(mut slot) => {
                        let count = &mut slot.get_mut().copies;
                        *count = count.checked_add(1).context("registered count overflow")?;
                    }
                }
            }
        }
        Ok(Self {
            viewer,
            cards: rows.into_values().collect(),
        })
    }
}

fn printing_facts(
    entry: &DeckEntry,
    rules: &RulesMetadata,
) -> anyhow::Result<Option<PrintingFacts7>> {
    let Some(reference) = entry.rule_print else {
        return Ok(None);
    };
    let print = rules
        .prints
        .get(usize::from(reference.get()))
        .context("missing own printing metadata")?;
    ensure!(
        print.card == entry.card,
        "own printing names a different card"
    );
    ensure!(
        !print.watermarks.is_empty() && u8::try_from(print.watermarks.len()).is_ok(),
        "invalid own printing face count"
    );
    let watermarks = print
        .watermarks
        .iter()
        .map(|id| {
            id.map(|id| {
                let tag = rules
                    .watermarks
                    .get(usize::from(id.get()))
                    .context("missing own watermark metadata")?;
                ensure!(
                    !tag.is_empty()
                        && tag.len() <= 80
                        && tag
                            .bytes()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-'),
                    "invalid own watermark tag"
                );
                Ok(tag.clone())
            })
            .transpose()
        })
        .collect::<anyhow::Result<_>>()?;
    Ok(Some(PrintingFacts7 {
        watermarks,
        is_un: print.is_un,
    }))
}

#[cfg(test)]
mod tests;
