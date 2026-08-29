//! Raugrin Triome — (no cost) — Land — ISLAND MOUNTAIN PLAINS
//! Oracle: ({T}: Add {U}, {R}, or {W}.)
//! Raugrin Triome enters the battlefield tapped.
//! Cycling {2}
//! Set: IKO #251 — Ikoria: Lair of Behemoths | Scryfall ID: 02138fbb-3962-4348-8d31-faaefba0b8b2 | Oracle ID: c7fa1dda-9312-4ec8-82cd-a1ba7bc33497
// IMPLEMENTED — triome (3 land types → intrinsic mana, ETB tapped, cycling {2}).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, CostPart,
    Coverage, Effect, EnterModifier, FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(123),
    oracle_id: "c7fa1dda-9312-4ec8-82cd-a1ba7bc33497",
    scryfall_id: "02138fbb-3962-4348-8d31-faaefba0b8b2",
    faces: &[FaceDef {
        name: "Raugrin Triome",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::land::ISLAND,
            subtypes::land::MOUNTAIN,
            subtypes::land::PLAINS,
        ],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[EnterModifier::Tapped],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
        disturb: false,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost {
            mana: baylee_core::mana!("{2}"),
            parts: &[CostPart::DiscardSelf],
        },
        effects: &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Hand,
    }],
};

#[cfg(test)]
mod tests {}
