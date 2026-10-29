//! Spitfire Handler — {1}{R} — Creature — Goblin
//! Oracle: This creature can't block creatures with power greater than this creature's power.
//! Oracle: {R}: This creature gets +1/+0 until end of turn.
//! Set: ONS #236 — Onslaught | Scryfall ID: efe72820-952f-4c53-9ee7-ea7ea54fc848 | Oracle ID: 01990628-12b4-49c9-af3d-03120891efc4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPITFIRE_HANDLER,
    oracle_id = "01990628-12b4-49c9-af3d-03120891efc4",
    scryfall_id = "efe72820-952f-4c53-9ee7-ea7ea54fc848",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Spitfire Handler",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
