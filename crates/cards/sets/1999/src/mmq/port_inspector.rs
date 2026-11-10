//! Port Inspector — {1}{U} — Creature — Human
//! Oracle: Whenever this creature becomes blocked, you may look at defending player's hand.
//! Set: MMQ #90 — Mercadian Masques | Scryfall ID: ef25d969-68d9-4580-bb29-f72bd5646a3d | Oracle ID: 25450b06-3065-4b4f-975e-586d045e5815
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PORT_INSPECTOR,
    oracle_id = "25450b06-3065-4b4f-975e-586d045e5815",
    scryfall_id = "ef25d969-68d9-4580-bb29-f72bd5646a3d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Port Inspector",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
