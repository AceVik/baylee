//! Death or Glory — {4}{W} — Sorcery
//! Oracle: Separate all creature cards in your graveyard into two piles. Exile the pile of an opponent's choice and return the other to the battlefield.
//! Set: INV #13 — Invasion | Scryfall ID: 81f967c9-b38d-489d-96cc-44a6b1804e10 | Oracle ID: b2ae0968-28ed-4ebb-a4e8-69ff6e6a6f1d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEATH_OR_GLORY,
    oracle_id = "b2ae0968-28ed-4ebb-a4e8-69ff6e6a6f1d",
    scryfall_id = "81f967c9-b38d-489d-96cc-44a6b1804e10",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Death or Glory",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
