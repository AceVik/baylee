//! Sylvan Might — {1}{G} — Instant
//! Oracle: Target creature gets +2/+2 and gains trample until end of turn.
//! Oracle: Flashback {2}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: 2XM #181 — Double Masters | Scryfall ID: a4e1ecf4-bf72-4ae9-9993-9b9a6ca80da7 | Oracle ID: c8187c8d-23fa-436f-9efa-ea4cb80cdc08
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SYLVAN_MIGHT,
    oracle_id = "c8187c8d-23fa-436f-9efa-ea4cb80cdc08",
    scryfall_id = "a4e1ecf4-bf72-4ae9-9993-9b9a6ca80da7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Sylvan Might",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
