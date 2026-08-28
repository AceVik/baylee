//! Bojuka Bog — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, exile target player's graveyard.
//! Oracle: {T}: Add {B}.
//! Set: SOC #363 — Secrets of Strixhaven Commander | Scryfall ID: 55b5b094-9d2d-4d96-b90c-78fecdae725a | Oracle ID: 04b7362d-0490-4cb0-b5d7-2a7732f659ce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(14),
    oracle_id: "04b7362d-0490-4cb0-b5d7-2a7732f659ce",
    scryfall_id: "55b5b094-9d2d-4d96-b90c-78fecdae725a",
    faces: &[FaceDef {
        name: "Bojuka Bog",
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
    color_identity: ColorSet::from_slice(&[Color::Black]),
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
