//! Urza's Saga — (no cost) — Enchantment Land — Urza's Saga
//! Oracle: (As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)
//! Oracle: I — This Saga gains "{T}: Add {C}."
//! Oracle: II — This Saga gains "{2}, {T}: Create a 0/0 colorless Construct artifact creature token with 'This token gets +1/+1 for each artifact you control.'"
//! Oracle: III — Search your library for an artifact card with mana cost {0} or {1}, put it onto the battlefield, then shuffle.
//! Set: MH2 #259 — Modern Horizons 2 | Scryfall ID: c1e0f201-42cb-46a1-901a-65bb4fc18f6c | Oracle ID: 4c6a0c30-b547-4eff-8ff4-0ca25803c076
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(179),
    oracle_id: "4c6a0c30-b547-4eff-8ff4-0ca25803c076",
    scryfall_id: "c1e0f201-42cb-46a1-901a-65bb4fc18f6c",
    faces: &[FaceDef {
        name: "Urza's Saga",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::ENCHANTMENT.union(TypeSet::LAND),
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::URZA_S, subtypes::enchantment::SAGA],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
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
