//! Reveillark — {4}{W} — Creature — Elemental
//! Oracle: Flying
//! Oracle: When this creature leaves the battlefield, return up to two target creature cards with power 2 or less from your graveyard to the battlefield.
//! Oracle: Evoke {5}{W} (You may cast this spell for its evoke cost. If you do, it's sacrificed when it enters.)
//! Set: 2X2 #26 — Double Masters 2022 | Scryfall ID: 53b4dcd6-b1b6-4f1c-9264-e58bdc87399b | Oracle ID: 1be13ede-98f8-497e-800c-03e5802932b3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(132),
    oracle_id: "1be13ede-98f8-497e-800c-03e5802932b3",
    scryfall_id: "53b4dcd6-b1b6-4f1c-9264-e58bdc87399b",
    faces: &[FaceDef {
        name: "Reveillark",
        mana_cost: baylee_core::mana!("{4}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::ELEMENTAL],
        power: Some(4),
        toughness: Some(3),
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
