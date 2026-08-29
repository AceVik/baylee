//! Riptide Laboratory — (no cost) — Land
//! Oracle: {T}: Add {C}. {T}: Return target Wizard to its owner\u{2019}s hand.
//! Set: C14 #305 — Commander 2014 | Scryfall ID: 25a9cb87-e572-4885-8561-1d4b158ec7e4 | Oracle ID: 444d50dd-a44a-42db-bbf6-d0978e3bd8b7
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

static WIZARD: Filter = Filter::HasSubtype(creature::WIZARD);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(134),
    oracle_id: "444d50dd-a44a-42db-bbf6-d0978e3bd8b7",
    scryfall_id: "25a9cb87-e572-4885-8561-1d4b158ec7e4",
    faces: &[FaceDef {
        name: "Riptide Laboratory",
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
        adventure: false,
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
                color: ManaColor::Colorless,
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
                target: TargetSpec::Object(&WIZARD),
            }],
            target: Some(TargetSpec::Object(&WIZARD)),
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
