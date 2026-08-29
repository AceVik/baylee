//! Plains — (no cost) — Basic Land — Plains
//! Oracle: ({T}: Add {W}.)
//! Set: TRK #317 — Star Trek | Scryfall ID: 8ab0f4c0-b331-4c57-b68f-2e24bb5ba06c | Oracle ID: bc71ebf6-2056-41f7-be35-b2e5c34afa99
// IMPLEMENTED — basic land mana ability.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(115),
    oracle_id: "bc71ebf6-2056-41f7-be35-b2e5c34afa99",
    scryfall_id: "8ab0f4c0-b331-4c57-b68f-2e24bb5ba06c",
    faces: &[FaceDef {
        name: "Plains",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[land::PLAINS],
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
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddMana {
            color: ManaColor::White,
            amount: 1,
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
};

#[cfg(test)]
mod tests {}
