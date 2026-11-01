//! Goblin Goon — {3}{R} — Creature — Goblin Mutant
//! Oracle: This creature can't attack unless you control more creatures than defending player.
//! Oracle: This creature can't block unless you control more creatures than attacking player.
//! Set: JMP #326 — Jumpstart | Scryfall ID: 5b629eb0-0e84-4eaf-bbc3-ec85ae17a8a7 | Oracle ID: e91bcf01-113b-4532-9f78-2a155b17be4f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_GOON,
    oracle_id = "e91bcf01-113b-4532-9f78-2a155b17be4f",
    scryfall_id = "5b629eb0-0e84-4eaf-bbc3-ec85ae17a8a7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Goon",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::MUTANT],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
