//! Battle Screech — {2}{W}{W} — Sorcery
//! Oracle: Create two 1/1 white Bird creature tokens with flying.
//! Oracle: Flashback—Tap three untapped white creatures you control. (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: CMM #17 — Commander Masters | Scryfall ID: 90301f8c-f7af-4179-8faf-9901931bba8d | Oracle ID: e73131cd-b454-405b-9539-9d777e232b9e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BATTLE_SCREECH,
    oracle_id = "e73131cd-b454-405b-9539-9d777e232b9e",
    scryfall_id = "90301f8c-f7af-4179-8faf-9901931bba8d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Battle Screech",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
