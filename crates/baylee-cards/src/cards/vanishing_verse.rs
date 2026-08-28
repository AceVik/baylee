//! Vanishing Verse — {W}{B} — Instant
//! Oracle: Exile target monocolored permanent.
//! Set: SOC #335 — Secrets of Strixhaven Commander | Scryfall ID: 8a475868-a335-45e7-9d59-9dc4c2cea1ae | Oracle ID: 5b8f0cdf-572d-4025-b930-79291f7c35be
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(180),
    oracle_id: "5b8f0cdf-572d-4025-b930-79291f7c35be",
    scryfall_id: "8a475868-a335-45e7-9d59-9dc4c2cea1ae",
    faces: &[FaceDef {
        name: "Vanishing Verse",
        mana_cost: baylee_core::mana!("{W}{B}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
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
