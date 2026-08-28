//! Fetid Heath — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {W/B}, {T}: Add {W}{W}, {W}{B}, or {B}{B}.
//! Set: SOC #372 — Secrets of Strixhaven Commander | Scryfall ID: f465ded8-0d38-42ac-bafc-a12185013c5d | Oracle ID: 42bf259d-4bb9-49c3-b4ec-223dca62f4d6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(49),
    oracle_id: "42bf259d-4bb9-49c3-b4ec-223dca62f4d6",
    scryfall_id: "f465ded8-0d38-42ac-bafc-a12185013c5d",
    faces: &[FaceDef {
        name: "Fetid Heath",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
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
