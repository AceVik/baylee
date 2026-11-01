//! Weaver of Lies — {5}{U}{U} — Creature — Beast
//! Oracle: Morph {4}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, turn any number of target creatures with morph abilities other than this creature face down.
//! Set: LGN #57 — Legions | Scryfall ID: 12172d0e-0c73-4482-9f83-2c23ace9b7a0 | Oracle ID: bb2fca84-fc19-4c04-b146-1059aa975535
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WEAVER_OF_LIES,
    oracle_id = "bb2fca84-fc19-4c04-b146-1059aa975535",
    scryfall_id = "12172d0e-0c73-4482-9f83-2c23ace9b7a0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Weaver of Lies",
        mana_cost = mana!("{5}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
