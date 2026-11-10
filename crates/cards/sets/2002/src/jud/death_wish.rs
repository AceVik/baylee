//! Death Wish — {1}{B}{B} — Sorcery
//! Oracle: You may put a card you own from outside the game into your hand. You lose half your life, rounded up. Exile Death Wish.
//! Set: JUD #64 — Judgment | Scryfall ID: 7bf134c9-a50d-4eff-a5a8-7cfe6a010080 | Oracle ID: ff0ca1a4-da67-407c-a5c0-998a35eb7d8d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEATH_WISH,
    oracle_id = "ff0ca1a4-da67-407c-a5c0-998a35eb7d8d",
    scryfall_id = "7bf134c9-a50d-4eff-a5a8-7cfe6a010080",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Death Wish",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
