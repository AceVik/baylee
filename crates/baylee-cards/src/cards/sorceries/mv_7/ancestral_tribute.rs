//! Ancestral Tribute — {5}{W}{W} — Sorcery
//! Oracle: You gain 2 life for each card in your graveyard.
//! Oracle: Flashback {9}{W}{W}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #2 — Odyssey | Scryfall ID: 3f28b2af-7891-49eb-a07f-5552b5fe3d15 | Oracle ID: 380ea3b6-8eb4-466a-8680-ce5cfed9b8c4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANCESTRAL_TRIBUTE,
    oracle_id = "380ea3b6-8eb4-466a-8680-ce5cfed9b8c4",
    scryfall_id = "3f28b2af-7891-49eb-a07f-5552b5fe3d15",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Ancestral Tribute",
        mana_cost = mana!("{5}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
