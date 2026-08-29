//! Taiga — (no cost) — Land — MOUNTAIN FOREST
//! Oracle: ({T}: Add {R} or {G}.)
//! Set: VMA #317 — Vintage Masters | Scryfall ID: 0c2c39fc-b564-4ab5-833c-ff029760b7a7 | Oracle ID: 22e3cf1d-3559-4ce1-954c-8dc815342979
// IMPLEMENTED — two-color mana choice.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static COLORS: &[ManaColor] = &[ManaColor::Red, ManaColor::Green];
static SUBS: &[baylee_core::ids::SubtypeId] = &[land::MOUNTAIN, land::FOREST];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(165),
    oracle_id: "22e3cf1d-3559-4ce1-954c-8dc815342979",
    scryfall_id: "0c2c39fc-b564-4ab5-833c-ff029760b7a7",
    faces: &[FaceDef {
        name: "Taiga",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: SUBS,
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Green]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: COLORS,
            amount: Amount::Fixed(1),
            combination: false,
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
};

#[cfg(test)]
mod tests {}
