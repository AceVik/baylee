//! Windfall — {2}{U} — Sorcery
//! Oracle: Each player discards their hand, then draws cards equal to the greatest number of cards a player discarded this way.
//! Set: OTC #123 — Outlaws of Thunder Junction Commander | Scryfall ID: 9ce7113b-08f7-4584-b65f-a7b5caa90c2f | Oracle ID: 08becc07-28bc-4a2f-a6b0-28a2998d2f50
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINDFALL,
    oracle_id = "08becc07-28bc-4a2f-a6b0-28a2998d2f50",
    scryfall_id = "9ce7113b-08f7-4584-b65f-a7b5caa90c2f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Windfall",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
