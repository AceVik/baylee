//! Viashino Bey — {2}{R}{R} — Creature — Lizard
//! Oracle: If this creature attacks, all creatures you control attack if able.
//! Set: ULG #93 — Urza's Legacy | Scryfall ID: 26cc07c6-60c7-4abe-8197-7544887ec64d | Oracle ID: d9c9b933-74f4-4898-bbb8-2da76b86ccda
// PARTIAL: playable as a 4/3 Lizard; requiring all creatures you control to attack if able when Viashino Bey is declared as an attacker is not in Modifier.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIASHINO_BEY,
    oracle_id = "d9c9b933-74f4-4898-bbb8-2da76b86ccda",
    scryfall_id = "26cc07c6-60c7-4abe-8197-7544887ec64d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "playable as a 4/3 Lizard; requiring all creatures you control to attack if able when Viashino Bey is declared as an attacker is not in Modifier",
    ),
    faces = &[face!(
        name = "Viashino Bey",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(4),
        toughness = Some(3),
    ),],
);
