//! Clearwater Pathway // Murkwater Pathway — (no cost) — Land // Land
//! Oracle: Clearwater Pathway: {T}: Add {U}. // Murkwater Pathway: {T}: Add {B}.
//! Set: ZNR #260 — Zendikar Rising | Scryfall ID: b4b99ebb-0d54-4fe5-a495-979aaa564aa8 | Oracle ID: 144119bc-7fd1-45c5-9e29-f742e7c255ac
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
        color: ManaColor::Black,
        amount: 1,
    }],
    target: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: true,
    zone: ActivationZone::Battlefield,
}];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(21),
    oracle_id: "144119bc-7fd1-45c5-9e29-f742e7c255ac",
    scryfall_id: "b4b99ebb-0d54-4fe5-a495-979aaa564aa8",
    faces: &[
        FaceDef {
            name: "Clearwater Pathway",
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
            miracle: None,
            delve: false,
            convoke: false,
            cost_reduction: None,
            disturb: false,
        },
        FaceDef {
            name: "Murkwater Pathway",
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
            miracle: None,
            delve: false,
            convoke: false,
            cost_reduction: None,
            disturb: false,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddMana {
            color: ManaColor::Blue,
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
