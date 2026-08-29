//! Wasteland — (no cost) — Land
//! Oracle: {T}: Add {C}. {T}, Sacrifice this land: Destroy target nonbasic land.
//! Set: C17 #264 — Commander 2017 | Scryfall ID: aaafb9bc-7cea-4624-a227-595544fa42b0 | Oracle ID: 09a70ae8-3859-4a09-901d-dce063fa3b5d
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

static NONBASIC_LAND: Filter = Filter::And(&[
    Filter::HasType(TypeSet::LAND),
    Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC)),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(188),
    oracle_id: "09a70ae8-3859-4a09-901d-dce063fa3b5d",
    scryfall_id: "aaafb9bc-7cea-4624-a227-595544fa42b0",
    faces: &[FaceDef {
        name: "Wasteland",
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
            cost: Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
            },
            effects: &[Effect::Destroy {
                target: TargetSpec::Object(&NONBASIC_LAND),
            }],
            target: Some(TargetSpec::Object(&NONBASIC_LAND)),
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
