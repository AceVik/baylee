//! Erode — {W} — Instant
//! Oracle: Destroy target creature or planeswalker. Its controller may search their library for a basic land card, put it onto the battlefield tapped, then shuffle.
//! Set: SOS #15 — Secrets of Strixhaven | Scryfall ID: 32e670da-7563-4f6a-a7db-4c126a440eb8 | Oracle ID: 2e467fab-e808-44d3-99bf-e3621baeb7cb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ERODE,
    oracle_id = "2e467fab-e808-44d3-99bf-e3621baeb7cb",
    scryfall_id = "32e670da-7563-4f6a-a7db-4c126a440eb8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Erode",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
