//! Life Burst — {1}{W} — Instant
//! Oracle: Target player gains 4 life, then gains 4 life for each card named Life Burst in each graveyard.
//! Set: ODY #30 — Odyssey | Scryfall ID: 7f8eee50-efd2-45fd-b815-051167ef4541 | Oracle ID: dc10544f-dd30-4d4a-b6e6-bbed7d1ed4a7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFE_BURST,
    oracle_id = "dc10544f-dd30-4d4a-b6e6-bbed7d1ed4a7",
    scryfall_id = "7f8eee50-efd2-45fd-b815-051167ef4541",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Life Burst",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
