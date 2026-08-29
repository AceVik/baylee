//! Mystic Gate — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! {1}, {T}: Add two mana in any combination of {White} and/or {Blue}.
//! Set: SHM #277 — Shadowmoor | Scryfall ID: 6f99714f-43bc-4048-b650-97dfef4c10fe | Oracle ID: e9f5feb2-2c1a-46ce-885a-4f378d7d10af
// IMPLEMENTED — filter land (colorless tap + {1},{T} for two combination mana).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(101),
    oracle_id: "e9f5feb2-2c1a-46ce-885a-4f378d7d10af",
    scryfall_id: "6f99714f-43bc-4048-b650-97dfef4c10fe",
    faces: &[FaceDef {
        name: "Mystic Gate",
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
                mana: baylee_core::mana!("{1}"),
                parts: &[baylee_cards_dsl::CostPart::TapSelf],
            },
            effects: &[Effect::AddManaChoice {
                colors: &[ManaColor::White, ManaColor::Blue],
                amount: Amount::Fixed(2),
                combination: true,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
