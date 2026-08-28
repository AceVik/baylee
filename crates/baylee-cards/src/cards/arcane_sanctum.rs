//! Arcane Sanctum — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}, {U}, or {B}.
//! Set: DSC #259 — Duskmourn: House of Horror Commander | Scryfall ID: c75eeb97-3249-4762-84b0-387f27fb255f | Oracle ID: 7d7cf15c-06b9-4062-a1eb-32614c458a3b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(5),
    oracle_id: "7d7cf15c-06b9-4062-a1eb-32614c458a3b",
    scryfall_id: "c75eeb97-3249-4762-84b0-387f27fb255f",
    faces: &[FaceDef {
        name: "Arcane Sanctum",
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
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
