//! Bleachbone Verge — (no cost) — Land
//! Oracle: {T}: Add {B}.
//! Oracle: {T}: Add {W}. Activate only if you control a Plains or a Swamp.
//! Set: DFT #250 — Aetherdrift | Scryfall ID: 52dcdabd-a186-45fe-9fee-6c0f1afeaf16 | Oracle ID: 2b8144a0-08d2-4c28-9fd7-5d90f90105e4
// IMPLEMENTED — {B} always; {W} only with a Plains or Swamp under your
// control (ActivationCondition::ControlCount).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationCondition, ActivationTiming, ActivationZone, CardDef, CommanderRule,
    Cost, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static PLAINS_OR_SWAMP: Filter = Filter::Or(&[
    Filter::HasSubtype(land::PLAINS),
    Filter::HasSubtype(land::SWAMP),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(12),
    oracle_id: "2b8144a0-08d2-4c28-9fd7-5d90f90105e4",
    scryfall_id: "52dcdabd-a186-45fe-9fee-6c0f1afeaf16",
    faces: &[FaceDef {
        name: "Bleachbone Verge",
        types: TypeSet::LAND,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
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
        AbilityDef::ActivatedConditional {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::White,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
            condition: ActivationCondition::ControlCount(&PLAINS_OR_SWAMP, 1),
        },
    ],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
