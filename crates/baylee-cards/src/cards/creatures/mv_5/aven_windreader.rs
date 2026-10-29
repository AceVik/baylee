//! Aven Windreader — {3}{U}{U} — Creature — Bird Soldier Wizard
//! Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.)
//! Oracle: {1}{U}: Target player reveals the top card of their library.
//! Set: 10E #69 — Tenth Edition | Scryfall ID: 3a67d7ae-3117-433f-ad82-44a4f015fecf | Oracle ID: 8406b724-a824-4905-83e0-a6c02d7f72e3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVEN_WINDREADER,
    oracle_id = "8406b724-a824-4905-83e0-a6c02d7f72e3",
    scryfall_id = "3a67d7ae-3117-433f-ad82-44a4f015fecf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Aven Windreader",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::BIRD,
            subtypes::creature::SOLDIER,
            subtypes::creature::WIZARD
        ],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
