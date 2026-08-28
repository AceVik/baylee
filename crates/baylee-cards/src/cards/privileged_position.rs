//! Privileged Position — {2}{G/W}{G/W}{G/W} — Enchantment
//! Oracle: ({G/W} can be paid with either {G} or {W}.)
//! Oracle: Other permanents you control have hexproof. (They can't be the targets of spells or abilities your opponents control.)
//! Set: 2X2 #263 — Double Masters 2022 | Scryfall ID: 9655bbe4-062f-4278-ad05-a326a64c5b69 | Oracle ID: abd62af0-c17d-4f62-af15-9ea83037b990
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(119),
    oracle_id: "abd62af0-c17d-4f62-af15-9ea83037b990",
    scryfall_id: "9655bbe4-062f-4278-ad05-a326a64c5b69",
    faces: &[FaceDef {
        name: "Privileged Position",
        mana_cost: baylee_core::mana!("{2}{G/W}{G/W}{G/W}"),
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
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
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
