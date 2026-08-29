//! Hengegate Pathway // Mistgate Pathway — (no cost) — Land // Land
//! Oracle: Hengegate Pathway: {T}: Add {W}. // Mistgate Pathway: {T}: Add {U}.
//! Set: ZNR #261 — Zendikar Rising | Scryfall ID: 7ef37cb3-d803-47d7-8a01-9c803aa2eadc | Oracle ID: 461b3f2f-fcee-4160-abfa-061f8b6a784f
// IMPLEMENTED — MDFC land-face choice on play (CR 712.4a) + per-face
// mana abilities.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static BACK_MANA: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost::TAP,
    effects: &[Effect::AddMana {
        color: ManaColor::Blue,
        amount: 1,
    }],
    target: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: true,
    zone: ActivationZone::Battlefield,
}];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(69),
    oracle_id: "461b3f2f-fcee-4160-abfa-061f8b6a784f",
    scryfall_id: "7ef37cb3-d803-47d7-8a01-9c803aa2eadc",
    faces: &[
        FaceDef {
            name: "Hengegate Pathway",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::LAND,
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
        },
        FaceDef {
            name: "Mistgate Pathway",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::LAND,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[],
            power: None,
            toughness: None,
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
            enter_modifiers: &[],
            abilities: BACK_MANA,
            castable_from_hand: true,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
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
