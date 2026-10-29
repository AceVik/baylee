//! Voice of the Woods — {3}{G}{G} — Creature — Elf
//! Oracle: Tap five untapped Elves you control: Create a 7/7 green Elemental creature token with trample. (It can deal excess combat damage to the player or planeswalker it's attacking.)
//! Set: KHC #78 — Kaldheim Commander | Scryfall ID: d7a6315c-aa6f-4be1-a002-d7824bfad622 | Oracle ID: 12da8ba3-ca60-44c8-8627-1b4af2a2c4a3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VOICE_OF_THE_WOODS,
    oracle_id = "12da8ba3-ca60-44c8-8627-1b4af2a2c4a3",
    scryfall_id = "d7a6315c-aa6f-4be1-a002-d7824bfad622",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Voice of the Woods",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
