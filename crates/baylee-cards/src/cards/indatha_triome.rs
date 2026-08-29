//! Indatha Triome — (no cost) — Land — PLAINS SWAMP FOREST
//! Oracle: ({T}: Add {W}, {B}, or {G}.)
//! Indatha Triome enters the battlefield tapped.
//! Cycling {2}
//! Set: IKO #248 — Ikoria: Lair of Behemoths | Scryfall ID: 2b74bb81-fb9a-40e5-a941-e517430b52f5 | Oracle ID: ec2b3779-55f7-4169-aa66-6312fb52721f
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
    index: CardIndex::new(72),
    oracle_id: "ec2b3779-55f7-4169-aa66-6312fb52721f",
    scryfall_id: "2b74bb81-fb9a-40e5-a941-e517430b52f5",
    faces: &[FaceDef {
        name: "Indatha Triome",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::land::PLAINS,
            subtypes::land::SWAMP,
            subtypes::land::FOREST,
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
        adventure: false,
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
