//! Pure run transitions. The service serializes them and binds every result to its reservation.
use crate::{
    DEFEAT_LIMIT, DUEL_INCOME, MAX_STAGES, Refusal, STARTING_GOLD, TARGET_WINS,
    cards::{Catalog, Collection, Stack},
    economy::Economy,
    items::Relic,
    shop::{Goods, Shop},
};
use serde::{Deserialize, Serialize};

/// One next action at a time; buying is impossible while any duel is outstanding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Buy, sell and edit the owned deck before committing to the next opponent.
    Shop,
    /// Loadout and opponent are frozen while execution capacity is prepared.
    Queued,
    /// An execution instance has claimed the duel.
    Playing,
    /// Awaiting the independent verification worker's terminal result.
    Verifying,
    /// Five verified victories.
    Conquered,
    /// Three unsuccessful duels.
    Exhausted,
    /// Explicitly ended between games, with no completion reward.
    Abandoned,
}

/// Actual terminal game outcome; no elapsed-time claims participate in advancement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The human won through the game rules.
    Victory,
    /// The human lost, including a deliberate concession.
    Defeat,
    /// An engine draw is unsuccessful for the ascent and spends one defeat allowance.
    Draw,
}

/// Account-owned progression. This contains no opponent deck, selection seed or credential.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// Optimistic revision checked in addition to the persistence lock/idempotency key.
    pub revision: u32,
    /// Current legal activity.
    pub phase: Phase,
    /// Verified victories so far.
    pub wins: u8,
    /// Verified defeats and draws so far.
    pub defeats: u8,
    /// Sum of turns across verified terminal games, never a wall clock.
    pub turns: u32,
    /// Run-only gold and equipment.
    pub economy: Economy,
    /// Run-only collection and current legal deck.
    pub cards: Collection,
    /// Immutable visit with purchased slots retained, even after selling an item.
    pub shop: Shop,
}

impl Run {
    /// Create a run from an authority-supplied starter, not a player's private collection.
    /// # Errors
    /// Starter/catalog is invalid.
    pub fn begin(catalog: &Catalog, starter: Collection, seed: &[u8; 32]) -> Result<Self, Refusal> {
        let economy = Economy::default();
        let shop = Shop::generate(seed, 1, 0, catalog, &economy, &starter)?;
        Ok(Self {
            revision: 0,
            phase: Phase::Shop,
            wins: 0,
            defeats: 0,
            turns: 0,
            economy,
            cards: starter,
            shop,
        })
    }

    /// One-based next duel ordinal. Meaningful only for an unfinished run.
    pub fn stage(&self) -> u16 {
        1 + u16::from(self.wins) + u16::from(self.defeats)
    }

    /// Validate stored state before any transition or player projection.
    /// # Errors
    /// Inconsistent terminal/progression phase, invalid cards/economy or impossible income.
    pub fn validate(&self, catalog: &Catalog) -> Result<(), Refusal> {
        self.cards.validate(catalog)?;
        self.economy.validate()?;
        self.shop.validate(catalog)?;
        let games = u16::from(self.wins) + u16::from(self.defeats);
        if self.wins > TARGET_WINS
            || self.defeats > DEFEAT_LIMIT
            || games > u16::from(MAX_STAGES)
            || self.economy.gold > STARTING_GOLD + DUEL_INCOME * games
            || self.turns > u32::from(MAX_STAGES) * 500
            || (games == 0 && self.turns != 0)
            || self.turns < u32::from(games)
        {
            return Err(Refusal::Invalid);
        }
        let won = self.wins == TARGET_WINS;
        let lost = self.defeats == DEFEAT_LIMIT;
        let consistent = match self.phase {
            Phase::Conquered => won && !lost,
            Phase::Exhausted => lost && !won,
            _ => !won && !lost && games < u16::from(MAX_STAGES),
        };
        if !consistent {
            return Err(Refusal::Invalid);
        }
        Ok(())
    }

    fn check(&self, revision: u32, catalog: &Catalog) -> Result<(), Refusal> {
        self.validate(catalog)?;
        if self.revision != revision {
            return Err(Refusal::Stale);
        }
        Ok(())
    }

    fn shopping(&self, revision: u32, catalog: &Catalog) -> Result<(), Refusal> {
        self.check(revision, catalog)?;
        if self.phase != Phase::Shop {
            return Err(Refusal::Unavailable);
        }
        Ok(())
    }

    fn revised(mut self, catalog: &Catalog) -> Result<Self, Refusal> {
        self.revision = self.revision.checked_add(1).ok_or(Refusal::Invalid)?;
        self.validate(catalog)?;
        Ok(self)
    }

    /// Buy exactly one still-offered slot. The same stale command cannot debit twice.
    /// # Errors
    /// Invalid/stale state, wrong phase, sold slot, insufficient gold or full inventory.
    pub fn buy(&self, revision: u32, slot: u8, catalog: &Catalog) -> Result<Self, Refusal> {
        self.shopping(revision, catalog)?;
        if !self.shop.available(slot) {
            return Err(Refusal::Stale);
        }
        let mut next = self.clone();
        match self.shop.offers[usize::from(slot)] {
            Goods::Item { item } => next.economy = self.economy.bought(item)?,
            goods @ Goods::Card { card } => {
                next.economy.gold = self
                    .economy
                    .gold
                    .checked_sub(goods.price(catalog)?)
                    .ok_or(Refusal::Gold)?;
                next.cards = self.cards.bought(catalog, card)?;
            }
        }
        next.shop.sold |= 1 << slot;
        next.revised(catalog)
    }

    /// Sell a carried relic; it does not reappear in the current shop.
    /// # Errors
    /// Wrong phase/revision, unowned relic or invalid state.
    pub fn sell(&self, revision: u32, relic: Relic, catalog: &Catalog) -> Result<Self, Refusal> {
        self.shopping(revision, catalog)?;
        let mut next = self.clone();
        next.economy = self.economy.sold(relic)?;
        next.revised(catalog)
    }

    /// Commit a legal deck using only this run's collection and free basic lands.
    /// # Errors
    /// Wrong phase/revision or illegal deck.
    pub fn edit(
        &self,
        revision: u32,
        deck: Vec<Stack>,
        catalog: &Catalog,
    ) -> Result<Self, Refusal> {
        self.shopping(revision, catalog)?;
        let mut next = self.clone();
        next.cards = self.cards.edited(catalog, deck)?;
        next.revised(catalog)
    }

    /// Pay for a new bounded visit roll. The authority supplies the saved private seed.
    /// # Errors
    /// Wrong phase/revision, reroll cap, invalid state or insufficient gold.
    pub fn reroll(
        &self,
        revision: u32,
        catalog: &Catalog,
        seed: &[u8; 32],
    ) -> Result<Self, Refusal> {
        self.shopping(revision, catalog)?;
        let cost = self.shop.reroll_price().ok_or(Refusal::Capacity)?;
        let mut next = self.clone();
        next.economy.gold = self.economy.gold.checked_sub(cost).ok_or(Refusal::Gold)?;
        next.shop = Shop::generate(
            seed,
            self.stage() as u8,
            self.shop.rerolls + 1,
            catalog,
            &next.economy,
            &next.cards,
        )?;
        next.revised(catalog)
    }

    /// Freeze shopping. Persistence atomically reserves the loadout, opponent and gateway.
    /// # Errors
    /// Wrong phase/revision or invalid state.
    pub fn queue(&self, revision: u32, catalog: &Catalog) -> Result<Self, Refusal> {
        self.shopping(revision, catalog)?;
        let mut next = self.clone();
        next.phase = Phase::Queued;
        next.revised(catalog)
    }

    /// Move execution to its next phase; not an account/client operation.
    /// # Errors
    /// Invalid state or a transition other than queued→playing→verifying.
    pub fn execution(&self, next_phase: Phase, catalog: &Catalog) -> Result<Self, Refusal> {
        self.validate(catalog)?;
        if !matches!(
            (self.phase, next_phase),
            (Phase::Queued, Phase::Playing) | (Phase::Playing, Phase::Verifying)
        ) {
            return Err(Refusal::Unavailable);
        }
        let mut next = self.clone();
        next.phase = next_phase;
        next.revised(catalog)
    }

    /// Apply a verified terminal record once. Draws spend a defeat allowance.
    /// # Errors
    /// Wrong phase, invalid state/record, charge overuse or impossible turn count.
    pub fn verified(
        &self,
        outcome: Outcome,
        turns: u16,
        used: [u8; 2],
        catalog: &Catalog,
        seed: &[u8; 32],
    ) -> Result<Self, Refusal> {
        self.validate(catalog)?;
        if self.phase != Phase::Verifying {
            return Err(Refusal::Unavailable);
        }
        if !(1..=500).contains(&turns) {
            return Err(Refusal::Invalid);
        }
        let mut next = self.clone();
        next.economy = self.economy.consumed(used)?;
        next.economy.gold += DUEL_INCOME;
        next.turns += u32::from(turns);
        match outcome {
            Outcome::Victory => next.wins += 1,
            Outcome::Defeat | Outcome::Draw => next.defeats += 1,
        }
        next.phase = if next.wins == TARGET_WINS {
            Phase::Conquered
        } else if next.defeats == DEFEAT_LIMIT {
            Phase::Exhausted
        } else {
            Phase::Shop
        };
        if next.phase == Phase::Shop {
            next.shop = Shop::generate(
                seed,
                next.stage() as u8,
                0,
                catalog,
                &next.economy,
                &next.cards,
            )?;
        }
        next.revised(catalog)
    }

    /// A trusted technical void retains the exact frozen loadout/opponent and grants no income.
    /// # Errors
    /// Invalid state or a run without an outstanding reservation.
    pub fn voided(&self, catalog: &Catalog) -> Result<Self, Refusal> {
        self.validate(catalog)?;
        if !matches!(
            self.phase,
            Phase::Queued | Phase::Playing | Phase::Verifying
        ) {
            return Err(Refusal::Unavailable);
        }
        let mut next = self.clone();
        next.phase = Phase::Queued;
        next.revised(catalog)
    }

    /// End between games. Active games must end through a verified concession instead.
    /// # Errors
    /// Wrong phase/revision or invalid state.
    pub fn abandon(&self, revision: u32, catalog: &Catalog) -> Result<Self, Refusal> {
        self.check(revision, catalog)?;
        if !matches!(self.phase, Phase::Shop | Phase::Queued) {
            return Err(Refusal::Unavailable);
        }
        let mut next = self.clone();
        next.phase = Phase::Abandoned;
        next.revised(catalog)
    }
}

#[cfg(test)]
mod tests;
