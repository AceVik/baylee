//! Mirror Wall — {3}{U} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: {W}: This creature can attack this turn as though it didn't have defender.
//! Set: JUD #47 — Judgment | Scryfall ID: d0e52b73-ebdf-4339-8780-84327a59ca57 | Oracle ID: c4ab446a-97e4-46a5-89a8-180a0bf3447c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MIRROR_WALL,
    oracle_id = "c4ab446a-97e4-46a5-89a8-180a0bf3447c",
    scryfall_id = "d0e52b73-ebdf-4339-8780-84327a59ca57",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Mirror Wall",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
