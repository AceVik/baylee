//! Pitchstone Wall — {2}{R} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Whenever you discard a card, you may sacrifice this creature. If you do, return the discarded card from your graveyard to your hand.
//! Set: TOR #110 — Torment | Scryfall ID: 9bfea263-6ff0-41be-9755-03bcdebfb255 | Oracle ID: da6a9a2e-7bed-46d7-b7dc-18d4a220b81e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PITCHSTONE_WALL,
    oracle_id = "da6a9a2e-7bed-46d7-b7dc-18d4a220b81e",
    scryfall_id = "9bfea263-6ff0-41be-9755-03bcdebfb255",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Pitchstone Wall",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(2),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
