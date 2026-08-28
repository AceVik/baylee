//! Zagoth Triome — (no cost) — Land — Swamp Forest Island
//! Oracle: ({T}: Add {B}, {G}, or {U}.)
//! Oracle: This land enters tapped.
//! Oracle: Cycling {3} ({3}, Discard this card: Draw a card.)
//! Set: IKO #259 — Ikoria: Lair of Behemoths | Scryfall ID: cc520518-2063-4b57-a0d4-10cf62a7175e | Oracle ID: fdd46004-eaba-4024-8687-39b23dc6a58c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(193),
    oracle_id: "fdd46004-eaba-4024-8687-39b23dc6a58c",
    scryfall_id: "cc520518-2063-4b57-a0d4-10cf62a7175e",
    faces: &[FaceDef {
        name: "Zagoth Triome",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::land::SWAMP,
            subtypes::land::FOREST,
            subtypes::land::ISLAND,
        ],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green, Color::Blue]),
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
