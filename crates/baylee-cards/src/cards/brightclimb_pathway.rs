//! Brightclimb Pathway // Grimclimb Pathway — (no cost) — Land // Land
//! Set: ZNR #259 — Zendikar Rising | Scryfall ID: d24c3d51-795d-4c01-a34a-3280fccd2d78 | Oracle ID: 1c633e02-95ef-445e-b4e0-fbfbc5ed9cc9
//! Face: Brightclimb Pathway —  — Land
//! Face: Grimclimb Pathway —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(16),
    oracle_id: "1c633e02-95ef-445e-b4e0-fbfbc5ed9cc9",
    scryfall_id: "d24c3d51-795d-4c01-a34a-3280fccd2d78",
    faces: &[
        FaceDef {
            name: "Brightclimb Pathway",
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
        },
        FaceDef {
            name: "Grimclimb Pathway",
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
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
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
