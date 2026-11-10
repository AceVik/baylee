//! Swamp Mosquito — {1}{B} — Creature — Insect
//! Oracle: Flying
//! Oracle: Whenever this creature attacks and isn't blocked, defending player gets a poison counter. (A player with ten or more poison counters loses the game.)
//! Set: TSB #49 — Time Spiral Timeshifted | Scryfall ID: 450800c5-28e0-4eeb-9164-db9fdebc595f | Oracle ID: 4bb4844c-2678-45c7-8f5e-9cf185fd484b
// IMPLEMENTED — flying; attacking and unblocked gives the defending player a poison counter.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SWAMP_MOSQUITO,
    oracle_id = "4bb4844c-2678-45c7-8f5e-9cf185fd484b",
    scryfall_id = "450800c5-28e0-4eeb-9164-db9fdebc595f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Swamp Mosquito",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        keywords = KeywordSet::FLYING,
        power = Some(0),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::Poison {
            who: PlayerRel::DefendingPlayer,
            amount: Amount::Fixed(1),
        }]
    )],
);
