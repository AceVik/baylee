//! Esper Sentinel — {W} — Artifact Creature — Human Soldier
//! Oracle: Whenever an opponent casts their first noncreature spell each turn, draw a card unless that player pays {X}, where X is this creature's power.
//! Set: MH2 #12 — Modern Horizons 2 | Scryfall ID: f3537373-ef54-4578-9d05-6216420ee349 | Oracle ID: 5def9f38-0a0b-4e8d-9f9d-29dcb46520b4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(46),
    oracle_id: "5def9f38-0a0b-4e8d-9f9d-29dcb46520b4",
    scryfall_id: "f3537373-ef54-4578-9d05-6216420ee349",
    faces: &[FaceDef {
        name: "Esper Sentinel",
        mana_cost: baylee_core::mana!("{W}"),
        types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power: Some(1),
        toughness: Some(1),
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
