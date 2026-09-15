//! Malakir Rebirth // Malakir Mire — {B} — Instant // Land
//! Oracle: Choose target creature. You lose 2 life. Until end of turn, that creature gains "When this creature dies, return it to the battlefield tapped under its owner's control."
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: ZNR #111 — Zendikar Rising | Scryfall ID: 609d3ecf-f88d-4268-a8d3-4bf2bcf5df60 | Oracle ID: a731e87b-8d99-4b64-8ee3-8e540d652366
//! Face: Malakir Rebirth — {B} — Instant
//! Face: Malakir Mire —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = 737,
    oracle_id = "a731e87b-8d99-4b64-8ee3-8e540d652366",
    scryfall_id = "609d3ecf-f88d-4268-a8d3-4bf2bcf5df60",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Malakir Rebirth",
            mana_cost = mana!("{B}"),
            types = TypeSet::INSTANT,
        ),
        face!(name = "Malakir Mire", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
