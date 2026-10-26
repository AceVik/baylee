//! Scouting Trek — {1}{G} — Sorcery
//! Oracle: Search your library for any number of basic land cards, reveal those cards, then shuffle and put them on top.
//! Set: INV #210 — Invasion | Scryfall ID: 1b882e68-5c03-4ec6-9982-8c3b09847969 | Oracle ID: 929c231a-12d9-4f1a-9849-64a8b5a65f20
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCOUTING_TREK,
    oracle_id = "929c231a-12d9-4f1a-9849-64a8b5a65f20",
    scryfall_id = "1b882e68-5c03-4ec6-9982-8c3b09847969",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Scouting Trek",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
