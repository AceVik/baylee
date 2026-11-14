//! Shop and inventory change together; the run adds phase/revision checks.
use super::{Collection, Delivery, Economy, Goods, PINS, Refusal, Shop, Stock};

/// One atomic shop purchase result, committed together by the authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Purchase {
    /// The purchased offer remains present and sold.
    pub shop: Shop,
    /// Updated wallet/equipment/charges.
    pub economy: Economy,
    /// Updated card ownership, with no automatic deck edits.
    pub cards: Collection,
}

impl Shop {
    /// Buy one exact live offer, optionally combining the new relic with owned copies.
    /// # Errors
    /// Stale offer, incorrect delivery, insufficient gold or invalid resulting inventory.
    pub fn buy(&self, id: u32, delivery: &Delivery, stock: Stock<'_>) -> Result<Purchase, Refusal> {
        self.validate(stock.catalog)?;
        stock.validate()?;
        let index = self
            .offers
            .iter()
            .position(|offer| offer.id == id && !offer.sold)
            .ok_or(Refusal::Stale)?;
        let mut next = Purchase {
            shop: self.clone(),
            economy: stock.economy.clone(),
            cards: stock.cards.clone(),
        };
        match (self.offers[index].goods, delivery) {
            (Goods::Card(card), Delivery::Inventory) => {
                next.cards = next.cards.bought(stock.catalog, card)?;
                next.economy.gold = next
                    .economy
                    .gold
                    .checked_sub(self.offers[index].goods.price(stock.catalog)?)
                    .ok_or(Refusal::Gold)?;
            }
            (Goods::Relic(definition), Delivery::Slot(slot)) => {
                next.economy = next.economy.bought(definition, *slot)?;
            }
            (Goods::Relic(definition), Delivery::Fuse(recipe)) => {
                next.economy = next.economy.bought_and_fused(definition, recipe)?;
            }
            (Goods::Potion(potion), Delivery::Inventory) => {
                next.economy = next.economy.bought_potion(potion)?;
            }
            (Goods::Mythic, Delivery::Inventory) => {
                next.economy = next.economy.bought_material()?;
            }
            _ => return Err(Refusal::Invalid),
        }
        next.shop.offers[index].sold = true;
        next.shop.offers[index].pinned = false;
        next.shop.mythic_choice = false;
        next.shop.validate(stock.catalog)?;
        next.economy.validate()?;
        next.cards.validate(stock.catalog)?;
        Ok(next)
    }

    /// Pin/unpin a still-live offer; pinned goods never create additional stock.
    /// # Errors
    /// Unknown/sold offer or more than two pins.
    pub fn pinned(
        &self,
        id: u32,
        pinned: bool,
        catalog: &crate::cards::Catalog,
    ) -> Result<Self, Refusal> {
        self.validate(catalog)?;
        let mut next = self.clone();
        let offer = next
            .offers
            .iter_mut()
            .find(|offer| offer.id == id && !offer.sold)
            .ok_or(Refusal::Stale)?;
        offer.pinned = pinned;
        if next.offers.iter().filter(|offer| offer.pinned).count() > PINS {
            return Err(Refusal::Capacity);
        }
        next.mythic_choice = false;
        Ok(next)
    }

    /// Pay the next 2/3/4-gold refresh and replace every unpinned slot, including sold ones.
    /// # Errors
    /// Refresh limit, insufficient gold or invalid stored state.
    pub fn refreshed(&self, seed: &[u8; 32], stock: Stock<'_>) -> Result<(Self, Economy), Refusal> {
        self.validate(stock.catalog)?;
        stock.validate()?;
        let price = self.reroll_price().ok_or(Refusal::Unavailable)?;
        let mut economy = stock.economy.clone();
        economy.gold = economy.gold.checked_sub(price).ok_or(Refusal::Gold)?;
        let mut next = self.clone();
        next.rerolls += 1;
        next.mythic_choice = false;
        next.refill(
            seed,
            Stock {
                economy: &economy,
                ..stock
            },
        )?;
        next.validate(stock.catalog)?;
        Ok((next, economy))
    }

    /// One full-price stage-seven material offer replaces a chosen utility slot.
    /// Replacing an inherited pin requires an explicit acknowledged choice.
    /// # Errors
    /// Choice already closed, non-utility slot or unacknowledged pin replacement.
    pub fn chose_mythic(
        &self,
        slot: u8,
        replace_pinned: bool,
        catalog: &crate::cards::Catalog,
    ) -> Result<Self, Refusal> {
        self.validate(catalog)?;
        if !self.mythic_choice || !(6..=7).contains(&slot) {
            return Err(Refusal::Unavailable);
        }
        let index = usize::from(slot);
        if self.offers[index].pinned && !replace_pinned {
            return Err(Refusal::Unavailable);
        }
        let mut next = self.clone();
        let mut offer = next.allocate(Goods::Mythic)?;
        offer.pinned = self.offers[index].pinned;
        next.offers[index] = offer;
        next.mythic_choice = false;
        next.validate(catalog)?;
        Ok(next)
    }
}
