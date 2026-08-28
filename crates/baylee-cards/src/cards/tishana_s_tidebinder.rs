//! Tishana's Tidebinder — {2}{U} — Creature — Merfolk Wizard
//! Oracle: Flash
//! Oracle: When this creature enters, counter up to one target activated or triggered ability. If an ability of an artifact, creature, or planeswalker is countered this way, that permanent loses all abilities for as long as this creature remains on the battlefield. (Mana abilities can't be targeted.)
//! Set: LCI #81 — The Lost Caverns of Ixalan | Scryfall ID: 907b3d1d-8c85-4707-80b5-c4d832df9846 | Oracle ID: 2993dc7d-723d-4a9b-94bd-4bb02a9f7243
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(170),
    oracle_id: "2993dc7d-723d-4a9b-94bd-4bb02a9f7243",
    scryfall_id: "907b3d1d-8c85-4707-80b5-c4d832df9846",
    faces: &[FaceDef {
        name: "Tishana's Tidebinder",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::MERFOLK, subtypes::creature::WIZARD],
        power: Some(3),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
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
