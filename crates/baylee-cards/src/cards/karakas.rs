//! Karakas — (no cost) — Land
//! Oracle: {T}: Add {W}. {T}: Return target legendary creature to its owner\u{2019}s hand.
//! Set: EMA #240 — Eternal Masters | Scryfall ID: e52214e1-404a-405a-b08e-20e13c087338 | Oracle ID: 59119143-c0fa-49dd-adf0-e2fd3029c48b
// IMPLEMENTED.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, PlayerRel, TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static LEGENDARY_CREATURE: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(79),
    oracle_id: "59119143-c0fa-49dd-adf0-e2fd3029c48b",
    scryfall_id: "e52214e1-404a-405a-b08e-20e13c087338",
    faces: &[FaceDef {
        name: "Karakas",
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
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::White,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::ReturnToHand {
                target: TargetSpec::Object(&LEGENDARY_CREATURE),
            }],
            target: Some(TargetSpec::Object(&LEGENDARY_CREATURE)),
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
