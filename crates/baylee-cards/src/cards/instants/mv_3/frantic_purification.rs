//! Frantic Purification — {2}{W} — Instant
//! Oracle: Destroy target enchantment.
//! Oracle: Madness {W} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)
//! Set: TOR #6 — Torment | Scryfall ID: 04f156d1-2655-42aa-a785-d8eb15cb5422 | Oracle ID: b4025136-3146-499a-8c17-ed9de548ab8d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FRANTIC_PURIFICATION,
    oracle_id = "b4025136-3146-499a-8c17-ed9de548ab8d",
    scryfall_id = "04f156d1-2655-42aa-a785-d8eb15cb5422",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Frantic Purification",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
