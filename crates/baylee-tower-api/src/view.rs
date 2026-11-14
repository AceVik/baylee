//! Deliberately excludes execution leases, package commitments and private record heads.
use baylee_tower::{
    cards::Collection,
    v2::{
        economy::Economy,
        run::{Outcome, Phase},
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
                .map(|duel| Duel {
                    outcome: duel.outcome,
                    turns: duel.turns,
                    used: duel.used,
                })
                .collect(),
            economy: state.economy,
            cards: state.cards,
            shop: state.shop,
        }
    }
}
