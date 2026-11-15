//! Only owner choices; no prices, result claims, definitions, seeds or opponent input.
use super::{Catalog, Phase, Refusal, Run};
use crate::{
    cards::{Formation, Stack},
    v2::{economy::Recipe, inventory::Slot, shop::Delivery},
};
use serde::{Deserialize, Serialize};

/// Phase-restricted shop command. Request identity and expected revision are envelopes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    /// Purchase one saved live offer with explicit placement/fusion.
    Buy {
        /// Run-local offer identity.
        offer: u32,
        /// Chosen destination or exact recipe.
        delivery: Delivery,
    },
    /// Sell one owned relic or material, deriving its refund from actual basis.
    Sell {
        /// Owned instance identity.
        item: u32,
    },
    /// Move/swap one relic, then validate all active resources together.
    Move {
        /// Owned instance identity.
        item: u32,
        /// Active/reserve destination.
        slot: Slot,
    },
    /// Consume precisely the selected owned ingredients.
    Fuse {
        /// Inputs and destination, never a caller-selected output.
        recipe: Recipe,
    },
    /// Replace the deck using the owned pool and free basics.
    Edit {
        /// Complete bounded legal deck.
        deck: Vec<Stack>,
    },
    /// Replace the run's library, commander and sideboard in one validated edit.
    Refit {
        /// Exactly 39 library cards.
        deck: Vec<Stack>,
        /// One owned commander and at most five sideboard cards.
        formation: Formation,
    },
    /// Claim the saved post-duel reward exactly once, then enter the next shop.
    Claim {
        /// Distinct identities from the saved offers, with the exact required count.
        cards: Vec<baylee_core::ids::CardIndex>,
    },
    /// Preserve/release one live offer in its existing slot.
    Pin {
        /// Live offer identity.
        offer: u32,
        /// Desired pin state.
        pinned: bool,
    },
    /// Pay the saved visit's next 2/3/4-gold refresh.
    Refresh {},
    /// Preserve all unsold shelf items across rerolls and later shop visits.
    Freeze {
        /// Desired persistent shelf state.
        frozen: bool,
    },
    /// Replace one utility slot with the full-price stage-seven material offer.
    ChooseMythic {
        /// Utility slot six or seven.
        slot: u8,
        /// Explicit consent if this replaces an inherited pin.
        replace_pinned: bool,
    },
    /// End between games, without completion credit or transferable gold.
    Abandon {},
}

impl Run {
    /// Apply an owner command atomically to a clone of the locked saved state.
    /// # Errors
    /// Stale revision, wrong phase, malformed choice, unavailable stock or capacity/gold.
    pub fn command(
        &self,
        revision: u64,
        command: &Command,
        catalog: &Catalog,
        seed: &[u8; 32],
    ) -> Result<Self, Refusal> {
        self.check(revision, catalog)?;
        if let Command::Claim { cards } = command {
            return self.claimed(cards, catalog, seed);
        }
        if self.phase != Phase::Shop {
            return Err(Refusal::Unavailable);
        }
        let mut next = self.clone();
        match command {
            Command::Buy { offer, delivery } => {
                let purchase = self.shop.buy(*offer, delivery, self.stock(catalog))?;
                next.shop = purchase.shop;
                next.economy = purchase.economy;
                next.cards = purchase.cards;
            }
            Command::Sell { item } => next.economy = self.economy.sold(*item)?,
            Command::Move { item, slot } => next.economy = self.economy.moved(*item, *slot)?,
            Command::Fuse { recipe } => next.economy = self.economy.fused(recipe)?,
            Command::Edit { deck } => next.cards = self.cards.edited(catalog, deck.clone())?,
            Command::Refit { deck, formation } => {
                next.cards = self
                    .cards
                    .refitted(catalog, deck.clone(), formation.clone())?;
            }
            Command::Claim { .. } => return Err(Refusal::Unavailable),
            Command::Pin { offer, pinned } => {
                next.shop = self.shop.pinned(*offer, *pinned, catalog)?;
            }
            Command::Refresh {} => {
                (next.shop, next.economy) = self.shop.refreshed(seed, self.stock(catalog))?;
            }
            Command::Freeze { frozen } => next.shop = self.shop.frozen(*frozen, catalog)?,
            Command::ChooseMythic {
                slot,
                replace_pinned,
            } => next.shop = self.shop.chose_mythic(*slot, *replace_pinned, catalog)?,
            Command::Abandon {} => next.phase = Phase::Abandoned,
        }
        next.shop.mythic_choice = false;
        next.ledger = self.ledger.traded(self.economy.gold, next.economy.gold)?;
        next.revised(catalog)
    }
}
