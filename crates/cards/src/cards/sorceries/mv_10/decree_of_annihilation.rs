//! Decree of Annihilation — {8}{R}{R} — Sorcery
//! Oracle: Exile all artifacts, creatures, and lands from the battlefield, all cards from all graveyards, and all cards from all hands.
//! Oracle: Cycling {5}{R}{R} ({5}{R}{R}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, destroy all lands.
//! Set: SCG #85 — Scourge | Scryfall ID: 73744717-518c-478e-9da9-201c49124f37 | Oracle ID: 6679b462-3c90-418c-8727-acd1a61d988b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DECREE_OF_ANNIHILATION,
    oracle_id = "6679b462-3c90-418c-8727-acd1a61d988b",
    scryfall_id = "73744717-518c-478e-9da9-201c49124f37",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Decree of Annihilation",
        mana_cost = mana!("{8}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
