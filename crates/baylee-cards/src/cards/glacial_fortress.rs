//! Glacial Fortress — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Plains or an Island.
//! Oracle: {T}: Add {W} or {U}.
//! Set: MSC #248 — Marvel Super Heroes Commander | Scryfall ID: d673a2d5-0c61-48dc-8c8d-06f0c7b6b8bf | Oracle ID: 027dd013-baa7-4111-b3c9-f4d1414e9c45
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(59),
    oracle_id: "027dd013-baa7-4111-b3c9-f4d1414e9c45",
    scryfall_id: "d673a2d5-0c61-48dc-8c8d-06f0c7b6b8bf",
    faces: &[FaceDef {
        name: "Glacial Fortress",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::White]),
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
