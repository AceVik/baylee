//! Phantom Flock — {3}{W}{W} — Creature — Bird Soldier Spirit
//! Oracle: Flying
//! Oracle: This creature enters with three +1/+1 counters on it.
//! Oracle: If damage would be dealt to this creature, prevent that damage. Remove a +1/+1 counter from this creature.
//! Set: DMR #20 — Dominaria Remastered | Scryfall ID: 314602ae-fb42-4afe-9fab-fa6976db39ad | Oracle ID: bcded242-6e54-416f-bdc8-093211c50e3f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTOM_FLOCK,
    oracle_id = "bcded242-6e54-416f-bdc8-093211c50e3f",
    scryfall_id = "314602ae-fb42-4afe-9fab-fa6976db39ad",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Phantom Flock",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::BIRD,
            subtypes::creature::SOLDIER,
            subtypes::creature::SPIRIT
        ],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
