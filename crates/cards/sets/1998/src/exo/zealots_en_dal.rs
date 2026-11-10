//! Zealots en-Dal — {3}{W} — Creature — Human Soldier
//! Oracle: At the beginning of your upkeep, if all nonland permanents you control are white, you gain 1 life.
//! Set: EXO #26 — Exodus | Scryfall ID: 6a9fb486-1d6a-478e-af6e-fd8539dc646d | Oracle ID: 3fa67c92-6328-4d0a-95e3-656d447ab20e
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NONWHITE_NONLAND_YOU_CONTROL: Filter = Filter::And(&[
    Filter::LacksType(TypeSet::LAND),
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::White]))),
    Filter::ControlledByYou,
]);

card!(
    index = index::ZEALOTS_EN_DAL,
    oracle_id = "3fa67c92-6328-4d0a-95e3-656d447ab20e",
    scryfall_id = "6a9fb486-1d6a-478e-af6e-fd8539dc646d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Zealots en-Dal",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::gain_life(1)],
        condition = Some(Condition::ControlCountAtMost(
            &NONWHITE_NONLAND_YOU_CONTROL,
            0,
        )),
    )],
);
