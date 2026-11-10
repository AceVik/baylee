//! Dwarven Recruiter — {2}{R} — Creature — Dwarf
//! Oracle: When this creature enters, search your library for any number of Dwarf cards, reveal them, then shuffle and put those cards on top in any order.
//! Set: ODY #186 — Odyssey | Scryfall ID: 6a15d274-85b4-4f3c-b502-f6dfe7db4d37 | Oracle ID: d047a819-5196-4a16-8d67-a4edaf6fdef0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DWARVEN_RECRUITER,
    oracle_id = "d047a819-5196-4a16-8d67-a4edaf6fdef0",
    scryfall_id = "6a15d274-85b4-4f3c-b502-f6dfe7db4d37",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dwarven Recruiter",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
