//! Aura Extraction — {1}{W} — Instant
//! Oracle: Put target enchantment on top of its owner's library.
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: ONS #5 — Onslaught | Scryfall ID: 55d16883-5e98-4dd2-92dd-0ba92f1099cb | Oracle ID: 84100106-1f97-4859-aadd-a35796ee000f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AURA_EXTRACTION,
    oracle_id = "84100106-1f97-4859-aadd-a35796ee000f",
    scryfall_id = "55d16883-5e98-4dd2-92dd-0ba92f1099cb",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Aura Extraction",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
