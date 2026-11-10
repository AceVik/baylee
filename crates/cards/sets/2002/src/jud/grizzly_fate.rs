//! Grizzly Fate — {3}{G}{G} — Sorcery
//! Oracle: Create two 2/2 green Bear creature tokens.
//! Oracle: Threshold — Create four 2/2 green Bear creature tokens instead if there are seven or more cards in your graveyard.
//! Oracle: Flashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: VMA #215 — Vintage Masters | Scryfall ID: 004e1457-d951-4029-8bb6-17a290793e79 | Oracle ID: f3073e4f-ca7e-4fde-88a4-bd7b6ebfbc8c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRIZZLY_FATE,
    oracle_id = "f3073e4f-ca7e-4fde-88a4-bd7b6ebfbc8c",
    scryfall_id = "004e1457-d951-4029-8bb6-17a290793e79",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Grizzly Fate",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
