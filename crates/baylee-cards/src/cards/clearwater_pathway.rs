//! Clearwater Pathway // Murkwater Pathway — (no cost) — Land // Land
//! Set: ZNR #260 — Zendikar Rising | Scryfall ID: b4b99ebb-0d54-4fe5-a495-979aaa564aa8 | Oracle ID: 144119bc-7fd1-45c5-9e29-f742e7c255ac
//! Face: Clearwater Pathway —  — Land
//! Face: Murkwater Pathway —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(21),
    oracle_id: "144119bc-7fd1-45c5-9e29-f742e7c255ac",
    scryfall_id: "b4b99ebb-0d54-4fe5-a495-979aaa564aa8",
    faces: &[
        FaceDef {
            name: "Clearwater Pathway",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::LAND,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[],
            power: None,
            toughness: None,
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
        enter_modifiers: &[],
        },
        FaceDef {
            name: "Murkwater Pathway",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::LAND,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[],
            power: None,
            toughness: None,
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
        enter_modifiers: &[],
        },
    ],
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
