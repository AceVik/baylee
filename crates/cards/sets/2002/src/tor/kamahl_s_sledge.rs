//! Kamahl's Sledge — {5}{R}{R} — Sorcery
//! Oracle: Kamahl's Sledge deals 4 damage to target creature.
//! Oracle: Threshold — If there are seven or more cards in your graveyard, instead Kamahl's Sledge deals 4 damage to that creature and 4 damage to that creature's controller.
//! Set: TOR #102 — Torment | Scryfall ID: 38c55518-7bdf-4a42-ae30-cd6525557a59 | Oracle ID: defa9b92-4b54-4b00-852e-6c5ec4015643
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KAMAHL_S_SLEDGE,
    oracle_id = "defa9b92-4b54-4b00-852e-6c5ec4015643",
    scryfall_id = "38c55518-7bdf-4a42-ae30-cd6525557a59",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Kamahl's Sledge",
        mana_cost = mana!("{5}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
