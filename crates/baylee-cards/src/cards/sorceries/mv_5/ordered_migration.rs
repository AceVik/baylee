//! Ordered Migration — {3}{W}{U} — Sorcery
//! Oracle: Domain — Create a 1/1 blue Bird creature token with flying for each basic land type among lands you control.
//! Set: INV #258 — Invasion | Scryfall ID: 04d83a07-6054-45f1-bdf9-07f2006238d2 | Oracle ID: ee0a59ba-8d34-43d5-b9a5-42015588e3d5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ORDERED_MIGRATION,
    oracle_id = "ee0a59ba-8d34-43d5-b9a5-42015588e3d5",
    scryfall_id = "04d83a07-6054-45f1-bdf9-07f2006238d2",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Ordered Migration",
        mana_cost = mana!("{3}{W}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
