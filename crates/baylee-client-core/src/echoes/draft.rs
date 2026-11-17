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
            _ if domain::v2::generation::is_run(run.shop.generator) => Delivery::Inventory,
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
        Command::SellCard { card } => sellable_card_gold(run, cohort, *card).is_some(),
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

/// Authoritative gold value for selling one unused copy of `card` from the confirmed `run`.
/// Returns `None` outside `Phase::Shop`, on a mismatched cohort, for basic lands,
/// or when every owned copy is committed to the commander, main deck or sideboard.
#[must_use]
pub fn sellable_card_gold(run: &Run, cohort: &Cohort, card: CardIndex) -> Option<u16> {
    if run.cohort != cohort.id
        || run.phase != Phase::Shop
        || !domain::v2::generation::is_run(run.shop.generator)
    {
        return None;
    }
    let (_, gold) = run.cards.sold(&cohort.catalog, card).ok()?;
    run.economy.sold_card(gold).ok()?;
    Some(gold)
}

/// All currently sellable unused nonbasic cards in canonical `CardIndex` order with their gold values.
#[must_use]
pub fn sellable_cards(run: &Run, cohort: &Cohort) -> Vec<(CardIndex, u16)> {
    run.cards
        .owned
        .iter()
        .filter_map(|stack| {
            sellable_card_gold(run, cohort, stack.card).map(|gold| (stack.card, gold))
        })
        .collect()
}

/// Authoritative post-duel gold breakdown for the latest settled duel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RewardGold {
    /// Fixed verified-duel completion income (`DUEL_INCOME`, 12g).
    pub base_duel: u16,
    /// Additional gold from equipped income relics frozen at reservation (e.g. Gilded Acorn).
    pub relic_bonus: u16,
    /// Number of `GildedAcorn` relics currently equipped in active slots.
    pub equipped_acorns: u8,
    /// Gold already credited to `run.economy.gold` upon duel settlement (`base_duel + relic_bonus`).
    pub settled_total: u16,
    /// Gold paid per duplicate card under this run's frozen generation (`3` in Runic, `1` legacy).
    pub duplicate_rate: u16,
    /// Number of selected (or already claimed) reward cards that convert into duplicate gold.
    pub duplicate_count: u8,
    /// Additional gold granted by claiming the selected/claimed duplicate cards.
    pub duplicate_total: u16,
    /// Total gold from this duel including duplicate conversions (`settled_total + duplicate_total`).
    pub total_with_duplicates: u16,
}

/// Compute the latest settled duel's exact gold sources and duplicate-card proceeds.
/// Pass `&ui.choices.rewards` while in `Phase::Reward` (or `&[]` for settled base + relic breakdown).
#[must_use]
pub fn reward_gold(run: &Run, selected_rewards: &[CardIndex]) -> Option<RewardGold> {
    let duel = run.history.last()?;
    let base_duel = domain::v2::DUEL_INCOME;
    let relic_bonus = duel.bonus_gold;
    let equipped_acorns = u8::try_from(
        run.economy
            .inventory
            .active()
            .into_iter()
            .filter(|def| def.family == domain::v2::catalog::Family::GildedAcorn)
            .count(),
    )
    .unwrap_or(u8::MAX);
    let settled_total = base_duel.saturating_add(relic_bonus);
    let duplicate_rate = domain::v2::run::duplicate_gold(run.shop.generator);
    let duplicate_count = if run.phase == Phase::Reward
        && let Some(reward) = &run.reward
    {
        let mut seen = Vec::with_capacity(selected_rewards.len());
        for &card in selected_rewards {
            if !seen.contains(&card) && reward.cards.contains(&card) && run.cards.copies(card) > 0 {
                seen.push(card);
            }
        }
        u8::try_from(seen.len()).unwrap_or(u8::MAX)
    } else if let Some(claim) = &duel.claim {
        claim.duplicates
    } else {
        0
    };
    let duplicate_total = u16::from(duplicate_count).saturating_mul(duplicate_rate);
    let total_with_duplicates = settled_total.saturating_add(duplicate_total);
    Some(RewardGold {
        base_duel,
        relic_bonus,
        equipped_acorns,
        settled_total,
        duplicate_rate,
        duplicate_count,
        duplicate_total,
        total_with_duplicates,
    })
}

/// Duplicate-conversion gold if `card` is already owned in this run (`Some(3)` in Runic, else `None`).
#[must_use]
pub fn reward_card_duplicate_gold(run: &Run, card: CardIndex) -> Option<u16> {
    (domain::v2::generation::is_run(run.shop.generator) && run.cards.copies(card) > 0)
        .then(|| domain::v2::run::duplicate_gold(run.shop.generator))
}
