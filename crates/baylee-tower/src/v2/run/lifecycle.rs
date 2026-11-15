//! The verifier supplies evidence bound to the sealed reservation, never client scores.
use super::{
    Catalog, DEFEAT_LIMIT, MAX_RECORD_BYTES, Outcome, Phase, Prefix, Refusal, Run, Shop,
    TARGET_WINS,
};
use serde::{Deserialize, Serialize};

/// Exact reservation generation and durable terminal prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// Authority-stored immutable package commitment.
    pub package: [u8; 32],
    /// Current verification generation, invalidated by quarantine.
    pub fence: u64,
    /// Exact sealed durable head, never a submitted replacement record.
    pub prefix: Prefix,
}

/// Verified terminal facts; constructing this value alone is NOT verification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settlement {
    /// Saved reservation and record binding.
    pub binding: Binding,
    /// Actual rules outcome, including concession as defeat.
    pub outcome: Outcome,
    /// Rules turns, for history only. Zero permits a concession before turn one.
    pub turns: u32,
    /// Actual human healing/mana activations in the verified record.
    pub used: [u8; 2],
}

impl Settlement {
    pub(super) fn validate(&self) -> Result<(), Refusal> {
        if self.binding.fence == 0
            || self.binding.prefix.chunks == 0
            || self.used.iter().map(|count| u16::from(*count)).sum::<u16>() > 2
        {
            return Err(Refusal::Invalid);
        }
        self.binding.prefix.validate(MAX_RECORD_BYTES)
    }
}

impl Run {
    /// Commit independently verified evidence once. The service must commit this,
    /// reservation result, snapshot eligibility and award outbox in one transaction.
    /// # Errors
    /// Stale revision/binding, wrong phase, impossible spent charges or invalid result.
    pub fn settled(
        &self,
        revision: u64,
        result: Settlement,
        catalog: &Catalog,
        seed: &[u8; 32],
    ) -> Result<Self, Refusal> {
        if self.shop.generator == crate::v2::generation::RUN {
            return Err(Refusal::Unavailable);
        }
        self.settlement(revision, result, catalog, seed, None)
    }

    /// Settle against the retained immutable package, creating run rewards once.
    /// The caller must have independently replayed the exact sealed record.
    /// # Errors
    /// Wrong package/stage/generation, stale verification or invalid terminal facts.
    pub fn settled_against(
        &self,
        revision: u64,
        result: Settlement,
        catalog: &Catalog,
        seed: &[u8; 32],
        package: &crate::v2::package::Package,
    ) -> Result<Self, Refusal> {
        self.check(revision, catalog)?;
        if package.stage != self.shop.stage
            || package.cohort.generator != self.shop.generator
            || package.digest(catalog)? != result.binding.package
        {
            return Err(Refusal::Stale);
        }
        let reward = if self.shop.generator == crate::v2::generation::RUN {
            if package.builds[0].active != self.economy.inventory.active() {
                return Err(Refusal::Stale);
            }
            Some((
                super::Reward::sample(package, catalog, &result, seed)?,
                super::Income::freeze(&package.builds[0].active)?,
            ))
        } else {
            None
        };
        self.settlement(revision, result, catalog, seed, reward)
    }

    fn settlement(
        &self,
        revision: u64,
        result: Settlement,
        catalog: &Catalog,
        seed: &[u8; 32],
        reward: Option<(super::Reward, super::Income)>,
    ) -> Result<Self, Refusal> {
        self.check(revision, catalog)?;
        result.validate()?;
        let execution = self.execution.as_ref().ok_or(Refusal::Unavailable)?;
        if self.phase != Phase::Verifying {
            return Err(Refusal::Unavailable);
        }
        if result.binding
            != (Binding {
                package: execution.package,
                fence: execution.fence,
                prefix: execution.prefix,
            })
        {
            return Err(Refusal::Stale);
        }
        let mut next = self.clone();
        next.economy = self.economy.settled(result.used)?;
        if let Some((_, income)) = &reward {
            next.economy.gold = next
                .economy
                .gold
                .checked_add(income.bonus(result.outcome)?)
                .ok_or(Refusal::Capacity)?;
            next.income.push(*income);
        }
        next.history.push(result);
        next.execution = None;
        next.reward = reward.map(|(reward, _)| reward);
        if next.reward.is_some() {
            next.phase = Phase::Reward;
        } else {
            next.advance(catalog, seed)?;
        }
        next.revised(catalog)
    }

    pub(super) fn advance(&mut self, catalog: &Catalog, seed: &[u8; 32]) -> Result<(), Refusal> {
        self.phase = if self.wins() == usize::from(TARGET_WINS) {
            Phase::Conquered
        } else if self.defeats() == usize::from(DEFEAT_LIMIT) {
            Phase::Exhausted
        } else {
            Phase::Shop
        };
        if self.phase == Phase::Shop {
            let stage = u8::try_from(self.history.len() + 1).map_err(|_| Refusal::Capacity)?;
            self.shop = Shop::open(seed, stage, self.stock(catalog), Some(&self.shop))?;
        }
        Ok(())
    }
}
