//! Wartime Protestors — {3}{R} — Creature — Human Rebel Ally
//! Oracle: Haste
//! Oracle: Whenever another Ally you control enters, put a +1/+1 counter on that creature and it gains haste until end of turn.
//! Set: TLA #160 — Avatar: The Last Airbender | Scryfall ID: bac81940-d717-49ff-83b2-16a22bb2c988 | Oracle ID: 6557813b-4ee7-4881-a37c-10c8ea097360
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(187),
    oracle_id: "6557813b-4ee7-4881-a37c-10c8ea097360",
    scryfall_id: "bac81940-d717-49ff-83b2-16a22bb2c988",
    faces: &[FaceDef {
        name: "Wartime Protestors",
        mana_cost: baylee_core::mana!("{3}{R}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::creature::HUMAN,
            subtypes::creature::REBEL,
            subtypes::creature::ALLY,
        ],
        power: Some(4),
        toughness: Some(4),
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Red]),
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
