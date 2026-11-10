//! Sylvan Messenger — {3}{G} — Creature — Elf
//! Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.)
//! Oracle: When this creature enters, reveal the top four cards of your library. Put all Elf cards revealed this way into your hand and the rest on the bottom of your library in any order.
//! Set: KHC #75 — Kaldheim Commander | Scryfall ID: 085f1b89-857b-4e2b-b796-0b95e53322bd | Oracle ID: ba310102-8272-4d64-a00b-9e7346cb5f46
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SYLVAN_MESSENGER,
    oracle_id = "ba310102-8272-4d64-a00b-9e7346cb5f46",
    scryfall_id = "085f1b89-857b-4e2b-b796-0b95e53322bd",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Sylvan Messenger",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
