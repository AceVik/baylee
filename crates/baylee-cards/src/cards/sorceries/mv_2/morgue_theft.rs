//! Morgue Theft — {1}{B} — Sorcery
//! Oracle: Return target creature card from your graveyard to your hand.
//! Oracle: Flashback {4}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #151 — Odyssey | Scryfall ID: 937465ca-4cf7-4412-86eb-264efb0fdddd | Oracle ID: 0b8a508b-20c0-4a87-a73a-08af72c39a0a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MORGUE_THEFT,
    oracle_id = "0b8a508b-20c0-4a87-a73a-08af72c39a0a",
    scryfall_id = "937465ca-4cf7-4412-86eb-264efb0fdddd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Morgue Theft",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
