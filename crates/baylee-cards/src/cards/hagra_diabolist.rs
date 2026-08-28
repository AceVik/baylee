//! Hagra Diabolist — {4}{B} — Creature — Ogre Shaman Ally
//! Oracle: Whenever this creature or another Ally you control enters, you may have target player lose life equal to the number of Allies you control.
//! Set: ZEN #95 — Zendikar | Scryfall ID: c303e7e2-cb22-4dea-889f-d03e2494ed0f | Oracle ID: 5e2c1e0e-0a10-416a-9b50-96ee0cbbc24e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(63),
    oracle_id: "5e2c1e0e-0a10-416a-9b50-96ee0cbbc24e",
    scryfall_id: "c303e7e2-cb22-4dea-889f-d03e2494ed0f",
    faces: &[FaceDef {
        name: "Hagra Diabolist",
        mana_cost: baylee_core::mana!("{4}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::creature::OGRE,
            subtypes::creature::SHAMAN,
            subtypes::creature::ALLY,
        ],
        power: Some(3),
        toughness: Some(2),
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
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
