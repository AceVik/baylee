//! Brainstorm — {U} — Instant
//! Oracle: Draw three cards, then put two cards from your hand on top of your library in any order.
//! Set: TLE #155 — Avatar: The Last Airbender Eternal | Scryfall ID: b5545882-6963-4729-b2c6-fb4bdc75ffcc | Oracle ID: 36cd2364-d113-47d1-b2c4-b088d9eb88dd
// IMPLEMENTED — draw 3, put 2 back on top (chosen order).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(15),
    oracle_id: "36cd2364-d113-47d1-b2c4-b088d9eb88dd",
    scryfall_id: "b5545882-6963-4729-b2c6-fb4bdc75ffcc",
    faces: &[FaceDef {
        name: "Brainstorm",
        mana_cost: baylee_core::mana!("{U}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
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
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[
            Effect::DrawCards {
                amount: Amount::Fixed(3),
            },
            Effect::PutFromHandOnTop { count: 2 },
        ],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {
    // Engine-level coverage via s4 scenario tests: draw 3 then put 2 back;
    // the top card of the library afterwards is the second chosen card.
}
