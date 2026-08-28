//! Luminarch Ascension — {1}{W} — Enchantment
//! Oracle: At the beginning of each opponent's end step, if you didn't lose life this turn, you may put a quest counter on this enchantment. (Damage causes loss of life.)
//! Oracle: {1}{W}: Create a 4/4 white Angel creature token with flying. Activate only if this enchantment has four or more quest counters on it.
//! Set: A25 #23 — Masters 25 | Scryfall ID: b3770d86-4496-4c06-aab1-2917cfec100e | Oracle ID: 90076bf5-aa9a-4a6e-9035-9aa97fd5561e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(88),
    oracle_id: "90076bf5-aa9a-4a6e-9035-9aa97fd5561e",
    scryfall_id: "b3770d86-4496-4c06-aab1-2917cfec100e",
    faces: &[FaceDef {
        name: "Luminarch Ascension",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::ENCHANTMENT,
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
