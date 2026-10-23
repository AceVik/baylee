//! Pallimud — {2}{R} — Creature — Beast
//! Oracle: As this creature enters, choose an opponent.
//! Oracle: Pallimud's power is equal to the number of tapped lands the chosen player controls.
//! Set: TMP #195 — Tempest | Scryfall ID: 61adc314-cfb2-4fdd-925c-cc1dc4692992 | Oracle ID: cad0f865-62e0-47ba-84ae-8e97cb52bbd1

// PARTIAL — Counting tapped lands controlled by the single chosen opponent is not supported; only the 0/3 body is played.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PALLIMUD,
    oracle_id = "cad0f865-62e0-47ba-84ae-8e97cb52bbd1",
    scryfall_id = "61adc314-cfb2-4fdd-925c-cc1dc4692992",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Counting tapped lands controlled by the single chosen opponent is not supported; only the 0/3 body is played."
    ),
    faces = &[face!(
        name = "Pallimud",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(0),
        toughness = Some(3),
    ),],
    // NOT SUPPORTED: Counting tapped lands controlled by the single chosen opponent is not supported; only the 0/3 body is played.
);
