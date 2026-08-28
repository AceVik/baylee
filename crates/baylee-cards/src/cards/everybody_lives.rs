//! Everybody Lives! — {1}{W} — Instant
//! Oracle: All creatures gain hexproof and indestructible until end of turn. Players gain hexproof until end of turn. Players can't lose life this turn and players can't lose the game or win the game this turn.
//! Set: WHO #18 — Doctor Who | Scryfall ID: 9dab0052-7f0c-4b56-847f-20552666a271 | Oracle ID: 39213de3-6a4a-4879-a7f9-70f45013765e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(47),
    oracle_id: "39213de3-6a4a-4879-a7f9-70f45013765e",
    scryfall_id: "9dab0052-7f0c-4b56-847f-20552666a271",
    faces: &[FaceDef {
        name: "Everybody Lives!",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::INSTANT,
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
