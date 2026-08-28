//! Sheoldred, the Apocalypse — {2}{B}{B} — Legendary Creature — Phyrexian Praetor
//! Oracle: Deathtouch
//! Oracle: Whenever you draw a card, you gain 2 life.
//! Oracle: Whenever an opponent draws a card, they lose 2 life.
//! Set: DMU #107 — Dominaria United | Scryfall ID: d67be074-cdd4-41d9-ac89-0a0456c4e4b2 | Oracle ID: 34f34409-326d-4994-a0ea-1a69aa278f03
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(145),
    oracle_id: "34f34409-326d-4994-a0ea-1a69aa278f03",
    scryfall_id: "d67be074-cdd4-41d9-ac89-0a0456c4e4b2",
    faces: &[FaceDef {
        name: "Sheoldred, the Apocalypse",
        mana_cost: baylee_core::mana!("{2}{B}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::PHYREXIAN, subtypes::creature::PRAETOR],
        power: Some(4),
        toughness: Some(5),
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
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
