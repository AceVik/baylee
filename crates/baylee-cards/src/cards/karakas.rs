//! Karakas — (no cost) — Legendary Land
//! Oracle: {T}: Add {W}.
//! Oracle: {T}: Return target legendary creature to its owner's hand.
//! Set: UMA #244 — Ultimate Masters | Scryfall ID: e52214e1-404a-405a-b08e-20e13c087338 | Oracle ID: 59119143-c0fa-49dd-adf0-e2fd3029c48b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(79),
    oracle_id: "59119143-c0fa-49dd-adf0-e2fd3029c48b",
    scryfall_id: "e52214e1-404a-405a-b08e-20e13c087338",
    faces: &[FaceDef {
        name: "Karakas",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
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
