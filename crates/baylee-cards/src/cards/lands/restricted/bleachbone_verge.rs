//! Bleachbone Verge — (no cost) — Land
//! Oracle: {T}: Add {B}.
//! Oracle: {T}: Add {W}. Activate only if you control a Plains or a Swamp.
//! Set: DFT #250 — Aetherdrift | Scryfall ID: 52dcdabd-a186-45fe-9fee-6c0f1afeaf16 | Oracle ID: 2b8144a0-08d2-4c28-9fd7-5d90f90105e4
// IMPLEMENTED — {B} always; {W} only with a Plains or Swamp under your
// control (ActivationCondition::ControlCount).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static PLAINS_OR_SWAMP: Filter = Filter::Or(&[
    Filter::HasSubtype(land::PLAINS),
    Filter::HasSubtype(land::SWAMP),
]);

card! {
    index: 12,
    oracle_id: "2b8144a0-08d2-4c28-9fd7-5d90f90105e4",
    scryfall_id: "52dcdabd-a186-45fe-9fee-6c0f1afeaf16",
    faces: &[face! {
        name: "Bleachbone Verge",
        types: TypeSet::LAND,
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        AbilityDef::ActivatedConditional {
            cost: Cost::TAP,
            effects: &[Effect::mana(ManaColor::White, 1)],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
            condition: ActivationCondition::ControlCount(&PLAINS_OR_SWAMP, 1),
        },
    ],
}
