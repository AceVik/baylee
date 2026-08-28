//! General Tazri — {4}{W} — Legendary Creature — Human Ally
//! Oracle: When General Tazri enters, you may search your library for an Ally creature card, reveal it, put it into your hand, then shuffle.
//! Oracle: {W}{U}{B}{R}{G}: Ally creatures you control get +X/+X until end of turn, where X is the number of colors among those creatures.
//! Set: OGW #19 — Oath of the Gatewatch | Scryfall ID: 34e9aa86-1a31-4c0f-928d-923f066286b6 | Oracle ID: b0f19cba-1339-4518-8320-d7b1dcaf2eb0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(57),
    oracle_id: "b0f19cba-1339-4518-8320-d7b1dcaf2eb0",
    scryfall_id: "34e9aa86-1a31-4c0f-928d-923f066286b6",
    faces: &[FaceDef {
        name: "General Tazri",
        mana_cost: baylee_core::mana!("{4}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::ALLY],
        power: Some(3),
        toughness: Some(4),
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White,
    ]),
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
