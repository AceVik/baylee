//! Abandoned Air Temple — (no cost) — Land
//! Oracle: This land enters tapped unless you control a basic land.
//! Oracle: {T}: Add {W}.
//! Oracle: {3}{W}, {T}: Put a +1/+1 counter on each creature you control.
//! Set: TLA #263 — Avatar: The Last Airbender | Scryfall ID: 9c0433f9-8f1e-4a19-a83f-a41925f1b1a9 | Oracle ID: 9575d7ce-f26d-4b90-87a3-6329e9799572
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(1),
    oracle_id: "9575d7ce-f26d-4b90-87a3-6329e9799572",
    scryfall_id: "9c0433f9-8f1e-4a19-a83f-a41925f1b1a9",
    faces: &[FaceDef {
        name: "Abandoned Air Temple",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
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
