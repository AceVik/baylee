//! Sunscape Apprentice — {W} — Creature — Human Wizard
//! Oracle: {G}, {T}: Target creature gets +1/+1 until end of turn.
//! Oracle: {U}, {T}: Put target creature you control on top of its owner's library.
//! Set: INV #41 — Invasion | Scryfall ID: a9d6bd19-77c9-4a1a-a2d5-6f9737693fea | Oracle ID: 4ab1411e-b255-4a6e-b81a-5de7b2d4ea12
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUNSCAPE_APPRENTICE,
    oracle_id = "4ab1411e-b255-4a6e-b81a-5de7b2d4ea12",
    scryfall_id = "a9d6bd19-77c9-4a1a-a2d5-6f9737693fea",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    faces = &[face!(
        name = "Sunscape Apprentice",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
