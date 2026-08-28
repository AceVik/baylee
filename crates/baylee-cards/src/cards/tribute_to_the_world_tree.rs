//! Tribute to the World Tree — {G}{G}{G} — Enchantment
//! Oracle: Whenever a creature you control enters, draw a card if its power is 3 or greater. Otherwise, put two +1/+1 counters on it.
//! Set: MOM #211 — March of the Machine | Scryfall ID: c0cdeaba-fc21-44e6-bf99-aa1ff379401b | Oracle ID: 72deedab-7c17-4505-aeca-4bc8596d80a5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(173),
    oracle_id: "72deedab-7c17-4505-aeca-4bc8596d80a5",
    scryfall_id: "c0cdeaba-fc21-44e6-bf99-aa1ff379401b",
    faces: &[FaceDef {
        name: "Tribute to the World Tree",
        mana_cost: baylee_core::mana!("{G}{G}{G}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
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
