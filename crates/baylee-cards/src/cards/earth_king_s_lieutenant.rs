//! Earth King's Lieutenant — {G}{W} — Creature — Human Soldier Ally
//! Oracle: Trample
//! Oracle: When this creature enters, put a +1/+1 counter on each other Ally creature you control.
//! Oracle: Whenever another Ally you control enters, put a +1/+1 counter on this creature.
//! Set: TLA #217 — Avatar: The Last Airbender | Scryfall ID: 4533d155-5c56-41a5-9d76-2d1414ac47c9 | Oracle ID: 9da9248d-1201-447f-b6c2-2b64af4f71c4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(37),
    oracle_id: "9da9248d-1201-447f-b6c2-2b64af4f71c4",
    scryfall_id: "4533d155-5c56-41a5-9d76-2d1414ac47c9",
    faces: &[FaceDef {
        name: "Earth King's Lieutenant",
        mana_cost: baylee_core::mana!("{G}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::creature::HUMAN,
            subtypes::creature::SOLDIER,
            subtypes::creature::ALLY,
        ],
        power: Some(1),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
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
