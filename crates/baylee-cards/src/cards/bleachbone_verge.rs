//! Bleachbone Verge — (no cost) — Land
//! Oracle: {T}: Add {B}.
//! Oracle: {T}: Add {W}. Activate only if you control a Plains or a Swamp.
//! Set: DFT #250 — Aetherdrift | Scryfall ID: 52dcdabd-a186-45fe-9fee-6c0f1afeaf16 | Oracle ID: 2b8144a0-08d2-4c28-9fd7-5d90f90105e4
// PARTIAL — both mana abilities work; the Plains/Swamp activation
// restriction needs activation conditions (own milestone, same as
// Mox Opal's metalcraft).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(12),
    oracle_id: "2b8144a0-08d2-4c28-9fd7-5d90f90105e4",
    scryfall_id: "52dcdabd-a186-45fe-9fee-6c0f1afeaf16",
    faces: &[FaceDef {
        name: "Bleachbone Verge",
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
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial(
        "Plains/Swamp activation restriction (activation conditions, own milestone)",
    ),
    abilities: &[
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::Black,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
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
    ],
};

#[cfg(test)]
mod tests {}
