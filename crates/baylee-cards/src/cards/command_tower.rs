//! Command Tower — (no cost) — Land
//! Oracle: {T}: Add one mana of any color in your commander's color identity.
//! Set: MSC #233 — Marvel Super Heroes Commander | Scryfall ID: 0548fb60-c843-4f8f-a029-6f10efc63a41 | Oracle ID: 0895c9b7-ae7d-4bb3-af17-3b75deb50a25
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(23),
    oracle_id: "0895c9b7-ae7d-4bb3-af17-3b75deb50a25",
    scryfall_id: "0548fb60-c843-4f8f-a029-6f10efc63a41",
    faces: &[FaceDef {
        name: "Command Tower",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
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
