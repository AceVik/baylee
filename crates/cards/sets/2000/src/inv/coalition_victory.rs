//! Coalition Victory — {3}{W}{U}{B}{R}{G} — Sorcery
//! Oracle: You win the game if you control a land of each basic land type and a creature of each color.
//! Set: TSB #91 — Time Spiral Timeshifted | Scryfall ID: 446824f7-dc42-42da-8bd5-6c37f3358d65 | Oracle ID: e9e25800-9ee7-40c9-b22d-611c7281c125
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COALITION_VICTORY,
    oracle_id = "e9e25800-9ee7-40c9-b22d-611c7281c125",
    scryfall_id = "446824f7-dc42-42da-8bd5-6c37f3358d65",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    faces = &[face!(
        name = "Coalition Victory",
        mana_cost = mana!("{3}{W}{U}{B}{R}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
