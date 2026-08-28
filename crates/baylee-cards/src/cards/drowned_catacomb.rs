//! Drowned Catacomb — (no cost) — Land
//! Oracle: This land enters tapped unless you control an Island or a Swamp.
//! Oracle: {T}: Add {U} or {B}.
//! Set: MSC #239 — Marvel Super Heroes Commander | Scryfall ID: ebea49ab-e5cf-46d9-ae35-226a7321ede0 | Oracle ID: 819fc966-434e-470f-91e9-a38df974ad17
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(36),
    oracle_id: "819fc966-434e-470f-91e9-a38df974ad17",
    scryfall_id: "ebea49ab-e5cf-46d9-ae35-226a7321ede0",
    faces: &[FaceDef {
        name: "Drowned Catacomb",
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
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue]),
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
