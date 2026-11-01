//! Shimmering Mirage — {1}{U} — Instant
//! Oracle: Target land becomes the basic land type of your choice until end of turn.
//! Oracle: Draw a card.
//! Set: APC #30 — Apocalypse | Scryfall ID: 7263e20e-5473-42e9-90c3-3bcd848644ca | Oracle ID: d1720ddc-d6ad-4d78-8dd7-29539bbb73da
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHIMMERING_MIRAGE,
    oracle_id = "d1720ddc-d6ad-4d78-8dd7-29539bbb73da",
    scryfall_id = "7263e20e-5473-42e9-90c3-3bcd848644ca",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Shimmering Mirage",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
