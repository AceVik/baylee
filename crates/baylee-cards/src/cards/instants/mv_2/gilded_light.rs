//! Gilded Light — {1}{W} — Instant
//! Oracle: You gain shroud until end of turn. (You can't be the target of spells or abilities.)
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: MH1 #12 — Modern Horizons | Scryfall ID: 8ff59aa2-5e97-4e6c-b568-470ac2371bd7 | Oracle ID: f7e7b2aa-9fd2-41dd-b11e-3a233530dc83
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GILDED_LIGHT,
    oracle_id = "f7e7b2aa-9fd2-41dd-b11e-3a233530dc83",
    scryfall_id = "8ff59aa2-5e97-4e6c-b568-470ac2371bd7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Gilded Light",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
