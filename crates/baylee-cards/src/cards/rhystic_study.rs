//! Rhystic Study — {2}{U} — Enchantment
//! Oracle: Whenever an opponent casts a spell, you may draw a card unless that player pays {1}.
//! Set: J22 #114 — Jumpstart 2022 | Scryfall ID: 9f37c5b6-a59c-45cd-9a99-e9357fe9ea1b | Oracle ID: 53236dd7-845a-444c-96d5-f41ed7325d8f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(133),
    oracle_id: "53236dd7-845a-444c-96d5-f41ed7325d8f",
    scryfall_id: "9f37c5b6-a59c-45cd-9a99-e9357fe9ea1b",
    faces: &[FaceDef {
        name: "Rhystic Study",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
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
