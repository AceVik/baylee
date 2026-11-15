//! Account-owned progression. Authorization, row locks, durable request receipts,
//! record storage and award outbox writes belong to one authority transaction.
use super::{
    DEFEAT_LIMIT, DUEL_INCOME, MAX_STAGES, STARTING_GOLD, TARGET_WINS,
    economy::Economy,
    execution::{Execution, MAX_RECORD_BYTES, Prefix},
    shop::{Shop, Stock},
};
use crate::{
    Refusal,
    cards::{Catalog, Collection},
};
use serde::{Deserialize, Serialize};

mod commands;
mod execution;
mod ledger;
mod lifecycle;
pub use crate::run::Outcome;
pub use commands::Command;
pub use execution::WorkerCommand;
pub use ledger::Ledger;
pub use lifecycle::{Binding, Settlement};

/// Exactly one permitted activity; technical states grant no advancement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Edit the owned deck, shop and equipment before learning an opponent.
    Shop,
    /// Private package is immutable and awaits an execution claim.
    Reserved,
    /// The leased worker runs the same reserved duel.
    Playing,
    /// Sealed record awaits independent verification.
    Verifying,
    /// Trusted technical failure; no shopping, spending or rewards.
    Quarantined,
    /// Eight victories, with no further shop.
    Conquered,
    /// Five defeats/rules draws, with no further shop.
    Exhausted,
    /// Player explicitly stopped between games.
    Abandoned,
    /// Operator explicitly retired an unrecoverable reservation without an outcome.
    Interrupted,
}

/// Saved private owner state. This contains no opponent deck, seed or credential.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// Optimistic command version in addition to transaction locking/idempotency.
    pub revision: u64,
    /// Current permitted activity.
    pub phase: Phase,
    /// At most twelve verified settlements, in duel ordinal order.
    pub history: Vec<Settlement>,
    /// Run-local wallet, relics and charges.
    pub economy: Economy,
    /// Owned card pool and selected legal deck.
    pub cards: Collection,
    /// Saved offers. Final runs keep the final visit without reopening it.
    pub shop: Shop,
    /// Conserved debits and sale proceeds; income derives from verified history.
    pub ledger: Ledger,
    /// At most one immutable reservation, retained through technical recovery.
    pub execution: Option<Execution>,
}

impl Run {
    /// Start with an authority-selected, cohort-bound starter and private shop seed.
    /// # Errors
    /// Invalid catalog or starter.
    pub fn begin(catalog: &Catalog, starter: Collection, seed: &[u8; 32]) -> Result<Self, Refusal> {
        Self::begin_for(catalog, starter, seed, super::GENERATOR)
    }

    /// Start under the saved cohort's supported generation rather than the latest default.
    /// # Errors
    /// Unknown generation, invalid catalog or starter.
    pub fn begin_for(
        catalog: &Catalog,
        starter: Collection,
        seed: &[u8; 32],
        generator: u16,
    ) -> Result<Self, Refusal> {
        let economy = if generator == super::generation::RUN {
            Economy::for_run()
        } else {
            Economy::default()
        };
        let shop = Shop::open_for(
            generator,
            seed,
            1,
            Stock {
                catalog,
                economy: &economy,
                cards: &starter,
            },
            None,
        )?;
        let result = Self {
            revision: 0,
            phase: Phase::Shop,
            history: Vec::new(),
            economy,
            cards: starter,
            shop,
            ledger: Ledger::default(),
            execution: None,
        };
        result.validate(catalog)?;
        Ok(result)
    }

    /// Verified victories only; never elapsed time or unverified claims.
    pub fn wins(&self) -> usize {
        self.history
            .iter()
            .filter(|result| result.outcome == Outcome::Victory)
            .count()
    }

    /// Defeats and ordinary rules draws each consume one allowance.
    pub fn defeats(&self) -> usize {
        self.history.len() - self.wins()
    }

    /// Validate loaded state; legal history is established by the verifier, not here.
    /// # Errors
    /// Inconsistent phase/history, impossible income or invalid owned state.
    pub fn validate(&self, catalog: &Catalog) -> Result<(), Refusal> {
        if self.history.len() > usize::from(MAX_STAGES) {
            return Err(Refusal::Capacity);
        }
        self.cards.validate(catalog)?;
        self.economy.validate()?;
        // New inventory rules are admitted only with their own frozen run
        // generation; legacy publications cannot acquire extra active slots.
        if (self.shop.generator == super::generation::RUN)
            != (self.economy.inventory.layout == super::inventory::Layout::Run)
            || (self.shop.generator == super::generation::RUN) != catalog.is_run()
        {
            return Err(Refusal::Invalid);
        }
        self.shop.validate(catalog)?;
        for relic in &self.economy.inventory.relics {
            super::generation::definition(self.shop.generator, relic.definition)?;
        }
        let (mut wins, mut defeats) = (0, 0);
        for (index, result) in self.history.iter().enumerate() {
            if wins == TARGET_WINS
                || defeats == DEFEAT_LIMIT
                || self.history[..index]
                    .iter()
                    .any(|earlier| earlier.binding.package == result.binding.package)
            {
                return Err(Refusal::Invalid);
            }
            result.validate()?;
            if result.outcome == Outcome::Victory {
                wins += 1;
            } else {
                defeats += 1;
            }
        }
        let completed = match self.phase {
            Phase::Conquered => wins == TARGET_WINS && defeats < DEFEAT_LIMIT,
            Phase::Exhausted => defeats == DEFEAT_LIMIT && wins < TARGET_WINS,
            _ => wins < TARGET_WINS && defeats < DEFEAT_LIMIT,
        };
        let terminal = matches!(self.phase, Phase::Conquered | Phase::Exhausted);
        let stage = self.history.len() + usize::from(!terminal);
        if !completed || usize::from(self.shop.stage) != stage {
            return Err(Refusal::Invalid);
        }
        self.validate_execution()?;
        self.ledger.validate(&self.economy, self.history.len())
    }

    /// Bind decoded private state to its immutable publication before any operation.
    /// # Errors
    /// Invalid state or a changed/mismatched cohort generation.
    pub fn validate_for(&self, catalog: &Catalog, generator: u16) -> Result<(), Refusal> {
        if self.shop.generator != generator {
            return Err(Refusal::Invalid);
        }
        self.validate(catalog)
    }

    fn validate_execution(&self) -> Result<(), Refusal> {
        let requires = matches!(
            self.phase,
            Phase::Reserved
                | Phase::Playing
                | Phase::Verifying
                | Phase::Quarantined
                | Phase::Interrupted
        );
        if requires != self.execution.is_some() {
            return Err(Refusal::Invalid);
        }
        if let Some(execution) = &self.execution {
            execution.validate()?;
            if self.shop.mythic_choice
                || self
                    .history
                    .iter()
                    .any(|result| result.binding.package == execution.package)
            {
                return Err(Refusal::Invalid);
            }
            let valid = match self.phase {
                Phase::Reserved => !execution.sealed && execution.lease_until == 0,
                Phase::Playing => !execution.sealed && execution.lease_until != 0,
                Phase::Verifying => execution.sealed,
                Phase::Quarantined | Phase::Interrupted => execution.lease_until == 0,
                _ => false,
            };
            if !valid {
                return Err(Refusal::Invalid);
            }
        }
        Ok(())
    }

    fn check(&self, revision: u64, catalog: &Catalog) -> Result<(), Refusal> {
        self.validate(catalog)?;
        if revision != self.revision {
            return Err(Refusal::Stale);
        }
        Ok(())
    }

    fn revised(mut self, catalog: &Catalog) -> Result<Self, Refusal> {
        self.revision = self.revision.checked_add(1).ok_or(Refusal::Capacity)?;
        self.validate(catalog)?;
        Ok(self)
    }

    fn stock<'a>(&'a self, catalog: &'a Catalog) -> Stock<'a> {
        Stock {
            catalog,
            economy: &self.economy,
            cards: &self.cards,
        }
    }
}

#[cfg(test)]
mod tests;
