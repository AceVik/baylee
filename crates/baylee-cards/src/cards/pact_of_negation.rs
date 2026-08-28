//! Pact of Negation — {0} — Instant
//! Oracle: Counter target spell.
//! Oracle: At the beginning of your next upkeep, pay {3}{U}{U}. If you don't, you lose the game.
//! Set: TSR #77 — Time Spiral Remastered | Scryfall ID: 1ed4c0bb-b710-44a1-b8bc-6bd11c27b8b8 | Oracle ID: f3e213a4-ba5a-468a-93b3-c0a34e1bd725
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(107),
    oracle_id: "f3e213a4-ba5a-468a-93b3-c0a34e1bd725",
    scryfall_id: "1ed4c0bb-b710-44a1-b8bc-6bd11c27b8b8",
    faces: &[FaceDef {
        name: "Pact of Negation",
        mana_cost: baylee_core::mana!("{0}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
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
