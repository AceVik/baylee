//! Volcanic Eruption — {X}{U}{U}{U} — Sorcery
//! Oracle: Destroy X target Mountains. Volcanic Eruption deals damage to each creature and each player equal to the number of Mountains put into a graveyard this way.
//! Set: 4ED #112 — Fourth Edition | Scryfall ID: 5828713a-edb3-4b11-b1f9-8f1bfc3c103f | Oracle ID: 4c15889e-3172-413b-b805-a2b7ad05f636
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VOLCANIC_ERUPTION,
    oracle_id = "4c15889e-3172-413b-b805-a2b7ad05f636",
    scryfall_id = "5828713a-edb3-4b11-b1f9-8f1bfc3c103f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Volcanic Eruption",
        mana_cost = mana!("{X}{U}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
