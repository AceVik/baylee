//! Coiling Oracle — {G}{U} — Creature — Snake Elf Druid
//! Oracle: When this creature enters, reveal the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, put that card into your hand.
//! Set: RVR #172 — Ravnica Remastered | Scryfall ID: 559ff1b1-018c-4e08-9531-8af20af47d05 | Oracle ID: 69fd4ddf-9ed8-4c56-bef3-9944daf05e4f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COILING_ORACLE,
    oracle_id = "69fd4ddf-9ed8-4c56-bef3-9944daf05e4f",
    scryfall_id = "559ff1b1-018c-4e08-9531-8af20af47d05",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Coiling Oracle",
        mana_cost = mana!("{G}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::SNAKE,
            subtypes::creature::ELF,
            subtypes::creature::DRUID
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
