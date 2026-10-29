//! Unstable Hulk — {1}{R}{R} — Creature — Goblin Mutant
//! Oracle: Morph {3}{R}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, it gets +6/+6 and gains trample until end of turn. You skip your next turn.
//! Set: LGN #115 — Legions | Scryfall ID: 889cfde2-42fa-4278-ae4e-7e4dd993cda8 | Oracle ID: 8d01c85c-5f83-4514-b81c-4165051d5722
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UNSTABLE_HULK,
    oracle_id = "8d01c85c-5f83-4514-b81c-4165051d5722",
    scryfall_id = "889cfde2-42fa-4278-ae4e-7e4dd993cda8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Unstable Hulk",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::MUTANT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
