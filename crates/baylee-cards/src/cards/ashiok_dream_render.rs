//! Ashiok, Dream Render — {1}{U/B}{U/B} — Legendary Planeswalker — Ashiok
//! Oracle: Spells and abilities your opponents control can't cause their controller to search their library.
//! Oracle: −1: Target player mills four cards. Then exile each opponent's graveyard.
//! Set: WAR #228 — War of the Spark | Scryfall ID: f2df3258-c053-48a8-974f-d80899b2cd93 | Oracle ID: 93723b12-db34-4047-885e-8606415b1553
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(8),
    oracle_id: "93723b12-db34-4047-885e-8606415b1553",
    scryfall_id: "f2df3258-c053-48a8-974f-d80899b2cd93",
    faces: &[FaceDef {
        name: "Ashiok, Dream Render",
        mana_cost: baylee_core::mana!("{1}{U/B}{U/B}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::planeswalker::ASHIOK],
        power: None,
        toughness: None,
        loyalty: Some(5),
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue]),
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
