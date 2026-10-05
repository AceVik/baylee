//! Giant Turtle — {1}{G}{G} — Creature — Turtle
//! Oracle: This creature can't attack if it attacked during your last turn.
//! Set: LEG #188 — Legends | Scryfall ID: 87e5fc19-3b10-476f-9a73-e8bf4b5fbec0 | Oracle ID: 9297c0a6-1a8e-4e6e-99d6-f0877b2ec46c
// PARTIAL — the attack restriction is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GIANT_TURTLE,
    oracle_id = "9297c0a6-1a8e-4e6e-99d6-f0877b2ec46c",
    scryfall_id = "87e5fc19-3b10-476f-9a73-e8bf4b5fbec0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Giant Turtle",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TURTLE],
        power = Some(2),
        toughness = Some(4),
    ),],
    coverage = Coverage::Partial(
        "the attack restriction is off the card: no `Modifier` forbids \
         attacking at all, and nothing keeps the previous turn's declared \
         attackers, so there is no condition to hang \"if it attacked during \
         your last turn\" on"
    ),
    // NOT SUPPORTED: "This creature can't attack if it attacked during your
    // last turn." — the engine has no attack prohibition to gate (Glacial
    // Chasm records the same hole) and no filter or condition reads a turn
    // before this one (`Filter::AttackedThisTurn` is the current turn's
    // attackers and is cleared at the turn boundary); the nearest piece,
    // `Modifier::CantAttackUnlessDefenderControls`, restricts an attack by
    // what the defender controls, not by the attacker's own history.
    abilities = &[],
);
