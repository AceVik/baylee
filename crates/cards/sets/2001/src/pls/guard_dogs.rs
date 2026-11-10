//! Guard Dogs — {3}{W} — Creature — Dog
//! Oracle: {2}{W}, {T}: Choose a permanent you control. Prevent all combat damage target creature would deal this turn if it shares a color with that permanent.
//! Set: PLS #5 — Planeshift | Scryfall ID: ba32eee7-10ba-4f0b-8a87-c3ecfa22ae41 | Oracle ID: 490af644-2e85-49c7-af63-d5f0a9babff8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GUARD_DOGS,
    oracle_id = "490af644-2e85-49c7-af63-d5f0a9babff8",
    scryfall_id = "ba32eee7-10ba-4f0b-8a87-c3ecfa22ae41",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Guard Dogs",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DOG],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
