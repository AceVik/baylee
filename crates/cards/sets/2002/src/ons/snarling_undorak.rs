//! Snarling Undorak — {2}{G}{G} — Creature — Beast
//! Oracle: {2}{G}: Target Beast creature gets +1/+1 until end of turn.
//! Oracle: Morph {1}{G}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #283 — Onslaught | Scryfall ID: 05788d63-6210-44f2-9ae4-e55e9507a3a9 | Oracle ID: a932e265-604a-4aed-b8a9-6234a3db16a5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SNARLING_UNDORAK,
    oracle_id = "a932e265-604a-4aed-b8a9-6234a3db16a5",
    scryfall_id = "05788d63-6210-44f2-9ae4-e55e9507a3a9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Snarling Undorak",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
