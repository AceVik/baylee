//! Metamorphosis Fanatic — {4}{B}{B} — Creature — Human Cleric
//! Oracle: Lifelink
//! Oracle: When this creature enters, return up to one target creature card from your graveyard to the battlefield with a lifelink counter on it.
//! Oracle: Miracle {1}{B} (You may cast this card for its miracle cost when you draw it if it's the first card you drew this turn.)
//! Set: DSC #21 — Duskmourn: House of Horror Commander | Scryfall ID: 16448d95-ee21-4def-b880-26f6f159c213 | Oracle ID: 017aa9b3-a8ea-4588-9c50-e914a7d8e4ee
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(94),
    oracle_id: "017aa9b3-a8ea-4588-9c50-e914a7d8e4ee",
    scryfall_id: "16448d95-ee21-4def-b880-26f6f159c213",
    faces: &[FaceDef {
        name: "Metamorphosis Fanatic",
        mana_cost: baylee_core::mana!("{4}{B}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power: Some(4),
        toughness: Some(4),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
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
