//! Void Rend — {W}{U}{B} — Instant
//! Oracle: This spell can't be countered.
//! Oracle: Destroy target nonland permanent.
//! Set: SNC #230 — Streets of New Capenna | Scryfall ID: 2daab74d-d66b-4164-aa19-24e8d5536f7d | Oracle ID: 713f16db-95ec-479e-a48c-7a69f7668d7f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(185),
    oracle_id: "713f16db-95ec-479e-a48c-7a69f7668d7f",
    scryfall_id: "2daab74d-d66b-4164-aa19-24e8d5536f7d",
    faces: &[FaceDef {
        name: "Void Rend",
        mana_cost: baylee_core::mana!("{W}{U}{B}"),
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
