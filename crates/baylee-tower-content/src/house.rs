//! The house's stand-in builds for a development or demonstration cohort.
//!
//! A cohort needs a fallback build per stage and archetype before any player
//! has left a ghost there (`docs/tower-operations.md` §`publish_house`). A
//! production cohort publishes reviewed builds; this ladder is what the local
//! stack and the gateway's held demo publish, written once.
use baylee_tower::{
    cards::Collection,
    v2::{
        catalog::{Definition, Family, Rarity},
        package::Build,
    },
};

/// The starter's own deck, with the equipment a run would plausibly hold by
/// `stage`: Heartwood from the second, Grove from the fourth, Compass from the
/// tenth, the first two uncommon from the seventh. No balance claim is made.
#[must_use]
pub fn development_house(starter: &Collection, stage: u8) -> Build {
    let rarity = if stage >= 7 {
        Rarity::Uncommon
    } else {
        Rarity::Common
    };
    let mut active = Vec::with_capacity(3);
    if stage >= 2 {
        active.push(Definition {
            family: Family::Heartwood,
            rarity,
        });
    }
    if stage >= 4 {
        active.push(Definition {
            family: Family::Grove,
            rarity,
        });
    }
    if stage >= 10 {
        active.push(Definition {
            family: Family::Compass,
            rarity: Rarity::Common,
        });
    }
    Build {
        formation: starter.formation.clone(),
        deck: starter.deck.clone(),
        active,
        potions: [0; 2],
    }
}

mod ladder;
mod worth;
pub use ladder::{Ladder, PATH, PICKS, TRIMS, candidates, check, ladder, trimmed};
