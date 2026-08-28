//! Isolated Chapel — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Plains or a Swamp.
//! Oracle: {T}: Add {W} or {B}.
//! Set: SOC #382 — Secrets of Strixhaven Commander | Scryfall ID: 78814c92-b52c-462a-866f-3e7da9db9f70 | Oracle ID: 7e5d9efe-48a9-434b-bb09-056e0e09cc9a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(75),
    oracle_id: "7e5d9efe-48a9-434b-bb09-056e0e09cc9a",
    scryfall_id: "78814c92-b52c-462a-866f-3e7da9db9f70",
    faces: &[FaceDef {
        name: "Isolated Chapel",
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
