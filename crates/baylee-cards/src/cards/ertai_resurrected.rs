//! Ertai Resurrected — {2}{U}{B} — Legendary Creature — Phyrexian Human Wizard
//! Oracle: Flash
//! Oracle: When Ertai Resurrected enters, choose up to one —
//! Oracle: • Counter target spell, activated ability, or triggered ability. Its controller draws a card.
//! Oracle: • Destroy another target creature or planeswalker. Its controller draws a card.
//! Set: DMU #199 — Dominaria United | Scryfall ID: 7f7e780e-fbc5-4dc0-b5c7-efcb8645c7c6 | Oracle ID: 3d038f7c-95fa-4b71-8f74-b9b4dd45cde0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(45),
    oracle_id: "3d038f7c-95fa-4b71-8f74-b9b4dd45cde0",
    scryfall_id: "7f7e780e-fbc5-4dc0-b5c7-efcb8645c7c6",
    faces: &[FaceDef {
        name: "Ertai Resurrected",
        mana_cost: baylee_core::mana!("{2}{U}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[
            subtypes::creature::PHYREXIAN,
            subtypes::creature::HUMAN,
            subtypes::creature::WIZARD,
        ],
        power: Some(3),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue]),
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
