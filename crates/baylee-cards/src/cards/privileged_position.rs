//! Privileged Position — {2}{G/W}{G/W}{G/W} — Enchantment
//! Oracle: ({G/W} can be paid with either {G} or {W}.)
//! Oracle: Other permanents you control have hexproof. (They can't be the targets of spells or abilities your opponents control.)
//! Set: 2X2 #263 — Double Masters 2022 | Scryfall ID: 9655bbe4-062f-4278-ad05-a326a64c5b69 | Oracle ID: abd62af0-c17d-4f62-af15-9ea83037b990
// IMPLEMENTED — hexproof grant to your other permanents (layer 6).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, Filter, KeywordSet, Layer, Modifier,
    PartnerKind, StaticAbility,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static OTHER_YOURS: Filter = Filter::And(&[Filter::ControlledByYou, Filter::Another]);

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
        abilities: &[],
        castable_from_hand: true,
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Green]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Static(StaticAbility {
        layer: Layer::Ability,
        filter: OTHER_YOURS,
        modifier: Modifier::AddKeyword(KeywordSet::HEXPROOF),
        cross_zone: false,
    })],
};

#[cfg(test)]
mod tests {}
