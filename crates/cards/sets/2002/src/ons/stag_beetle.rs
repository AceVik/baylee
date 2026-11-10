//! Stag Beetle — {3}{G}{G} — Creature — Insect
//! Oracle: This creature enters with X +1/+1 counters on it, where X is the number of other creatures on the battlefield.
//! Set: ONS #285 — Onslaught | Scryfall ID: 72cc64b9-f5b9-42d3-9921-564c4c9f2c77 | Oracle ID: 75b2da25-e551-4add-a252-6d4789366b5f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STAG_BEETLE,
    oracle_id = "75b2da25-e551-4add-a252-6d4789366b5f",
    scryfall_id = "72cc64b9-f5b9-42d3-9921-564c4c9f2c77",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Stag Beetle",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
