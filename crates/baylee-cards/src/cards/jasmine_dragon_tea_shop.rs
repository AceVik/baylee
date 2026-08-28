//! Jasmine Dragon Tea Shop — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast an Ally spell or activate an ability of an Ally source.
//! Oracle: {5}, {T}: Create a 1/1 white Ally creature token.
//! Set: TLA #270 — Avatar: The Last Airbender | Scryfall ID: da2c83d4-a95f-47ff-a08f-694eb78d6b9b | Oracle ID: d9a24444-289f-473f-9985-8df275257555
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(77),
    oracle_id: "d9a24444-289f-473f-9985-8df275257555",
    scryfall_id: "da2c83d4-a95f-47ff-a08f-694eb78d6b9b",
    faces: &[FaceDef {
        name: "Jasmine Dragon Tea Shop",
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
