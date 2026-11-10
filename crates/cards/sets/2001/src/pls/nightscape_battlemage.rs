//! Nightscape Battlemage — {2}{B} — Creature — Zombie Wizard
//! Oracle: Kicker {2}{U} and/or {2}{R} (You may pay an additional {2}{U} and/or {2}{R} as you cast this spell.)
//! Oracle: When this creature enters, if it was kicked with its {2}{U} kicker, return up to two target nonblack creatures to their owners' hands.
//! Oracle: When this creature enters, if it was kicked with its {2}{R} kicker, destroy target land.
//! Set: PLS #47 — Planeshift | Scryfall ID: d5389643-4cc0-4a17-bc2d-7f9b76d30f9f | Oracle ID: e96e68b4-cb32-4b75-a885-e1781f4b74ab
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NIGHTSCAPE_BATTLEMAGE,
    oracle_id = "e96e68b4-cb32-4b75-a885-e1781f4b74ab",
    scryfall_id = "d5389643-4cc0-4a17-bc2d-7f9b76d30f9f",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Nightscape Battlemage",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
