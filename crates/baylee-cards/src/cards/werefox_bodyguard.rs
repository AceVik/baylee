//! Werefox Bodyguard — {1}{W}{W} — Creature — Elf Fox Knight
//! Oracle: Flash
//! Oracle: When this creature enters, exile up to one other target non-Fox creature until this creature leaves the battlefield.
//! Oracle: {1}{W}, Sacrifice this creature: You gain 2 life.
//! Set: WOE #39 — Wilds of Eldraine | Scryfall ID: 4494dfa1-1343-417e-b0c5-2b096442dd0e | Oracle ID: d5ee2ced-29f4-430f-962e-2f930b92624c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(190),
    oracle_id: "d5ee2ced-29f4-430f-962e-2f930b92624c",
    scryfall_id: "4494dfa1-1343-417e-b0c5-2b096442dd0e",
    faces: &[FaceDef {
        name: "Werefox Bodyguard",
        mana_cost: baylee_core::mana!("{1}{W}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::creature::ELF,
            subtypes::creature::FOX,
            subtypes::creature::KNIGHT,
        ],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
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
