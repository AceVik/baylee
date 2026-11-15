//! Local, disposable choices. Validation reuses the authority's pure domain rules.
use super::{
    api::{Cohort, Phase, Run},
    domain,
};
use baylee_core::ids::CardIndex;
use domain::{
    cards::{DECK_MAX, Formation, Stack},
    v2::{
        catalog::Rarity,
        economy::Recipe,
        inventory::Slot,
        run::Command,
        shop::{Delivery, Goods, Stock},
    },
};
use uuid::Uuid;

mod formation;
pub use formation::DeckZone;

/// Selection and unsaved deck changes, always bound to one confirmed revision.
pub struct Draft {
    bound: Option<(Uuid, u64)>,
    /// Selected saved offer, never a newly generated offer.
    pub offer: Option<u32>,
    /// Selected owned relic/material for moving or selling.
    pub item: Option<u32>,
    /// Explicit equipment destination.
    pub slot: Slot,
    /// Explicitly selected owned fusion inputs.
    pub ingredients: Vec<u32>,
    /// Explicitly selected Mythic crafting material.
    pub material: Option<u32>,
    /// Unsaved deck, separate from the confirmed collection.
    pub deck: Vec<Stack>,
    /// Commander and sideboard draft, absent for retained legacy runs.
    pub formation: Option<Formation>,
    /// Selected post-duel cards, bound to the same immutable pending reward.
    pub rewards: Vec<CardIndex>,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            bound: None,
            offer: None,
            item: None,
            slot: Slot::Reserve(0),
            ingredients: Vec::new(),
            material: None,
            deck: Vec::new(),
            formation: None,
            rewards: Vec::new(),
        }
    }
}

impl Draft {
    /// Toggle a saved reward candidate without exceeding the required choice count.
    pub fn reward(&mut self, run: &Run, card: CardIndex) {
        if self.bound != Some((run.id, run.revision)) || run.phase != Phase::Reward {
            return;
        }
        let Some(reward) = &run.reward else {
            return;
        };
        if let Some(index) = self.rewards.iter().position(|selected| *selected == card) {
            self.rewards.remove(index);
        } else if self.rewards.len() < usize::from(reward.take) && reward.cards.contains(&card) {
            self.rewards.push(card);
        }
    }

    /// Discard obsolete choices when the authority confirms another revision.
    pub fn bind(&mut self, run: &Run) {
        if self.bound != Some((run.id, run.revision)) {
            *self = Self {
                bound: Some((run.id, run.revision)),
                deck: run.cards.deck.clone(),
                formation: run.cards.formation.clone(),
                ..Self::default()
            };
        }
    }

    /// Whether entering a game would use exactly the deck currently displayed.
    #[must_use]
    pub fn deck_unchanged(&self, run: Option<&Run>) -> bool {
        run.is_none_or(|run| {
            self.bound != Some((run.id, run.revision))
                || (self.deck == run.cards.deck && self.formation == run.cards.formation)
        })
    }

    /// Toggle one real ingredient. A purchase may supply the third copy.
    pub fn ingredient(&mut self, run: &Run, id: u32) {
        if let Some(index) = self.ingredients.iter().position(|old| *old == id) {
            self.ingredients.remove(index);
        } else if self.ingredients.len() < 3
            && run
                .economy
                .inventory
                .relics
                .iter()
                .any(|item| item.id == id)
        {
            self.ingredients.push(id);
        }
    }

    /// Current recipe; server and client derive the output from the same ingredients.
    #[must_use]
    pub fn recipe(&self) -> Recipe {
        Recipe {
            relics: self.ingredients.clone(),
            mythic: self.material,
            destination: self.slot,
        }
    }

    /// Purchase exactly the selected offer, optionally fusing it in the same transaction.
    #[must_use]
    pub fn purchase(&self, run: &Run) -> Option<Command> {
        let offer = run
            .shop
            .offers
            .iter()
            .find(|offer| Some(offer.id) == self.offer && !offer.sold)?;
        let delivery = match offer.goods {
            _ if run.shop.generator == domain::v2::generation::RUN => Delivery::Inventory,
            Goods::Relic(_) if !self.ingredients.is_empty() || self.material.is_some() => {
                Delivery::Fuse(self.recipe())
            }
            Goods::Relic(_) => Delivery::Slot(self.slot),
            _ => Delivery::Inventory,
        };
        Some(Command::Buy {
            offer: offer.id,
            delivery,
        })
    }

    /// Edit locally; temporary decks under forty cards are allowed until Save.
    pub fn card(&mut self, run: &Run, cohort: &Cohort, card: CardIndex, add: bool) {
        if self.formation.is_some() {
            self.card_in(run, cohort, card, DeckZone::Main, add);
            return;
        }
        if self.bound != Some((run.id, run.revision)) || run.phase != Phase::Shop {
            return;
        }
        let Some(entry) = cohort.catalog.card(card) else {
            return;
        };
        let position = self.deck.iter().position(|entry| entry.card == card);
        if add {
            let total: u16 = self.deck.iter().map(|entry| u16::from(entry.copies)).sum();
            let copies = position.map_or(0, |index| self.deck[index].copies);
            if total >= DECK_MAX || (!entry.basic_land && copies >= run.cards.copies(card)) {
                return;
            }
            if let Some(index) = position {
                self.deck[index].copies += 1;
            } else {
                self.deck.push(Stack { card, copies: 1 });
            }
        } else if let Some(index) = position {
            if self.deck[index].copies <= 1 {
                self.deck.remove(index);
            } else {
                self.deck[index].copies -= 1;
            }
        }
        self.deck.sort_by_key(|entry| entry.card);
    }
}

/// Advisory button validation, rerun on confirm. Authorization still belongs to the server.
#[must_use]
pub fn allowed(run: &Run, cohort: &Cohort, command: &Command) -> bool {
    if run.cohort != cohort.id {
        return false;
    }
    if let Command::Claim { cards } = command {
        return run.phase == Phase::Reward
            && run.reward.as_ref().is_some_and(|reward| {
                cards.len() == usize::from(reward.take)
                    && cards.iter().enumerate().all(|(index, card)| {
                        reward.cards.contains(card) && !cards[..index].contains(card)
                    })
            });
    }
    if run.phase != Phase::Shop {
        return false;
    }
    let stock = Stock {
        catalog: &cohort.catalog,
        economy: &run.economy,
        cards: &run.cards,
    };
    match command {
        Command::Buy { offer, delivery } => run.shop.buy(*offer, delivery, stock).is_ok(),
        Command::Sell { item } => run.economy.sold(*item).is_ok(),
        Command::Move { item, slot } => run.economy.moved(*item, *slot).is_ok(),
        Command::Fuse { recipe } => run.economy.fused(recipe).is_ok(),
        Command::Edit { deck } => run.cards.edited(&cohort.catalog, deck.clone()).is_ok(),
        Command::Refit { deck, formation } => run
            .cards
            .refitted(&cohort.catalog, deck.clone(), formation.clone())
            .is_ok(),
        Command::Claim { .. } => false,
        Command::Pin { offer, pinned } => run.shop.pinned(*offer, *pinned, &cohort.catalog).is_ok(),
        Command::Freeze { frozen } => run.shop.frozen(*frozen, &cohort.catalog).is_ok(),
        Command::Refresh {} => run
            .shop
            .reroll_price()
            .is_some_and(|price| price <= run.economy.gold),
        Command::ChooseMythic {
            slot,
            replace_pinned,
        } => run
            .shop
            .chose_mythic(*slot, *replace_pinned, &cohort.catalog)
            .is_ok(),
        Command::Abandon {} => true,
    }
}

/// Derived refund, never the original purchase price or a price provided by a player.
#[must_use]
pub fn refund(run: &Run, id: u32) -> Option<u16> {
    run.economy
        .inventory
        .relics
        .iter()
        .find(|item| item.id == id)
        .map(|item| (item.basis / 2).min(item.definition.rarity.sale_cap()))
        .or_else(|| {
            run.economy
                .inventory
                .materials
                .iter()
                .find(|item| item.id == id)
                .map(|item| item.basis / 2)
        })
}

/// Exact output tier, only after a recipe has passed the domain's validation.
#[must_use]
pub fn fused_rarity(rarity: Rarity) -> Option<Rarity> {
    match rarity {
        Rarity::Common => Some(Rarity::Uncommon),
        Rarity::Uncommon => Some(Rarity::Rare),
        Rarity::Rare => Some(Rarity::MythicRare),
        Rarity::MythicRare => None,
    }
}
