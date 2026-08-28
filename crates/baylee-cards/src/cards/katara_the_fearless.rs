//! Katara, the Fearless — {G}{W}{U} — Legendary Creature — Human Warrior Ally
//! Oracle: If a triggered ability of an Ally you control triggers, that ability triggers an additional time.
//! Set: TLA #230 — Avatar: The Last Airbender | Scryfall ID: b0a18f8b-7364-4375-b2e1-e2f15978517f | Oracle ID: 0972d46e-423b-454e-87c7-a2d40fb6fb6d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(82),
    oracle_id: "0972d46e-423b-454e-87c7-a2d40fb6fb6d",
    scryfall_id: "b0a18f8b-7364-4375-b2e1-e2f15978517f",
    faces: &[FaceDef {
        name: "Katara, the Fearless",
        mana_cost: baylee_core::mana!("{G}{W}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[
            subtypes::creature::HUMAN,
            subtypes::creature::WARRIOR,
            subtypes::creature::ALLY,
        ],
        power: Some(3),
        toughness: Some(3),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
