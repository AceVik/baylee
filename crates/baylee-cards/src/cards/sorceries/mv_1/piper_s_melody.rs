//! Piper's Melody — {G} — Sorcery
//! Oracle: Shuffle any number of target creature cards from your graveyard into your library.
//! Set: ODY #261 — Odyssey | Scryfall ID: 642ea1f9-49f0-4e0d-8122-c3a305c149e9 | Oracle ID: 1ed80dd0-0980-4830-aad2-5b78ca373098
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PIPER_S_MELODY,
    oracle_id = "1ed80dd0-0980-4830-aad2-5b78ca373098",
    scryfall_id = "642ea1f9-49f0-4e0d-8122-c3a305c149e9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Piper's Melody",
        mana_cost = mana!("{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
