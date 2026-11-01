//! Prison Barricade — {1}{W} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Kicker {1}{W} (You may pay an additional {1}{W} as you cast this spell.)
//! Oracle: If this creature was kicked, it enters with a +1/+1 counter on it and with "This creature can attack as though it didn't have defender."
//! Set: INV #25 — Invasion | Scryfall ID: 449c4800-8718-4593-a61e-03ad7f348c6d | Oracle ID: 8531b38a-8cc9-452b-8373-3febc733a95b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PRISON_BARRICADE,
    oracle_id = "8531b38a-8cc9-452b-8373-3febc733a95b",
    scryfall_id = "449c4800-8718-4593-a61e-03ad7f348c6d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Prison Barricade",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
