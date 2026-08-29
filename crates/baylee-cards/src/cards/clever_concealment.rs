//! Clever Concealment — {2}{W}{W} — Instant
//! Oracle: Convoke (Your creatures can help cast this spell. Each creature you tap while casting this spell pays for {1} or one mana of that creature's color.)
//! Oracle: Any number of target nonland permanents you control phase out. (Treat them and anything attached to them as though they don't exist until your next turn.)
//! Set: MSC #125 — Marvel Super Heroes Commander | Scryfall ID: 41d45a8a-ea1d-4fbc-86d2-5d6340f3b639 | Oracle ID: 42bb7ea9-f6e4-4551-8d93-3b1eae84b865
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(22),
    oracle_id: "42bb7ea9-f6e4-4551-8d93-3b1eae84b865",
    scryfall_id: "41d45a8a-ea1d-4fbc-86d2-5d6340f3b639",
    faces: &[FaceDef {
        name: "Clever Concealment",
        mana_cost: baylee_core::mana!("{2}{W}{W}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
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
