//! Jace, the Mind Sculptor — {2}{U}{U} — Legendary Planeswalker — Jace
//! Oracle: +2: Look at the top card of target player's library. You may put that card on the bottom of that player's library.
//! Oracle: 0: Draw three cards, then put two cards from your hand on top of your library in any order.
//! Oracle: −1: Return target creature to its owner's hand.
//! Oracle: −12: Exile all cards from target player's library, then that player shuffles their hand into their library.
//! Set: 2XM #56 — Double Masters | Scryfall ID: c8817585-0d32-4d56-9142-0d29512e86a9 | Oracle ID: 7f77a84e-5a4b-4834-aefa-3cecc175ae8e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(76),
    oracle_id: "7f77a84e-5a4b-4834-aefa-3cecc175ae8e",
    scryfall_id: "c8817585-0d32-4d56-9142-0d29512e86a9",
    faces: &[FaceDef {
        name: "Jace, the Mind Sculptor",
        mana_cost: baylee_core::mana!("{2}{U}{U}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::planeswalker::JACE],
        power: None,
        toughness: None,
        loyalty: Some(3),
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
