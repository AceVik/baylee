//! Sampling occurs once at verified settlement. Owner commands name only saved
//! card identities, never an opponent, a random seed, an outcome or a gold value.
use super::{Catalog, Outcome, Phase, Refusal, Run, Settlement};
use crate::{cards::Collection, v2::package::Package};
use baylee_core::ids::CardIndex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Private pending reward, bound to the exact independently verified package.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reward {
    package: [u8; 32],
    /// At most five distinct nonbasic identities, in saved reveal order.
    pub cards: Vec<CardIndex>,
    /// Three for victory, one otherwise, reduced only for a smaller eligible deck.
    pub take: u8,
}

/// Retained owner choice. Duplicate owned cards yield one gold each instead.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    /// Chosen cards, in canonical identity order.
    pub cards: Vec<CardIndex>,
    /// Number already owned at claim time; also the exact bonus gold granted.
    pub duplicates: u8,
}

const fn allowance(outcome: Outcome) -> u8 {
    if matches!(outcome, Outcome::Victory) {
        3
    } else {
        1
    }
}

impl Reward {
    pub(super) fn sample(
        package: &Package,
        catalog: &Catalog,
        result: &Settlement,
        seed: &[u8; 32],
    ) -> Result<Self, Refusal> {
        if package.digest(catalog)? != result.binding.package {
            return Err(Refusal::Stale);
        }
        let opponent = &package.builds[1];
        let mut candidates: Vec<_> = opponent
            .deck
            .iter()
            .map(|stack| stack.card)
            .chain(
                opponent
                    .formation
                    .iter()
                    .map(|formation| formation.commander),
            )
            .filter(|card| catalog.card(*card).is_some_and(|entry| !entry.basic_land))
            .collect();
        candidates.sort_unstable();
        candidates.dedup();
        let mut scored: Vec<([u8; 32], CardIndex)> = candidates
            .into_iter()
            .map(|card| {
                let score = Sha256::new()
                    .chain_update(b"baylee/tower/card-reward/v1\0")
                    .chain_update(seed)
                    .chain_update(result.binding.package)
                    .chain_update(card.get().to_le_bytes())
                    .finalize()
                    .into();
                (score, card)
            })
            .collect();
        scored.sort_unstable();
        let cards: Vec<_> = scored.into_iter().take(5).map(|(_, card)| card).collect();
        let take = allowance(result.outcome)
            .min(u8::try_from(cards.len()).map_err(|_| Refusal::Capacity)?);
        Ok(Self {
            package: result.binding.package,
            cards,
            take,
        })
    }

    fn validate(&self, catalog: &Catalog, result: &Settlement) -> Result<(), Refusal> {
        if self.package != result.binding.package
            || self.cards.len() > 5
            || usize::from(self.take)
                != usize::from(allowance(result.outcome)).min(self.cards.len())
        {
            return Err(Refusal::Invalid);
        }
        unique_nonbasics(&self.cards, catalog)
    }

    /// Pure advisory/authority check for the exact owner choice.
    pub fn accepts(&self, cards: &[CardIndex]) -> bool {
        cards.len() == usize::from(self.take)
            && cards
                .iter()
                .enumerate()
                .all(|(index, card)| self.cards.contains(card) && !cards[..index].contains(card))
    }
}

fn unique_nonbasics(cards: &[CardIndex], catalog: &Catalog) -> Result<(), Refusal> {
    if cards.iter().enumerate().any(|(index, card)| {
        cards[..index].contains(card) || catalog.card(*card).is_none_or(|entry| entry.basic_land)
    }) {
        return Err(Refusal::Invalid);
    }
    Ok(())
}

impl Run {
    pub(super) fn validate_rewards(&self, catalog: &Catalog) -> Result<u16, Refusal> {
        if !crate::v2::generation::is_run(self.shop.generator) {
            return if self.reward.is_none() && self.claims.is_empty() {
                Ok(0)
            } else {
                Err(Refusal::Invalid)
            };
        }
        let pending = self.phase == Phase::Reward;
        if pending != self.reward.is_some()
            || self.claims.len() + usize::from(pending) != self.history.len()
        {
            return Err(Refusal::Invalid);
        }
        if let Some(reward) = &self.reward {
            reward.validate(catalog, self.history.last().ok_or(Refusal::Invalid)?)?;
        }
        let mut gold = 0;
        for (claim, result) in self.claims.iter().zip(&self.history) {
            if claim.cards.len() > usize::from(allowance(result.outcome))
                || usize::from(claim.duplicates) > claim.cards.len()
                || claim.cards.windows(2).any(|pair| pair[0] >= pair[1])
                || claim.cards.iter().any(|card| self.cards.copies(*card) != 1)
            {
                return Err(Refusal::Invalid);
            }
            unique_nonbasics(&claim.cards, catalog)?;
            gold += u16::from(claim.duplicates);
        }
        Ok(gold)
    }

    pub(super) fn claimed(
        &self,
        cards: &[CardIndex],
        catalog: &Catalog,
        seed: &[u8; 32],
    ) -> Result<Self, Refusal> {
        if self.phase != Phase::Reward
            || !self
                .reward
                .as_ref()
                .is_some_and(|reward| reward.accepts(cards))
        {
            return Err(Refusal::Unavailable);
        }
        let mut next = self.clone();
        let mut claim = Claim {
            cards: cards.to_vec(),
            duplicates: 0,
        };
        claim.cards.sort_unstable();
        for &card in &claim.cards {
            if next.cards.copies(card) == 0 {
                next.cards = grant(&next.cards, catalog, card)?;
            } else {
                claim.duplicates += 1;
            }
        }
        next.economy.gold = next
            .economy
            .gold
            .checked_add(u16::from(claim.duplicates))
            .ok_or(Refusal::Capacity)?;
        next.claims.push(claim);
        next.reward = None;
        next.advance(catalog, seed)?;
        next.revised(catalog)
    }
}

fn grant(cards: &Collection, catalog: &Catalog, card: CardIndex) -> Result<Collection, Refusal> {
    // The same checked owned-card transition as an admitted purchase, without a
    // price operand. Only the verified pending-reward branch reaches this grant.
    cards.bought(catalog, card)
}

#[cfg(test)]
mod tests;
