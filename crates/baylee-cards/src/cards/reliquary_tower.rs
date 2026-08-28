//! Reliquary Tower — (no cost) — Land
//! Oracle: You have no maximum hand size.
//! Oracle: {T}: Add {C}.
//! Set: SOC #398 — Secrets of Strixhaven Commander | Scryfall ID: e2a27742-08c1-4153-af7f-25a7a98f585e | Oracle ID: c23e5b80-08d2-4e24-9908-fe2aa4f30f6f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(130),
    oracle_id: "c23e5b80-08d2-4e24-9908-fe2aa4f30f6f",
    scryfall_id: "e2a27742-08c1-4153-af7f-25a7a98f585e",
    faces: &[FaceDef {
        name: "Reliquary Tower",
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
