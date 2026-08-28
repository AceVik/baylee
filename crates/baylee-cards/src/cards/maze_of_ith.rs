//! Maze of Ith — (no cost) — Land
//! Oracle: {T}: Untap target attacking creature. Prevent all combat damage that would be dealt to and dealt by that creature this turn.
//! Set: DMR #250 — Dominaria Remastered | Scryfall ID: 5889fde1-730d-43d0-aaa4-499784a80530 | Oracle ID: 38a12bd7-4394-44a8-91a0-6a4ff7fa4f71
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(93),
    oracle_id: "38a12bd7-4394-44a8-91a0-6a4ff7fa4f71",
    scryfall_id: "5889fde1-730d-43d0-aaa4-499784a80530",
    faces: &[FaceDef {
        name: "Maze of Ith",
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
    color_identity: ColorSet::EMPTY,
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
