//! Thief of Blood — {4}{B}{B} — Creature — Vampire
//! Oracle: Flying
//! Oracle: As this creature enters, remove all counters from all permanents. This creature enters with a +1/+1 counter on it for each counter removed this way.
//! Set: CMA #71 — Commander Anthology | Scryfall ID: 1625be56-a8e9-44f3-a213-b758bffd447f | Oracle ID: 97d61346-bd53-4eb8-a920-6ae0382eb20d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(169),
    oracle_id: "97d61346-bd53-4eb8-a920-6ae0382eb20d",
    scryfall_id: "1625be56-a8e9-44f3-a213-b758bffd447f",
    faces: &[FaceDef {
        name: "Thief of Blood",
        mana_cost: baylee_core::mana!("{4}{B}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::VAMPIRE],
        power: Some(1),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
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
