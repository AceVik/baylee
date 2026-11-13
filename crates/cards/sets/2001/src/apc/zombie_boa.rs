//! Zombie Boa — {4}{B} — Creature — Zombie Snake
//! Oracle: {1}{B}: Choose a color. Whenever this creature becomes blocked by a creature of that color this turn, destroy that creature. Activate only as a sorcery.
//! Set: APC #54 — Apocalypse | Scryfall ID: 1fb8c277-3154-47c9-835f-327cac297a5e | Oracle ID: c4949d62-c2f9-4425-a77e-444824955ecc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZOMBIE_BOA,
    oracle_id = "c4949d62-c2f9-4425-a77e-444824955ecc",
    scryfall_id = "1fb8c277-3154-47c9-835f-327cac297a5e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Zombie Boa",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::SNAKE],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
