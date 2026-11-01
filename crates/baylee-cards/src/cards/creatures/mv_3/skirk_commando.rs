//! Skirk Commando — {1}{R}{R} — Creature — Goblin
//! Oracle: Whenever this creature deals combat damage to a player, you may have it deal 2 damage to target creature that player controls.
//! Oracle: Morph {2}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: A25 #150 — Masters 25 | Scryfall ID: 259a6b6d-368a-4ee0-b0ed-b52301bfd78f | Oracle ID: c92f1d40-0441-4ede-9363-78821ff15f88
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKIRK_COMMANDO,
    oracle_id = "c92f1d40-0441-4ede-9363-78821ff15f88",
    scryfall_id = "259a6b6d-368a-4ee0-b0ed-b52301bfd78f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Skirk Commando",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
