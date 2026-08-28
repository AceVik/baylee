//! Zagoth Triome — (no cost) — Land — SWAMP FOREST ISLAND
//! Oracle: ({T}: Add {B}, {G}, or {U}.)
//! Zagoth Triome enters the battlefield tapped.
//! Cycling {2}
//! Set: IKO #259 — Ikoria: Lair of Behemoths | Scryfall ID: cc520518-2063-4b57-a0d4-10cf62a7175e | Oracle ID: fdd46004-eaba-4024-8687-39b23dc6a58c
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
    index: CardIndex::new(193),
    oracle_id: "fdd46004-eaba-4024-8687-39b23dc6a58c",
    scryfall_id: "cc520518-2063-4b57-a0d4-10cf62a7175e",
    faces: &[FaceDef {
        name: "Zagoth Triome",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::land::SWAMP,
            subtypes::land::FOREST,
            subtypes::land::ISLAND,
        ],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[EnterModifier::Tapped],
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
