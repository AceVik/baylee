//! Lichenthrope — {3}{G}{G} — Creature — Plant Fungus
//! Oracle: If damage would be dealt to this creature, put that many -1/-1 counters on it instead.
//! Oracle: At the beginning of your upkeep, remove a -1/-1 counter from this creature.
//! Set: VIS #112 — Visions | Scryfall ID: 76f0c356-a81d-41d4-a8b7-8c159146a8b8 | Oracle ID: ee7f698d-55c5-4a7a-803d-304febe6a758
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LICHENTHROPE,
    oracle_id = "ee7f698d-55c5-4a7a-803d-304febe6a758",
    scryfall_id = "76f0c356-a81d-41d4-a8b7-8c159146a8b8",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Lichenthrope",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PLANT, subtypes::creature::FUNGUS],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
