//! Stronghold Gambit — {1}{R} — Sorcery
//! Oracle: Each player chooses a card in their hand. Then each player reveals their chosen card. The owner of each creature card revealed this way with the lowest mana value puts it onto the battlefield.
//! Set: NEM #100 — Nemesis | Scryfall ID: 0d18050e-9aad-471e-a6ae-66e5fa2bbb6f | Oracle ID: 797bb53d-b111-41fe-95d7-64cb0f833b22
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STRONGHOLD_GAMBIT,
    oracle_id = "797bb53d-b111-41fe-95d7-64cb0f833b22",
    scryfall_id = "0d18050e-9aad-471e-a6ae-66e5fa2bbb6f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Stronghold Gambit",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
