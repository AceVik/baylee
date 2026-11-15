//! Persisted, bounded shop visits. Account/cohort seed allocation belongs to the service.
use super::{MAX_STAGES, economy::Economy};
use crate::{
    Refusal,
    cards::{Catalog, Collection},
};
use serde::{Deserialize, Serialize};

mod generate;
mod goods;
mod transactions;
pub use goods::{Delivery, Goods};
pub use transactions::Purchase;

/// Exactly eight persisted offers; pinned offers occupy these same slots.
pub const OFFERS: usize = 8;
/// Maximum paid refreshes per visit, costing two, three, then four gold.
pub const REROLLS: u8 = 3;
/// Offers carried into a refresh or the next visit.
pub const PINS: usize = 2;

/// Validated purchase context; never opponent information.
#[derive(Clone, Copy)]
pub struct Stock<'a> {
    /// Frozen card catalog.
    pub catalog: &'a Catalog,
    /// Current authoritative wallet and equipment.
    pub economy: &'a Economy,
    /// Entire owned pool, including cards outside the current deck.
    pub cards: &'a Collection,
}

impl Stock<'_> {
    fn validate(self) -> Result<(), Refusal> {
        self.economy.validate()?;
        self.cards.validate(self.catalog)
    }
}

/// One quantity-one offer. Identical goods may have different instance identities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offer {
    /// Monotonic run-local offer ordinal; unrelated to the private generation seed.
    pub id: u32,
    /// Fixed catalog merchandise; no client-supplied price.
    pub goods: Goods,
    /// Already purchased; selling does not restock it.
    pub sold: bool,
    /// Retained in the same slot on refresh/next visit, until purchased or unpinned.
    pub pinned: bool,
}

/// A stored visit; reconnecting reads it and does not generate another.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shop {
    /// Frozen generator; omitted original encoding stays byte-identical in receipts.
    #[serde(
        default = "super::generation::original",
        skip_serializing_if = "super::generation::is_original"
    )]
    pub generator: u16,
    /// Next duel ordinal, starting at one and counting preceding losses/draws.
    pub stage: u8,
    /// Paid refreshes in this visit.
    pub rerolls: u8,
    /// Next never-used offer ordinal, carried between visits.
    pub next_id: u32,
    /// Stable visible slot order.
    pub offers: Vec<Offer>,
    /// One stage-seven choice, closed by choosing or the first shop transaction.
    pub mythic_choice: bool,
    /// Run-wide shelf freeze; bought slots still refill. Absent in legacy bytes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub frozen: bool,
}

impl Shop {
    /// Reject malformed persisted state, identities, merchandise and availability.
    /// # Errors
    /// Invalid visit, repeated IDs, unsupported goods or too many pins.
    pub fn validate(&self, catalog: &Catalog) -> Result<(), Refusal> {
        catalog.validate()?;
        super::generation::families(self.generator)?;
        if !(1..=MAX_STAGES).contains(&self.stage)
            || (self.generator < super::generation::RUN && self.rerolls > REROLLS)
            || self.next_id == 0
            || self.offers.len() != OFFERS
            || self.offers.iter().filter(|offer| offer.pinned).count() > PINS
            || (self.generator < super::generation::RUN && self.frozen)
            || (self.generator == super::generation::RUN
                && self
                    .offers
                    .iter()
                    .any(|offer| offer.pinned || matches!(offer.goods, Goods::Card(_))))
            || (self.mythic_choice
                && (self.stage != 7
                    || self.rerolls != 0
                    || self.offers.iter().any(|offer| offer.sold)))
        {
            return Err(Refusal::Invalid);
        }
        for (index, offer) in self.offers.iter().enumerate() {
            if let Goods::Relic(definition) = offer.goods {
                super::generation::definition(self.generator, definition)?;
            }
            if offer.id == 0
                || offer.id >= self.next_id
                || (offer.sold && offer.pinned)
                || self.offers[..index]
                    .iter()
                    .any(|previous| previous.id == offer.id)
                || !offer.goods.admitted(self.stage)
            {
                return Err(Refusal::Invalid);
            }
            offer.goods.price(catalog)?;
        }
        Ok(())
    }

    /// Fixed next refresh price, or none after the third refresh.
    pub fn reroll_price(&self) -> Option<u16> {
        if self.generator == super::generation::RUN {
            return (self.rerolls < u8::MAX
                && (!self.frozen || self.offers.iter().any(|offer| offer.sold)))
            .then_some(2 + u16::from(self.rerolls.min(4)));
        }
        (self.rerolls < REROLLS).then_some(2 + u16::from(self.rerolls))
    }

    fn check_stock(&self, stock: Stock<'_>) -> Result<(), Refusal> {
        stock.validate()?;
        let run = self.generator == super::generation::RUN;
        if run != (stock.economy.inventory.layout == super::inventory::Layout::Run) {
            return Err(Refusal::Invalid);
        }
        Ok(())
    }

    fn allocate(&mut self, goods: Goods) -> Result<Offer, Refusal> {
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or(Refusal::Capacity)?;
        Ok(Offer {
            id,
            goods,
            sold: false,
            pinned: false,
        })
    }
}

#[cfg(test)]
mod tests;
