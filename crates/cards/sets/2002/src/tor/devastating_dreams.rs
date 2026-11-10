//! Devastating Dreams — {R}{R} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard X cards at random.
//! Oracle: Each player sacrifices X lands of their choice. Devastating Dreams deals X damage to each creature.
//! Set: TOR #95 — Torment | Scryfall ID: 9fffeed0-a5ea-47ac-a7a4-0cc3bb1d408a | Oracle ID: 18994c4c-9b5d-4b58-a5c3-646fbc92f9c7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEVASTATING_DREAMS,
    oracle_id = "18994c4c-9b5d-4b58-a5c3-646fbc92f9c7",
    scryfall_id = "9fffeed0-a5ea-47ac-a7a4-0cc3bb1d408a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Devastating Dreams",
        mana_cost = mana!("{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
