//! Dega Sanctuary — {2}{W} — Enchantment
//! Oracle: At the beginning of your upkeep, if you control a black or red permanent, you gain 2 life. If you control a black permanent and a red permanent, you gain 4 life instead.
//! Set: APC #5 — Apocalypse | Scryfall ID: b7ddfdb5-3981-4954-af5f-2459d22ec575 | Oracle ID: 75c626fb-9dfc-4a94-b59f-f0e45c7b2f56
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEGA_SANCTUARY,
    oracle_id = "75c626fb-9dfc-4a94-b59f-f0e45c7b2f56",
    scryfall_id = "b7ddfdb5-3981-4954-af5f-2459d22ec575",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Dega Sanctuary",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
