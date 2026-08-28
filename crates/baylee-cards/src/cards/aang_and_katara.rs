//! Aang and Katara — {3}{G}{W}{U} — Legendary Creature — Human Avatar Ally
//! Oracle: Whenever Aang and Katara enter or attack, create X 1/1 white Ally creature tokens, where X is the number of tapped artifacts and/or creatures you control.
//! Set: TLE #69 — Avatar: The Last Airbender Eternal | Scryfall ID: f333ea01-124f-4125-87ab-609be40e774c | Oracle ID: 481c3e14-b670-4fab-aa9f-6ce5b514096d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(0),
    oracle_id: "481c3e14-b670-4fab-aa9f-6ce5b514096d",
    scryfall_id: "f333ea01-124f-4125-87ab-609be40e774c",
    faces: &[FaceDef {
        name: "Aang and Katara",
        mana_cost: baylee_core::mana!("{3}{G}{W}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[
            subtypes::creature::HUMAN,
            subtypes::creature::AVATAR,
            subtypes::creature::ALLY,
        ],
        power: Some(5),
        toughness: Some(5),
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
