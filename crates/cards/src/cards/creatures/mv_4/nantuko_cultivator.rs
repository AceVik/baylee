//! Nantuko Cultivator — {3}{G} — Creature — Insect Druid
//! Oracle: When this creature enters, you may discard any number of land cards. Put that many +1/+1 counters on this creature and draw that many cards.
//! Set: OTC #198 — Outlaws of Thunder Junction Commander | Scryfall ID: c6e02dbb-4cbb-469a-a377-c856625f956f | Oracle ID: b8dab16c-681c-4abe-b164-fcfb5dc66678
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NANTUKO_CULTIVATOR,
    oracle_id = "b8dab16c-681c-4abe-b164-fcfb5dc66678",
    scryfall_id = "c6e02dbb-4cbb-469a-a377-c856625f956f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Nantuko Cultivator",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT, subtypes::creature::DRUID],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
