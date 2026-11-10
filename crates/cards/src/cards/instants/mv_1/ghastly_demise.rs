//! Ghastly Demise — {B} — Instant
//! Oracle: Destroy target nonblack creature if its toughness is less than or equal to the number of cards in your graveyard.
//! Set: CMR #129 — Commander Legends | Scryfall ID: 7fcfe11a-fcc9-41e1-91ef-755bb4e22389 | Oracle ID: e02e7239-304b-4755-b913-bb65bc693e86
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GHASTLY_DEMISE,
    oracle_id = "e02e7239-304b-4755-b913-bb65bc693e86",
    scryfall_id = "7fcfe11a-fcc9-41e1-91ef-755bb4e22389",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Ghastly Demise",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
