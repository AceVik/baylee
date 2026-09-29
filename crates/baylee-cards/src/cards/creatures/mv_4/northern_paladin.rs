//! Northern Paladin — {2}{W}{W} — Creature — Human Knight
//! Oracle: {W}{W}, {T}: Destroy target black permanent.
//! Set: 7ED #28 — Seventh Edition | Scryfall ID: 5ed1fb50-b7c3-43e9-a0ef-a33135a12300 | Oracle ID: f5975294-508a-453e-893a-2fbea2487d17
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NORTHERN_PALADIN,
    oracle_id = "f5975294-508a-453e-893a-2fbea2487d17",
    scryfall_id = "5ed1fb50-b7c3-43e9-a0ef-a33135a12300",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Northern Paladin",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
