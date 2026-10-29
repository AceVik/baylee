//! Walking Desecration — {2}{B} — Creature — Zombie
//! Oracle: {B}, {T}: Creatures of the creature type of your choice attack this turn if able.
//! Set: ONS #180 — Onslaught | Scryfall ID: c39f3e91-571a-4990-b1e8-db2a5bac34af | Oracle ID: dab6de91-29c1-4bbc-987f-b0e613999d7f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALKING_DESECRATION,
    oracle_id = "dab6de91-29c1-4bbc-987f-b0e613999d7f",
    scryfall_id = "c39f3e91-571a-4990-b1e8-db2a5bac34af",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Walking Desecration",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
