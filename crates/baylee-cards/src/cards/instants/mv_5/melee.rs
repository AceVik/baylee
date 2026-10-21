//! Melee — {4}{R} — Instant
//! Oracle: Cast this spell only during combat on your turn before blockers are declared.
//! Oracle: You choose which creatures block this combat and how those creatures block.
//! Oracle: Whenever a creature attacks and isn't blocked this combat, untap it and remove it from combat.
//! Set: ICE #200 — Ice Age | Scryfall ID: b13a064d-bff4-4a48-a158-1b61951b0ac3 | Oracle ID: 93fc1ff8-6ef0-42a2-b090-ba95b6f3c98e
// NOT SUPPORTED: choosing which creatures block this combat and how they block, the pre-blockers combat cast restriction, and untapping and removing unblocked attackers from combat are not supported; resolves with no effect.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MELEE,
    oracle_id = "93fc1ff8-6ef0-42a2-b090-ba95b6f3c98e",
    scryfall_id = "b13a064d-bff4-4a48-a158-1b61951b0ac3",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "choosing which creatures block this combat and how they block, the pre-blockers combat cast restriction, and untapping and removing unblocked attackers from combat are not supported; resolves with no effect",
    ),
    faces = &[face!(
        name = "Melee",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[])],
);
