//! Riptide Biologist — {1}{U} — Creature — Human Wizard
//! Oracle: Protection from Beasts
//! Oracle: Morph {2}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #106 — Onslaught | Scryfall ID: 4d399b71-c365-492c-976e-2c79d97d08bc | Oracle ID: fe378f26-5c80-4548-8127-10724ac0c452
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RIPTIDE_BIOLOGIST,
    oracle_id = "fe378f26-5c80-4548-8127-10724ac0c452",
    scryfall_id = "4d399b71-c365-492c-976e-2c79d97d08bc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Riptide Biologist",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
