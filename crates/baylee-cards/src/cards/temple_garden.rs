//! Temple Garden — (no cost) — Land — Forest Plains
//! Oracle: ({T}: Add {G} or {W}.)
//! Oracle: As this land enters, you may pay 2 life. If you don't, it enters tapped.
//! Set: TRK #301 — Star Trek | Scryfall ID: b9b0589d-f327-46a7-8bac-06b7654c547a | Oracle ID: f413a83d-a40d-434c-b20a-4c707c0527fa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(167),
    oracle_id: "f413a83d-a40d-434c-b20a-4c707c0527fa",
    scryfall_id: "b9b0589d-f327-46a7-8bac-06b7654c547a",
    faces: &[FaceDef {
        name: "Temple Garden",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::FOREST, subtypes::land::PLAINS],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
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
