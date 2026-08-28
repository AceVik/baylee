//! Indatha Triome — (no cost) — Land — Plains Swamp Forest
//! Oracle: ({T}: Add {W}, {B}, or {G}.)
//! Oracle: This land enters tapped.
//! Oracle: Cycling {3} ({3}, Discard this card: Draw a card.)
//! Set: IKO #248 — Ikoria: Lair of Behemoths | Scryfall ID: 2b74bb81-fb9a-40e5-a941-e517430b52f5 | Oracle ID: ec2b3779-55f7-4169-aa66-6312fb52721f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(72),
    oracle_id: "ec2b3779-55f7-4169-aa66-6312fb52721f",
    scryfall_id: "2b74bb81-fb9a-40e5-a941-e517430b52f5",
    faces: &[FaceDef {
        name: "Indatha Triome",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::land::PLAINS,
            subtypes::land::SWAMP,
            subtypes::land::FOREST,
        ],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green, Color::White]),
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
