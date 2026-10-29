//! Chatter of the Squirrel — {G} — Sorcery
//! Oracle: Create a 1/1 green Squirrel creature token.
//! Oracle: Flashback {1}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: 2XM #157 — Double Masters | Scryfall ID: 5f7e1991-9ffa-4a57-b8eb-ebe542a47f09 | Oracle ID: b0aaa4b1-5188-43a6-997d-7a9b2ad452bc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHATTER_OF_THE_SQUIRREL,
    oracle_id = "b0aaa4b1-5188-43a6-997d-7a9b2ad452bc",
    scryfall_id = "5f7e1991-9ffa-4a57-b8eb-ebe542a47f09",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Chatter of the Squirrel",
        mana_cost = mana!("{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
