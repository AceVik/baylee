//! Scalding Salamander — {2}{R} — Creature — Salamander
//! Oracle: Whenever this creature attacks, you may have it deal 1 damage to each creature without flying defending player controls.
//! Set: EXO #100 — Exodus | Scryfall ID: 5a0e9433-88d7-4bfc-99a0-ff47807fd594 | Oracle ID: fcca3c9d-a2b6-4f29-b43c-2f061074a332
// PARTIAL: Attack trigger may deal 1 damage to each creature without flying opponents control; multiplayer defending-player distinction is not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OPPONENT_NONFLYING_CREATURE: Filter = Filter::And(&[
    Filter::OPPONENT_CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);

card!(
    index = index::SCALDING_SALAMANDER,
    oracle_id = "fcca3c9d-a2b6-4f29-b43c-2f061074a332",
    scryfall_id = "5a0e9433-88d7-4bfc-99a0-ff47807fd594",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Attack trigger may deal 1 damage to each creature without flying opponents control; multiplayer defending-player distinction is not supported.",
    ),
    faces = &[face!(
        name = "Scalding Salamander",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SALAMANDER],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::MayDo {
            effects: &[Effect::damage_each(1, &OPPONENT_NONFLYING_CREATURE)],
        }]
    ),],
);
