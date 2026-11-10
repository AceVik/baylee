//! Suq'Ata Assassin — {1}{B}{B} — Creature — Human Assassin
//! Oracle: Fear (This creature can't be blocked except by artifact creatures and/or black creatures.)
//! Oracle: Whenever this creature attacks and isn't blocked, defending player gets a poison counter. (A player with ten or more poison counters loses the game.)
//! Set: VIS #69 — Visions | Scryfall ID: 1b7178c6-f989-437d-83e3-04b9817f2c54 | Oracle ID: db43b866-13e0-4336-a580-b64fe1ee5954
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUQ_ATA_ASSASSIN,
    oracle_id = "db43b866-13e0-4336-a580-b64fe1ee5954",
    scryfall_id = "1b7178c6-f989-437d-83e3-04b9817f2c54",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FEAR,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Suq'Ata Assassin",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ASSASSIN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::Poison {
            who: PlayerRel::DefendingPlayer,
            amount: Amount::Fixed(1),
        }],
    )],
);
