//! Feeding Frenzy — {2}{B} — Instant
//! Oracle: Target creature gets -X/-X until end of turn, where X is the number of Zombies on the battlefield.
//! Set: ONS #147 — Onslaught | Scryfall ID: a6d74c30-ebca-4684-ad84-3ca19193ad88 | Oracle ID: d14fa263-a6ae-4ab4-b391-2f1ff356fa54
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FEEDING_FRENZY,
    oracle_id = "d14fa263-a6ae-4ab4-b391-2f1ff356fa54",
    scryfall_id = "a6d74c30-ebca-4684-ad84-3ca19193ad88",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Feeding Frenzy",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
