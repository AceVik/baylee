//! Reflections of Littjara — {4}{U} — Enchantment
//! Oracle: As this enchantment enters, choose a creature type.
//! Oracle: Whenever you cast a spell of the chosen type, copy that spell. (A copy of a permanent spell becomes a token.)
//! Set: TDC #164 — Tarkir: Dragonstorm Commander | Scryfall ID: 578a1846-8c1a-4013-b669-1d3f4ddbbaa3 | Oracle ID: c3fdfb94-2d10-4743-864c-a59fdd57d8b7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(129),
    oracle_id: "c3fdfb94-2d10-4743-864c-a59fdd57d8b7",
    scryfall_id: "578a1846-8c1a-4013-b669-1d3f4ddbbaa3",
    faces: &[FaceDef {
        name: "Reflections of Littjara",
        mana_cost: baylee_core::mana!("{4}{U}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
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
