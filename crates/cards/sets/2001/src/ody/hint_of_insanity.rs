//! Hint of Insanity — {2}{B} — Sorcery
//! Oracle: Target player reveals their hand. That player discards all nonland cards with the same name as another card in their hand.
//! Set: ODY #143 — Odyssey | Scryfall ID: d6abaca0-7ce1-4024-adf6-ef6cc0fbcb75 | Oracle ID: 4db59824-6bcb-4707-977b-0ff699d8662b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HINT_OF_INSANITY,
    oracle_id = "4db59824-6bcb-4707-977b-0ff699d8662b",
    scryfall_id = "d6abaca0-7ce1-4024-adf6-ef6cc0fbcb75",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Hint of Insanity",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
