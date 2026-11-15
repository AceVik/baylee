//! Bounded selection, with an affordable legal fallback and no rejection loop.
use super::{Goods, OFFERS, Refusal, Shop, Stock};
use crate::v2::{
    catalog::{Definition, Potion, Rarity},
    generation,
};
use sha2::{Digest, Sha256};

fn score(seed: &[u8; 32], stage: u8, reroll: u8, slot: u8, identity: u32) -> [u8; 32] {
    Sha256::new()
        .chain_update(b"baylee/tower/shop/v2\0")
        .chain_update(seed)
        .chain_update([stage, reroll, slot])
        .chain_update(identity.to_le_bytes())
        .finalize()
        .into()
}

impl Shop {
    /// Open the next visit, carrying at most two pinned offers and the ID sequence.
    /// The service persists the private seed and disallows free initial resampling.
    /// # Errors
    /// Invalid state, skipped/repeated stage or exhausted offer identities.
    pub fn open(
        seed: &[u8; 32],
        stage: u8,
        stock: Stock<'_>,
        previous: Option<&Self>,
    ) -> Result<Self, Refusal> {
        Self::open_for(
            previous.map_or(crate::v2::GENERATOR, |shop| shop.generator),
            seed,
            stage,
            stock,
            previous,
        )
    }

    /// Open under an explicitly frozen cohort generation, including retained runs.
    /// # Errors
    /// Unknown/changed generation, invalid state or skipped/repeated stage.
    pub fn open_for(
        generator: u16,
        seed: &[u8; 32],
        stage: u8,
        stock: Stock<'_>,
        previous: Option<&Self>,
    ) -> Result<Self, Refusal> {
        generation::families(generator)?;
        stock.validate()?;
        if let Some(previous) = previous {
            previous.validate(stock.catalog)?;
            if previous.stage.checked_add(1) != Some(stage) || previous.generator != generator {
                return Err(Refusal::Stale);
            }
        } else if stage != 1 {
            return Err(Refusal::Invalid);
        }
        if stage > super::MAX_STAGES {
            return Err(Refusal::Unavailable);
        }
        let mut shop = Self {
            generator,
            stage,
            rerolls: 0,
            next_id: previous.map_or(1, |shop| shop.next_id),
            offers: previous.map_or_else(Vec::new, |shop| shop.offers.clone()),
            mythic_choice: stage == 7,
        };
        shop.refill(seed, stock)?;
        if !shop
            .offers
            .iter()
            .any(|offer| !offer.pinned && offer.goods.usable(stock))
            && let Some(goods) = candidates(stock)
                .into_iter()
                .filter(|goods| match goods {
                    Goods::Relic(definition) => {
                        generation::definition(generator, *definition).is_ok()
                    }
                    _ => true,
                })
                .filter(|goods| goods.admitted(stage) && goods.usable(stock))
                .min_by_key(|goods| goods.price(stock.catalog).unwrap_or(u16::MAX))
            && let Some(slot) = shop.offers.iter().position(|offer| !offer.pinned)
        {
            shop.offers[slot] = shop.allocate(goods)?;
        }
        shop.validate(stock.catalog)?;
        Ok(shop)
    }

    pub(super) fn refill(&mut self, seed: &[u8; 32], stock: Stock<'_>) -> Result<(), Refusal> {
        for slot in 0..OFFERS {
            if self
                .offers
                .get(slot)
                .is_some_and(|offer| offer.pinned && !offer.sold)
            {
                continue;
            }
            let goods = self.select(seed, u8::try_from(slot).expect("eight slots"), stock)?;
            let offer = self.allocate(goods)?;
            if slot < self.offers.len() {
                self.offers[slot] = offer;
            } else {
                self.offers.push(offer);
            }
        }
        Ok(())
    }

    fn select(&self, seed: &[u8; 32], slot: u8, stock: Stock<'_>) -> Result<Goods, Refusal> {
        let random = score(seed, self.stage, self.rerolls, slot, 0);
        if slot < if self.generator == 1 { 3 } else { 2 } {
            if let Some(card) = stock
                .catalog
                .cards
                .iter()
                .filter(|card| {
                    !card.basic_land && stock.cards.copies(card.card) < crate::cards::COPIES
                })
                .min_by_key(|card| {
                    (
                        score(seed, self.stage, self.rerolls, slot, card.card.get()),
                        card.card,
                    )
                })
            {
                return Ok(Goods::Card(card.card));
            }
        } else if slot >= 6 {
            return Ok(match random[0] {
                0..=11 if self.stage >= 5 => Goods::Mythic,
                0..=127 => Goods::Potion(Potion::Healing),
                _ => Goods::Potion(Potion::Mana),
            });
        }
        let rarity = match random[1] {
            0..=11 if self.stage >= 4 => Rarity::Rare,
            0..=71 if self.stage >= 2 => Rarity::Uncommon,
            _ => Rarity::Common,
        };
        let families = generation::families(self.generator)?;
        let mut family = families[usize::from(random[2]) % families.len()];
        // A bounded third of new-generation relic offers follows an owned family.
        // Canonical catalog order, not item order or active/reserve placement.
        // Selection never reads an opponent and never rewrites already saved stock.
        if self.generator == 2 && random[3] < 85 {
            let owned: Vec<_> = families
                .iter()
                .copied()
                .filter(|candidate| {
                    stock.economy.inventory.relics.iter().any(|item| {
                        item.definition.family == *candidate
                            && item.definition.rarity != Rarity::MythicRare
                    })
                })
                .collect();
            if !owned.is_empty() {
                family = owned[usize::from(random[4]) % owned.len()];
            }
        }
        Ok(Goods::Relic(Definition { family, rarity }))
    }
}

fn candidates(stock: Stock<'_>) -> Vec<Goods> {
    let mut cards: Vec<_> = stock
        .catalog
        .cards
        .iter()
        .filter(|card| !card.basic_land)
        .map(|card| card.card)
        .collect();
    cards.sort_unstable();
    let mut candidates: Vec<_> = cards.into_iter().map(Goods::Card).collect();
    for family in crate::v2::catalog::Family::ALL {
        for rarity in [Rarity::Common, Rarity::Uncommon, Rarity::Rare] {
            candidates.push(Goods::Relic(Definition { family, rarity }));
        }
    }
    candidates.extend([
        Goods::Potion(Potion::Healing),
        Goods::Potion(Potion::Mana),
        Goods::Mythic,
    ]);
    candidates
}
