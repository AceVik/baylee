//! Deterministic, bounded shop visits. Private seeds never appear in player projections.
use crate::{
    Refusal,
    cards::{Catalog, Collection},
    economy::{Economy, Merchandise},
    items::{Potion, Relic},
};
use baylee_core::ids::CardIndex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Four card offers, two relic offers and two potions at most.
pub const MAX_OFFERS: usize = 8;
/// Limits random-search work and discourages buying only a perfect roll.
pub const MAX_REROLLS: u8 = 5;

/// An immutable offer in this visit, with no price field a caller can override.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Goods {
    /// One admitted nonbasic card.
    Card {
        /// Stable rules identity.
        card: CardIndex,
    },
    /// A relic rank or a one-shot potion.
    Item {
        /// Fixed catalog merchandise.
        item: Merchandise,
    },
}

impl Goods {
    /// Resolve the authoritative price in this immutable catalog.
    /// # Errors
    /// Unknown/basic card or invalid merchandise rank.
    pub fn price(self, catalog: &Catalog) -> Result<u16, Refusal> {
        match self {
            Self::Card { card } => {
                let card = catalog.card(card).ok_or(Refusal::Unavailable)?;
                if card.basic_land {
                    return Err(Refusal::Unavailable);
                }
                Ok(u16::from(card.price))
            }
            Self::Item { item } => item.price(),
        }
    }
}

/// Offers persist until bought, explicitly rerolled or the next verified duel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shop {
    /// Stable display order and command indices, at most eight.
    pub offers: Vec<Goods>,
    /// Purchased slots. Sold objects do not restock this shop.
    pub sold: u8,
    /// Number of paid rerolls in this visit, reset only after a verified duel.
    pub rerolls: u8,
}

fn order(seed: &[u8; 32], stage: u8, rerolls: u8, category: u8, identity: u32) -> [u8; 32] {
    Sha256::new()
        .chain_update(b"baylee/tower/shop/v1\0")
        .chain_update(seed)
        .chain_update([stage, rerolls, category])
        .chain_update(identity.to_le_bytes())
        .finalize()
        .into()
}

impl Shop {
    /// Construct a visit from a private run seed and public progression counters.
    /// Account reconnects reuse the saved shop, rather than constructing a new one.
    /// # Errors
    /// Invalid progression, catalog, inventory or deck.
    pub fn generate(
        seed: &[u8; 32],
        stage: u8,
        rerolls: u8,
        catalog: &Catalog,
        economy: &Economy,
        cards: &Collection,
    ) -> Result<Self, Refusal> {
        if !(1..=crate::MAX_STAGES).contains(&stage) || rerolls > MAX_REROLLS {
            return Err(Refusal::Invalid);
        }
        cards.validate(catalog)?;
        economy.validate()?;
        let mut available: Vec<_> = catalog
            .cards
            .iter()
            .filter(|card| !card.basic_land && cards.copies(card.card) < crate::cards::COPIES)
            .collect();
        available.sort_by_key(|card| (order(seed, stage, rerolls, 0, card.card.get()), card.card));
        let mut offers: Vec<_> = available
            .iter()
            .take(4)
            .map(|card| Goods::Card { card: card.card })
            .collect();
        let mut relics: Vec<_> = Relic::ALL
            .into_iter()
            .enumerate()
            .filter_map(|(index, relic)| {
                let rank = economy
                    .equipment
                    .iter()
                    .find(|item| item.relic == relic)
                    .map_or(1, |item| item.rank + 1);
                (rank <= relic.max_rank()
                    && (rank > 1 || economy.equipment.len() < crate::items::RELIC_SLOTS))
                    .then_some((index, Merchandise::Relic { relic, rank }))
            })
            .collect();
        relics.sort_by_key(|(index, _)| order(seed, stage, rerolls, 1, *index as u32));
        offers.extend(
            relics
                .into_iter()
                .take(2)
                .map(|(_, item)| Goods::Item { item }),
        );
        offers.extend([Potion::Healing, Potion::Mana].map(|potion| Goods::Item {
            item: Merchandise::Potion { potion },
        }));
        Ok(Self {
            offers,
            sold: 0,
            rerolls,
        })
    }

    /// Check the saved visit without regenerating its availability after purchases.
    /// # Errors
    /// Invalid count, duplicate offers, unknown merchandise or out-of-range sold bits.
    pub fn validate(&self, catalog: &Catalog) -> Result<(), Refusal> {
        catalog.validate()?;
        if self.offers.len() > MAX_OFFERS
            || self.rerolls > MAX_REROLLS
            || u16::from(self.sold) >= (1_u16 << self.offers.len())
        {
            return Err(Refusal::Invalid);
        }
        for (i, offer) in self.offers.iter().enumerate() {
            offer.price(catalog)?;
            if self.offers[..i].contains(offer) {
                return Err(Refusal::Invalid);
            }
        }
        Ok(())
    }

    /// Is a specific offered slot still in stock?
    pub fn available(&self, slot: u8) -> bool {
        usize::from(slot) < self.offers.len() && slot < 8 && self.sold & (1_u8 << slot) == 0
    }

    /// Next reroll price. There is no free reroll after seeing an opponent.
    pub fn reroll_price(&self) -> Option<u16> {
        (self.rerolls < MAX_REROLLS).then_some(2 + u16::from(self.rerolls))
    }
}
