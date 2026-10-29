//! Insist — {G} — Sorcery
//! Oracle: The next creature spell you cast this turn can't be countered.
//! Oracle: Draw a card.
//! Set: TOR #127 — Torment | Scryfall ID: 48e1f72d-ef5e-4cea-a0ee-645570a36104 | Oracle ID: 518684d9-467a-4936-a3d9-17d23db4622c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INSIST,
    oracle_id = "518684d9-467a-4936-a3d9-17d23db4622c",
    scryfall_id = "48e1f72d-ef5e-4cea-a0ee-645570a36104",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Insist",
        mana_cost = mana!("{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
