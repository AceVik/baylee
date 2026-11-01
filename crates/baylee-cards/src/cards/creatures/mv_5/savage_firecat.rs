//! Savage Firecat — {3}{R}{R} — Creature — Elemental Cat
//! Oracle: Trample
//! Oracle: This creature enters with seven +1/+1 counters on it.
//! Oracle: Whenever you tap a land for mana, remove a +1/+1 counter from this creature.
//! Set: ODY #218 — Odyssey | Scryfall ID: 52e14dbe-4d33-405c-ab84-2fc6b1e000b8 | Oracle ID: 084e11d9-dd1d-4e8e-afc1-9be4ee04a510
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAVAGE_FIRECAT,
    oracle_id = "084e11d9-dd1d-4e8e-afc1-9be4ee04a510",
    scryfall_id = "52e14dbe-4d33-405c-ab84-2fc6b1e000b8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Savage Firecat",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::CAT],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
