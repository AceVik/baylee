//! Sokka, Tenacious Tactician — {1}{U}{R}{W} — Legendary Creature — Human Warrior Ally
//! Oracle: Menace, prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
//! Oracle: Other Allies you control have menace and prowess.
//! Oracle: Whenever you cast a noncreature spell, create a 1/1 white Ally creature token.
//! Set: TLA #242 — Avatar: The Last Airbender | Scryfall ID: f0fa5897-1da7-488f-bb19-1632e969c050 | Oracle ID: 6b68acc2-b9d5-495b-8054-c04bae1349f1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(149),
    oracle_id: "6b68acc2-b9d5-495b-8054-c04bae1349f1",
    scryfall_id: "f0fa5897-1da7-488f-bb19-1632e969c050",
    faces: &[FaceDef {
        name: "Sokka, Tenacious Tactician",
        mana_cost: baylee_core::mana!("{1}{U}{R}{W}"),
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Blue, Color::White]),
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
