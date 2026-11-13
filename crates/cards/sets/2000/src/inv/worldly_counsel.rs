//! Worldly Counsel — {1}{U} — Instant
//! Oracle: Domain — Look at the top X cards of your library, where X is the number of basic land types among lands you control. Put one of those cards into your hand and the rest on the bottom of your library in any order.
//! Set: CON #39 — Conflux | Scryfall ID: b454cbdc-b2ff-4b6a-acaf-b7cf52359f0b | Oracle ID: 079b1f07-6910-491b-83ba-5ec41260f32d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORLDLY_COUNSEL,
    oracle_id = "079b1f07-6910-491b-83ba-5ec41260f32d",
    scryfall_id = "b454cbdc-b2ff-4b6a-acaf-b7cf52359f0b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Worldly Counsel",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
