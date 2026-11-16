//! Run-owned cards and legal deck editing. Basic lands are free and never have resale value.
use crate::Refusal;
pub use baylee_core::ids::CardIndex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

mod commander;
pub use commander::{Formation, RunCard};

/// A deliberately finite reviewed card catalog for a rules revision.
pub const MAX_CATALOG: usize = 2048;
/// Tower's limited-size decks allow extra cards, but cannot be unbounded.
pub const DECK_MIN: u16 = 40;
/// Hard deck bound; basic lands do not bypass it.
pub const DECK_MAX: u16 = 60;
/// Nonbasic copy cap, independently enforced on both collection and deck.
pub const COPIES: u8 = 4;

/// One admitted card. The package worker separately validates full engine support.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    /// Stable rules identity, never an image/printing identity.
    pub card: CardIndex,
    /// Authoritative price per copy; zero only for a basic land.
    pub price: u8,
    /// Unlimited free deck-building supply, excluded from paid inventory and sales.
    pub basic_land: bool,
    /// Reviewed Commander facts, absent in legacy catalog commitments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<RunCard>,
}

/// Immutable catalog, content-bound to a matching cohort.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    /// At most [`MAX_CATALOG`] distinct, reviewed rules identities.
    pub cards: Vec<Card>,
}

impl Catalog {
    /// Structural check before constructing offers or accepting edits.
    /// # Errors
    /// Oversized/empty catalog, duplicate identity, invalid free card or price.
    pub fn validate(&self) -> Result<(), Refusal> {
        let limit = if self.is_run() { MAX_CATALOG } else { 128 };
        if self.cards.is_empty() || self.cards.len() > limit {
            return Err(Refusal::Invalid);
        }
        let mut seen = BTreeSet::new();
        for card in &self.cards {
            if !seen.insert(card.card)
                || (card.basic_land && card.price != 0)
                || (!card.basic_land && !(2..=12).contains(&card.price))
            {
                return Err(Refusal::Invalid);
            }
        }
        commander::catalog(self)?;
        Ok(())
    }

    /// Whether the immutable catalog uses the forty-card Commander run format.
    pub fn is_run(&self) -> bool {
        self.cards.first().is_some_and(|card| card.run.is_some())
    }

    /// Find an admitted card; arbitrary registry cards are not part of this run.
    pub fn card(&self, card: CardIndex) -> Option<&Card> {
        self.cards.iter().find(|entry| entry.card == card)
    }
}

/// A count of one canonical card, not a client-supplied script or text document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    /// Rules identity in the immutable catalog.
    pub card: CardIndex,
    /// Number of copies, bounded by the role of this list.
    pub copies: u8,
}

/// The run's card collection and current deck. Neither is the account's private vault.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Collection {
    /// Commander and sideboard for an independent run, absent in older runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formation: Option<Formation>,
    /// Nonbasic cards granted at start or bought, including ones currently in the deck.
    pub owned: Vec<Stack>,
    /// Forty to sixty cards; edits cannot introduce unowned nonbasics.
    pub deck: Vec<Stack>,
}

impl Collection {
    /// Check all catalog, ownership, copy and deck-size bounds before use.
    /// # Errors
    /// Malformed lists, unknown cards, unowned copies or an illegal deck size.
    pub fn validate(&self, catalog: &Catalog) -> Result<(), Refusal> {
        catalog.validate()?;
        self.validate_cards(catalog)
    }

    // The caller has validated this immutable catalog once for the whole batch.
    fn validate_cards(&self, catalog: &Catalog) -> Result<(), Refusal> {
        if catalog.is_run() {
            return commander::validate(self, catalog);
        }
        if self.formation.is_some() {
            return Err(Refusal::Invalid);
        }
        if self.owned.len() > MAX_CATALOG || self.deck.len() > usize::from(DECK_MAX) {
            return Err(Refusal::Capacity);
        }
        let mut seen = BTreeSet::new();
        for stack in &self.owned {
            let card = catalog.card(stack.card).ok_or(Refusal::Unavailable)?;
            if card.basic_land || !(1..=COPIES).contains(&stack.copies) || !seen.insert(stack.card)
            {
                return Err(Refusal::Invalid);
            }
        }
        seen.clear();
        let mut count = 0_u16;
        for stack in &self.deck {
            let card = catalog.card(stack.card).ok_or(Refusal::Unavailable)?;
            if stack.copies == 0 || u16::from(stack.copies) > DECK_MAX || !seen.insert(stack.card) {
                return Err(Refusal::Invalid);
            }
            if !card.basic_land && stack.copies > self.copies(stack.card) {
                return Err(Refusal::Unavailable);
            }
            count += u16::from(stack.copies);
        }
        if !(DECK_MIN..=DECK_MAX).contains(&count) {
            return Err(Refusal::Invalid);
        }
        Ok(())
    }

    /// Owned nonbasic count. Basic lands are supplied separately by the catalog.
    pub fn copies(&self, card: CardIndex) -> u8 {
        self.owned
            .iter()
            .find(|entry| entry.card == card)
            .map_or(0, |entry| entry.copies)
    }

    /// Grant one purchased nonbasic copy; the authority separately debits its price.
    /// # Errors
    /// Invalid state, unknown/basic card or copy cap reached.
    pub fn bought(&self, catalog: &Catalog, card: CardIndex) -> Result<Self, Refusal> {
        self.validate(catalog)?;
        if catalog.card(card).ok_or(Refusal::Unavailable)?.basic_land {
            return Err(Refusal::Unavailable);
        }
        if self.copies(card) == if catalog.is_run() { 1 } else { COPIES } {
            return Err(Refusal::Capacity);
        }
        let mut next = self.clone();
        if let Some(stack) = next.owned.iter_mut().find(|stack| stack.card == card) {
            stack.copies += 1;
        } else {
            next.owned.push(Stack { card, copies: 1 });
            next.owned.sort_by_key(|stack| stack.card);
        }
        Ok(next)
    }

    /// Replace the complete deck with a validated edit, preserving inventory on failure.
    /// # Errors
    /// Any catalog, ownership, count or deck-size rule fails.
    pub fn edited(&self, catalog: &Catalog, deck: Vec<Stack>) -> Result<Self, Refusal> {
        if deck.len() > usize::from(DECK_MAX) {
            return Err(Refusal::Capacity);
        }
        self.validate(catalog)?;
        let mut next = Self {
            formation: self.formation.clone(),
            owned: self.owned.clone(),
            deck,
        };
        next.validate(catalog)?;
        next.deck.sort_by_key(|stack| stack.card);
        Ok(next)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod run_tests;
