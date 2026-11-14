//! Bounded selection, with an affordable legal fallback and no rejection loop.
use super::{Goods, OFFERS, Refusal, Shop, Stock};
use crate::v2::catalog::{Definition, Family, Potion, Rarity};
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
        stock.validate()?;
        if let Some(previous) = previous {
            previous.validate(stock.catalog)?;
            if previous.stage.checked_add(1) != Some(stage) {
                return Err(Refusal::Stale);
            }
        } else if stage != 1 {
            return Err(Refusal::Invalid);
        }
        if stage > super::MAX_STAGES {
            return Err(Refusal::Unavailable);
        }
        let mut shop = Self {
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
            let goods = self.select(seed, u8::try_from(slot).expect("eight slots"), stock);
            let offer = self.allocate(goods)?;
            if slot < self.offers.len() {
                self.offers[slot] = offer;
            } else {
                self.offers.push(offer);
            }
        }
        Ok(())
    }

    fn select(&self, seed: &[u8; 32], slot: u8, stock: Stock<'_>) -> Goods {
        let random = score(seed, self.stage, self.rerolls, slot, 0);
        if slot < 3 {
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
                return Goods::Card(card.card);
            }
        } else if slot >= 6 {
            return match random[0] {
                0..=11 if self.stage >= 5 => Goods::Mythic,
                0..=127 => Goods::Potion(Potion::Healing),
                _ => Goods::Potion(Potion::Mana),
            };
        }
        let rarity = match random[1] {
            0..=11 if self.stage >= 4 => Rarity::Rare,
            0..=71 if self.stage >= 2 => Rarity::Uncommon,
            _ => Rarity::Common,
        };
        Goods::Relic(Definition {
            family: Family::ALL[usize::from(random[2] % 4)],
            rarity,
        })
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
    for family in Family::ALL {
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
