//! Island — (no cost) — Basic Land — Island
//! Oracle: ({T}: Add {U}.)
//! Set: TRK #319 — Star Trek | Scryfall ID: f3cc07cd-cc79-4745-b0b7-eade60175cc3 | Oracle ID: b2c6aa39-2d2a-459c-a555-fb48ba993373
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(74),
    oracle_id: "b2c6aa39-2d2a-459c-a555-fb48ba993373",
    scryfall_id: "f3cc07cd-cc79-4745-b0b7-eade60175cc3",
    faces: &[FaceDef {
        name: "Island",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[subtypes::land::ISLAND],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
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
