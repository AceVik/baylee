//! Solitude — {3}{W}{W} — Creature — Elemental Incarnation
//! Oracle: Flash
//! Oracle: Lifelink
//! Oracle: When this creature enters, exile up to one other target creature. That creature's controller gains life equal to its power.
//! Oracle: Evoke—Exile a white card from your hand.
//! Set: MH2 #32 — Modern Horizons 2 | Scryfall ID: 47a6234f-309f-4e03-9263-66da48b57153 | Oracle ID: dcb9c2a7-ae54-4ddc-a567-640bf4bf4366
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(152),
    oracle_id: "dcb9c2a7-ae54-4ddc-a567-640bf4bf4366",
    scryfall_id: "47a6234f-309f-4e03-9263-66da48b57153",
    faces: &[FaceDef {
        name: "Solitude",
        mana_cost: baylee_core::mana!("{3}{W}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::creature::ELEMENTAL,
            subtypes::creature::INCARNATION,
        ],
        power: Some(3),
        toughness: Some(2),
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
