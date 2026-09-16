//! Green Sun's Zenith — {X}{G} — Sorcery
//! Oracle: Search your library for a green creature card with mana value X or less, put it onto the battlefield, then shuffle. Shuffle Green Sun's Zenith into its owner's library.
//! Set: 2X2 #150 — Double Masters 2022 | Scryfall ID: 70291c7b-a86f-4466-8502-c28765a89b2a | Oracle ID: 0d96b60b-a060-48ee-bb83-93f1c4a10669
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GREEN_SUN_S_ZENITH,
    oracle_id = "0d96b60b-a060-48ee-bb83-93f1c4a10669",
    scryfall_id = "70291c7b-a86f-4466-8502-c28765a89b2a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Green Sun's Zenith",
        mana_cost = mana!("{X}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
