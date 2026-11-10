//! Crypt Cobra — {3}{B} — Creature — Snake
//! Oracle: Whenever this creature attacks and isn't blocked, defending player gets a poison counter. (A player with ten or more poison counters loses the game.)
//! Set: MIR #114 — Mirage | Scryfall ID: 4f7bcd36-13e2-4ac7-a449-246cecb3fc0f | Oracle ID: 456bbc0e-a648-481f-b37e-9996029556bd
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRYPT_COBRA,
    oracle_id = "456bbc0e-a648-481f-b37e-9996029556bd",
    scryfall_id = "4f7bcd36-13e2-4ac7-a449-246cecb3fc0f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crypt Cobra",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::Poison {
            who: PlayerRel::DefendingPlayer,
            amount: Amount::Fixed(1),
        }],
    )],
);
