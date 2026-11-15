//! Deliberately excludes execution leases, package commitments and private record heads.
use baylee_tower::{
    cards::Collection,
    v2::{
        economy::Economy,
        run::{Claim, Outcome, Phase},
        shop::Shop,
    },
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One independently verified duel in the player's history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Duel {
    /// Actual rules outcome.
    pub outcome: Outcome,
    /// Rules turns, never used as an administrative timeout.
    pub turns: u32,
    /// Healing/mana potions actually consumed in this duel.
    pub used: [u8; 2],
    /// Additional gold from the player's equipped items at this duel's reservation.
    #[serde(default)]
    pub bonus_gold: u16,
    /// The owner's completed card choice, absent until chosen or for a legacy duel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<Claim>,
}

/// Only the intentionally revealed reward; no opponent deck or package commitment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reward {
    /// At most five distinct eligible cards from the settled opponent.
    pub cards: Vec<baylee_tower::cards::CardIndex>,
    /// Exact number to choose, reduced for an opponent with fewer eligible cards.
    pub take: u8,
}

/// Owner projection. It cannot be submitted as an authoritative replacement state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// Persistent run identity.
    pub id: Uuid,
    /// Immutable enrollment identity.
    pub cohort: Uuid,
    /// Required optimistic command version.
    pub revision: u64,
    /// Current available activity.
    pub phase: Phase,
    /// Chronological verified duels, at most twelve.
    pub history: Vec<Duel>,
    /// Run-local wallet, items and potion charges.
    pub economy: Economy,
    /// Owned cards and the chosen deck.
    pub cards: Collection,
    /// Exact saved offers; reconnecting cannot refresh them.
    pub shop: Shop,
    /// Persisted post-duel card choice, present only in the Reward phase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reward: Option<Reward>,
}

impl Run {
    /// Consume a validated owner state, explicitly selecting player-visible fields.
    #[must_use]
    pub fn project(id: Uuid, cohort: Uuid, state: baylee_tower::v2::run::Run) -> Self {
        Self {
            id,
            cohort,
            revision: state.revision,
            phase: state.phase,
            history: state
                .history
                .into_iter()
                .enumerate()
                .map(|(index, duel)| Duel {
                    outcome: duel.outcome,
                    turns: duel.turns,
                    used: duel.used,
                    bonus_gold: state
                        .income
                        .get(index)
                        .and_then(|income| income.bonus(duel.outcome).ok())
                        .unwrap_or(0),
                    claim: state.claims.get(index).cloned(),
                })
                .collect(),
            economy: state.economy,
            cards: state.cards,
            shop: state.shop,
            reward: state.reward.map(|reward| Reward {
                cards: reward.cards,
                take: reward.take,
            }),
        }
    }
}
