//! Time Walk — {1}{U} — Sorcery
//! Oracle: Take an extra turn after this one.
//! Set: VMA #2 — Vintage Masters | Scryfall ID: 70901356-3266-4bd9-aacc-f06c27271de5 | Oracle ID: d0209d3f-3f7e-4fd5-bce5-10bce6f29c86
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TIME_WALK,
    oracle_id = "d0209d3f-3f7e-4fd5-bce5-10bce6f29c86",
    scryfall_id = "70901356-3266-4bd9-aacc-f06c27271de5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Time Walk",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
